use engine::analyzer::{ActivePair, TimeframePipeline};
use engine::config::AppConfig;
use engine::config::FibonacciConfig;
use engine::db;
use engine::instance::{Instance, TimeframeBuffers};
use engine::llm::LlmClient;
use engine::server::{self, AppState};
use engine::sr_engine::SrRoleTracker;
use engine::workspace::Workspace;
use http_body_util::BodyExt;
use hyper::body::Buf;
use rust_decimal_macros::dec;
use shared::indicators::DivergenceDetector;
use shared::models::MarketSnapshot;
use shared::normalized::SymbolMapper;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

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
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(RwLock::new(AppConfig {
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
        pivot_points: Default::default(),
        candlestick: Default::default(),
        profile: Default::default(),
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
    }));

    let symbol_mapper = Arc::new(SymbolMapper::new());
    symbol_mapper
        .register(shared::normalized::Exchange::Hyperliquid, "BTC", "BTC")
        .await;

    let (telemetry_tx, telemetry_rx) = mpsc::channel::<db::TelemetryMsg>(100);
    let (llm_client, _) = LlmClient::from_env();
    let llm = Arc::new(llm_client);
    let api_key_configured = Arc::new(AtomicBool::new(false));
    let ws_url = "ws://127.0.0.1:1".to_string();

    let logger_pool = pool.clone();
    tokio::spawn(async move {
        db::run_telemetry_logger(logger_pool, telemetry_rx, llm).await;
    });

    let workspace = Arc::new(Workspace::new(
        config.clone(),
        pool.clone(),
        symbol_mapper.clone(),
        telemetry_tx.clone(),
        api_key_configured.clone(),
        ws_url.clone(),
        ws_url.clone(),
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
            prev_day_px: None,
            open: Some(open),
            high: Some(high),
            low: Some(low),
            close: Some(close),
            volume: Some(dec!(10.0)),
            average_volume: None,
            indicators: std::collections::HashMap::new(),
            context: None,
            decision_context: None,
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
        fast: build_pipeline_empty(b2, 300),
        slow: build_pipeline_empty(b3, 900),
        r#macro: build_pipeline_empty(b4, 3600),
        snapshot_tx,
        cancel,
    });

    let snap_hist = Arc::new(RwLock::new(
        std::collections::VecDeque::<MarketSnapshot>::new(),
    ));
    let instance = Arc::new(Instance::new(
        "inst_test".to_string(),
        ("BTC".to_string(), "USDT".to_string()),
        pair.clone(),
        pool.clone(),
        config.clone(),
        Default::default(),
        Default::default(),
        TimeframeBuffers { history: pair.micro.history.clone(), latest: pair.micro.latest_snapshot.clone(), snapshot_history: snap_hist.clone() },
        TimeframeBuffers { history: pair.fast.history.clone(), latest: pair.fast.latest_snapshot.clone(), snapshot_history: snap_hist.clone() },
        TimeframeBuffers { history: pair.slow.history.clone(), latest: pair.slow.latest_snapshot.clone(), snapshot_history: snap_hist.clone() },
        TimeframeBuffers { history: pair.r#macro.history.clone(), latest: pair.r#macro.latest_snapshot.clone(), snapshot_history: snap_hist.clone() },
        Default::default(),
    ));
    workspace
        .instances
        .write()
        .await
        .insert("BTC-USDT".to_string(), instance);

    let state = Arc::new(AppState {
        workspace,
        config,
        pool: pool.clone(),
        llm_client: Arc::new(LlmClient::from_env().0),
        api_key_configured,
        symbol_mapper,
        telemetry_tx,
        ws_url: ws_url.clone(),
        bitget_ws_url: ws_url,
    });

    (state, pool)
}

fn build_pipeline(
    history: std::collections::VecDeque<shared::normalized::NormalizedCandle>,
    snap_history: std::collections::VecDeque<MarketSnapshot>,
    bcast: broadcast::Sender<MarketSnapshot>,
) -> TimeframePipeline {
    TimeframePipeline {
        history: Arc::new(RwLock::new(history)),
        broadcast_tx: bcast,
        latest_snapshot: Arc::new(RwLock::new(None)),
        snapshot_history: Arc::new(RwLock::new(snap_history)),
        timeframe_secs: 60,
        timeframe_label: "Micro",
        divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
        sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.3))),
        fibonacci: FibonacciConfig::default(),
    }
}

fn build_pipeline_empty(bcast: broadcast::Sender<MarketSnapshot>, tf_secs: u64) -> TimeframePipeline {
    TimeframePipeline {
        history: Arc::new(RwLock::new(std::collections::VecDeque::new())),
        broadcast_tx: bcast,
        latest_snapshot: Arc::new(RwLock::new(None)),
        snapshot_history: Arc::new(RwLock::new(std::collections::VecDeque::new())),
        timeframe_secs: tf_secs,
        timeframe_label: "Micro",
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
                "current_price": 51000.0,
                "volume": 150.0,
                "average_volume": 120.0,
                "indicators": {
                    "rsi": { "raw_value": 62.5, "normalized": -0.44, "state_label": "BEARISH_PREMIUM" },
                    "macd": {
                        "raw_value": 5.0, "normalized": 0.55,
                        "state_label": "BULLISH_MOMENTUM_EXPANDING",
                        "values": { "line": 15.0, "signal": 10.0, "histogram": 5.0, "histogram_peak": 8.0 }
                    },
                    "ema_stack": {
                        "raw_value": 51000.0, "normalized": 1.0,
                        "state_label": "ESTABLISHED_BULLISH_STACK",
                        "values": { "fast": 50900.0, "medium": 50500.0, "slow": 50000.0, "long": 49500.0 }
                    },
                    "adx": {
                        "raw_value": 28.0, "normalized": 0.6, "state_label": "STRONG_BULL_TREND",
                        "values": { "adx": 28.0, "plus_di": 30.0, "minus_di": 18.0, "adx_slope": 1.5 }
                    },
                    "rvol": { "raw_value": 1.25, "normalized": 0.2, "state_label": "NORMAL_PARTICIPATION_VOLUME" }
                }
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
        assert_eq!(
            status,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "Should return 503 when no API key configured"
        );

        // Verify master record was created in DB (use raw query since query_master_records filters PENDING)
        let row: (i64, String, String) = sqlx::query_as(
            "SELECT id, position, symbol FROM master_assistant_records ORDER BY id DESC LIMIT 1",
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
        assert!(
            response.status().is_success(),
            "History endpoint should succeed"
        );

        // Read response body
        let body = response.collect().await.unwrap().aggregate();
        let history_resp: serde_json::Value =
            serde_json::from_reader(body.reader()).unwrap_or_default();
        let candles = history_resp["candles"]
            .as_array()
            .expect("History response should have a 'candles' array");
        assert!(
            !candles.is_empty(),
            "History should return pre-populated candles"
        );
        assert!(
            candles.len() >= 50,
            "Should have at least 50 candles, got {}",
            candles.len()
        );

        // Nested schema: indicator_history carries a map-driven `indicators`
        // object plus core parameters (symbol, timeframe_secs, times).
        let ih = &history_resp["indicator_history"];
        assert!(ih["indicators"].is_object(), "indicator_history.indicators must be an object");
        assert!(ih["times"].is_array(), "indicator_history.times must be an array");
        assert_eq!(ih["timeframe_secs"], 60);
    })
    .await
    .expect("E2E history test timed out");
}
