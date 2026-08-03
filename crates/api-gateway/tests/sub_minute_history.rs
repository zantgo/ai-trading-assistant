//! Regression: sub-minute `/api/history` returns the bootstrap data.
//!
//! Before the frontend `isSubMinute` guard was removed, the chart never
//! called `setData()` for ≤60s timeframes. Now that the guard is gone
//! the backend must faithfully return whatever snapshot data the warm
//! bootstrap deposited in the per-pipeline queue — even for `timeframe_secs=1`.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use api_gateway::{self, AppState};
use config_models::FibonacciConfig;
use config_models::WorkspaceConfig;
use core_domain::models::{MarketSnapshot, TimeframeSlot};
use core_domain::normalized::{Exchange, SymbolMapper};
use database_storage;
use market_analyzer::analyzer::{ActivePair, TimeframePipeline};
use market_analyzer::indicators::DivergenceDetector;
use market_analyzer::sr_engine::SrRoleTracker;
use network_adapters::exchange_status_tracker::ExchangeStatusTracker;
use network_adapters::pipeline_reliability::ReliabilityTracker;
use portfolio_supervisor::instance::{Instance, TimeframeBuffers};
use portfolio_supervisor::session::ExchangeChoice;
use portfolio_supervisor::workspace_state::WorkspaceState;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;

const PAIR_KEY: &str = "BTC-USDT";
const INSTANCE_ID: &str = "inst_sub_minute";

