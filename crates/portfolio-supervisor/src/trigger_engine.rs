use config_models::TriggerMode;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Minimal trigger message sent when trigger conditions are met.
#[derive(Debug, Clone)]
pub struct TriggerMessage {
    pub reason: String,
    pub trigger_type_detail: String,
}

/// Manages trigger dispatch for interval, candle-close, and event-driven modes.
pub struct TriggerEngine {
    /// Per-timeframe candle counters.
    pub candle_counters: Arc<RwLock<HashMap<String, u32>>>,
    /// Trigger dispatch channel sender.
    pub trigger_tx: mpsc::Sender<TriggerMessage>,
    /// Active trigger configuration.
    pub active_mode: TriggerMode,
}

impl TriggerEngine {
    pub fn new(
        candle_counters: Arc<RwLock<HashMap<String, u32>>>,
        trigger_tx: mpsc::Sender<TriggerMessage>,
        active_mode: TriggerMode,
    ) -> Self {
        Self {
            candle_counters,
            trigger_tx,
            active_mode,
        }
    }

    /// Called when a completed candle is available. Increments the timeframe
    /// counter and dispatches a trigger if the candle-close threshold is met.
    pub async fn on_candle_completed(&self, timeframe: &str, target_count: u32) -> bool {
        let mut counters = self.candle_counters.write().await;
        let count = counters.entry(timeframe.to_string()).or_insert(0);
        *count += 1;
        if *count >= target_count {
            *count = 0;
            drop(counters);
            let msg = TriggerMessage {
                reason: format!("candle_close:{}:{}", timeframe, target_count),
                trigger_type_detail: format!("candle:{timeframe}:{target_count}"),
            };
            let _ = self.trigger_tx.send(msg).await;
            return true;
        }
        false
    }

    /// Updates the active trigger mode (e.g., when config changes).
    pub fn set_mode(&mut self, mode: TriggerMode) {
        self.active_mode = mode;
    }

    /// Returns the number of completed candles for a timeframe, resetting it.
    pub async fn take_candle_count(&self, timeframe: &str) -> u32 {
        let mut counters = self.candle_counters.write().await;
        counters.remove(timeframe).unwrap_or(0)
    }
}

/// Resolve the default trigger interval from the trigger mode.
/// For Interval mode, returns the configured seconds. For other modes,
/// returns a sensible fallback.
pub fn trigger_interval_seconds(mode: &TriggerMode) -> u64 {
    match mode {
        TriggerMode::Interval { seconds } => *seconds,
        _ => 900,
    }
}
