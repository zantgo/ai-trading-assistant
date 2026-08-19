// v7.1 — mode toggle integration test: POST /api/instances/:id/mode
// switches the engine paper ↔ live, requires an active API key for live,
// and persists the mode into the workspace config.

use api_gateway::AppState;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use config_models::ExecutionMode;
use portfolio_supervisor::instance::Instance;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

async fn build_state() -> (Arc<AppState>, Arc<Instance>) {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("mem pool");
    database_storage::run_migrations(&pool)
        .await
        .expect("migrations");
    database_storage::crypto::init_master_key("mode-test-secret");

    let instance = Arc::new(Instance::new_test(
        "inst_test".to_string(),
        ("BTC".to_string(), "USDT".to_string()),
        portfolio_supervisor::instance::TimeframeBuffers::new(),
    ));
    let workspace = portfolio_supervisor::workspace_state::WorkspaceState::empty();
    workspace
        .insert("BTC-USDT".to_string(), instance.clone())
        .await;
    // Seed the workspace config so the mode toggle has an entry to persist.
    {
        let mut cfg = config_models::WorkspaceConfig::default();
        cfg.default_exchange = "Hyperliquid".to_string();
        cfg.instances.push(config_models::InstanceEntry {
            id: "inst_test".to_string(),
            symbol: "BTC-USDT".to_string(),
            quote: "USDT".to_string(),
            initial_capital_usd: 1000.0,
            status: config_models::InstanceStatus::Running,
            micro_term: config_models::TimeframeConfig::new(
                60,
                config_models::IndicatorsConfig::default(),
            ),
            fast_term: config_models::TimeframeConfig::new(
                180,
                config_models::IndicatorsConfig::default(),
            ),
            slow_term: None,
            macro_term: None,
            automation: Default::default(),
            operational_mode: Default::default(),
            mode: config_models::ExecutionMode::Paper,
            weight_overrides: None,
            position_scaling: None,
            activation: None,
            custom_pipelines: Default::default(),
        });
        workspace.set_config(cfg).await;
    }

    // Seed an active Hyperliquid key.
    let secret_enc = database_storage::crypto::encrypt_field("0xdeadbeef").unwrap();
    sqlx::query(
        "INSERT INTO exchange_keys (exchange, account_name, api_key, api_secret, is_active) \
         VALUES ('Hyperliquid', 'main', '0xabc', ?, 1)",
    )
    .bind(&secret_enc)
    .execute(&pool)
    .await
    .unwrap();

    let state = Arc::new(AppState {
        workspace,
        session: Arc::new(portfolio_supervisor::session::SessionState::new()),
        platform: Arc::new(RwLock::new(config_models::PlatformConfig::default())),
        pool,
        symbol_mapper: Arc::new(core_domain::normalized::SymbolMapper::new()),
        telemetry_tx: tokio::sync::mpsc::channel::<database_storage::TelemetryMsg>(100).0,
        connection_quality: Arc::new(
            network_adapters::connection_quality_tracker::ConnectionQualityRegistry::new(),
        ),
        ws_url: "ws://127.0.0.1:1".to_string(),
        bitget_ws_url: "".to_string(),
        clock_monitor: None,
        reliability: Arc::new(network_adapters::pipeline_reliability::ReliabilityTracker::new()),
        exchange_status: Arc::new(
            network_adapters::exchange_status_tracker::ExchangeStatusTracker::new(),
        ),
        latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
        overview: Arc::new(RwLock::new(None)),
        automation: None,
        execution_engine: Arc::new(portfolio_supervisor::execution::ExecutionEngine::new(
            portfolio_supervisor::paper_trading::FeesConfig::default(),
        )),
        recharge_tx: tokio::sync::broadcast::channel::<api_gateway::RechargeNotice>(64).0,
        snapshot_export: Arc::new(RwLock::new(
            core_domain::snapshot_export::SnapshotExportRuntime::default(),
        )),
        snapshot_export_manual_tick: Arc::new(tokio::sync::Notify::new()),
    });
    (state, instance)
}

async fn post_mode(state: Arc<AppState>, mode: &str) -> axum::response::Response {
    let router = api_gateway::build_router(state);
    router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/instances/inst_test/mode")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"mode":"{}"}}"#, mode)))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn mode_toggle_switches_engine_and_persists() {
    let (state, _inst) = build_state().await;
    assert_eq!(state.execution_engine.mode().await, ExecutionMode::Paper);

    // Live requires a key — present, so it succeeds and the engine flips.
    let resp = post_mode(state.clone(), "live").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(state.execution_engine.mode().await, ExecutionMode::Live);

    // Persisted in the workspace config.
    let cfg = state.workspace.config().await;
    let entry = cfg.instances.iter().find(|i| i.id == "inst_test").unwrap();
    assert_eq!(entry.mode, ExecutionMode::Live);

    // Toggle back to paper.
    let resp = post_mode(state.clone(), "paper").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(state.execution_engine.mode().await, ExecutionMode::Paper);
    let cfg = state.workspace.config().await;
    let entry = cfg.instances.iter().find(|i| i.id == "inst_test").unwrap();
    assert_eq!(entry.mode, ExecutionMode::Paper);
}

