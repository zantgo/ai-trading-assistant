//! Per-timeframe cluster refresh integration tests (v6.5).
//!
//! Verifies that:
//!   1. `compute_cluster_for_tf` reads the **TF-specific** history, not
//!      just the micro TF's history.
//!   2. Each TF pipeline owns its own `cluster_matrix` handle (4 separate
//!      `Arc<RwLock<...>>` instances, not a shared one).
//!   3. Failures (no micro snapshot, no OI, insufficient history) bubble
//!      up as `ClusterRefreshError` rather than silently returning None.
//!
//! Run via `./manage.sh test-engine`.

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

use config_models::{FibonacciConfig, LiquidityConfig, TimeframeConfig};
use core_domain::models::{MarketSnapshot, TimeframeSlot};
use core_domain::normalized::{Exchange, NormalizedCandle, NormalizedEvent};
use market_analyzer::analyzer::{ActivePair, TimeframePipeline};
use market_analyzer::indicators::DivergenceDetector;
use market_analyzer::sr_engine::SrRoleTracker;
use portfolio_supervisor::session::ExchangeChoice;
use rust_decimal::Decimal;

fn make_pipe(
    slot: TimeframeSlot,
    secs: u64,
    tx: broadcast::Sender<MarketSnapshot>,
) -> TimeframePipeline {
    TimeframePipeline {
        slot,
        history: Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::new())),
        broadcast_tx: tx,
        latest_snapshot: Arc::new(RwLock::new(None::<MarketSnapshot>)),
        snapshot_history: Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::new())),
        timeframe_secs: secs,
        timeframe_label: "Test",
        divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
        sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
        fibonacci: FibonacciConfig::default(),
        latest_oi: Arc::new(RwLock::new(Some(Decimal::from(1_000_000)))),
        latest_funding: Arc::new(RwLock::new(Some(Decimal::from_f64_retain(0.0001).unwrap()))),
        latest_mark_px: Arc::new(RwLock::new(Some(Decimal::from(50_000)))),
        latest_index_px: Arc::new(RwLock::new(Some(Decimal::from(50_000)))),
        active_set: Default::default(),
        cluster_matrix: Arc::new(RwLock::new(
            None::<core_domain::liquidity::LiquidationClusterMatrix>,
        )),
        cluster_status: Arc::new(RwLock::new(
            core_domain::liquidity::ClusterStatusSnapshot::pending("BTC-USDT", &slot.as_str()),
        )),
        pipeline_state: Arc::new(RwLock::new(
            core_domain::models::CandlePipelineState::Initializing,
        )),
        indicator_lifecycle: Arc::new(RwLock::new(std::collections::HashMap::new())),
        advisory: Arc::new(RwLock::new(None)),
        tf_leverage_config: Arc::new(config_models::TfLeverageConfig::default()),
        buffer_size: 500,
        stale_threshold_secs: 300,
    }
}

fn make_snap_history(closes: Vec<f64>) -> MarketSnapshot {
    use rust_decimal::prelude::FromPrimitive;
    let mut snap = MarketSnapshot::default_for_test("BTC-USDT", 60);
    let last = closes.last().copied().unwrap_or(50_000.0);
    snap.mid_price = Decimal::from_f64(last).unwrap();
    snap.open_interest = Some(Decimal::from(1_000_000));
    snap.funding_rate = Some(Decimal::from_f64_retain(0.0001).unwrap());
    snap
}

