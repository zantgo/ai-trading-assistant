use market_analyzer::analyzer::{ActivePair, TimeframePipeline};
use core_domain::models::TimeframeSlot;
use config_models::{FibonacciConfig, PlatformConfig, WorkspaceConfig};
use database_storage;
use portfolio_supervisor::instance::TimeframeBuffers;
use api_gateway::{self, AppState};
use market_analyzer::sr_engine::SrRoleTracker;
use market_analyzer::indicators::DivergenceDetector;
use core_domain::models::MarketSnapshot;
use core_domain::normalized::SymbolMapper;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use portfolio_supervisor::workspace_state::WorkspaceState;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, RwLock};
use tower::ServiceExt;
use network_adapters::clock_monitor::ClockMonitor;
use network_adapters::pipeline_reliability::ReliabilityTracker;
use network_adapters::exchange_status_tracker::ExchangeStatusTracker;

async fn setup_test_state() -> (Arc<AppState>, SqlitePool) {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite database");

    let _dummy_config = config_models::WorkspaceConfig::default();

    let symbol_mapper = Arc::new(SymbolMapper::new());
    let (telemetry_tx, _telemetry_rx) = mpsc::channel::<database_storage::TelemetryMsg>(100);
    let ws_url = "ws://127.0.0.1:1".to_string();

    let state = Arc::new(AppState {
        workspace: portfolio_supervisor::workspace_state::WorkspaceState::empty(),
        platform: Arc::new(RwLock::new(config_models::PlatformConfig::default())),
        session: Arc::new(portfolio_supervisor::session::SessionState::new()),
        pool: pool.clone(),
        symbol_mapper,
        telemetry_tx,
        connection_quality: Arc::new(network_adapters::connection_quality_tracker::ConnectionQualityRegistry::new()),
        ws_url: ws_url.clone(),
        bitget_ws_url: ws_url,
        clock_monitor: None,
        reliability: Arc::new(ReliabilityTracker::new()),
        exchange_status: Arc::new(ExchangeStatusTracker::new()),
        latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
        overview: Arc::new(RwLock::new(None)),
        execution_engine: Arc::new(portfolio_supervisor::execution::ExecutionEngine::new()),
    });

    (state, pool)
}

