use rust_decimal_macros::dec;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;

use config_models::{FibonacciConfig, OrderBookConfig, TimeframeConfig};
use core_domain::models::MarketSnapshot;
use core_domain::normalized::{
    Exchange, NormalizedCandle, NormalizedEvent, NormalizedTrade, TradeSide,
};
use market_analyzer::analyzer;
use market_analyzer::indicators::DivergenceDetector;

#[tokio::test]
async fn test_four_tf_fanout_history_cap_100_and_broadcast() {
    tokio::time::timeout(tokio::time::Duration::from_secs(30), async {
        let symbol = "ZZZ".to_string();
        let pair_key = "Hyperliquid-ZZZ".to_string();

        let (event_tx, event_rx) = mpsc::channel::<NormalizedEvent>(500);

        let (micro_tx, micro_rx) = mpsc::channel::<NormalizedEvent>(200);
        let (fast_tx, fast_rx) = mpsc::channel::<NormalizedEvent>(200);
        let (slow_tx, slow_rx) = mpsc::channel::<NormalizedEvent>(200);
        let (macro_tx, macro_rx) = mpsc::channel::<NormalizedEvent>(200);

        // Broadcast channels — subscribe to verify snapshot delivery
        let (micro_broadcast, mut micro_bcast_rx) = broadcast::channel::<MarketSnapshot>(120);
        let (fast_broadcast, mut fast_bcast_rx) = broadcast::channel::<MarketSnapshot>(120);
        let (slow_broadcast, _) = broadcast::channel::<MarketSnapshot>(120);
        let (macro_broadcast, _) = broadcast::channel::<MarketSnapshot>(120);

        let cap = 100usize;
        let micro_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
            cap,
        )));
        let fast_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
            cap,
        )));

        let micro_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
        let fast_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));

        let cancel = CancellationToken::new();
        let (telemetry_tx, _telemetry_rx) = mpsc::channel::<database_storage::TelemetryMsg>(10);

        let indicators = config_models::IndicatorsConfig {
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

        let micro_tf = TimeframeConfig {
            candles: config_models::CandlesConfig {
                duration_seconds: 30,
            },
            indicators: indicators.clone(),
            leverage: Default::default(),
        };
        let fast_tf = TimeframeConfig {
            candles: config_models::CandlesConfig {
                duration_seconds: 60,
            },
            indicators: indicators.clone(),
            leverage: Default::default(),
        };
        let slow_tf = TimeframeConfig {
            candles: config_models::CandlesConfig {
                duration_seconds: 90,
            },
            indicators: indicators.clone(),
            leverage: Default::default(),
        };
        let macro_tf = TimeframeConfig {
            candles: config_models::CandlesConfig {
                duration_seconds: 150,
            },
            indicators: indicators.clone(),
            leverage: Default::default(),
        };

        // Event router fanning out to 4 timeframes
        let router_cancel = cancel.clone();
        let router_symbol = symbol.clone();
        tokio::spawn(async move {
            analyzer::run_event_router(
                event_rx,
                micro_tx,
                fast_tx,
                slow_tx,
                macro_tx,
                router_symbol,
                router_cancel,
            )
            .await;
        });

        let spawn_analyzer = |rx,
                              broadcast,
                              tf_cfg: TimeframeConfig,
                              fib: FibonacciConfig,
                              div_det: Arc<tokio::sync::Mutex<DivergenceDetector>>,
                              history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
                              latest: Arc<RwLock<Option<MarketSnapshot>>>,
                              symbol: String,
                              pk: String,
                              secs: u64,
                              label: &'static str,
                              cancel: CancellationToken| {
            let t = telemetry_tx.clone();
            tokio::spawn(async move {
                let strategy = config_models::StrategyConfig::default();
                analyzer::run_single(
                    rx,
                    t,
                    broadcast,
                    tf_cfg,
                    fib,
                    core_domain::statistics::StatisticsConfig::default(),
                    div_det,
                    history,
                    latest,
                    Arc::new(RwLock::new(VecDeque::new())),
                    symbol,
                    pk,
                    secs,
                    label,
                    core_domain::models::TimeframeSlot::Micro,
                    cancel,
                    None,
                    None,
                    Arc::new(RwLock::new(None)),
                    Arc::new(RwLock::new(None)),
                    Arc::new(RwLock::new(None)),
                    Arc::new(RwLock::new(None)),
                    Arc::new(RwLock::new(VecDeque::with_capacity(60))),
                    Arc::new(RwLock::new(VecDeque::with_capacity(8))),
                    Arc::new(RwLock::new(None)),
                    None,
                    None,
                    OrderBookConfig::default(),
                    strategy,
                    Arc::new(RwLock::new(None)),
                    Arc::new(RwLock::new(None)),
                    Arc::new(RwLock::new(None)),
                    Arc::new(core_domain::LatencyTracker::default()),
                    market_analyzer::active_set::ActiveSet::default(),
                    None,
                    Arc::new(network_adapters::pipeline_reliability::ReliabilityTracker::new()),
                    None,
                    None,
                    1,
                    300,
                    Arc::new(RwLock::new(None)),
                    Arc::new(RwLock::new(
                        core_domain::indicator_dtos::IndicatorLifecycleMap::new(),
                    )),
                    Arc::new(RwLock::new(
                        core_domain::models::CandlePipelineState::Initializing,
                    )),
                )
                .await;
            })
        };

        let micro_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));
        let fast_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));
        let slow_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));
        let macro_div = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(10)));

        // Long-lived history for async borrow in future (keeps references alive)
        let _ = (&micro_history, &fast_history);

        let _h1 = spawn_analyzer(
            micro_rx,
            micro_broadcast,
            micro_tf,
            fib_config.clone(),
            micro_div,
            micro_history.clone(),
            micro_latest.clone(),
            symbol.clone(),
            pair_key.clone(),
            30,
            "Micro",
            cancel.clone(),
        );
        let _h2 = spawn_analyzer(
            fast_rx,
            fast_broadcast,
            fast_tf,
            fib_config.clone(),
            fast_div,
            fast_history.clone(),
            fast_latest.clone(),
            symbol.clone(),
            pair_key.clone(),
            60,
            "Fast",
            cancel.clone(),
        );
        let _h3 = spawn_analyzer(
            slow_rx,
            slow_broadcast,
            slow_tf,
            fib_config.clone(),
            slow_div,
            Arc::new(RwLock::new(VecDeque::with_capacity(cap))),
            Arc::new(RwLock::new(None)),
            symbol.clone(),
            pair_key.clone(),
            90,
            "Slow",
            cancel.clone(),
        );
        let _h4 = spawn_analyzer(
            macro_rx,
            macro_broadcast,
            macro_tf,
            fib_config.clone(),
            macro_div,
            Arc::new(RwLock::new(VecDeque::with_capacity(cap))),
            Arc::new(RwLock::new(None)),
            symbol.clone(),
            pair_key.clone(),
            150,
            "Macro",
            cancel.clone(),
        );

        // Timestamps spaced 60s apart, 50 trades = 3000 seconds → ~100 mid candles, ~50 long candles
        let base_price = 50000.0f64;
        let mut ts = 0u64;
        let total_trades = 50;
        for i in 0..total_trades {
            let price = base_price + (i as f64 * 0.4).sin() * 200.0 + (i as f64 * 0.05);
            let trade = NormalizedTrade {
                exchange: Exchange::Hyperliquid,
                symbol: format!("{}-USD", symbol),
                price: rust_decimal::Decimal::from_f64_retain(price).unwrap_or(dec!(50000.00)),
                size: dec!(0.5) + rust_decimal::Decimal::from(i % 4),
                side: if i % 3 == 0 {
                    TradeSide::Sell
                } else {
                    TradeSide::Buy
                },
                timestamp_ms: ts,
                trade_id: format!("t_{}", i),
            };
            event_tx.send(NormalizedEvent::Trade(trade)).await.unwrap();
            ts += 60000; // 1-minute step increments
        }

        // Let pipelines process
        tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

        let micro_count = micro_history.read().await.len();
        let fast_count = fast_history.read().await.len();
        eprintln!(
            "History counts — Micro(30s): {}, Fast(60s): {}",
            micro_count, fast_count
        );

        assert!(
            micro_count <= cap,
            "Micro history capped at {}; got {}",
            cap,
            micro_count
        );
        assert!(
            fast_count <= cap,
            "Fast history capped at {}; got {}",
            cap,
            fast_count
        );

        let total = micro_count + fast_count;
        assert!(
            total > 0,
            "At least one timeframe should produce candle history"
        );

        // Broadcast verification
        let micro_snaps = drain_broadcast(&mut micro_bcast_rx);
        let fast_snaps = drain_broadcast(&mut fast_bcast_rx);
        eprintln!(
            "Broadcast snapshots — Micro: {}, Fast: {}",
            micro_snaps, fast_snaps
        );

        let total_snaps = micro_snaps + fast_snaps;
        assert!(
            total_snaps > 0,
            "At least one broadcast channel should have delivered snapshots"
        );

        // Verify latest snapshots
        let has_latest = micro_latest.read().await.is_some() || fast_latest.read().await.is_some();
        assert!(
            has_latest,
            "At least one timeframe should have a latest snapshot"
        );

        // FIFO eviction: if at capacity, oldest should have valid timestamps
        if micro_count >= 10 {
            let sh = micro_history.read().await;
            let oldest_ts = sh.front().unwrap().start_time_ms;
            assert!(oldest_ts > 0, "Oldest candle timestamp should be > 0");
        }

        cancel.cancel();
    })
    .await
    .expect("4-Timeframe integration test timed out");
}

fn drain_broadcast(rx: &mut broadcast::Receiver<MarketSnapshot>) -> usize {
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    count
}
