use async_trait::async_trait;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc::Sender;
use shared::normalized::{
    Exchange, ExchangeAdapter, NormalizedEvent, SymbolMapper, ConnectionStatus,
};

struct MockFaultyAdapter {
    execution_count: Arc<AtomicUsize>,
}

#[async_trait]
impl ExchangeAdapter for MockFaultyAdapter {
    fn exchange(&self) -> Exchange {
        Exchange::Hyperliquid
    }

    async fn start(
        &self,
        _symbols: Vec<String>,
        event_tx: Sender<NormalizedEvent>,
        _mapper: Arc<SymbolMapper>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let count = self.execution_count.fetch_add(1, Ordering::SeqCst);

        if count == 0 {
            return Err("Simulated Network Loss".into());
        }

        let _ = event_tx.send(NormalizedEvent::Status {
            exchange: Exchange::Hyperliquid,
            status: ConnectionStatus::Connected,
            message: "Reconnection recovered".to_string(),
        }).await;

        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        Ok(())
    }
}

#[tokio::test]
async fn test_supervisor_retry_and_reconnection_loop() {
    tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        let mapper = Arc::new(SymbolMapper::new());
        mapper.register(Exchange::Hyperliquid, "BTC-USD", "BTC-USD").await;
        let mut orchestrator = engine::orchestrator::MarketDataOrchestrator::new(Arc::clone(&mapper));

        let execs = Arc::new(AtomicUsize::new(0));
        let faulty_adapter = MockFaultyAdapter {
            execution_count: Arc::clone(&execs),
        };

        orchestrator.register_adapter(Box::new(faulty_adapter));

        let mut status_rx = orchestrator.run(vec!["BTC-USD".to_string()]).await;

        let mut successfully_reconnected = false;

        for _ in 0..10 {
            if let Some(event) = status_rx.recv().await {
                if let NormalizedEvent::Status { status, .. } = event {
                    if status == ConnectionStatus::Connected {
                        successfully_reconnected = true;
                        break;
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        assert!(
            successfully_reconnected,
            "Orchestrator must automatically reschedule and restore failed connections."
        );
        assert!(
            execs.load(Ordering::SeqCst) >= 2,
            "Adapter must be restarted at least once. Actual: {}",
            execs.load(Ordering::SeqCst)
        );
    })
    .await
    .expect("Fault tolerance test timed out after 10 seconds");
}
