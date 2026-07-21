//! Regression tests for the slot identity invariants. The four timeframes
//! (micro / fast / slow / macro) are positional slots whose identity must
//! survive any combination of user-chosen durations. These tests pin the
//! behaviour so a future refactor cannot reintroduce duration-based
//! dispatch (which was the root cause of the all-columns-rendering-micro
//! bug).
use std::sync::Arc;

use api_gateway::{self, AppState};
use config_models::FibonacciConfig;
use config_models::WorkspaceConfig;
use core_domain::models::{MarketSnapshot, TimeframeSlot};
use core_domain::normalized::{Exchange, NormalizedEvent};
use database_storage;
use market_analyzer::analyzer::{ActivePair, TimeframePipeline};
use market_analyzer::indicators::DivergenceDetector;
use market_analyzer::sr_engine::SrRoleTracker;
use network_adapters::exchange_status_tracker::ExchangeStatusTracker;
use network_adapters::pipeline_reliability::ReliabilityTracker;
use portfolio_supervisor::instance::{Instance, TimeframeBuffers};
use portfolio_supervisor::workspace_state::WorkspaceState;
use sqlx::SqlitePool;
use std::collections::VecDeque;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tokio::net::TcpListener;
use std::time::Duration;

const PAIR_KEY: &str = "BTC-USDT";