#[tokio::test]
async fn mode_live_requires_key() {
    // Build a state WITHOUT a seeded key.
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    database_storage::run_migrations(&pool).await.unwrap();
    database_storage::crypto::init_master_key("no-key-secret");
    let instance = Arc::new(Instance::new_test(
        "inst_test".to_string(),
        ("BTC".to_string(), "USDT".to_string()),
        portfolio_supervisor::instance::TimeframeBuffers::new(),
    ));
    let workspace = portfolio_supervisor::workspace_state::WorkspaceState::empty();
    workspace
        .insert("BTC-USDT".to_string(), instance.clone())
        .await;
    // Seed the workspace config so the mode toggle has an entry to persist.
    {
        let mut cfg = config_models::WorkspaceConfig::default();
        cfg.default_exchange = "Hyperliquid".to_string();
        cfg.instances.push(config_models::InstanceEntry {
            id: "inst_test".to_string(),
            symbol: "BTC-USDT".to_string(),
            quote: "USDT".to_string(),
            initial_capital_usd: 1000.0,
            status: config_models::InstanceStatus::Running,
            micro_term: config_models::TimeframeConfig::new(
                60,
                config_models::IndicatorsConfig::default(),
            ),
            fast_term: config_models::TimeframeConfig::new(
                180,
                config_models::IndicatorsConfig::default(),
            ),
            slow_term: None,
            macro_term: None,
            automation: Default::default(),
            operational_mode: Default::default(),
            mode: config_models::ExecutionMode::Paper,
            weight_overrides: None,
            position_scaling: None,
            activation: None,
            custom_pipelines: Default::default(),
        });
        workspace.set_config(cfg).await;
    }
    let state = Arc::new(AppState {
        workspace,
        session: Arc::new(portfolio_supervisor::session::SessionState::new()),
        platform: Arc::new(RwLock::new(config_models::PlatformConfig::default())),
        pool,
        symbol_mapper: Arc::new(core_domain::normalized::SymbolMapper::new()),
        telemetry_tx: tokio::sync::mpsc::channel::<database_storage::TelemetryMsg>(100).0,
        connection_quality: Arc::new(
            network_adapters::connection_quality_tracker::ConnectionQualityRegistry::new(),
        ),
        ws_url: "ws://127.0.0.1:1".to_string(),
        bitget_ws_url: "".to_string(),
        clock_monitor: None,
        reliability: Arc::new(network_adapters::pipeline_reliability::ReliabilityTracker::new()),
        exchange_status: Arc::new(
            network_adapters::exchange_status_tracker::ExchangeStatusTracker::new(),
        ),
        latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
        overview: Arc::new(RwLock::new(None)),
        automation: None,
        execution_engine: Arc::new(portfolio_supervisor::execution::ExecutionEngine::new(
            portfolio_supervisor::paper_trading::FeesConfig::default(),
        )),
        recharge_tx: tokio::sync::broadcast::channel::<api_gateway::RechargeNotice>(64).0,
        snapshot_export: Arc::new(RwLock::new(
            core_domain::snapshot_export::SnapshotExportRuntime::default(),
        )),
        snapshot_export_manual_tick: Arc::new(tokio::sync::Notify::new()),
    });

    let resp = post_mode(state.clone(), "live").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(state.execution_engine.mode().await, ExecutionMode::Paper);
}

#[tokio::test]
async fn mode_observe_gates_instance_without_key() {
    let (state, inst) = build_state().await;

    // Observe requires no API key and never routes to a venue broker.
    let resp = post_mode(state.clone(), "observe").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(inst.execution_mode().await, ExecutionMode::Observe);
    // Engine backend stays on simulation — observe never dispatches orders.
    assert_eq!(state.execution_engine.mode().await, ExecutionMode::Paper);

    // Persisted in the workspace config.
    let cfg = state.workspace.config().await;
    let entry = cfg.instances.iter().find(|i| i.id == "inst_test").unwrap();
    assert_eq!(entry.mode, ExecutionMode::Observe);

    // Back to paper restores the runtime gate.
    let resp = post_mode(state.clone(), "paper").await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(inst.execution_mode().await, ExecutionMode::Paper);
}

#[tokio::test]
async fn mode_rejects_unknown_values() {
    let (state, _inst) = build_state().await;
    let resp = post_mode(state.clone(), "monitor").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
