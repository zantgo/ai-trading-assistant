use config_models::TriggerMode;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone)]
pub struct TriggerMessage {
    pub reason: String,
    pub trigger_type_detail: String,
    pub policy_id: Option<String>,
}

pub struct TriggerEngine {
    pub candle_counters: Arc<RwLock<HashMap<String, u32>>>,
    pub last_interval_trigger: Arc<RwLock<HashMap<String, u64>>>,
    pub pending_events: Arc<RwLock<Vec<String>>>,
    pub trigger_tx: mpsc::Sender<TriggerMessage>,
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
            last_interval_trigger: Arc::new(RwLock::new(HashMap::new())),
            pending_events: Arc::new(RwLock::new(Vec::new())),
            trigger_tx,
            active_mode,
        }
    }

    pub fn should_evaluate(
        &self,
        policy_id: &str,
        trigger_mode: &TriggerMode,
        current_time_secs: u64,
        timeframe_label: &str,
    ) -> bool {
        match trigger_mode {
            TriggerMode::Interval { seconds } => {
                let last_trigger = {
                    let map = self.last_interval_trigger.try_read();
                    map.map(|m| m.get(policy_id).copied().unwrap_or(0))
                        .unwrap_or(0)
                };
                let elapsed = current_time_secs.saturating_sub(last_trigger);
                elapsed >= *seconds
            }
            TriggerMode::CandleClose { timeframe: _, count } => {
                let counters = self.candle_counters.try_read();
                counters
                    .map(|c| c.get(timeframe_label).copied().unwrap_or(0) >= *count)
                    .unwrap_or(false)
            }
            TriggerMode::EventDriven { events } => {
                let pending = self.pending_events.try_read();
                pending
                    .map(|p| events.iter().any(|ev| p.contains(ev)))
                    .unwrap_or(false)
            }
        }
    }

    pub fn mark_interval_triggered(&self, policy_id: &str, current_time_secs: u64) {
        if let Ok(mut map) = self.last_interval_trigger.try_write() {
            map.insert(policy_id.to_string(), current_time_secs);
        }
    }

    pub fn record_event(&self, event_name: &str) {
        if let Ok(mut pending) = self.pending_events.try_write() {
            if !pending.contains(&event_name.to_string()) {
                pending.push(event_name.to_string());
            }
        }
    }

    pub fn clear_events(&self) {
        if let Ok(mut pending) = self.pending_events.try_write() {
            pending.clear();
        }
    }

    pub fn clear_specific_events(&self, events_to_clear: &[String]) {
        if let Ok(mut pending) = self.pending_events.try_write() {
            pending.retain(|e| !events_to_clear.contains(e));
        }
    }

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
                policy_id: None,
            };
            let _ = self.trigger_tx.send(msg).await;
            return true;
        }
        false
    }

    pub fn set_mode(&mut self, mode: TriggerMode) {
        self.active_mode = mode;
    }

    pub async fn take_candle_count(&self, timeframe: &str) -> u32 {
        let mut counters = self.candle_counters.write().await;
        counters.remove(timeframe).unwrap_or(0)
    }
}

pub fn trigger_interval_seconds(mode: &TriggerMode) -> u64 {
    match mode {
        TriggerMode::Interval { seconds } => *seconds,
        _ => 0,
    }
}

pub fn is_event_driven(mode: &TriggerMode) -> bool {
    matches!(mode, TriggerMode::EventDriven { .. })
}
