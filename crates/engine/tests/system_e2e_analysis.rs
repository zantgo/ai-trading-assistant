use engine::config::AppConfig;
use engine::db;
use engine::llm::LlmClient;
use engine::server::{self, AppState};
use engine::workspace::Workspace;
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
        macro_timeframe: Default::default(), supermacro_timeframe: Default::default(),
        leverage: Default::default(), scoring: Default::default(),
        fees: Default::default(), costs: Default::default(),
        workspace: Default::default(), safety: Default::default(),
        intervals: Default::default(), api_failover: Default::default(),
        pairs: HashMap::new(),
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
    for i in 0..100i64 {
        let price = 50000.0 + i as f64 * 10.0;
        history.push_back(shared::normalized::NormalizedCandle {
            symbol: "BTC".to_string(),
            start_time_ms: (1000000 + i * 60000) as u64,
            duration_ms: 60000,
            open: dec!(50000.00) + rust_decimal::Decimal::from(i) * dec!(10.00),
            high: dec!(50100.00) + rust_decimal::Decimal::from(i) * dec!(10.00),
            low: dec!(49900.00) + rust_decimal::Decimal::from(i) * dec!(10.00),
            close: rust_decimal::Decimal::from_f64_retain(price).unwrap_or(dec!(50000.00)),
            volume: dec!(10.0),
            trades_count: 5,
        });
    }

    let (snapshot_tx, _) = mpsc::channel(100);
    let cancel = tokio_util::sync::CancellationToken::new();
    let b1 = mid_bcast.clone();
    let b2 = mid_bcast.clone();
    let b3 = mid_bcast.clone();
    let b4 = mid_bcast.clone();

    let pair = Arc::new(ActivePair {
        symbol: "BTC".to_string(),
        short: build_pipeline_empty(b1),
        mid: build_pipeline(history, mid_bcast),
        long: build_pipeline_empty(b2),
        r#macro: build_pipeline_empty(b3),
        supermacro: build_pipeline_empty(b4),
        snapshot_tx,
        cancel,
    });

    let mut pairs_map = HashMap::new();
    pairs_map.insert("Hyperliquid-BTC".to_string(), pair);

    let state = Arc::new(AppState {
        pairs: Arc::new(RwLock::new(pairs_map)),
        workspace, config, pool: pool.clone(),
        llm_client: Arc::new(RwLock::new(LlmClient::from_env().0)),
        api_key_configured, symbol_mapper, telemetry_tx,
        ws_url: "ws://127.0.0.1:1".to_string(),
    });

    (state, pool)
}

fn build_pipeline(history: std::collections::VecDeque<shared::normalized::NormalizedCandle>, bcast: broadcast::Sender<MarketSnapshot>) -> TimeframePipeline {
    TimeframePipeline {
        history: Arc::new(RwLock::new(history)),
        broadcast_tx: bcast,
        latest_snapshot: Arc::new(RwLock::new(None)),
        timeframe_secs: 60, timeframe_label: "Mid",
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
        timeframe_secs: 60, timeframe_label: "Mid",
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
            "symbol": "Hyperliquid-BTC"
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
        assert_eq!(row.2, "Hyperliquid-BTC");
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
            .uri("/api/history?symbol=Hyperliquid-BTC&timeframe_secs=60")
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
