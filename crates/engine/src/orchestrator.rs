use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{sleep, Duration};
use shared::normalized::{NormalizedEvent, SymbolMapper, ExchangeAdapter, ConnectionStatus};

pub struct MarketDataOrchestrator {
    mapper: Arc<SymbolMapper>,
    adapters: Vec<Arc<dyn ExchangeAdapter>>,
    event_tx: Sender<NormalizedEvent>,
    event_rx: Option<Receiver<NormalizedEvent>>,
}

impl MarketDataOrchestrator {
    pub fn new(mapper: Arc<SymbolMapper>) -> Self {
        let (tx, rx) = channel::<NormalizedEvent>(10000);
        Self {
            mapper,
            adapters: Vec::new(),
            event_tx: tx,
            event_rx: Some(rx),
        }
    }

    pub fn register_adapter(&mut self, adapter: Box<dyn ExchangeAdapter>) {
        self.adapters.push(Arc::from(adapter));
    }

    pub async fn run(&mut self, symbols: Vec<String>) -> Receiver<NormalizedEvent> {
        let rx = self.event_rx.take().expect("Orchestrator already running");

        for adapter in &self.adapters {
            let adapter_clone = Arc::clone(adapter);
            let tx_clone = self.event_tx.clone();
            let mapper_clone = Arc::clone(&self.mapper);
            let symbols_clone = symbols.clone();

            tokio::spawn(async move {
                let mut retry_cooldown_secs = 2u64;
                let mut consecutive_failures = 0u32;
                loop {
                    let exchange_label = adapter_clone.exchange();

                    let _ = tx_clone.send(NormalizedEvent::Status {
                        exchange: exchange_label,
                        status: ConnectionStatus::Connecting,
                        message: format!("Supervisor: Starting {} adapter", exchange_label),
                    }).await;

                    match adapter_clone.start(symbols_clone.clone(), tx_clone.clone(), mapper_clone.clone()).await {
                        Ok(()) => {
                            consecutive_failures = 0;
                            eprintln!("⚠️  Orchestrator: {} adapter terminated cleanly.", exchange_label);
                        }
                        Err(e) => {
                            consecutive_failures += 1;
                            eprintln!("❌ Orchestrator: {} adapter crashed: {}.", exchange_label, e);
                        }
                    }

                    if consecutive_failures >= 3 {
                        let _ = tx_clone.send(NormalizedEvent::Status {
                            exchange: exchange_label,
                            status: ConnectionStatus::Disconnected,
                            message: "Permanently disabled after 3 consecutive failed attempts.".to_string(),
                        }).await;
                        eprintln!("🛑 Orchestrator: {} adapter permanently disabled.", exchange_label);
                        break;
                    }

                    let _ = tx_clone.send(NormalizedEvent::Status {
                        exchange: exchange_label,
                        status: ConnectionStatus::Disconnected,
                        message: format!("Retrying in {}s...", retry_cooldown_secs),
                    }).await;

                    sleep(Duration::from_secs(retry_cooldown_secs)).await;
                    retry_cooldown_secs = (retry_cooldown_secs * 2).min(60);
                }
            });
        }

        rx
    }
}
