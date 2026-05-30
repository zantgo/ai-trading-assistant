use tokio::sync::mpsc::{channel, Sender};
use rust_decimal_macros::dec;
use shared::normalized::{Exchange, NormalizedEvent, NormalizedTrade, TradeSide};

pub struct ReplayEngine {
    symbol: String,
    historical_data: Vec<NormalizedTrade>,
}

impl ReplayEngine {
    pub fn new(symbol: &str, data: Vec<NormalizedTrade>) -> Self {
        Self {
            symbol: symbol.to_string(),
            historical_data: data,
        }
    }

    pub async fn run_simulation(self, event_tx: Sender<NormalizedEvent>, artificial_delay_ms: u64) {
        println!(
            "Replay Engine: Commencing simulation for {} containing {} trades.",
            self.symbol,
            self.historical_data.len()
        );

        for trade in self.historical_data {
            let _ = event_tx.send(NormalizedEvent::Trade(trade)).await;
            if artificial_delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(artificial_delay_ms)).await;
            }
        }

        println!("Replay Engine: Simulation sequence finished.");
    }
}

#[tokio::test]
async fn test_replay_simulation_pipeline() {
    tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
        let mock_trades = vec![
            NormalizedTrade {
                exchange: Exchange::Hyperliquid,
                symbol: "BTC-USD".to_string(),
                price: dec!(48000.00),
                size: dec!(0.1),
                side: TradeSide::Buy,
                timestamp_ms: 1000,
                trade_id: "sim_1".to_string(),
            },
            NormalizedTrade {
                exchange: Exchange::Hyperliquid,
                symbol: "BTC-USD".to_string(),
                price: dec!(48100.00),
                size: dec!(0.2),
                side: TradeSide::Buy,
                timestamp_ms: 1500,
                trade_id: "sim_2".to_string(),
            },
        ];

        let (tx, mut rx) = channel::<NormalizedEvent>(100);
        let engine = ReplayEngine::new("BTC-USD", mock_trades);

        tokio::spawn(engine.run_simulation(tx, 1));

        let mut count = 0;
        while let Some(event) = rx.recv().await {
            if let NormalizedEvent::Trade(trade) = event {
                assert_eq!(trade.symbol, "BTC-USD");
                count += 1;
            }
        }

        assert_eq!(count, 2, "All replayed trades must be processed by the engine stream.");
    })
    .await
    .expect("Replay simulation test timed out after 5 seconds");
}
