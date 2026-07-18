use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{sleep, Duration};
use core_domain::normalized::{NormalizedEvent, SymbolMapper, ExchangeAdapter, ConnectionStatus};

use crate::exchange_status_tracker::ExchangeStatusTracker;

pub struct MarketDataOrchestrator {
    mapper: Arc<SymbolMapper>,
    adapters: Vec<Arc<dyn ExchangeAdapter>>,
    event_tx: Sender<NormalizedEvent>,
    event_rx: Option<Receiver<NormalizedEvent>>,
    exchange_status: Option<Arc<ExchangeStatusTracker>>,
}

impl MarketDataOrchestrator {
    pub fn new(mapper: Arc<SymbolMapper>) -> Self {
        let (tx, rx) = channel::<NormalizedEvent>(10000);
        Self {
            mapper,
            adapters: Vec::new(),
            event_tx: tx,
            event_rx: Some(rx),
            exchange_status: None,
        }
    }

    /// Attach an `ExchangeStatusTracker` so connection-state transitions are
    /// surfaced to the `/api/exchange-status` endpoint.
    pub fn set_exchange_status_tracker(&mut self, tracker: Arc<ExchangeStatusTracker>) {
        self.exchange_status = Some(tracker);
    }

    pub fn register_adapter(&mut self, adapter: Box<dyn ExchangeAdapter>) {
        self.adapters.push(Arc::from(adapter));
    }

    pub async fn run(&mut self, _symbols: Vec<String>) -> Receiver<NormalizedEvent> {
        let rx = self.event_rx.take().expect("Orchestrator already running");
        let exchange_status = self.exchange_status.clone();

        for adapter in &self.adapters {
            let adapter_clone = Arc::clone(adapter);
            let tx_clone = self.event_tx.clone();
            let mapper_clone = Arc::clone(&self.mapper);
            let es = exchange_status.clone();
            tokio::spawn(async move {
                let mut retry_cooldown_secs = 1u64;
                let mut consecutive_failures = 0u32;
                let mut last_failure_ts = std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_secs(301))
                    .unwrap_or_else(std::time::Instant::now);
                loop {
                    let exchange_label = adapter_clone.exchange();
                    let exchange_label_str = exchange_label.to_string();

                    let active_symbols = mapper_clone.get_normalized_for_exchange(exchange_label).await;

                    if active_symbols.is_empty() {
                        let _ = tx_clone.send(NormalizedEvent::Status {
                            exchange: exchange_label,
                            status: ConnectionStatus::Disconnected,
                            message: "Dormant (no configured symbols)".to_string(),
                        }).await;

                        if let Some(ref es) = es {
                            es.set_disconnected(&exchange_label_str).await;
                        }

                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }

                    // Register and notify connecting
                    if let Some(ref es) = es {
                        es.set_connecting(&exchange_label_str).await;
                        es.update_active_pairs(&exchange_label_str, active_symbols.len() as u32).await;
                    }

                    let _ = tx_clone.send(NormalizedEvent::Status {
                        exchange: exchange_label,
                        status: ConnectionStatus::Connecting,
                        message: format!("Supervisor: Starting {} adapter handshake", exchange_label),
                    }).await;

                    match adapter_clone.start(active_symbols.clone(), tx_clone.clone(), mapper_clone.clone()).await {
                        Ok(()) => {
                            consecutive_failures = 0;
                            if let Some(ref es) = es {
                                es.set_connected(&exchange_label_str).await;
                            }
                            eprintln!("⚠️  Orchestrator: {} adapter terminated cleanly.", exchange_label);
                        }
                        Err(e) => {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_failure_ts) > Duration::from_secs(300) {
                                consecutive_failures = 0;
                            }
                            last_failure_ts = now;
                            consecutive_failures += 1;
                            if let Some(ref es) = es {
                                es.set_disconnected(&exchange_label_str).await;
                                es.increment_reconnect(&exchange_label_str).await;
                            }
                            eprintln!("❌ Orchestrator: {} adapter crashed: {}.", exchange_label, e);
                        }
                    }

                    if consecutive_failures >= 5 {
                        let _ = tx_clone.send(NormalizedEvent::Status {
                            exchange: exchange_label,
                            status: ConnectionStatus::Failed,
                            message: "Permanently disabled after 5 consecutive failed attempts.".to_string(),
                        }).await;
                        if let Some(ref es) = es {
                            es.set_disabled(&exchange_label_str).await;
                        }
                        eprintln!("🛑 Orchestrator: {} adapter permanently disabled.", exchange_label);
                        break;
                    }

                    if let Some(ref es) = es {
                        es.set_reconnecting(&exchange_label_str).await;
                    }

                    let _ = tx_clone.send(NormalizedEvent::Status {
                        exchange: exchange_label,
                        status: ConnectionStatus::Reconnecting,
                        message: format!("Retrying in {}s...", retry_cooldown_secs),
                    }).await;

                    // Exponential backoff with ±20% jitter applied before the
                    // 30 s cap, per 03-01-01 §4.1: effective delay range is
                    // [delay × 0.8, min(delay × 1.2, max_backoff)].
                    let delay = crate::adapters::resilience::apply_jitter(
                        Duration::from_secs(retry_cooldown_secs),
                        0.2,
                    )
                    .min(Duration::from_secs(30));
                    sleep(delay).await;
                    retry_cooldown_secs = (retry_cooldown_secs * 2).min(30);
                }
            });
        }

        rx
    }
}
