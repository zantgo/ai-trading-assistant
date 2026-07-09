use crate::automation::TriggerMessage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Manages trigger dispatch for interval, candle-close, and event-driven modes.
pub struct TriggerEngine {
    /// Per-timeframe candle counters.
    pub candle_counters: Arc<RwLock<HashMap<String, u32>>>,
    /// Trigger dispatch channel sender.
    pub trigger_tx: mpsc::Sender<TriggerMessage>,
}

impl TriggerEngine {
    pub fn new(
        candle_counters: Arc<RwLock<HashMap<String, u32>>>,
        trigger_tx: mpsc::Sender<TriggerMessage>,
    ) -> Self {
        Self {
            candle_counters,
            trigger_tx,
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

    /// Returns the number of completed candles for a timeframe, resetting it.
    pub async fn take_candle_count(&self, timeframe: &str) -> u32 {
        let mut counters = self.candle_counters.write().await;
        counters.remove(timeframe).unwrap_or(0)
    }
}
