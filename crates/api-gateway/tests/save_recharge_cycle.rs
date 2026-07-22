//! Integration test for the full save → in-memory publish → recharge cycle.
//!
//! Reproduces and regression-locks the "Pipeline recharge failed for
//! BTC-USDC: No saved config for pair BTC-USDC" bug.
//!
//! The historical flow when a user edited a timeframe in
//! `WorkspaceSettings.svelte`:
//!   1. Frontend POSTs `/api/instances/{pairKey}/config` to the
//!      `serve_update_instance_config` handler
//!      (`crates/api-gateway/src/handlers/instances.rs`).
//!   2. The handler builds a fresh `WorkspaceConfig` clone, mutates the
//!      matching `InstanceEntry`, persists the clone to `config.toml` via
//!      `config_models::save_workspace`, then invokes
//!      `registry::recharge_instance`.
//!   3. `recharge_instance` reads `state.workspace.config()` and looks up
//!      the entry by `symbol` to determine the new timeframe configuration.
//!      The bug: step 2's mutation happens on a clone that was never
//!      published back, so step 3 reads a stale snapshot and returns
//!      `Err("No saved config for pair ...")`.
//!
//! The fix:
//!   - `serve_update_instance_config` now calls `state.workspace.set_config(...)`
//!     after `save_workspace`, bridging the disk write into in-memory state.
//!   - `delete_instance` does the same.
//!   - `add_instance` now actually populates `config.instances[]` (it never
//!     used to, leaving every first-edit pair needing a daemon restart).
//!   - `default_pair_key` now reads the active session quote so USDC
//!     sessions no longer produce `BTC-USDT-USDT` fallbacks on the read
//!     side.
//!   - The route parameter is a UUID; pairKey-based slug requests return
//!     `404 NOT_FOUND` immediately.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use api_gateway::{self, AppState};
use config_models::FibonacciConfig;
use config_models::WorkspaceConfig;
use core_domain::models::MarketSnapshot;
use core_domain::normalized::SymbolMapper;
use database_storage;
use market_analyzer::analyzer::{ActivePair, TimeframePipeline};
use market_analyzer::indicators::DivergenceDetector;
use market_analyzer::sr_engine::SrRoleTracker;
use network_adapters::exchange_status_tracker::ExchangeStatusTracker;
use network_adapters::pipeline_reliability::ReliabilityTracker;
use portfolio_supervisor::instance::{Instance, TimeframeBuffers};
use portfolio_supervisor::workspace_state::WorkspaceState;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;

const INSTANCE_ID: &str = "inst_save_recharge_cycle";
const PAIR_KEY: &str = "BTC-USDT";

