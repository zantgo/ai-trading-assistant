use engine::config::AppConfig;
use engine::db;
use engine::llm::LlmClient;
use engine::server::{self, AppState};
use engine::workspace::Workspace;
use engine::instance::Instance;
use shared::models::MarketSnapshot;
use shared::normalized::SymbolMapper;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{mpsc, RwLock, broadcast};
use hyper::body::Buf;
use http_body_util::BodyExt;
use engine::analyzer::{ActivePair, TimeframePipeline};
use shared::indicators::DivergenceDetector;
use engine::sr_engine::SrRoleTracker;
use engine::config::FibonacciConfig;
use rust_decimal_macros::dec;

async fn build_e2e_state() -> (Arc<AppState>, SqlitePool) {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory DB");

    // Create master_assistant_records table (minimal)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS master_assistant_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT (datetime('now')),
            position TEXT NOT NULL,
            entry_price TEXT,
            price_at_analysis TEXT NOT NULL,
            general_trend TEXT NOT NULL,
            support_levels TEXT NOT NULL,
            resistance_levels TEXT NOT NULL,
            indicator_synthesis_summary TEXT NOT NULL,
            indicator_synthesis_evaluation TEXT NOT NULL,
            recommended_action TEXT NOT NULL,
            recommendation_rationale TEXT NOT NULL,
            symbol TEXT NOT NULL,
            trigger_type TEXT NOT NULL DEFAULT 'Manual'
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(RwLock::new(AppConfig {
        symbols: vec!["Hyperliquid:BTC".to_string()],
        candles: engine::config::CandlesConfig { duration_seconds: 60, analysis_limit: 100 },
        indicators: Default::default(), hyperliquid: Default::default(),
        fibonacci: Default::default(), pivots: Default::default(),
        medium_timeframe: Default::default(), large_timeframe: Default::default(),
        leverage: Default::default(), scoring: Default::default(),
        fees: Default::default(), costs: Default::default(),
        workspace: Default::default(), safety: Default::default(),
        intervals: Default::default(), api_failover: Default::default(),
        instances: HashMap::new(),
    }));

    let symbol_mapper = Arc::new(SymbolMapper::new());
    symbol_mapper.register(shared::normalized::Exchange::Hyperliquid, "BTC", "BTC").await;

    let (telemetry_tx, telemetry_rx) = mpsc::channel::<db::TelemetryMsg>(100);
    let (llm_client, _) = LlmClient::from_env();
    let llm = Arc::new(RwLock::new(llm_client));
    let api_key_configured = Arc::new(AtomicBool::new(false));
    let ws_url = "ws://127.0.0.1:1".to_string();

    let logger_pool = pool.clone();
    tokio::spawn(async move {
        db::run_telemetry_logger(logger_pool, telemetry_rx, llm).await;
    });

    let workspace = Arc::new(Workspace::new(
        config.clone(), pool.clone(), symbol_mapper.clone(),
        telemetry_tx.clone(), api_key_configured.clone(), ws_url.clone(),
    ));

    let (mid_bcast, _) = broadcast::channel::<MarketSnapshot>(10);
    let mut history = std::collections::VecDeque::new();
    let mut snap_history = std::collections::VecDeque::<MarketSnapshot>::new();
    for i in 0..100i64 {
        let price = 50000.0 + i as f64 * 10.0;
        let open = dec!(50000.00) + rust_decimal::Decimal::from(i) * dec!(10.00);
        let high = dec!(50100.00) + rust_decimal::Decimal::from(i) * dec!(10.00);
        let low = dec!(49900.00) + rust_decimal::Decimal::from(i) * dec!(10.00);
        let close = rust_decimal::Decimal::from_f64_retain(price).unwrap_or(dec!(50000.00));
        history.push_back(shared::normalized::NormalizedCandle {
            symbol: "BTC".to_string(),
            start_time_ms: (1000000 + i * 60000) as u64,
            duration_ms: 60000,
            open,
            high,
            low,
            close,
            volume: dec!(10.0),
            trades_count: 5,
        });
        snap_history.push_back(MarketSnapshot {
            exchange: Some(shared::normalized::Exchange::Hyperliquid),
            timeframe_secs: 60,
            timestamp: (1000 + i * 60) as u64,
            symbol: "BTC".to_string(),
            is_completed: Some(true),
            mid_price: close,
            bid_price: dec!(0),
            ask_price: dec!(0),
            bid_size: Some(dec!(10.0)),
            ask_size: Some(dec!(10.0)),
            funding_rate: None,
            open: Some(open),
            high: Some(high),
            low: Some(low),
            close: Some(close),
            volume: Some(dec!(10.0)),
            average_volume: None,
            rvol: None,
            bb_upper: None, bb_middle: None, bb_lower: None,
            atr_14: None, atr_slope: None, atr_volatility_regime: None,
            atr_stop_loss_level: None, atr_take_profit_level: None,
            vwap: None, vwap_bias: None,
            adx_14: None, adx_plus: None, adx_minus: None,
            ema_fast: None, ema_medium: None, ema_slow: None, ema_long: None,
            ema_stack_state: None, rsi_14: None,
            macd_line: None, macd_signal: None, macd_hist: None,
            squeeze_on: None, squeeze_momentum: None,
            squeeze_duration: None, squeeze_release_trigger: None,
            squeeze_momentum_direction: None, bbwp: None,
            support_levels: None, resistance_levels: None, sr_flip_events: None,
            fib_golden_pocket_low: None, fib_golden_pocket_high: None,
            fib_extension_1618: None, fib_extension_2618: None,
            swing_high: None, swing_low: None,
            chart_pattern: None, chart_pattern_confidence: None,
            rsi_divergence_status: None, rsi_divergence_coords: None,
            macd_divergence_status: None, macd_divergence_coords: None,
            macd_histogram_peak: None, macd_trend_state: None,
            macd_crossover_detected: None, macd_crossover_direction: None,
            adx_slope: None, adx_peak: None, adx_regime: None,
            adx_di_crossover_detected: None, adx_di_crossover_direction: None,
        });
    }

    let (snapshot_tx, _) = mpsc::channel(100);
    let cancel = tokio_util::sync::CancellationToken::new();
    let b2 = mid_bcast.clone();
    let b3 = mid_bcast.clone();
    let b4 = mid_bcast.clone();

    let pair = Arc::new(ActivePair {
        symbol: "BTC".to_string(),
        micro: build_pipeline(history, snap_history, mid_bcast),
        short: build_pipeline_empty(b2),
        medium: build_pipeline_empty(b3),
        large: build_pipeline_empty(b4),
        snapshot_tx,
        cancel,
    });

    let snap_hist = Arc::new(RwLock::new(std::collections::VecDeque::<MarketSnapshot>::new()));
    let instance = Arc::new(Instance::new(
        "inst_test".to_string(),
        ("BTC".to_string(), "USDT".to_string()),
        pair.clone(),
        pool.clone(),
        config.clone(),
        Default::default(),
        Default::default(),
        pair.micro.history.clone(),
        pair.short.history.clone(),
        pair.medium.history.clone(),
        pair.large.history.clone(),
        pair.micro.latest_snapshot.clone(),
        pair.short.latest_snapshot.clone(),
        pair.medium.latest_snapshot.clone(),
        pair.large.latest_snapshot.clone(),
        snap_hist.clone(),
        snap_hist.clone(),
        snap_hist.clone(),
        snap_hist.clone(),
    ));
    workspace.instances.write().await.insert("BTC-USDT".to_string(), instance);

    let state = Arc::new(AppState {
        workspace, config, pool: pool.clone(),
        llm_client: Arc::new(RwLock::new(LlmClient::from_env().0)),
        api_key_configured, symbol_mapper, telemetry_tx,
        ws_url: "ws://127.0.0.1:1".to_string(),
    });

    (state, pool)
}