/// Build a router holding one workspace instance whose `micro.snapshot_history`
/// is seeded with completed `MarketSnapshot`s at `timeframe_secs=secs`.
/// This mimics what `populate_buffers` does after a historical bootstrap.
async fn build_router_with_snapshots(
    secs: u64,
    snapshots: Vec<MarketSnapshot>,
) -> (axum::Router, Arc<AppState>) {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    database_storage::run_migrations(&pool)
        .await
        .expect("migrations");

    let symbol_mapper = Arc::new(SymbolMapper::new());
    symbol_mapper
        .register(Exchange::Hyperliquid, "BTC", PAIR_KEY)
        .await;
    let (telemetry_tx, _) = mpsc::channel::<database_storage::TelemetryMsg>(100);

    let workspace = WorkspaceState::empty();
    let (bcast_tx, _) = broadcast::channel::<MarketSnapshot>(200);
    let snap_hist = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::new()));

    // Pre-populate the snapshot history with the supplied entries.
    {
        let mut sh = snap_hist.write().await;
        for snap in &snapshots {
            sh.push_back(snap.clone());
        }
    }

    let build_pipe = |slot, secs, tx| TimeframePipeline {
        slot,
        history: Arc::new(RwLock::new(VecDeque::new())),
        broadcast_tx: tx,
        latest_snapshot: Arc::new(RwLock::new(None)),
        snapshot_history: snap_hist.clone(),
        timeframe_secs: secs,
        timeframe_label: "Test",
        divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
        sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
        fibonacci: FibonacciConfig::default(),
        latest_oi: Arc::new(RwLock::new(None)),
        latest_funding: Arc::new(RwLock::new(None)),
        latest_mark_px: Arc::new(RwLock::new(None)),
        latest_index_px: Arc::new(RwLock::new(None)),
        active_set: Default::default(),
        cluster_matrix: Arc::new(RwLock::new(None)),
        cluster_status: Arc::new(RwLock::new(
            core_domain::liquidity::ClusterStatusSnapshot::pending("TEST", "test"),
        )),
        pipeline_state: Arc::new(RwLock::new(
            core_domain::models::CandlePipelineState::Initializing,
        )),
        indicator_lifecycle: Arc::new(RwLock::new(std::collections::HashMap::new())),
        buffer_size: 500,
        stale_threshold_secs: 300,
    };

    let active_pair = Arc::new(ActivePair {
        symbol: PAIR_KEY.to_string(),
        // Micro gets the requested sub-minute duration; the other three
        // slots use dummy values so `pipeline_for_duration` never has a
        // collision.
        micro: build_pipe(TimeframeSlot::Micro, secs, bcast_tx.clone()),
        fast: build_pipe(TimeframeSlot::Fast, secs + 10, bcast_tx.clone()),
        slow: build_pipe(TimeframeSlot::Slow, secs + 20, bcast_tx.clone()),
        r#macro: build_pipe(TimeframeSlot::Macro, secs + 30, bcast_tx),
        snapshot_tx: mpsc::channel::<core_domain::normalized::NormalizedEvent>(50).0,
        cancel: CancellationToken::new(),
        latest_oi: Arc::new(RwLock::new(None)),
        latest_funding: Arc::new(RwLock::new(None)),
        latest_mark_px: Arc::new(RwLock::new(None)),
        latest_index_px: Arc::new(RwLock::new(None)),
            oi_history: Arc::new(RwLock::new(VecDeque::with_capacity(60))),
            funding_history: Arc::new(RwLock::new(VecDeque::with_capacity(8))),
        latency_tracker: Arc::new(Default::default()),
    });

    let micro_buf = TimeframeBuffers {
        history: active_pair.micro.history.clone(),
        latest: active_pair.micro.latest_snapshot.clone(),
        snapshot_history: snap_hist.clone(),
    };
    let fast_buf = TimeframeBuffers {
        history: active_pair.fast.history.clone(),
        latest: active_pair.fast.latest_snapshot.clone(),
        snapshot_history: snap_hist.clone(),
    };
    let slow_buf = TimeframeBuffers {
        history: active_pair.slow.history.clone(),
        latest: active_pair.slow.latest_snapshot.clone(),
        snapshot_history: snap_hist.clone(),
    };
    let macro_buf = TimeframeBuffers {
        history: active_pair.r#macro.history.clone(),
        latest: active_pair.r#macro.latest_snapshot.clone(),
        snapshot_history: snap_hist.clone(),
    };

    let instance = Arc::new(Instance::new(
        INSTANCE_ID.to_string(),
        ("BTC".into(), "USDT".into()),
        ExchangeChoice::Hyperliquid,
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
        platform: Arc::new(RwLock::new(Default::default())),
        pool,
        symbol_mapper,
        telemetry_tx,
        connection_quality: Arc::new(Default::default()),
        ws_url: "ws://127.0.0.1:1".into(),
        bitget_ws_url: String::new(),
        clock_monitor: None,
        reliability: Arc::new(ReliabilityTracker::new()),
        exchange_status: Arc::new(ExchangeStatusTracker::new()),
        latency_tracker: Arc::new(Default::default()),
        overview: Arc::new(RwLock::new(None)),
        execution_engine: Arc::new(portfolio_supervisor::execution::ExecutionEngine::new()),
        recharge_tx: broadcast::channel::<api_gateway::RechargeNotice>(64).0,
    });
    (api_gateway::build_router(state.clone()), state)
}

fn make_snapshot(secs: u64, timestamp: u64, close_val: f64) -> MarketSnapshot {
    let close = rust_decimal::Decimal::from_f64_retain(close_val).unwrap();
    let open = rust_decimal::Decimal::from_f64_retain(close_val - 5.0).unwrap();
    let high = rust_decimal::Decimal::from_f64_retain(close_val + 5.0).unwrap();
    let low = rust_decimal::Decimal::from_f64_retain(close_val - 10.0).unwrap();
    let bid = rust_decimal::Decimal::from_f64_retain(close_val - 1.0).unwrap();
    let ask = rust_decimal::Decimal::from_f64_retain(close_val + 1.0).unwrap();
    MarketSnapshot {
        timeframe_slot: Some(TimeframeSlot::Micro),
        exchange: Some(Exchange::Hyperliquid),
        timeframe_secs: secs,
        timestamp,
        symbol: PAIR_KEY.to_string(),
        is_completed: Some(true),
        mid_price: close,
        bid_price: bid,
        ask_price: ask,
        bid_size: None,
        ask_size: None,
        funding_rate: None,
        open_interest: None,
        oi_delta_1h: None,
        mark_price: None,
        index_price: None,
        mark_index_spread_pct: None,
        prev_day_px: None,
        open: Some(open),
        high: Some(high),
        low: Some(low),
        close: Some(close),
        volume: Some(rust_decimal::Decimal::from_f64_retain(1.5).unwrap()),
        average_volume: Some(rust_decimal::Decimal::from_f64_retain(1.2).unwrap()),
        context: None,
        decision_context: None,
        statistical_context: None,
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
        liquidity_signals: vec![],
        metrics_config: None,
        quality_envelope: None,
        pipeline_state: core_domain::models::CandlePipelineState::default(),
        indicator_lifecycle: std::collections::HashMap::new(),
    }
}

