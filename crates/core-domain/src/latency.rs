use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Cross-cutting latency telemetry for the DIE observation loop.
///
/// Fields are updated atomically from the hot path (adapter, candle generator,
/// broadcast) and read by the `/api/system/status` handler with no lock
/// contention.
#[derive(Debug)]
pub struct LatencyTracker {
    /// Difference between local receipt time and the trade's `timestamp_ms`
    /// (most recent measurement, microseconds).
    pub ingest_skew_ms: AtomicU64,
    /// End-to-end latency from raw WS frame arrival to completed-snapshot
    /// broadcast (most recent measurement, milliseconds).
    pub observation_loop_latency_ms: AtomicU64,
    /// Round-trip time of the most recent WebSocket ping/pong exchange
    /// (milliseconds). Zero when no measurement is available.
    pub system_heartbeat_latency_ms: AtomicU64,
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self {
            ingest_skew_ms: AtomicU64::new(0),
            observation_loop_latency_ms: AtomicU64::new(0),
            system_heartbeat_latency_ms: AtomicU64::new(0),
        }
    }
}

impl LatencyTracker {
    /// Record a trade's ingest skew: `(receive_time_ms - trade_timestamp_ms)`.
    /// Positive means the trade arrived late relative to its exchange timestamp.
    pub fn record_ingest_skew(&self, receive_time_ms: u64, trade_timestamp_ms: u64) {
        let skew = receive_time_ms.saturating_sub(trade_timestamp_ms);
        self.ingest_skew_ms.store(skew, Ordering::Relaxed);
    }

    /// Record the end-to-end latency for a completed-snapshot broadcast.
    pub fn record_observation_latency(&self, latency_ms: u64) {
        self.observation_loop_latency_ms
            .store(latency_ms, Ordering::Relaxed);
    }

    /// Record a heartbeat round-trip time in milliseconds.
    pub fn record_heartbeat(&self, rtt_ms: u64) {
        self.system_heartbeat_latency_ms
            .store(rtt_ms, Ordering::Relaxed);
    }

    /// Current epoch millisecond timestamp.
    pub fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Snapshot all three values for API responses.
    pub fn snapshot(&self) -> LatencySnapshot {
        LatencySnapshot {
            ingest_skew_ms: self.ingest_skew_ms.load(Ordering::Relaxed),
            observation_loop_latency_ms: self.observation_loop_latency_ms.load(Ordering::Relaxed),
            system_heartbeat_latency_ms: self.system_heartbeat_latency_ms.load(Ordering::Relaxed),
        }
    }
}

/// Read-only snapshot of the latency tracker for API serialization.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct LatencySnapshot {
    pub ingest_skew_ms: u64,
    pub observation_loop_latency_ms: u64,
    pub system_heartbeat_latency_ms: u64,
}

pub type SharedLatencyTracker = Arc<LatencyTracker>;