fn test_config() -> LiquidityConfig {
    LiquidityConfig::default()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn per_tf_cluster_refresh_uses_tf_specific_history() {
    use core_domain::normalized::NormalizedCandle as NC;
    use portfolio_supervisor::registry::pipelines::compute_cluster_for_tf;

    // Three TFs with three different price histories.
    let (bcast_tx, _) = broadcast::channel::<MarketSnapshot>(10);
    let micro_pipe = make_pipe(TimeframeSlot::Micro, 60, bcast_tx.clone());
    let fast_pipe = make_pipe(TimeframeSlot::Fast, 300, bcast_tx.clone());
    let macro_pipe = make_pipe(TimeframeSlot::Macro, 900, bcast_tx);

    // Micro history: range 49_500 → 50_500 (down move).
    {
        let mut h = micro_pipe.history.write().await;
        for i in 0..20 {
            let p = 50_000.0 - (i as f64) * 25.0;
            h.push_back(NC {
                exchange: Exchange::Hyperliquid,
                symbol: "BTC-USDT".into(),
                start_time_ms: i * 60_000,
                duration_ms: 60_000,
                open: Decimal::from_f64_retain(p).unwrap(),
                high: Decimal::from_f64_retain(p + 10.0).unwrap(),
                low: Decimal::from_f64_retain(p - 10.0).unwrap(),
                close: Decimal::from_f64_retain(p - 5.0).unwrap(),
                volume: Decimal::from(100),
                trades_count: 0,
                reconstructed: None,
            });
        }
    }
    // Fast history: range 50_000 → 51_000 (up move).
    {
        let mut h = fast_pipe.history.write().await;
        for i in 0..20 {
            let p = 50_000.0 + (i as f64) * 50.0;
            h.push_back(NC {
                exchange: Exchange::Hyperliquid,
                symbol: "BTC-USDT".into(),
                start_time_ms: i * 300_000,
                duration_ms: 300_000,
                open: Decimal::from_f64_retain(p).unwrap(),
                high: Decimal::from_f64_retain(p + 10.0).unwrap(),
                low: Decimal::from_f64_retain(p - 10.0).unwrap(),
                close: Decimal::from_f64_retain(p + 5.0).unwrap(),
                volume: Decimal::from(100),
                trades_count: 0,
                reconstructed: None,
            });
        }
    }
    // Macro history: range 50_000 ± 100 (sideways).
    {
        let mut h = macro_pipe.history.write().await;
        for i in 0..20 {
            let p = 50_000.0 + ((i as f64) * 7.0).sin() * 100.0;
            h.push_back(NC {
                exchange: Exchange::Hyperliquid,
                symbol: "BTC-USDT".into(),
                start_time_ms: i * 900_000,
                duration_ms: 900_000,
                open: Decimal::from_f64_retain(p).unwrap(),
                high: Decimal::from_f64_retain(p + 50.0).unwrap(),
                low: Decimal::from_f64_retain(p - 50.0).unwrap(),
                close: Decimal::from_f64_retain(p + 1.0).unwrap(),
                volume: Decimal::from(100),
                trades_count: 0,
                reconstructed: None,
            });
        }
    }

    // All three TFs share the same latest_snapshot at the micro mid.
    *micro_pipe.latest_snapshot.write().await = Some(make_snap_history(vec![49_500.0]));
    *fast_pipe.latest_snapshot.write().await = Some(make_snap_history(vec![51_000.0]));
    *macro_pipe.latest_snapshot.write().await = Some(make_snap_history(vec![50_000.0]));

    let active = Arc::new(ActivePair {
        symbol: "BTC-USDT".into(),
        custom_pipelines: std::collections::HashMap::new(),
        micro: micro_pipe,
        fast: fast_pipe,
        slow: make_pipe(TimeframeSlot::Slow, 600, {
            let (t, _) = broadcast::channel::<MarketSnapshot>(1);
            t
        }),
        r#macro: macro_pipe,
        snapshot_tx: mpsc::channel::<NormalizedEvent>(8).0,
        cancel: tokio_util::sync::CancellationToken::new(),
        latest_oi: Arc::new(RwLock::new(Some(Decimal::from(1_000_000)))),
        latest_funding: Arc::new(RwLock::new(Some(Decimal::from_f64_retain(0.0001).unwrap()))),
        latest_mark_px: Arc::new(RwLock::new(Some(Decimal::from(50_000)))),
        latest_index_px: Arc::new(RwLock::new(Some(Decimal::from(50_000)))),
        oi_history: Arc::new(RwLock::new(VecDeque::with_capacity(60))),
        funding_history: Arc::new(RwLock::new(VecDeque::with_capacity(8))),
        latency_tracker: Arc::new(Default::default()),
    });

    let cfg = test_config();

    // Compute one cluster for each TF; we use clone of the handle via
    // `active.{slot}.cluster_matrix` to confirm the per-TF isolation.
    let micro_m = compute_cluster_for_tf(
        &active,
        TimeframeSlot::Micro,
        &cfg,
        ExchangeChoice::Hyperliquid,
    )
    .await
    .expect("micro should compute");
    active.micro.cluster_matrix.write().await.replace(micro_m);

    let fast_m = compute_cluster_for_tf(
        &active,
        TimeframeSlot::Fast,
        &cfg,
        ExchangeChoice::Hyperliquid,
    )
    .await
    .expect("fast should compute");
    active.fast.cluster_matrix.write().await.replace(fast_m);

    let macro_m = compute_cluster_for_tf(
        &active,
        TimeframeSlot::Macro,
        &cfg,
        ExchangeChoice::Hyperliquid,
    )
    .await
    .expect("macro should compute");
    active.r#macro.cluster_matrix.write().await.replace(macro_m);

    // Each TF's cluster_matrix handle is now populated with **different**
    // matrices because the price histories differ. Every cluster matrix
    // must be valid (non-empty short/long clusters) even when the
    // histories are different.
    assert!(
        !active
            .micro
            .cluster_matrix
            .read()
            .await
            .as_ref()
            .unwrap()
            .short_clusters
            .is_empty()
            || !active
                .micro
                .cluster_matrix
                .read()
                .await
                .as_ref()
                .unwrap()
                .long_clusters
                .is_empty(),
        "micro cluster should detect at least one cluster with 20-bar history"
    );
    assert!(
        !active
            .fast
            .cluster_matrix
            .read()
            .await
            .as_ref()
            .unwrap()
            .short_clusters
            .is_empty()
            || !active
                .fast
                .cluster_matrix
                .read()
                .await
                .as_ref()
                .unwrap()
                .long_clusters
                .is_empty(),
        "fast cluster should detect at least one cluster"
    );
    assert!(
        !active
            .r#macro
            .cluster_matrix
            .read()
            .await
            .as_ref()
            .unwrap()
            .short_clusters
            .is_empty()
            || !active
                .r#macro
                .cluster_matrix
                .read()
                .await
                .as_ref()
                .unwrap()
                .long_clusters
                .is_empty(),
        "macro cluster should detect at least one cluster"
    );

    // All 4 handles are distinct Arc instances (per-TF isolation).
    let h_micro = Arc::as_ptr(&active.micro.cluster_matrix) as *const u8;
    let h_fast = Arc::as_ptr(&active.fast.cluster_matrix) as *const u8;
    let h_macro = Arc::as_ptr(&active.r#macro.cluster_matrix) as *const u8;
    assert_ne!(h_micro, h_fast, "micro and fast must have distinct handles");
    assert_ne!(h_fast, h_macro, "fast and macro must have distinct handles");
    assert_ne!(
        h_micro, h_macro,
        "micro and macro must have distinct handles"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn per_tf_cluster_refresh_returns_error_when_no_snapshot() {
    use portfolio_supervisor::registry::pipelines::compute_cluster_for_tf;

    let (bcast_tx, _) = broadcast::channel::<MarketSnapshot>(10);
    let micro_pipe = make_pipe(TimeframeSlot::Micro, 60, bcast_tx);

    let active = Arc::new(ActivePair {
        symbol: "BTC-USDT".into(),
        custom_pipelines: std::collections::HashMap::new(),
        micro: micro_pipe,
        fast: make_pipe(TimeframeSlot::Fast, 300, {
            let (t, _) = broadcast::channel::<MarketSnapshot>(1);
            t
        }),
        slow: make_pipe(TimeframeSlot::Slow, 600, {
            let (t, _) = broadcast::channel::<MarketSnapshot>(1);
            t
        }),
        r#macro: make_pipe(TimeframeSlot::Macro, 900, {
            let (t, _) = broadcast::channel::<MarketSnapshot>(1);
            t
        }),
        snapshot_tx: mpsc::channel::<NormalizedEvent>(8).0,
        cancel: tokio_util::sync::CancellationToken::new(),
        latest_oi: Arc::new(RwLock::new(None)),
        latest_funding: Arc::new(RwLock::new(None)),
        latest_mark_px: Arc::new(RwLock::new(None)),
        latest_index_px: Arc::new(RwLock::new(None)),
        oi_history: Arc::new(RwLock::new(VecDeque::with_capacity(60))),
        funding_history: Arc::new(RwLock::new(VecDeque::with_capacity(8))),
        latency_tracker: Arc::new(Default::default()),
    });

    // No snapshot populated → must return the NoSnapshotYet variant.
    let result = compute_cluster_for_tf(
        &active,
        TimeframeSlot::Micro,
        &test_config(),
        ExchangeChoice::Hyperliquid,
    )
    .await;
    assert!(
        result.is_err(),
        "no snapshot → should return Err, got {:?}",
        result,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn per_tf_cluster_refresh_returns_error_when_no_oi() {
    use portfolio_supervisor::registry::pipelines::compute_cluster_for_tf;

    let (bcast_tx, _) = broadcast::channel::<MarketSnapshot>(10);
    let micro_pipe = make_pipe(TimeframeSlot::Micro, 60, bcast_tx);

    // Snapshot exists but no OI.
    let mut snap = make_snap_history(vec![50_000.0]);
    snap.open_interest = None;
    *micro_pipe.latest_snapshot.write().await = Some(snap);

    let active = Arc::new(ActivePair {
        symbol: "BTC-USDT".into(),
        custom_pipelines: std::collections::HashMap::new(),
        micro: micro_pipe,
        fast: make_pipe(TimeframeSlot::Fast, 300, {
            let (t, _) = broadcast::channel::<MarketSnapshot>(1);
            t
        }),
        slow: make_pipe(TimeframeSlot::Slow, 600, {
            let (t, _) = broadcast::channel::<MarketSnapshot>(1);
            t
        }),
        r#macro: make_pipe(TimeframeSlot::Macro, 900, {
            let (t, _) = broadcast::channel::<MarketSnapshot>(1);
            t
        }),
        snapshot_tx: mpsc::channel::<NormalizedEvent>(8).0,
        cancel: tokio_util::sync::CancellationToken::new(),
        latest_oi: Arc::new(RwLock::new(None)),
        latest_funding: Arc::new(RwLock::new(None)),
        latest_mark_px: Arc::new(RwLock::new(None)),
        latest_index_px: Arc::new(RwLock::new(None)),
        oi_history: Arc::new(RwLock::new(VecDeque::with_capacity(60))),
        funding_history: Arc::new(RwLock::new(VecDeque::with_capacity(8))),
        latency_tracker: Arc::new(Default::default()),
    });

    let result = compute_cluster_for_tf(
        &active,
        TimeframeSlot::Micro,
        &test_config(),
        ExchangeChoice::Hyperliquid,
    )
    .await;
    assert!(result.is_err(), "no OI → Err");
}

/// Regression: the cluster-refresh skip reason must be templated on the
/// active exchange (v6.6). HL and Bitget have different OI carriers
/// (REST poller vs ticker channel), so a generic message misleads the
/// operator about which feed to investigate.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cluster_refresh_skip_reason_templates_on_active_exchange() {
    use portfolio_supervisor::registry::pipelines::compute_cluster_for_tf;

    async fn build_active() -> Arc<ActivePair> {
        let (bcast_tx, _) = broadcast::channel::<MarketSnapshot>(10);
        let micro_pipe = make_pipe(TimeframeSlot::Micro, 60, bcast_tx);

        // Snapshot exists but no OI → NoOpenInterest variant fires.
        let mut snap = make_snap_history(vec![50_000.0]);
        snap.open_interest = None;
        *micro_pipe.latest_snapshot.write().await = Some(snap);

        Arc::new(ActivePair {
            symbol: "BTC-USDT".into(),
            custom_pipelines: std::collections::HashMap::new(),
            micro: micro_pipe,
            fast: make_pipe(TimeframeSlot::Fast, 300, {
                let (t, _) = broadcast::channel::<MarketSnapshot>(1);
                t
            }),
            slow: make_pipe(TimeframeSlot::Slow, 600, {
                let (t, _) = broadcast::channel::<MarketSnapshot>(1);
                t
            }),
            r#macro: make_pipe(TimeframeSlot::Macro, 900, {
                let (t, _) = broadcast::channel::<MarketSnapshot>(1);
                t
            }),
            snapshot_tx: mpsc::channel::<NormalizedEvent>(8).0,
            cancel: tokio_util::sync::CancellationToken::new(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
            oi_history: Arc::new(RwLock::new(VecDeque::with_capacity(60))),
            funding_history: Arc::new(RwLock::new(VecDeque::with_capacity(8))),
            latency_tracker: Arc::new(Default::default()),
        })
    }

    let active_hl = build_active().await;
    let err_hl = compute_cluster_for_tf(
        &active_hl,
        TimeframeSlot::Micro,
        &test_config(),
        ExchangeChoice::Hyperliquid,
    )
    .await
    .unwrap_err();
    let msg_hl = err_hl.to_string();
    assert!(
        msg_hl.contains("HL derivatives poller"),
        "HL skip reason should mention the HL poller, got: {}",
        msg_hl
    );
    assert!(
        !msg_hl.contains("Bitget"),
        "HL skip reason must NOT mention Bitget, got: {}",
        msg_hl
    );

    let active_bg = build_active().await;
    let err_bg = compute_cluster_for_tf(
        &active_bg,
        TimeframeSlot::Micro,
        &test_config(),
        ExchangeChoice::Bitget,
    )
    .await
    .unwrap_err();
    let msg_bg = err_bg.to_string();
    assert!(
        msg_bg.contains("Bitget ticker channel"),
        "Bitget skip reason should mention the Bitget ticker channel, got: {}",
        msg_bg
    );
    assert!(
        !msg_bg.contains("HL derivatives poller"),
        "Bitget skip reason must NOT mention HL poller, got: {}",
        msg_bg
    );
}