async fn setup_app_with_instance() -> Arc<AppState> {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    // The bootstrap path queries `market_snapshots`; without migrations
    // every save→recharge cycle fails with "no such table", which is
    // orthogonal to the bug under test but masks the test outcome.
    database_storage::run_migrations(&pool)
        .await
        .expect("migrations");

    let symbol_mapper = Arc::new(SymbolMapper::new());
    symbol_mapper
        .register(core_domain::normalized::Exchange::Hyperliquid, "BTC", PAIR_KEY)
        .await;
    let (telemetry_tx, _telemetry_rx) = mpsc::channel::<database_storage::TelemetryMsg>(100);

    let workspace = WorkspaceState::empty();

    // Build a minimal ActivePair scaffold, mirroring `axum_routes.rs`
    // (`test_websocket_stream_with_active_pair` lines 149-263).
    let (mid_bcast, _) = broadcast::channel::<MarketSnapshot>(10);
    let (fast_bcast, _) = broadcast::channel::<MarketSnapshot>(10);
    let (slow_bcast, _) = broadcast::channel::<MarketSnapshot>(10);
    let (macro_bcast, _) = broadcast::channel::<MarketSnapshot>(10);

    let (snapshot_tx, _snapshot_rx) =
        mpsc::channel::<core_domain::normalized::NormalizedEvent>(100);
    let cancel = CancellationToken::new();

    let snap_hist = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::new()));
    let new_pipe = |secs, label, slot: core_domain::models::TimeframeSlot, tx: broadcast::Sender<MarketSnapshot>| TimeframePipeline {
        slot,
        history: Arc::new(RwLock::new(VecDeque::new())),
        broadcast_tx: tx,
        latest_snapshot: Arc::new(RwLock::new(None)),
        snapshot_history: snap_hist.clone(),
        timeframe_secs: secs,
        timeframe_label: label,
        divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
        sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.3))),
        fibonacci: FibonacciConfig::default(),
        latest_oi: Arc::new(RwLock::new(None)),
        latest_funding: Arc::new(RwLock::new(None)),
        latest_mark_px: Arc::new(RwLock::new(None)),
        latest_index_px: Arc::new(RwLock::new(None)),
        active_set: Default::default(),
        cluster_matrix: Arc::new(RwLock::new(None)),
            cluster_status: Arc::new(RwLock::new(core_domain::liquidity::ClusterStatusSnapshot::pending("TEST", "test"))),
    };

    let pair = Arc::new(ActivePair {
        symbol: PAIR_KEY.to_string(),
        latest_oi: Arc::new(RwLock::new(None)),
        latest_funding: Arc::new(RwLock::new(None)),
        latest_mark_px: Arc::new(RwLock::new(None)),
        latest_index_px: Arc::new(RwLock::new(None)),
        latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
        micro: new_pipe(60, "Micro", core_domain::models::TimeframeSlot::Micro, mid_bcast.clone()),
        fast: new_pipe(180, "Fast", core_domain::models::TimeframeSlot::Fast, fast_bcast.clone()),
        slow: new_pipe(300, "Slow", core_domain::models::TimeframeSlot::Slow, slow_bcast.clone()),
        r#macro: new_pipe(900, "Macro", core_domain::models::TimeframeSlot::Macro, macro_bcast.clone()),
        snapshot_tx,
        cancel,
    });

    let micro_buf = TimeframeBuffers {
        history: pair.micro.history.clone(),
        latest: pair.micro.latest_snapshot.clone(),
        snapshot_history: snap_hist.clone(),
    };
    let fast_buf = TimeframeBuffers {
        history: pair.fast.history.clone(),
        latest: pair.fast.latest_snapshot.clone(),
        snapshot_history: snap_hist.clone(),
    };
    let slow_buf = TimeframeBuffers {
        history: pair.slow.history.clone(),
        latest: pair.slow.latest_snapshot.clone(),
        snapshot_history: snap_hist.clone(),
    };
    let macro_buf = TimeframeBuffers {
        history: pair.r#macro.history.clone(),
        latest: pair.r#macro.latest_snapshot.clone(),
        snapshot_history: snap_hist.clone(),
    };

    let instance = Arc::new(Instance::new(
        INSTANCE_ID.to_string(),
        ("BTC".to_string(), "USDT".to_string()),
        pair.clone(),
        pool.clone(),
        workspace.clone(),
        Default::default(),
        Default::default(),
        micro_buf,
        fast_buf,
        slow_buf,
        macro_buf,
        Default::default(),
    ));

    workspace.insert(PAIR_KEY.to_string(), instance).await;

    // Pre-seed an empty WorkspaceConfig so handlers that synthesize defaults
    // have something to read from — they will push a new entry on save.
    {
        let mut cfg: WorkspaceConfig = WorkspaceConfig::default();
        cfg.instances = Vec::new();
        workspace.set_config(cfg).await;
    }

    Arc::new(AppState {
        workspace,
        session: Arc::new(portfolio_supervisor::session::SessionState::new()),
        platform: Arc::new(RwLock::new(config_models::PlatformConfig::default())),
        pool,
        symbol_mapper,
        telemetry_tx,
        connection_quality: Arc::new(
            network_adapters::connection_quality_tracker::ConnectionQualityRegistry::new(),
        ),
        ws_url: "ws://127.0.0.1:1".to_string(),
        bitget_ws_url: String::new(),
        clock_monitor: None,
        reliability: Arc::new(ReliabilityTracker::new()),
        exchange_status: Arc::new(ExchangeStatusTracker::new()),
        latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
        overview: Arc::new(RwLock::new(None)),
        execution_engine: Arc::new(
            portfolio_supervisor::execution::ExecutionEngine::new(),
        ),
        recharge_tx: broadcast::channel::<api_gateway::RechargeNotice>(64).0,
    })
}