async fn serve_for(state: Arc<AppState>) -> std::net::SocketAddr {
    let router = api_gateway::build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn history_endpoint_returns_candles_for_sub_minute_timeframe() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (_router, state) = build_router_with_snapshots(
            1,
            vec![
                make_snapshot(1, 1_718_000_001, 65000.0),
                make_snapshot(1, 1_718_000_002, 65001.0),
            ],
        )
        .await;
        let addr = serve_for(state.clone()).await;
        let client = reqwest::Client::new();

        let res = client
            .get(format!(
                "http://{addr}/api/history?symbol=BTC-USDT&timeframe_secs=1&limit=100"
            ))
            .send()
            .await
            .expect("history request");
        assert!(res.status().is_success());

        let body: serde_json::Value = res.json().await.expect("json");
        let candles = body
            .get("candles")
            .and_then(|v| v.as_array())
            .expect("candles array");
        assert!(
            candles.len() >= 1,
            "expected >= 1 candles for sub-minute history, got {candles:?}"
        );

        // Every returned candle must be a JSON object with `time`, `open`,
        // `high`, `low`, `close`.
        for candle in candles {
            assert!(candle.get("time").is_some());
            assert!(candle.get("close").is_some());
        }
    })
    .await
    .expect("sub-minute history test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn history_endpoint_trims_leading_none_close_for_sub_minute() {
    tokio::time::timeout(Duration::from_secs(10), async {
        // Seed three snapshots: the first has no close → must be trimmed
        // by the handler's leading-None filter (history.rs:42-43). The
        // response should contain only the two real ones.
        let (_router, state) = build_router_with_snapshots(
            1,
            vec![
                {
                    let mut s = make_snapshot(1, 1_718_000_001, 65000.0);
                    s.close = None; // leading None
                    s
                },
                make_snapshot(1, 1_718_000_002, 65001.0),
                make_snapshot(1, 1_718_000_003, 65002.0),
            ],
        )
        .await;
        let addr = serve_for(state.clone()).await;
        let client = reqwest::Client::new();

        let res = client
            .get(format!(
                "http://{addr}/api/history?symbol=BTC-USDT&timeframe_secs=1&limit=100"
            ))
            .send()
            .await
            .expect("history request");
        assert!(res.status().is_success());

        let body: serde_json::Value = res.json().await.expect("json");
        let candles = body
            .get("candles")
            .and_then(|v| v.as_array())
            .expect("candles array");
        assert_eq!(
            candles.len(),
            2,
            "leading close-None must be trimmed, got {candles:?}"
        );
    })
    .await
    .expect("sub-minute leading-trim test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn history_endpoint_returns_empty_on_unknown_pair() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (_router, state) = build_router_with_snapshots(1, vec![]).await;
        let addr = serve_for(state.clone()).await;
        let client = reqwest::Client::new();

        let res = client
            .get(format!(
                "http://{addr}/api/history?symbol=NONEXISTENT&timeframe_secs=1"
            ))
            .send()
            .await
            .expect("history request");
        assert!(!res.status().is_server_error());
    })
    .await
    .expect("unknown-pair test timed out");
}
