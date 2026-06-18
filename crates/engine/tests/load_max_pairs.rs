use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use rust_decimal_macros::dec;

use engine::analyzer;
use engine::config::{TimeframeConfig, FibonacciConfig};
use shared::models::MarketSnapshot;
use shared::normalized::{Exchange, NormalizedEvent, NormalizedTrade, TradeSide, NormalizedCandle};
use shared::indicators::DivergenceDetector;

#[tokio::test]
#[ignore]
async fn test_load_max_pairs_stability() {
    tokio::time::timeout(tokio::time::Duration::from_secs(60), async {
        let num_pairs = 5; // Smaller for faster CI
        let trades_per_pair = 30;
        let cap = 100usize;

        let cancel = CancellationToken::new();
        let (telemetry_tx, _telemetry_rx) = mpsc::channel::<engine::db::TelemetryMsg>(100);

        let mut event_txs = Vec::new();
        let mut mid_histories = Vec::new();

        let indicators = engine::config::IndicatorsConfig {
            ema_fast: 5, ema_medium: 10, ema_slow: 20, ema_long: 30,
            rsi_period: 5, adx_period: 5, squeeze_period: 5, atr_period: 5,
            bbwp_lookback: 10, bbwp_period: 5,
            ..Default::default()
        };

        for pair_idx in 0..num_pairs {
            let symbol = format!("SYM{:02}", pair_idx);

            let (event_tx, event_rx) = mpsc::channel::<NormalizedEvent>(200);
            let (mid_tx, mid_rx) = mpsc::channel::<NormalizedEvent>(100);

            let (mid_broadcast, _) = broadcast::channel::<MarketSnapshot>(50);

            let mid_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(cap)));

            let fib_config = FibonacciConfig {
                swing_lookback: 5, swing_scan_range: 20,
                retracement_coefficients: vec![0.618, 0.660],
                extension_coefficients: vec![1.618, 2.618],
            };

            let mid_tf = TimeframeConfig { candles: engine::config::CandlesConfig { duration_seconds: 60, analysis_limit: 100 }, indicators: indicators.clone() };

            // Event router fanning out to 4 timeframes
            let r_cancel = cancel.clone();
            let r_sym = symbol.clone();
            let m1 = mid_tx.clone();
            let m2 = mid_tx.clone();
            let m3 = mid_tx.clone();
            tokio::spawn(async move {
                analyzer::run_event_router(event_rx, mid_tx, m1, m2, m3, r_sym, r_cancel).await;
            });

            // Mid analyzer
            let m_cancel = cancel.clone();
            let m_sym = symbol.clone();
            let m_pk = format!("Hyperliquid-{}", symbol);
            let m_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));
            let m_t = telemetry_tx.clone();
            let mh = mid_history.clone();
            let mfib = fib_config.clone();
            tokio::spawn(async move {
                analyzer::run_single(mid_rx, m_t, mid_broadcast, mid_tf, mfib, m_div, mh, Arc::new(RwLock::new(None)), m_sym, m_pk, 60, "Mid", m_cancel, None).await;
            });

            event_txs.push(event_tx);
            mid_histories.push(mid_history);
        }

        // Feed trades across all pairs from a background task
        tokio::spawn(async move {
            let mut ts = 0u64;
            for trade_idx in 0..trades_per_pair {
                for pair_idx in 0..num_pairs {
                    let price = 50000.0 + (trade_idx as f64 * 0.5).sin() * 200.0 + pair_idx as f64 * 100.0;
                    let trade = NormalizedTrade {
                        exchange: Exchange::Hyperliquid,
                        symbol: format!("SYM{:02}-USD", pair_idx),
                        price: rust_decimal::Decimal::from_f64_retain(price).unwrap_or(dec!(50000.00)),
                        size: dec!(0.5),
                        side: if trade_idx % 2 == 0 { TradeSide::Buy } else { TradeSide::Sell },
                        timestamp_ms: ts,
                        trade_id: format!("t_{}_{}", pair_idx, trade_idx),
                    };
                    let _ = event_txs[pair_idx].send(NormalizedEvent::Trade(trade)).await;
                }
                ts += 15000;
            }
        });

        // Let pipelines process
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let mut total_candles = 0usize;
        for i in 0..num_pairs {
            let mc = mid_histories[i].read().await.len();
            assert!(mc <= cap, "Mid history for pair {} over cap: {} > {}", i, mc, cap);
            total_candles += mc;
        }

        eprintln!("Load test: {} pairs, {} total candles", num_pairs, total_candles);
        assert!(total_candles > 0, "At least some candles should be produced");

        cancel.cancel();
    })
    .await
    .expect("Load test timed out");
}