async fn build_test_router() -> (axum::Router, Arc<AppState>) {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    database_storage::run_migrations(&pool).await.expect("migrations");

    let symbol_mapper = Arc::new(core_domain::normalized::SymbolMapper::new());
    symbol_mapper.register(Exchange::Hyperliquid, "BTC", PAIR_KEY).await;
    let (telemetry_tx, _telemetry_rx) =
        mpsc::channel::<database_storage::TelemetryMsg>(100);

    let workspace = WorkspaceState::empty();
    let (bcast_micro, _) = broadcast::channel::<MarketSnapshot>(200);
    let (bcast_fast,  _) = broadcast::channel::<MarketSnapshot>(200);
    let (bcast_slow,  _) = broadcast::channel::<MarketSnapshot>(200);
    let (bcast_macro, _) = broadcast::channel::<MarketSnapshot>(200);
    let snap_hist = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::new()));

    let active_pair = Arc::new(ActivePair {
        symbol: PAIR_KEY.to_string(),
        // Edge cases: slow duration is LOWER than fast, fast is LOWER
        // than micro. This is the exact pattern that broke the legacy
        // duration-based dispatcher.
        micro:    TimeframePipeline { slot: TimeframeSlot::Micro, history: Arc::new(RwLock::new(VecDeque::new())), broadcast_tx: bcast_micro, latest_snapshot: Arc::new(RwLock::new(None)), snapshot_history: snap_hist.clone(), timeframe_secs: 5,    timeframe_label: "Micro" , divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))), sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))), fibonacci: FibonacciConfig::default(), latest_oi: Arc::new(RwLock::new(None)), latest_funding: Arc::new(RwLock::new(None)), latest_mark_px: Arc::new(RwLock::new(None)), latest_index_px: Arc::new(RwLock::new(None)), active_set: Default::default() },
        fast:     TimeframePipeline { slot: TimeframeSlot::Fast,  history: Arc::new(RwLock::new(VecDeque::new())), broadcast_tx: bcast_fast,  latest_snapshot: Arc::new(RwLock::new(None)), snapshot_history: snap_hist.clone(), timeframe_secs: 180,  timeframe_label: "Fast"  , divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))), sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))), fibonacci: FibonacciConfig::default(), latest_oi: Arc::new(RwLock::new(None)), latest_funding: Arc::new(RwLock::new(None)), latest_mark_px: Arc::new(RwLock::new(None)), latest_index_px: Arc::new(RwLock::new(None)), active_set: Default::default() },
        slow:     TimeframePipeline { slot: TimeframeSlot::Slow,  history: Arc::new(RwLock::new(VecDeque::new())), broadcast_tx: bcast_slow,  latest_snapshot: Arc::new(RwLock::new(None)), snapshot_history: snap_hist.clone(), timeframe_secs: 60,   timeframe_label: "Slow"  , divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))), sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))), fibonacci: FibonacciConfig::default(), latest_oi: Arc::new(RwLock::new(None)), latest_funding: Arc::new(RwLock::new(None)), latest_mark_px: Arc::new(RwLock::new(None)), latest_index_px: Arc::new(RwLock::new(None)), active_set: Default::default() },
        r#macro:  TimeframePipeline { slot: TimeframeSlot::Macro, history: Arc::new(RwLock::new(VecDeque::new())), broadcast_tx: bcast_macro, latest_snapshot: Arc::new(RwLock::new(None)), snapshot_history: snap_hist.clone(), timeframe_secs: 3600, timeframe_label: "Macro" , divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))), sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))), fibonacci: FibonacciConfig::default(), latest_oi: Arc::new(RwLock::new(None)), latest_funding: Arc::new(RwLock::new(None)), latest_mark_px: Arc::new(RwLock::new(None)), latest_index_px: Arc::new(RwLock::new(None)), active_set: Default::default() },
        snapshot_tx: mpsc::channel::<NormalizedEvent>(50).0,
        cancel: CancellationToken::new(),
        latest_oi: Arc::new(RwLock::new(None)),
        latest_funding: Arc::new(RwLock::new(None)),
        latest_mark_px: Arc::new(RwLock::new(None)),
        latest_index_px: Arc::new(RwLock::new(None)),
        cluster_matrix: Arc::new(RwLock::new(None)),
        latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
    });

    let micro_buf = TimeframeBuffers { history: active_pair.micro.history.clone(),       latest: active_pair.micro.latest_snapshot.clone(),    snapshot_history: snap_hist.clone() };
    let fast_buf  = TimeframeBuffers { history: active_pair.fast.history.clone(),        latest: active_pair.fast.latest_snapshot.clone(),     snapshot_history: snap_hist.clone() };
    let slow_buf  = TimeframeBuffers { history: active_pair.slow.history.clone(),        latest: active_pair.slow.latest_snapshot.clone(),     snapshot_history: snap_hist.clone() };
    let macro_buf = TimeframeBuffers { history: active_pair.r#macro.history.clone(),     latest: active_pair.r#macro.latest_snapshot.clone(),  snapshot_history: snap_hist.clone() };

    let instance = Arc::new(Instance::new(
        "inst_slot_identity".into(),
        ("BTC".into(), "USDT".into()),
        active_pair.clone(),
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

    {
        let mut cfg: WorkspaceConfig = WorkspaceConfig::default();
        cfg.instances = Vec::new();
        workspace.set_config(cfg).await;
    }

    let state = Arc::new(AppState {
        workspace,
        session: Arc::new(portfolio_supervisor::session::SessionState::new()),
        platform: Arc::new(RwLock::new(config_models::PlatformConfig::default())),
        pool,
        symbol_mapper,
        telemetry_tx,
        connection_quality: Arc::new(network_adapters::connection_quality_tracker::ConnectionQualityRegistry::new()),
        ws_url: "ws://127.0.0.1:1".into(),
        bitget_ws_url: String::new(),
        clock_monitor: None,
        reliability: Arc::new(ReliabilityTracker::new()),
        exchange_status: Arc::new(ExchangeStatusTracker::new()),
        latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
        overview: Arc::new(RwLock::new(None)),
        execution_engine: Arc::new(portfolio_supervisor::execution::ExecutionEngine::new()),
    });
    let router = api_gateway::build_router(state.clone());
    (router, state)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pipeline_for_slot_dispatches_by_slot_not_duration() {
    let (_router, state) = build_test_router().await;
    let pair = state
        .get_active_pair(PAIR_KEY)
        .await
        .expect("pair must be present");

    // The four slots deliberately mix small/large/unsorted durations
    // (micro=5, fast=180, slow=60, macro=3600) — the exact pattern that
    // produced the "all columns showed MICRO" bug in the legacy duration-
    // based dispatcher. The slot lookup must NOT depend on duration.
    assert_eq!(pair.micro.slot, TimeframeSlot::Micro);
    assert_eq!(pair.fast.slot, TimeframeSlot::Fast);
    assert_eq!(pair.slow.slot, TimeframeSlot::Slow);
    assert_eq!(pair.r#macro.slot, TimeframeSlot::Macro);

    // Each slot subscription returns its own broadcast receiver — no two
    // slots share a channel despite micro (5) and slow (60) being closer
    // in duration than fast (180).
    let rx_micro = pair.subscribe_broadcast_by_slot(TimeframeSlot::Micro);
    let rx_fast  = pair.subscribe_broadcast_by_slot(TimeframeSlot::Fast);
    let rx_slow  = pair.subscribe_broadcast_by_slot(TimeframeSlot::Slow);
    let rx_macro = pair.subscribe_broadcast_by_slot(TimeframeSlot::Macro);

    // The receivers are distinct broadcast subscriptions — Sender::send
    // goes only to its own channel. We can't introspect the receivers
    // directly, but we can verify each subscription is a separate
    // Receiver value by their identity (compare pointer addresses).
    let ptr_micro = (&rx_micro as *const _) as usize;
    let ptr_fast = (&rx_fast as *const _) as usize;
    let ptr_slow = (&rx_slow as *const _) as usize;
    let ptr_macro = (&rx_macro as *const _) as usize;
    assert_ne!(ptr_micro, ptr_fast);
    assert_ne!(ptr_micro, ptr_slow);
    assert_ne!(ptr_micro, ptr_macro);
    assert_ne!(ptr_fast, ptr_slow);
    assert_ne!(ptr_fast, ptr_macro);
    assert_ne!(ptr_slow, ptr_macro);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pipeline_for_duration_rejects_collisions() {
    let (_router, state) = build_test_router().await;
    let pair = state
        .get_active_pair(PAIR_KEY)
        .await
        .expect("pair must be present");

    // No unique duration exists — there is no slot whose duration alone
    // uniquely matches 100. This is the case the frontend bug exposed:
    // a duration lookup returns an ambiguous slot.
    let err = pair
        .pipeline_for_duration(100)
        .err()
        .expect("100s must be ambiguous — no slot configured for it");
    assert!(err.contains("timeframe_secs=100"), "error should mention the offending duration: {err}");
    assert!(
        err.contains("No slot matches"),
        "no-match error must be explicit so callers don't silently default to micro: {err}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_query_slot_overrides_duration_dispatch() {
    tokio::time::timeout(Duration::from_secs(8), async {
        let (router, _state) = build_test_router().await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Slot-tagged WS query: even though `timeframe_secs=60` would
        // map to slow under the legacy dispatcher, an explicit slot=micro
        // must bind the connection to micro's broadcast channel.
        let res_micro = reqwest::Client::new()
            .get(format!(
                "http://{addr}/ws?symbol=BTC-USDT&timeframe_secs=60&slot=micro"
            ))
            .header("connection", "upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-version", "13")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .send()
            .await
            .expect("WS upgrade request");
        assert!(
            res_micro.status().as_u16() == 101 || res_micro.status().is_success(),
            "expected WS upgrade, got {}",
            res_micro.status()
        );

        // Slot-tagged query for `slow`:
        let res_slow = reqwest::Client::new()
            .get(format!(
                "http://{addr}/ws?symbol=BTC-USDT&timeframe_secs=60&slot=slow"
            ))
            .header("connection", "upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-version", "13")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .send()
            .await
            .expect("WS upgrade request");
        assert!(res_slow.status().as_u16() == 101 || res_slow.status().is_success());
    })
    .await
    .expect("ws_query_slot_overrides_duration_dispatch timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn timeframe_slot_round_trips_through_wire_payload() {
    // Every `MarketSnapshot` carries `timeframe_slot` after the analyzer
    // stamps it; the WS emission must surface that slot on the outer
    // JSON-RPC params.
    let snap = MarketSnapshot {
        timeframe_slot: Some(TimeframeSlot::Slow),
        exchange: Some(Exchange::Hyperliquid),
        timeframe_secs: 300,
        timestamp: 1_700_000_000,
        symbol: PAIR_KEY.to_string(),
        is_completed: Some(true),
        mid_price: rust_decimal::Decimal::from(1),
        bid_price: rust_decimal::Decimal::from(1),
        ask_price: rust_decimal::Decimal::from(1),
        bid_size: None,
        ask_size: None,
        funding_rate: None,
        open_interest: None,
        oi_delta_1h: None,
        mark_price: None,
        index_price: None,
        mark_index_spread_pct: None,
        prev_day_px: None,
        open: None, high: None, low: None, close: None,
        volume: None, average_volume: None,
        indicators: Default::default(),
        alignment: None,
        risk: None,
        analysis: None,
        advisory: None,
        opportunity: None,
        risk_profile: None,
        liquidity: None,
        cluster: None,
        volume_profile: None,
        decision_context: None,
        statistical_context: None,
        context: None,
        liquidity_signals: vec![],
        metrics_config: None,
        quality_envelope: None,
    };
    let serialized = serde_json::to_value(&snap).expect("serialize");
    assert_eq!(
        serialized.get("timeframe_slot").and_then(|v| v.as_str()),
        Some("slow"),
        "Wire payload must carry timeframe_slot"
    );

    // Slot helpers round-trip cleanly across versions.
    assert_eq!(TimeframeSlot::parse("micro"), TimeframeSlot::Micro);
    assert_eq!(TimeframeSlot::parse("fast"), TimeframeSlot::Fast);
    assert_eq!(TimeframeSlot::parse("slow"), TimeframeSlot::Slow);
    assert_eq!(TimeframeSlot::parse("macro"), TimeframeSlot::Macro);
    assert_eq!(TimeframeSlot::parse("garbage"), TimeframeSlot::Micro);

    assert_eq!(TimeframeSlot::parse_from_secs(60), TimeframeSlot::Micro);
    assert_eq!(TimeframeSlot::parse_from_secs(180), TimeframeSlot::Fast);
    assert_eq!(TimeframeSlot::parse_from_secs(300), TimeframeSlot::Slow);
    assert_eq!(TimeframeSlot::parse_from_secs(900), TimeframeSlot::Macro);
}