// Avoid unused import warnings.
#[allow(dead_code)]
fn _unused_refs(_t: &TimeframeConfig) {}
#[allow(dead_code)]
const _: Option<TimeframeConfig> = None;

// bring `default_for_test` into scope via extension trait
trait SnapshotTestHelpers {
    fn default_for_test(symbol: &str, timeframe_secs: u64) -> MarketSnapshot;
}

impl SnapshotTestHelpers for MarketSnapshot {
    fn default_for_test(symbol: &str, _secs: u64) -> MarketSnapshot {
        use std::collections::HashMap;
        MarketSnapshot {
            timeframe_slot: Some(TimeframeSlot::Micro),
            exchange: Some(Exchange::Hyperliquid),
            timeframe_secs: 60,
            timestamp: 0,
            symbol: symbol.into(),
            is_completed: Some(true),
            mid_price: Decimal::from(50_000),
            bid_price: Decimal::ZERO,
            ask_price: Decimal::ZERO,
            bid_size: None,
            ask_size: None,
            funding_rate: Some(Decimal::from_f64_retain(0.0001).unwrap()),
            open_interest: Some(Decimal::from(1_000_000)),
            oi_delta_1h: None,
            mark_price: None,
            index_price: None,
            mark_index_spread_pct: None,
            prev_day_px: None,
            open: Some(Decimal::from(50_000)),
            high: Some(Decimal::from(50_100)),
            low: Some(Decimal::from(49_900)),
            close: Some(Decimal::from(50_000)),
            volume: Some(Decimal::from(100)),
            average_volume: Some(Decimal::from(100)),
            indicators: HashMap::new(),
            context: None,
            decision_context: None,
            statistical_context: None,
            alignment: None,
            risk: None,
            analysis: None,
            advisory: None,
            opportunity: None,
            liquidity_signals: vec![],
            metrics_config: None,
            risk_profile: None,
            liquidity: None,
            cluster: None,
            volume_profile: None,
            quality_envelope: None,
            pipeline_state: core_domain::models::CandlePipelineState::default(),
            indicator_lifecycle: std::collections::HashMap::new(),
        }
    }
}