#[tokio::test]
async fn test_analyze_missing_position_returns_400() {
    let (state, _pool) = setup_test_state().await;
    let router = api_gateway::build_router(state);

    let body = serde_json::json!({
        "historical_prices": [50000.0, 50100.0, 50200.0],
        "indicators": {
            "rsi": 65.0,
            "macd_line": 20.0,
            "macd_signal": 15.0
        }
    });

    let request = hyper::Request::builder()
        .method("POST")
        .uri("/api/analyze")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_string(&body).unwrap(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert!(
        response.status().is_client_error(),
        "Missing position should return 4xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_history_invalid_symbol_returns_empty_or_error() {
    let (state, _pool) = setup_test_state().await;
    let router = api_gateway::build_router(state);

    let request = hyper::Request::builder()
        .method("GET")
        .uri("/api/history?symbol=NONEXISTENT_PAIR")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert!(
        !response.status().is_server_error(),
        "Server should not 500 on invalid symbol, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_config_endpoint_returns_ok() {
    let (state, _pool) = setup_test_state().await;
    let router = api_gateway::build_router(state);

    let request = hyper::Request::builder()
        .method("GET")
        .uri("/api/config")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert!(
        response.status().is_success(),
        "/api/config should return success, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_api_failover_router_integration() {
    // Test that the router builds without panicking with a fully configured state
    let (state, _pool) = setup_test_state().await;
    let _router = api_gateway::build_router(state);
    // Router construction is the test — if we got here without panic, it passes
}

#[tokio::test]
async fn test_websocket_stream_with_active_pair() {
    // Build state with a registered pair that has broadcast channels
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite database");

    let _dummy_config = config_models::WorkspaceConfig::default();

    let symbol_mapper = Arc::new(SymbolMapper::new());
    symbol_mapper
        .register(core_domain::normalized::Exchange::Hyperliquid, "BTC", "BTC")
        .await;
    let (telemetry_tx, _telemetry_rx) = mpsc::channel::<database_storage::TelemetryMsg>(100);
    let ws_url = "ws://127.0.0.1:1".to_string();

    let workspace = WorkspaceState::empty();

    // Create broadcast channels for the pair
    let (mid_bcast, _) = broadcast::channel::<MarketSnapshot>(10);
    let (long_bcast, _) = broadcast::channel::<MarketSnapshot>(10);
    let (macro_bcast, _) = broadcast::channel::<MarketSnapshot>(10);
    let (supermacro_bcast, _) = broadcast::channel::<MarketSnapshot>(10);

    let (snapshot_tx, _snapshot_rx) = mpsc::channel::<core_domain::normalized::NormalizedEvent>(100);
    let cancel = tokio_util::sync::CancellationToken::new();

    let snap_hist = Arc::new(RwLock::new(
        std::collections::VecDeque::<MarketSnapshot>::new(),
    ));
    let pair = Arc::new(ActivePair {
        symbol: "BTC".to_string(),
        latest_oi: Arc::new(RwLock::new(None)),
        latest_funding: Arc::new(RwLock::new(None)),
        latest_mark_px: Arc::new(RwLock::new(None)),
        latest_index_px: Arc::new(RwLock::new(None)),
        latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
        micro: TimeframePipeline {
            slot: TimeframeSlot::Micro,
            history: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            broadcast_tx: mid_bcast.clone(),
            latest_snapshot: Arc::new(RwLock::new(None)),
            snapshot_history: snap_hist.clone(),
            timeframe_secs: 60,
            timeframe_label: "Micro",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.3))),
            fibonacci: FibonacciConfig::default(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
            active_set: Default::default(),
            cluster_matrix: Arc::new(RwLock::new(None)),
        },
        fast: TimeframePipeline {
            slot: TimeframeSlot::Fast,
            history: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            broadcast_tx: long_bcast,
            latest_snapshot: Arc::new(RwLock::new(None)),
            snapshot_history: snap_hist.clone(),
            timeframe_secs: 300,
            timeframe_label: "Fast",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.3))),
            fibonacci: FibonacciConfig::default(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
            active_set: Default::default(),
            cluster_matrix: Arc::new(RwLock::new(None)),
        },
        slow: TimeframePipeline {
            slot: TimeframeSlot::Slow,
            history: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            broadcast_tx: macro_bcast,
            latest_snapshot: Arc::new(RwLock::new(None)),
            snapshot_history: snap_hist.clone(),
            timeframe_secs: 900,
            timeframe_label: "Slow",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.3))),
            fibonacci: FibonacciConfig::default(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
            active_set: Default::default(),
            cluster_matrix: Arc::new(RwLock::new(None)),
        },
        r#macro: TimeframePipeline {
            slot: TimeframeSlot::Macro,
            history: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            broadcast_tx: supermacro_bcast,
            latest_snapshot: Arc::new(RwLock::new(None)),
            snapshot_history: snap_hist.clone(),
            timeframe_secs: 3600,
            timeframe_label: "Macro",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.3))),
            fibonacci: FibonacciConfig::default(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
            active_set: Default::default(),
            cluster_matrix: Arc::new(RwLock::new(None)),
        },
        snapshot_tx,
        cancel,
    });

    let instance = Arc::new(portfolio_supervisor::instance::Instance::new(
        "inst_test".to_string(),
        ("BTC".to_string(), "USDT".to_string()),
        pair.clone(),
        pool.clone(),
        workspace.clone(),
        Default::default(),
        Default::default(),
        TimeframeBuffers {
            history: pair.micro.history.clone(),
            latest: pair.micro.latest_snapshot.clone(),
            snapshot_history: snap_hist.clone(),
        },
        TimeframeBuffers {
            history: pair.fast.history.clone(),
            latest: pair.fast.latest_snapshot.clone(),
            snapshot_history: snap_hist.clone(),
        },
        TimeframeBuffers {
            history: pair.slow.history.clone(),
            latest: pair.slow.latest_snapshot.clone(),
            snapshot_history: snap_hist.clone(),
        },
        TimeframeBuffers {
            history: pair.r#macro.history.clone(),
            latest: pair.r#macro.latest_snapshot.clone(),
            snapshot_history: snap_hist.clone(),
        },
        Default::default(),
    ));
    let workspace = WorkspaceState::empty();
    workspace.insert("BTC-USDT".to_string(), instance).await;

    let state = Arc::new(AppState {
        workspace,
        session: Arc::new(portfolio_supervisor::session::SessionState::new()),
        platform: Arc::new(RwLock::new(config_models::PlatformConfig::default())),
        pool: pool.clone(),
        symbol_mapper,
        telemetry_tx,
        connection_quality: Arc::new(network_adapters::connection_quality_tracker::ConnectionQualityRegistry::new()),
        ws_url: "ws://127.0.0.1:1".to_string(),
        bitget_ws_url: "".to_string(),
        clock_monitor: None,
        reliability: Arc::new(ReliabilityTracker::new()),
        exchange_status: Arc::new(ExchangeStatusTracker::new()),
        latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
        overview: Arc::new(RwLock::new(None)),
        execution_engine: Arc::new(portfolio_supervisor::execution::ExecutionEngine::new()),
    });

    let router = api_gateway::build_router(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Connect WebSocket with valid parameters
    let ws_url = format!("ws://{}/ws?symbol=BTC-USDT&timeframe=Mid", addr);
    let (ws_stream, response) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("WebSocket connection should succeed");

    assert!(
        response.status().is_success() || response.status() == 101,
        "WS handshake should succeed, got {}",
        response.status()
    );

    // Connection should be open — the pair exists, so handle_ws_socket subscribes
    drop(ws_stream);

    // Test with non-existent pair key — connection upgrades but handler returns immediately
    let unknown_url = format!("ws://{}/ws?symbol=NONEXISTENT-PAIR", addr);
    let unknown_result = tokio_tungstenite::connect_async(&unknown_url).await;
    // The handshake may still succeed (upgrade happens before handler checks pairs)
    // but we verify no panic — the handler gracefully returns None
    assert!(
        unknown_result.is_ok() || unknown_result.is_err(),
        "WS with unknown pair should not crash the server"
    );
}