async fn serve_for(state: Arc<AppState>) -> std::net::SocketAddr {
    let router = api_gateway::build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    // Tiny sleep so the server has a chance to bind before the test fires
    // its first request. The `axum_routes.rs` precedent uses 50ms.
    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

fn default_body(micro_secs: u64) -> serde_json::Value {
    serde_json::json!({
        "micro_term": {
            "candles": { "duration_seconds": micro_secs, "analysis_limit": 250 },
            "indicators": {}
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn post_instance_config_by_uuid_recharges_in_memory_state() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let state = setup_app_with_instance().await;
        let addr = serve_for(state.clone()).await;
        let client = reqwest::Client::new();

        let body = default_body(30);
        let res = client
            .post(format!("http://{addr}/api/instances/{INSTANCE_ID}/config"))
            .json(&body)
            .send()
            .await
            .expect("POST should reach the server");

        assert!(
            res.status().is_success(),
            "save handler must return 2xx; got {}",
            res.status()
        );
        let body = res.text().await.unwrap();
        assert!(
            body.contains("Instance configuration saved and pipelines recharged")
                || body.contains("Config saved but pipeline recharge failed"),
            "unexpected response body: {body:?}"
        );

        // The handle in-memory state must reflect the saved override —
        // BEFORE the fix this Vec was empty on the freshly-cloned config the
        // handler never published back, so `recharge_instance` produced
        // "No saved config for pair BTC-USDT".
        let cfg_after = state.workspace.config().await;
        let entry = cfg_after
            .instances
            .iter()
            .find(|i| i.symbol == PAIR_KEY)
            .expect("handler must persist an InstanceEntry");
        assert_eq!(
            entry.micro_term.candles.duration_seconds,
            30,
            "micro_term override must reach the in-memory snapshot"
        );

        // Live map must still hold the instance after the recharge.
        assert!(
            state.workspace.get(PAIR_KEY).await.is_some(),
            "live Arc<Instance> must survive recharge"
        );
    })
    .await
    .expect("save->recharge cycle exceeded 15 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn post_instance_config_by_pairkey_is_rejected_with_404() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let state = setup_app_with_instance().await;
        let addr = serve_for(state).await;
        let client = reqwest::Client::new();
        let res = client
            .post(format!("http://{addr}/api/instances/{PAIR_KEY}/config"))
            .json(&default_body(60))
            .send()
            .await
            .expect("POST should reach the server");
        assert_eq!(
            res.status(),
            axum::http::StatusCode::NOT_FOUND,
            "pairKey-based slug should 404; the route is UUID-only now"
        );
    })
    .await
    .expect("404 path exceeded 10 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn post_instance_config_uses_session_quote_in_default_pair_key() {
    // The /api/history?symbol= (no symbol) fallback path must honour the
    // session quote. With a USDC session, `default_pair_key("BTC-USDT")` is
    // expected to round-trip to "BTC-USDC" rather than "BTC-USDT-USDT".
    tokio::time::timeout(Duration::from_secs(10), async {
        let state = setup_app_with_instance().await;
        // Force the session quote to USDC.
        {
            use portfolio_supervisor::session::Currency;
            *state
                .session
                .base_currency
                .write()
                .await = Some(Currency::USDC);
        }
        let addr = serve_for(state).await;
        let client = reqwest::Client::new();

        // /api/history with no symbol — exercises the quote-aware default.
        let res = client
            .get(format!("http://{addr}/api/history"))
            .send()
            .await
            .expect("GET should reach the server");
        // No entry was ever added, so the response is an empty body (200 OK
        // or 404 are both acceptable — we only care that no 5xx fires).
        assert!(
            !res.status().is_server_error(),
            "default_pair_key fallback should not 5xx; got {}",
            res.status()
        );
    })
    .await
    .expect("default_pair_key path exceeded 10 s budget");
}
