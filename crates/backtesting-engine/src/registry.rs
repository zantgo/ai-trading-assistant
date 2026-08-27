//! BTE runtime registry — the single-run lock, the run progress map, and
//! the backfill progress map.
//!
//! Lives on `AppState` so every handler shares one source of truth:
//! - `runs` tracks live backtest runs (phase progress + cancel flag); at
//!   most one running run exists — concurrent runs return 409.
//! - `backfills` holds live progress for the backfill endpoint; the DB
//!   `backfill_jobs` table is the persisted shadow (resumable across
//!   restarts).

use crate::backfill::{BackfillProgress, BackfillStatus};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// One tracked backfill: live progress + cancel token.
pub struct TrackedBackfill {
    pub progress: Arc<Mutex<BackfillProgress>>,
    pub cancel: Arc<AtomicBool>,
}

/// One tracked backtest run (v8.2 async runs).
pub struct TrackedRun {
    /// Phase: `fetching` | `warming` | `replaying` | `analyzing`.
    pub phase: Mutex<String>,
    /// 0..=100 progress within the phase.
    pub pct: Mutex<f32>,
    /// Human-readable message for the progress surface.
    pub message: Mutex<String>,
    /// `running` | `completed` | `failed` | `cancelled`.
    pub status: Mutex<String>,
    /// The persisted `backtest_runs` row id once the run completes (the
    /// progress endpoint hands it to the launcher so it can load the
    /// result via `GET /api/backtest/:id`).
    pub backtest_id: Mutex<Option<i64>>,
    /// Set by the cancel endpoint; checked by the runner between chunks.
    pub cancel: Arc<AtomicBool>,
}

impl Default for TrackedRun {
    fn default() -> Self {
        Self::new()
    }
}

impl TrackedRun {
    pub fn new() -> Self {
        Self {
            phase: Mutex::new("fetching".to_string()),
            pct: Mutex::new(0.0),
            message: Mutex::new(String::new()),
            status: Mutex::new("running".to_string()),
            backtest_id: Mutex::new(None),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Default)]
pub struct BacktestRegistry {
    /// v8.2: live backtest runs keyed by run id. A run whose status is
    /// `running` blocks new runs (409 busy).
    pub runs: RwLock<HashMap<i64, Arc<TrackedRun>>>,
    /// Live backfill progress keyed by job id.
    pub backfills: RwLock<HashMap<i64, TrackedBackfill>>,
    /// Next run id (assigned by the run handler; mirrors the
    /// `backtest_runs` table ids).
    pub next_run_id: AtomicI64,
    /// Next backfill job id (mirrors the `backfill_jobs` table ids).
    pub next_backfill_id: AtomicI64,
}

impl BacktestRegistry {
    pub fn new() -> Self {
        Self {
            runs: RwLock::new(HashMap::new()),
            backfills: RwLock::new(HashMap::new()),
            next_run_id: AtomicI64::new(1),
            next_backfill_id: AtomicI64::new(1),
        }
    }

    pub fn alloc_run_id(&self) -> i64 {
        self.next_run_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn alloc_backfill_id(&self) -> i64 {
        self.next_backfill_id.fetch_add(1, Ordering::SeqCst)
    }

    /// True when any tracked run is still `running`.
    pub async fn has_running_run(&self) -> bool {
        let map = self.runs.read().await;
        for run in map.values() {
            if *run.status.lock().await == "running" {
                return true;
            }
        }
        false
    }

    /// Atomically check and reserve a run slot. Returns `Some(id)` when no
    /// running run exists (and the slot is now occupied), `None` when busy.
    /// Holds the write lock across check+insert so concurrent POSTs cannot
    /// both see `false` and insert.
    pub async fn try_alloc_run(&self, run: Arc<TrackedRun>) -> Option<i64> {
        let mut map = self.runs.write().await;
        for existing in map.values() {
            if *existing.status.lock().await == "running" {
                return None;
            }
        }
        let id = self.alloc_run_id();
        map.insert(id, run);
        Some(id)
    }

    /// True when the instance already has a running backfill job.
    pub async fn instance_has_active_backfill(&self, instance_id: &str) -> bool {
        let map = self.backfills.read().await;
        // Use try_lock but fail-closed: if the progress mutex is contended we
        // assume the job is still active rather than allowing a duplicate.
        map.values().any(|t| {
            t.progress
                .try_lock()
                .map(|p| p.instance_id == instance_id && p.status == BackfillStatus::Running)
                .unwrap_or(true)
        })
    }

    /// Atomically check and reserve a backfill slot for `instance_id`.
    /// Returns `true` when the slot was acquired, `false` when busy.
    /// Holds the write lock across check+insert so concurrent POSTs cannot
    /// both see `false` and insert. Fail-closed on contended progress lock.
    pub async fn try_alloc_backfill(
        &self,
        job_id: i64,
        instance_id: &str,
        tracked: TrackedBackfill,
    ) -> bool {
        let mut map = self.backfills.write().await;
        for v in map.values() {
            match v.progress.try_lock() {
                Ok(p) => {
                    if p.instance_id == instance_id && p.status == BackfillStatus::Running {
                        return false;
                    }
                }
                Err(_) => return false, // contended → assume busy (fail closed)
            }
        }
        map.insert(job_id, tracked);
        true
    }

    /// Cancels a running job (if any) — used for cleanup.
    pub async fn cancel_backfill(&self, job_id: i64) {
        if let Some(tracked) = self.backfills.read().await.get(&job_id) {
            tracked.cancel.store(true, Ordering::SeqCst);
        }
    }

    /// Cancels a running backtest (best-effort).
    pub async fn cancel_run(&self, run_id: i64) {
        if let Some(tracked) = self.runs.read().await.get(&run_id) {
            tracked.cancel.store(true, Ordering::SeqCst);
        }
    }
}
