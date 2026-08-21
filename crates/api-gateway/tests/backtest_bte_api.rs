// v8 BTE — archive backfill + coverage endpoint tests.
//
// Validation paths (depth bounds, instance-not-found / not-running,
// duplicate-job 409) are fully covered without network. The 200 path
// spawns the production pager as a detached task; the assertions only
// cover the endpoint contract (job id + live progress), never the
// network result.

use api_gateway::AppState;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use portfolio_supervisor::instance::{Instance, InstanceStatus};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

async fn build_state() -> Arc<AppState> {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("mem pool");
    database_storage::run_migrations(&pool)
        .await
        .expect("migrations");

    let instance = Arc::new(Instance::new_test(
        "inst_bte".to_string(),
        ("BTC".to_string(), "USDC".to_string()),
        portfolio_supervisor::instance::TimeframeBuffers::new(),
    ));

    let workspace = portfolio_supervisor::workspace_state::WorkspaceState::empty();
    workspace
        .insert("BTC-USDC".to_string(), instance.clone())
        .await;

    Arc::new(AppState {
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
        backtest: Arc::new(backtesting_engine::registry::BacktestRegistry::new()),
    })
}

async fn post_json(
    state: Arc<AppState>,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let router = api_gateway::build_router(state);
    let resp = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap_or_default();
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

async fn get_json(state: Arc<AppState>, uri: &str) -> (StatusCode, serde_json::Value) {
    let router = api_gateway::build_router(state);
    let resp = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap_or_default();
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

#[tokio::test]
async fn backfill_validates_depth_bounds() {
    let state = build_state().await;
    for bad in [0u32, 366u32] {
        let (status, json) = post_json(
            state.clone(),
            "/api/backtest/archive/backfill",
            serde_json::json!({ "instance_id": "inst_bte", "depth_days": bad }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "depth {bad} must be rejected");
        assert_eq!(json["code"], "invalid_depth");
    }
}

#[tokio::test]
async fn backfill_requires_running_instance() {
    let state = build_state().await;

    // Unknown instance.
    let (status, json) = post_json(
        state.clone(),
        "/api/backtest/archive/backfill",
        serde_json::json!({ "instance_id": "nope", "depth_days": 7 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "instance_not_found");

    // Known instance, stopped.
    if let Some(inst) = state
        .workspace
        .list()
        .await
        .into_iter()
        .find(|i| i.id == "inst_bte")
    {
        inst.set_status(InstanceStatus::Stopped).await;
    }
    let (status, json) = post_json(
        state.clone(),
        "/api/backtest/archive/backfill",
        serde_json::json!({ "instance_id": "inst_bte", "depth_days": 7 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "instance_not_running");
}

#[tokio::test]
async fn backfill_rejects_duplicate_active_job() {
    let state = build_state().await;

    // Pre-register a running job in the in-memory registry (no network).
    let progress = Arc::new(tokio::sync::Mutex::new(
        backtesting_engine::backfill::BackfillProgress::new(
            1,
            "inst_bte".into(),
            "BTC-USDC".into(),
            "Hyperliquid".into(),
            7,
        ),
    ));
    {
        let mut map = state.backtest.backfills.write().await;
        map.insert(
            1,
            backtesting_engine::registry::TrackedBackfill {
                progress,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );
    }

    let (status, json) = post_json(
        state.clone(),
        "/api/backtest/archive/backfill",
        serde_json::json!({ "instance_id": "inst_bte", "depth_days": 7 }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(json["code"], "backfill_busy");
}

#[tokio::test]
async fn backfill_start_and_progress_contract() {
    let state = build_state().await;

    let (status, json) = post_json(
        state.clone(),
        "/api/backtest/archive/backfill",
        serde_json::json!({ "instance_id": "inst_bte", "depth_days": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json);
    let job_id = json["job_id"].as_i64().expect("job id");

    // Live progress endpoint answers while the detached job runs.
    let (pstatus, pjson) = get_json(state.clone(), &format!("/api/backtest/archive/progress/{job_id}"))
        .await;
    assert_eq!(pstatus, StatusCode::OK);
    assert_eq!(pjson["job_id"], job_id);
    assert_eq!(pjson["instance_id"], "inst_bte");
    assert_eq!(pjson["depth_days"], 1);
    assert!(
        matches!(pjson["status"].as_str(), Some("running") | Some("done") | Some("failed")),
        "unexpected status: {}",
        pjson["status"]
    );

    // Cancel endpoint answers.
    let (cstatus, _) = post_json(
        state.clone(),
        &format!("/api/backtest/archive/cancel/{job_id}"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(cstatus, StatusCode::OK);

    // Unknown job → 404.
    let (nstatus, _) = get_json(state.clone(), "/api/backtest/archive/progress/999999").await;
    assert_eq!(nstatus, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn coverage_returns_extended_shape() {
    let state = build_state().await;

    let (status, json) = get_json(state.clone(), "/api/backtest/coverage?instance_id=inst_bte").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["archive_depth_days"].as_i64().unwrap_or(0) >= 1);
    assert!(json["snapshots"].is_array(), "recorded-snapshot rows present");
    assert!(json["archive"].is_array(), "archive rows present");
    assert!(json["backfill_jobs"].is_array(), "job list present");
    // v8.1: the data-prep contract — burn-in + the four-timeframe ladder.
    assert!(json["burn_in_secs"].as_i64().unwrap_or(0) > 0, "burn_in_secs present");
    let ladder = json["ladder"].as_array().expect("ladder present");
    assert_eq!(ladder.len(), 4, "micro/fast/slow/macro ladder");
}

#[tokio::test]
async fn run_is_rejected_while_another_run_holds_the_lock() {
    let state = build_state().await;

    let _guard = state.backtest.run_lock.lock().await;
    let (status, json) = post_json(
        state.clone(),
        "/api/backtest/run",
        serde_json::json!({
            "symbol": "BTC-USDC",
            "timeframe_secs": 60,
            "from_ms": 0,
            "to_ms": 10_000,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", json);
    assert_eq!(json["code"], "backtest_busy");
}

#[tokio::test]
async fn historical_mode_requires_instance_id() {
    let state = build_state().await;

    let (status, json) = post_json(
        state.clone(),
        "/api/backtest/run",
        serde_json::json!({
            "symbol": "BTC-USDC",
            "timeframe_secs": 60,
            "from_ms": 1_000_000,
            "to_ms": 1_010_000,
            "mode": "historical",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "instance_required");
}

#[tokio::test]
async fn recorded_run_persists_ds_rows() {
    let state = build_state().await;

    // Seed one recorded snapshot so the recorded path passes validation.
    let snap = core_domain::models::MarketSnapshot {
        symbol: "BTC-USDC".to_string(),
        timeframe_secs: 60,
        timestamp: 1_000_000,
        is_completed: Some(true),
        mid_price: rust_decimal::Decimal::from_f64_retain(100.0).unwrap(),
        bid_price: rust_decimal::Decimal::from_f64_retain(100.0).unwrap(),
        ask_price: rust_decimal::Decimal::from_f64_retain(100.0).unwrap(),
        close: Some(rust_decimal::Decimal::from_f64_retain(100.0).unwrap()),
        ..core_domain::models::MarketSnapshot::default()
    };
    database_storage::insert_snapshot_internal(&state.pool, &snap).await;

    let (status, json) = post_json(
        state.clone(),
        "/api/backtest/run",
        serde_json::json!({
            "symbol": "BTC-USDC",
            "timeframe_secs": 60,
            "from_ms": 1_000_000_000,
            "to_ms": 1_010_000_000,
            "mode": "recorded",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json);
    let id = json["backtest_id"].as_i64().expect("id");
    assert_eq!(json["mode"], "recorded");

    // DS read endpoints answer for the run.
    let (s, t) = get_json(state.clone(), &format!("/api/backtest/{id}/trades")).await;
    assert_eq!(s, StatusCode::OK);
    assert!(t["trades"].is_array());
    let (s, e) = get_json(state.clone(), &format!("/api/backtest/{id}/equity")).await;
    assert_eq!(s, StatusCode::OK);
    assert!(e["equity"].is_array());
    let (s, m) = get_json(state.clone(), &format!("/api/backtest/{id}/metrics")).await;
    assert_eq!(s, StatusCode::OK);
    assert!(m["metrics"].is_array());
    // The run row carries the mode via the list endpoint.
    let (s, list) = get_json(state.clone(), "/api/backtest/list?limit=5").await;
    assert_eq!(s, StatusCode::OK);
    let first = &list[0];
    assert_eq!(first["mode"], "recorded");
}
