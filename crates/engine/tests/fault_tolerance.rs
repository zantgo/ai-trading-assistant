use engine::adapters;
use engine::analyzer;
use engine::config::{AppConfig, FibonacciConfig, TimeframeConfig};
use shared::indicators::DivergenceDetector;
use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedCandle, NormalizedEvent};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_per_pair_ws_and_analyzer_cancellation_loop() {
    tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        let symbol = "BTC".to_string();
        let pair_key = "BTC-USDT".to_string();

        let (snapshot_tx, snapshot_rx) = mpsc::channel::<NormalizedEvent>(100);
        let (broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(100);
        let history = Arc::new(tokio::sync::RwLock::new(
            VecDeque::<NormalizedCandle>::with_capacity(100),
        ));
        let latest_snap = Arc::new(tokio::sync::RwLock::new(None::<MarketSnapshot>));
        let cancel = CancellationToken::new();
        let divergence_detector = Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20)));

        let (telemetry_tx, _telemetry_rx) = mpsc::channel(10);

        let test_config = AppConfig {
            symbols: vec!["Hyperliquid:BTC".to_string()],
            candles: engine::config::CandlesConfig {
                duration_seconds: 60,
                analysis_limit: 100,
            },
            indicators: Default::default(),
            hyperliquid: Default::default(),
            bitget: Default::default(),
            fibonacci: Default::default(),
            pivots: Default::default(),
            slow_timeframe: Default::default(),
            macro_timeframe: Default::default(),
            leverage: Default::default(),
            scoring: Default::default(),
            fees: Default::default(),
            costs: Default::default(),
            workspace: Default::default(),
            safety: Default::default(),
            intervals: Default::default(),
            api_failover: Default::default(),
            instances: HashMap::new(),
        };
        let indicators = test_config.indicators.clone();
        let tf_cfg = TimeframeConfig::new(60, indicators);
        let fib_config = FibonacciConfig::default();

        let analyzer_cancel = cancel.clone();
        let analyzer_history = history.clone();
        let analyzer_latest_snap = latest_snap.clone();
        let analyzer_broadcast = broadcast_tx.clone();
        let analyzer_telemetry = telemetry_tx.clone();
        let analyzer_symbol = symbol.clone();
        let analyzer_pair_key = pair_key.clone();
        let analyzer_div_det = divergence_detector.clone();
        let analyzer_handle = tokio::spawn(async move {
            analyzer::run_single(
                snapshot_rx,
                analyzer_telemetry,
                analyzer_broadcast,
                tf_cfg,
                fib_config,
                analyzer_div_det,
                analyzer_history,
                analyzer_latest_snap,
                Arc::new(RwLock::new(VecDeque::new())),
                analyzer_symbol,
                analyzer_pair_key,
                60,
                "Micro",
                analyzer_cancel,
                None,
                None,
                None,
            )
            .await;
        });

        let ws_cancel = cancel.clone();
        let ws_tx = snapshot_tx.clone();
        let ws_symbol = symbol.clone();
        let ws_internal = format!("{}-USDT", symbol);
        let ws_handle = tokio::spawn(async move {
            adapters::hyperliquid::run_for_symbol(ws_symbol, ws_internal, ws_tx, ws_cancel, "ws://127.0.0.1:1")
                .await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        cancel.cancel();

        let ws_result = tokio::time::timeout(tokio::time::Duration::from_secs(5), ws_handle).await;

        assert!(
            ws_result.is_ok(),
            "WS ingestion task should exit cleanly when cancellation is triggered"
        );

        let analyzer_result =
            tokio::time::timeout(tokio::time::Duration::from_secs(5), analyzer_handle).await;

        assert!(
            analyzer_result.is_ok(),
            "Analysis task should exit cleanly when cancellation is triggered"
        );
    })
    .await
    .expect("Per-pair cancellation test timed out after 10 seconds");
}
