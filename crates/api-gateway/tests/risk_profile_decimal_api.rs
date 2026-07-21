//! Regression test: `/api/risk-profiles` must serialize Decimal fields as JSON
//! strings (not JSON numbers), preserving full precision end-to-end.
//!
//! Pre-migration bug: `commission_pct` serialized as `0.060000000000000005`
//! because the `REAL` SQLite column + default Decimal serde derive → float JSON.

use database_storage;
use api_gateway::{self, AppState};
use rust_decimal::Decimal;
use core_domain::normalized::SymbolMapper;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use portfolio_supervisor::workspace_state::WorkspaceState;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, RwLock};
use network_adapters::clock_monitor::ClockMonitor;
use network_adapters::pipeline_reliability::ReliabilityTracker;
use network_adapters::exchange_status_tracker::ExchangeStatusTracker;

async fn setup_test_state_with_decimal_profile() -> (Arc<AppState>, SqlitePool, String) {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite database");

    database_storage::run_migrations(&pool)
        .await
        .expect("migrations should succeed on fresh in-memory db");

    let symbol_mapper = Arc::new(SymbolMapper::new());
    symbol_mapper
        .register(core_domain::normalized::Exchange::Hyperliquid, "BTC", "BTC")
        .await;

    let (telemetry_tx, telemetry_rx) = mpsc::channel::<database_storage::TelemetryMsg>(100);

    let logger_pool = pool.clone();
    tokio::spawn(async move {
        database_storage::run_telemetry_logger(logger_pool, telemetry_rx, 90).await;
    });

    database_storage::risk_profile_insert(
        &pool,
        "decimal-precision-profile",
        Decimal::from_str("12345.678901234567890123456789").unwrap(),
        Decimal::from_str("2.5").unwrap(),
        20,
        Decimal::from_str("0.06").unwrap(),
        Decimal::from_str("0.01").unwrap(),
        Decimal::from_str("0.05").unwrap(),
    )
    .await;

    let state = Arc::new(AppState {
        workspace: portfolio_supervisor::workspace_state::WorkspaceState::empty(),
        platform: Arc::new(RwLock::new(config_models::PlatformConfig::default())),
        session: Arc::new(portfolio_supervisor::session::SessionState::new()),
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
        recharge_tx: broadcast::channel::<api_gateway::RechargeNotice>(64).0,
    });

    let router = api_gateway::build_router(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (state, pool, format!("http://{}", addr))
}

#[tokio::test]
async fn risk_profile_api_returns_decimals_as_strings() {
    let (_state, _pool, base_url) = setup_test_state_with_decimal_profile().await;

    let res = reqwest::get(format!("{}/api/risk-profiles", base_url))
        .await
        .expect("GET /api/risk-profiles");
    assert!(res.status().is_success(), "expected 200, got {}", res.status());

    let body: serde_json::Value = res.json().await.expect("response is JSON");
    let profiles = body.as_array().expect("response is an array");
    assert_eq!(profiles.len(), 1, "expected one seeded profile");

    let profile = &profiles[0];
    assert_eq!(profile["profile_name"], "decimal-precision-profile");

    assert!(profile["capital"].is_string(), "capital must be a JSON string");
    assert!(profile["max_risk_pct"].is_string(), "max_risk_pct must be a JSON string");
    assert!(profile["commission_pct"].is_string(), "commission_pct must be a JSON string");
    assert!(profile["funding_rate_8h"].is_string(), "funding_rate_8h must be a JSON string");
    assert!(profile["spread"].is_string(), "spread must be a JSON string");

    assert_eq!(
        profile["commission_pct"].as_str().unwrap(),
        "0.06",
        "commission_pct must be the exact Decimal string, not the f64 artifact"
    );
    assert_eq!(
        profile["max_risk_pct"].as_str().unwrap(),
        "2.5"
    );
}

#[tokio::test]
async fn risk_profile_round_trip_preserves_full_decimal_precision() {
    let (_state, pool, _base_url) = setup_test_state_with_decimal_profile().await;

    let profiles = database_storage::risk_profiles_list(&pool).await;
    assert_eq!(profiles.len(), 1);

    let original = Decimal::from_str("12345.678901234567890123456789").unwrap();
    let read_back = profiles[0].capital;
    assert_eq!(read_back, original, "30-digit Decimal must round-trip exactly");
}

#[tokio::test]
async fn risk_profile_insert_then_read_returns_text_columns() {
    let (_state, pool, _base_url) = setup_test_state_with_decimal_profile().await;

    let row: (String, String, String) = sqlx::query_as(
        "SELECT capital, max_risk_pct, commission_pct FROM risk_profiles WHERE profile_name = 'decimal-precision-profile'"
    )
    .fetch_one(&pool)
    .await
    .expect("row should exist");

    assert_eq!(row.0, "12345.678901234567890123456789");
    assert_eq!(row.1, "2.5");
    assert_eq!(row.2, "0.06");
}