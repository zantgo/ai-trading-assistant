use tokio::sync::mpsc;
use rust_decimal_macros::dec;
use shared::normalized::{Exchange, NormalizedEvent, NormalizedTrade, TradeSide, CandleGenerator};

#[tokio::test]
async fn test_ingestion_to_candle_generation_pipeline() {
    tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
        let (tx, mut rx) = mpsc::channel::<NormalizedEvent>(10);
        let mut generator = CandleGenerator::new("ETH-USD", 1);

        tokio::spawn(async move {
            let trades = vec![
                NormalizedTrade {
                    exchange: Exchange::Hyperliquid,
                    symbol: "ETH-USD".to_string(),
                    price: dec!(3000.00),
                    size: dec!(10.0),
                    side: TradeSide::Buy,
                    timestamp_ms: 1000,
                    trade_id: "id_1".to_string(),
                },
                NormalizedTrade {
                    exchange: Exchange::Hyperliquid,
                    symbol: "ETH-USD".to_string(),
                    price: dec!(3010.00),
                    size: dec!(5.0),
                    side: TradeSide::Buy,
                    timestamp_ms: 1500,
                    trade_id: "id_2".to_string(),
                },
                NormalizedTrade {
                    exchange: Exchange::Hyperliquid,
                    symbol: "ETH-USD".to_string(),
                    price: dec!(2990.00),
                    size: dec!(8.0),
                    side: TradeSide::Sell,
                    timestamp_ms: 2100,
                    trade_id: "id_3".to_string(),
                },
            ];

            for trade in trades {
                tx.send(NormalizedEvent::Trade(trade)).await.unwrap();
            }
        });

        let mut closed_candles = Vec::new();

        while let Some(event) = rx.recv().await {
            if let NormalizedEvent::Trade(trade) = event {
                let (closed, _live) = generator.process_trade(&trade);
                if let Some(candle) = closed {
                    closed_candles.push(candle);
                }
            }
        }

        assert_eq!(closed_candles.len(), 1, "Exactly one candle should have completed and closed.");
        let target = &closed_candles[0];
        assert_eq!(target.start_time_ms, 1000);
        assert_eq!(target.open, dec!(3000.00));
        assert_eq!(target.high, dec!(3010.00));
        assert_eq!(target.low, dec!(3000.00));
        assert_eq!(target.close, dec!(3010.00));
        assert_eq!(target.volume, dec!(15.0));
        assert_eq!(target.trades_count, 2);
    })
    .await
    .expect("Pipeline integration test timed out after 5 seconds");
}