fn build_pipeline(history: std::collections::VecDeque<shared::normalized::NormalizedCandle>, snap_history: std::collections::VecDeque<MarketSnapshot>, bcast: broadcast::Sender<MarketSnapshot>) -> TimeframePipeline {
    TimeframePipeline {
        history: Arc::new(RwLock::new(history)),
        broadcast_tx: bcast,
        latest_snapshot: Arc::new(RwLock::new(None)),
        snapshot_history: Arc::new(RwLock::new(snap_history)),
        timeframe_secs: 60, timeframe_label: "Micro",
        divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
        sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.3))),
        fibonacci: FibonacciConfig::default(),
    }
}

fn build_pipeline_empty(bcast: broadcast::Sender<MarketSnapshot>) -> TimeframePipeline {
    TimeframePipeline {
        history: Arc::new(RwLock::new(std::collections::VecDeque::new())),
        broadcast_tx: bcast,
        latest_snapshot: Arc::new(RwLock::new(None)),
        snapshot_history: Arc::new(RwLock::new(std::collections::VecDeque::new())),
        timeframe_secs: 60, timeframe_label: "Micro",
        divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
        sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.3))),
        fibonacci: FibonacciConfig::default(),
    }
}

#[tokio::test]
async fn test_e2e_analysis_master_record_created_and_error_when_no_key() {
    tokio::time::timeout(tokio::time::Duration::from_secs(15), async {
        let (state, pool) = build_e2e_state().await;

        let payload = serde_json::json!({
            "position": "Long",
            "entry_price": "50000.00",
            "historical_prices": (0..100).map(|i| 50000.0 + i as f64 * 10.0).collect::<Vec<f64>>(),
            "indicators": {
                "rsi": 62.5, "squeeze_on": false, "squeeze_momentum": 0.12,
                "macd_line": 15.0, "macd_signal": 10.0, "macd_histogram": 5.0,
                "macd_histogram_trend": "Accelerating",
                "adx": 28.0, "adx_plus": 30.0, "adx_minus": 18.0,
                "bb_upper": 51000.0, "bb_middle": 50000.0, "bb_lower": 49000.0,
                "atr": 250.0, "atr_trend": "Stable", "atr_volatility_regime": "Stable",
                "current_price": 51000.0, "volume": 150.0, "average_volume": 120.0,
                "rvol": 1.25, "ema_fast": 50900.0, "ema_medium": 50500.0,
                "ema_slow": 50000.0, "ema_long": 49500.0,
                "ema_stack_state": "Bullish", "vwap": 50950.0, "vwap_bias": "Premium"
            },
            "symbol": "BTC-USDT"
        });

        let router = server::build_router(state.clone());
        let body_str = serde_json::to_string(&payload).unwrap();
        let request = hyper::Request::builder()
            .method("POST")
            .uri("/api/analyze")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body_str))
            .unwrap();

        use tower::ServiceExt;
        let response = router.oneshot(request).await.unwrap();
        let status = response.status();

        // Without API key, should return 503
        assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "Should return 503 when no API key configured");

        // Verify master record was created in DB (use raw query since query_master_records filters PENDING)
        let row: (i64, String, String) = sqlx::query_as(
            "SELECT id, position, symbol FROM master_assistant_records ORDER BY id DESC LIMIT 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(row.0 > 0, "Master record ID should be > 0");
        assert_eq!(row.1, "Long");
        assert_eq!(row.2, "BTC-USDT");
    })
    .await
    .expect("E2E analysis test timed out");
}

#[tokio::test]
async fn test_e2e_history_endpoint_with_populated_data() {
    tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        let (state, _pool) = build_e2e_state().await;

        let router = server::build_router(state.clone());
        let request = hyper::Request::builder()
            .method("GET")
            .uri("/api/history?symbol=BTC-USDT&timeframe_secs=60")
            .body(axum::body::Body::empty())
            .unwrap();

        use tower::ServiceExt;
        let response = router.oneshot(request).await.unwrap();
        assert!(response.status().is_success(), "History endpoint should succeed");

        // Read response body
        let body = response.collect().await.unwrap().aggregate();
        let history_resp: serde_json::Value = serde_json::from_reader(body.reader()).unwrap_or_default();
        let candles = history_resp["candles"].as_array()
            .expect("History response should have a 'candles' array");
        assert!(!candles.is_empty(), "History should return pre-populated candles");
        assert!(candles.len() >= 50, "Should have at least 50 candles, got {}", candles.len());
    })
    .await
    .expect("E2E history test timed out");
}
