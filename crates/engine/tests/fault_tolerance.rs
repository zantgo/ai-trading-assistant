use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use engine::adapters;
use engine::analyzer;
use engine::config::AppConfig;
use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedEvent, NormalizedCandle};

#[tokio::test]
async fn test_per_pair_ws_and_analyzer_cancellation_loop() {
    tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        let symbol = "BTC".to_string();
        let pair_key = "Hyperliquid-BTC".to_string();

        let (snapshot_tx, snapshot_rx) = mpsc::channel::<NormalizedEvent>(100);
        let (broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(100);
        let history = Arc::new(tokio::sync::RwLock::new(
            VecDeque::<NormalizedCandle>::with_capacity(100),
        ));
        let latest_snap = Arc::new(tokio::sync::RwLock::new(None::<MarketSnapshot>));
        let cancel = CancellationToken::new();

        let (telemetry_tx, _telemetry_rx) = mpsc::channel(10);

        let test_config = AppConfig {
            symbols: vec!["Hyperliquid:BTC".to_string()],
            candles: engine::config::CandlesConfig { duration_seconds: 60 },
            indicators: engine::config::IndicatorsConfig {
                ema_fast: 10,
                ema_medium: 50,
                ema_slow: 100,
                ema_long: 200,
                rsi_period: 14,
                macd_fast: 12,
                macd_slow: 26,
                macd_signal: 9,
                adx_period: 14,
                atr_period: 14,
                squeeze_period: 20,
            },
            hyperliquid: Default::default(),
            pairs: HashMap::new(),
        };
        let config = Arc::new(tokio::sync::RwLock::new(test_config));

        let analyzer_cancel = cancel.clone();
        let analyzer_history = history.clone();
        let analyzer_latest_snap = latest_snap.clone();
        let analyzer_broadcast = broadcast_tx.clone();
        let analyzer_telemetry = telemetry_tx.clone();
        let analyzer_config = config.clone();
        let analyzer_symbol = symbol.clone();
        let analyzer_pair_key = pair_key.clone();
        let analyzer_handle = tokio::spawn(async move {
            analyzer::run_single(
                snapshot_rx,
                analyzer_telemetry,
                analyzer_broadcast,
                analyzer_config,
                analyzer_history,
                analyzer_latest_snap,
                analyzer_symbol,
                analyzer_pair_key,
                analyzer_cancel,
            )
            .await;
        });

        let ws_cancel = cancel.clone();
        let ws_tx = snapshot_tx.clone();
        let ws_symbol = symbol.clone();
        let ws_handle = tokio::spawn(async move {
            adapters::hyperliquid::run_for_symbol(ws_symbol, ws_tx, ws_cancel, "ws://127.0.0.1:1").await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        cancel.cancel();

        let ws_result = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            ws_handle,
        )
        .await;

        assert!(
            ws_result.is_ok(),
            "WS ingestion task should exit cleanly when cancellation is triggered"
        );

        let analyzer_result = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            analyzer_handle,
        )
        .await;

        assert!(
            analyzer_result.is_ok(),
            "Analysis task should exit cleanly when cancellation is triggered"
        );
    })
    .await
    .expect("Per-pair cancellation test timed out after 10 seconds");
}
