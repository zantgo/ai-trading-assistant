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
async fn test_five_tf_fanout_history_cap_100_and_broadcast() {
    tokio::time::timeout(tokio::time::Duration::from_secs(30), async {
        let symbol = "ZZZ".to_string();
        let pair_key = "Hyperliquid-ZZZ".to_string();

        let (event_tx, event_rx) = mpsc::channel::<NormalizedEvent>(500);

        let (short_tx, short_rx) = mpsc::channel::<NormalizedEvent>(200);
        let (mid_tx, mid_rx) = mpsc::channel::<NormalizedEvent>(200);
        let (long_tx, long_rx) = mpsc::channel::<NormalizedEvent>(200);
        let (macro_tx, macro_rx) = mpsc::channel::<NormalizedEvent>(200);
        let (supermacro_tx, supermacro_rx) = mpsc::channel::<NormalizedEvent>(200);

        // Broadcast channels — subscribe to verify snapshot delivery
        let (short_broadcast, mut short_bcast_rx) = broadcast::channel::<MarketSnapshot>(120);
        let (mid_broadcast, mut mid_bcast_rx) = broadcast::channel::<MarketSnapshot>(120);
        let (long_broadcast, _) = broadcast::channel::<MarketSnapshot>(120);
        let (macro_broadcast, _) = broadcast::channel::<MarketSnapshot>(120);
        let (supermacro_broadcast, _) = broadcast::channel::<MarketSnapshot>(120);

        let cap = 100usize;
        let short_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(cap)));
        let mid_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(cap)));
        let long_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(cap)));

        let short_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
        let mid_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));

        let cancel = CancellationToken::new();
        let (telemetry_tx, _telemetry_rx) = mpsc::channel::<engine::db::TelemetryMsg>(10);

        let indicators = engine::config::IndicatorsConfig {
            ema_fast: 5,
            ema_medium: 10,
            ema_slow: 20,
            ema_long: 30,
            rsi_period: 5,
            adx_period: 5,
            adx_trend_threshold: 20,
            adx_exhaustion_threshold: 40,
            adx_slope_lookback: 3,
            squeeze_period: 5,
            squeeze_min_duration: 2,
            bbwp_lookback: 10,
            bbwp_period: 5,
            atr_period: 5,
            ..Default::default()
        };

        let fib_config = FibonacciConfig {
            swing_lookback: 5,
            swing_scan_range: 20,
            retracement_coefficients: vec![0.618, 0.660],
            extension_coefficients: vec![1.618, 2.618],
        };

        let short_tf = TimeframeConfig { candles: engine::config::CandlesConfig { duration_seconds: 15, analysis_limit: 100 }, indicators: indicators.clone() };
        let mid_tf = TimeframeConfig { candles: engine::config::CandlesConfig { duration_seconds: 30, analysis_limit: 100 }, indicators: indicators.clone() };
        let long_tf = TimeframeConfig { candles: engine::config::CandlesConfig { duration_seconds: 60, analysis_limit: 100 }, indicators: indicators.clone() };
        let macro_tf = TimeframeConfig { candles: engine::config::CandlesConfig { duration_seconds: 120, analysis_limit: 100 }, indicators: indicators.clone() };
        let supermacro_tf = TimeframeConfig { candles: engine::config::CandlesConfig { duration_seconds: 300, analysis_limit: 100 }, indicators: indicators.clone() };

        // Event router
        let router_cancel = cancel.clone();
        let router_symbol = symbol.clone();
        tokio::spawn(async move {
            analyzer::run_event_router(
                event_rx,
                short_tx, mid_tx, long_tx, macro_tx, supermacro_tx,
                router_symbol,
                router_cancel,
            ).await;
        });

        let spawn_analyzer = |rx, broadcast, tf_cfg: TimeframeConfig, fib: FibonacciConfig,
            div_det: Arc<tokio::sync::Mutex<DivergenceDetector>>,
            history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
            latest: Arc<RwLock<Option<MarketSnapshot>>>,
            symbol: String, pk: String, secs: u64, label: &'static str,
            cancel: CancellationToken| {
            let t = telemetry_tx.clone();
            tokio::spawn(async move {
                analyzer::run_single(rx, t, broadcast, tf_cfg, fib, div_det, history, latest, symbol, pk, secs, label, cancel, None).await;
            })
        };

        let s_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));
        let m_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));
        let l_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));
        let ma_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));
        let sm_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));

        // Long-lived history for async borrow in future (keeps references alive)
        let _ = (&short_history, &mid_history, &long_history);

        let _h1 = spawn_analyzer(short_rx, short_broadcast, short_tf, fib_config.clone(), s_div, short_history.clone(), short_latest.clone(), symbol.clone(), pair_key.clone(), 15, "Short", cancel.clone());
        let _h2 = spawn_analyzer(mid_rx, mid_broadcast, mid_tf, fib_config.clone(), m_div, mid_history.clone(), mid_latest.clone(), symbol.clone(), pair_key.clone(), 30, "Mid", cancel.clone());
        let _h3 = spawn_analyzer(long_rx, long_broadcast, long_tf, fib_config.clone(), l_div, long_history.clone(), Arc::new(RwLock::new(None)), symbol.clone(), pair_key.clone(), 60, "Long", cancel.clone());
        let _h4 = spawn_analyzer(macro_rx, macro_broadcast, macro_tf, fib_config.clone(), ma_div, Arc::new(RwLock::new(VecDeque::with_capacity(cap))), Arc::new(RwLock::new(None)), symbol.clone(), pair_key.clone(), 120, "Macro", cancel.clone());
        let _h5 = spawn_analyzer(supermacro_rx, supermacro_broadcast, supermacro_tf, fib_config.clone(), sm_div, Arc::new(RwLock::new(VecDeque::with_capacity(cap))), Arc::new(RwLock::new(None)), symbol.clone(), pair_key.clone(), 300, "SuperMacro", cancel.clone());

        // Send trades rapidly — use timestamps 1 second apart, 200 trades = 200 seconds
        // 15s candles → ~13 candles. Not enough. Instead, use timestamp gaps.
        // Timestamps spaced 15s apart, 150 trades = 2250 seconds → ~150 short candles
        let base_price = 50000.0f64;
        let mut ts = 0u64;
        let total_trades = 150;
        for i in 0..total_trades {
            let price = base_price + (i as f64 * 0.4).sin() * 200.0 + (i as f64 * 0.05);
            let trade = NormalizedTrade {
                exchange: Exchange::Hyperliquid,
                symbol: format!("{}-USD", symbol),
                price: rust_decimal::Decimal::from_f64_retain(price).unwrap_or(dec!(50000.00)),
                size: dec!(0.5) + rust_decimal::Decimal::from(i % 4),
                side: if i % 3 == 0 { TradeSide::Sell } else { TradeSide::Buy },
                timestamp_ms: ts,
                trade_id: format!("t_{}", i),
            };
            event_tx.send(NormalizedEvent::Trade(trade)).await.unwrap();
            ts += 15000; // 15 seconds between trades → 2250 seconds total → ~150 short candles
        }

        // Let pipelines process
        tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

        let short_count = short_history.read().await.len();
        let mid_count = mid_history.read().await.len();
        eprintln!("History counts — Short(15s): {}, Mid(30s): {}", short_count, mid_count);

        // With one trade every 15s, each trade produces its own 15s candle → ~150 candles
        // Capped at 100
        assert!(short_count <= cap, "Short history capped at {}; got {}", cap, short_count);
        assert!(mid_count <= cap, "Mid history capped at {}; got {}", cap, mid_count);

        let total = short_count + mid_count;
        assert!(total > 0, "At least one timeframe should produce candle history");

        // Broadcast verification
        let short_snaps = drain_broadcast(&mut short_bcast_rx);
        let mid_snaps = drain_broadcast(&mut mid_bcast_rx);
        eprintln!("Broadcast snapshots — Short: {}, Mid: {}", short_snaps, mid_snaps);

        let total_snaps = short_snaps + mid_snaps;
        assert!(total_snaps > 0, "At least one broadcast channel should have delivered snapshots");

        // Verify latest snapshots
        let has_latest = short_latest.read().await.is_some() || mid_latest.read().await.is_some();
        assert!(has_latest, "At least one timeframe should have a latest snapshot");

        // FIFO eviction: if at capacity, oldest should have valid timestamps
        if short_count >= 10 {
            let sh = short_history.read().await;
            let oldest_ts = sh.front().unwrap().start_time_ms;
            assert!(oldest_ts > 0, "Oldest candle timestamp should be > 0");
        }

        cancel.cancel();
    })
    .await
    .expect("Multi-timeframe integration test timed out");
}

fn drain_broadcast(rx: &mut broadcast::Receiver<MarketSnapshot>) -> usize {
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    count
}
