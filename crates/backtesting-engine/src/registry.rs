//! BTE runtime registry — the single-run lock and the backfill progress map.
//!
//! Lives on `AppState` so every handler shares one source of truth:
//! - `run_lock` serializes synchronous backtest runs (concurrent run = 409).
//! - `backfills` holds live progress for the progress endpoint; the DB
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

#[derive(Default)]
pub struct BacktestRegistry {
    /// Serializes synchronous backtest runs (one at a time).
    pub run_lock: Mutex<()>,
    /// Live backfill progress keyed by job id.
    pub backfills: RwLock<HashMap<i64, TrackedBackfill>>,
    /// Next backfill job id (mirrors the `backfill_jobs` table ids).
    pub next_backfill_id: AtomicI64,
}

impl BacktestRegistry {
    pub fn new() -> Self {
        Self {
            run_lock: Mutex::new(()),
            backfills: RwLock::new(HashMap::new()),
            next_backfill_id: AtomicI64::new(1),
        }
    }

    pub fn alloc_backfill_id(&self) -> i64 {
        self.next_backfill_id.fetch_add(1, Ordering::SeqCst)
    }

    /// True when the instance already has a running backfill job.
    pub async fn instance_has_active_backfill(&self, instance_id: &str) -> bool {
        let map = self.backfills.read().await;
        map.values().any(|t| {
            t.progress.try_lock().map(|p| {
                p.instance_id == instance_id && p.status == BackfillStatus::Running
            })
            .unwrap_or(false)
        })
    }

    /// Cancels a running job (if any) — used for cleanup.
    pub async fn cancel_backfill(&self, job_id: i64) {
        if let Some(tracked) = self.backfills.read().await.get(&job_id) {
            tracked.cancel.store(true, Ordering::SeqCst);
        }
    }
}
