use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use rust_decimal::prelude::ToPrimitive;

use crate::analyzer;
use crate::config::{
    FibonacciConfig, IntervalsConfig, LiquidityConfig, OperationalMode, PositionScalingConfig,
    SafetyConfig, TimeframeConfig,
};
use crate::db;
use crate::instance::{Instance, TimeframeBuffers};
use crate::sr_engine::SrRoleTracker;
use crate::session::{Currency, ExchangeChoice};
use crate::server::AppState;
use shared::indicators::DivergenceDetector;
use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedCandle, NormalizedEvent};
use tokio_util::sync::CancellationToken;

pub struct PipelineContext {
    pub base: String,
    /// Unified internal symbol (e.g. "BTC-USDT") used across the state.
    pub internal_symbol: String,
    /// Settlement/quote currency for this session.
    pub quote: Currency,
    pub pair_key: String,
    pub exchange_choice: ExchangeChoice,
    pub micro_cfg: TimeframeConfig,
    pub fast_cfg: TimeframeConfig,
    pub slow_cfg: TimeframeConfig,
    pub macro_cfg: TimeframeConfig,
    pub fib_config: FibonacciConfig,
    pub safety_config: SafetyConfig,
    pub intervals_config: IntervalsConfig,
    pub cancel: CancellationToken,
    pub operational_mode: OperationalMode,
    pub weight_overrides: Option<std::collections::HashMap<String, i32>>,
    pub position_scaling: Option<PositionScalingConfig>,
    pub liquidity_config: LiquidityConfig,
}

pub struct PipelineArtifacts {
    pub instance: Arc<Instance>,
    pub micro: TimeframeBuffers,
    pub fast: TimeframeBuffers,
    pub slow: TimeframeBuffers,
    pub r#macro: TimeframeBuffers,
}

pub async fn build_pipelines(
    ctx: &PipelineContext,
    state: &Arc<AppState>,
    warmed_states: Option<(
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
    )>,
) -> PipelineArtifacts {
    let (snapshot_tx, snapshot_rx) = mpsc::channel::<NormalizedEvent>(500);
    let cancel = ctx.cancel.clone();

    let (micro_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (fast_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (slow_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (macro_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);

    let micro_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
        ctx.micro_cfg.candles.analysis_limit,
    )));
    let fast_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
        ctx.fast_cfg.candles.analysis_limit,
    )));
    let slow_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
        ctx.slow_cfg.candles.analysis_limit,
    )));
    let macro_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
        ctx.macro_cfg.candles.analysis_limit,
    )));

    let micro_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let fast_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let slow_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let macro_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));

    let micro_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(
        analyzer::HIST_BUFFER_MAX,
    )));
    let fast_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(
        analyzer::HIST_BUFFER_MAX,
    )));
    let slow_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(
        analyzer::HIST_BUFFER_MAX,
    )));
    let macro_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(
        analyzer::HIST_BUFFER_MAX,
    )));

    let active_pair = Arc::new(analyzer::ActivePair {
        symbol: ctx.internal_symbol.clone(),
        micro: analyzer::TimeframePipeline {
            history: micro_history.clone(),
            broadcast_tx: micro_broadcast_tx.clone(),
            latest_snapshot: micro_latest.clone(),
            snapshot_history: micro_snapshot_history.clone(),
            timeframe_secs: ctx.micro_cfg.candles.duration_seconds,
            timeframe_label: "Micro",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: ctx.fib_config.clone(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
        },
        fast: analyzer::TimeframePipeline {
            history: fast_history.clone(),
            broadcast_tx: fast_broadcast_tx.clone(),
            latest_snapshot: fast_latest.clone(),
            snapshot_history: fast_snapshot_history.clone(),
            timeframe_secs: ctx.fast_cfg.candles.duration_seconds,
            timeframe_label: "Fast",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: ctx.fib_config.clone(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
        },
        slow: analyzer::TimeframePipeline {
            history: slow_history.clone(),
            broadcast_tx: slow_broadcast_tx.clone(),
            latest_snapshot: slow_latest.clone(),
            snapshot_history: slow_snapshot_history.clone(),
            timeframe_secs: ctx.slow_cfg.candles.duration_seconds,
            timeframe_label: "Slow",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: ctx.fib_config.clone(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
        },
        r#macro: analyzer::TimeframePipeline {
            history: macro_history.clone(),
            broadcast_tx: macro_broadcast_tx.clone(),
            latest_snapshot: macro_latest.clone(),
            snapshot_history: macro_snapshot_history.clone(),
            timeframe_secs: ctx.macro_cfg.candles.duration_seconds,
            timeframe_label: "Macro",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: ctx.fib_config.clone(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
        },
        snapshot_tx: snapshot_tx.clone(),
        cancel: cancel.clone(),
        latest_oi: Arc::new(RwLock::new(None)),
        latest_funding: Arc::new(RwLock::new(None)),
        latest_mark_px: Arc::new(RwLock::new(None)),
        latest_index_px: Arc::new(RwLock::new(None)),
        cluster_matrix: Arc::new(RwLock::new(None)),
    });

    spawn_tasks(
        snapshot_rx,
        &ctx.base,
        &ctx.internal_symbol,
        &ctx.pair_key,
        &ctx.micro_cfg,
        &ctx.fast_cfg,
        &ctx.slow_cfg,
        &ctx.macro_cfg,
        &ctx.fib_config,
        &cancel,
        &micro_broadcast_tx,
        &fast_broadcast_tx,
        &slow_broadcast_tx,
        &macro_broadcast_tx,
        &micro_history,
        &fast_history,
        &slow_history,
        &macro_history,
        &micro_latest,
        &fast_latest,
        &slow_latest,
        &macro_latest,
        &micro_snapshot_history,
        &fast_snapshot_history,
        &slow_snapshot_history,
        &macro_snapshot_history,
        &active_pair,
        state,
        warmed_states,
        ctx.exchange_choice.clone(),
        ctx.quote.clone(),
        ctx.liquidity_config.clone(),
    )
    .await;

    let micro_buf = TimeframeBuffers {
        history: micro_history.clone(),
        latest: micro_latest.clone(),
        snapshot_history: micro_snapshot_history.clone(),
    };
    let fast_buf = TimeframeBuffers {
        history: fast_history.clone(),
        latest: fast_latest.clone(),
        snapshot_history: fast_snapshot_history.clone(),
    };
    let slow_buf = TimeframeBuffers {
        history: slow_history.clone(),
        latest: slow_latest.clone(),
        snapshot_history: slow_snapshot_history.clone(),
    };
let macro_buf = TimeframeBuffers {
        history: macro_history.clone(),
        latest: macro_latest.clone(),
        snapshot_history: macro_snapshot_history.clone(),
    };

    let instance = Arc::new(Instance::new(
        format!("inst_{}", uuid_v4_simple()),
        (ctx.base.clone(), ctx.quote.as_str().to_string()),
        active_pair.clone(),
        state.pool.clone(),
        state.config.clone(),
        ctx.intervals_config.clone(),
        ctx.safety_config.clone(),
        micro_buf.clone(),
        fast_buf.clone(),
        slow_buf.clone(),
        macro_buf.clone(),
        ctx.operational_mode.clone(),
    ));

    PipelineArtifacts {
        instance,
        micro: micro_buf,
        fast: fast_buf,
        slow: slow_buf,
        r#macro: macro_buf,
    }
}

#[allow(clippy::too_many_arguments)]
async fn spawn_tasks(
    snapshot_rx: mpsc::Receiver<NormalizedEvent>,
    base: &str,
    internal_symbol: &str,
    pair_key: &str,
    micro_cfg: &TimeframeConfig,
    fast_cfg: &TimeframeConfig,
    slow_cfg: &TimeframeConfig,
    macro_cfg: &TimeframeConfig,
    fib_config: &FibonacciConfig,
    cancel: &CancellationToken,
    micro_broadcast_tx: &tokio::sync::broadcast::Sender<MarketSnapshot>,
    fast_broadcast_tx: &tokio::sync::broadcast::Sender<MarketSnapshot>,
    slow_broadcast_tx: &tokio::sync::broadcast::Sender<MarketSnapshot>,
    macro_broadcast_tx: &tokio::sync::broadcast::Sender<MarketSnapshot>,
    micro_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    fast_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    slow_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    macro_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    micro_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    fast_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    slow_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    macro_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    micro_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    fast_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    slow_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    macro_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    active_pair: &Arc<analyzer::ActivePair>,
    state: &Arc<AppState>,
    warmed_states: Option<(
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
    )>,
    exchange_choice: ExchangeChoice,
    quote: Currency,
    liquidity_config: LiquidityConfig,
) {
    let (micro_chan_tx, micro_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (fast_chan_tx, fast_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (slow_chan_tx, slow_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (macro_chan_tx, macro_chan_rx) = mpsc::channel::<NormalizedEvent>(200);

    let router_symbol = internal_symbol.to_string();
    let router_cancel = cancel.clone();
    tokio::spawn(async move {
        analyzer::run_event_router(
            snapshot_rx,
            micro_chan_tx,
            fast_chan_tx,
            slow_chan_tx,
            macro_chan_tx,
            router_symbol,
            router_cancel,
        )
        .await;
    });

    let (candle_fwd_tx, mut candle_fwd_rx) =
        tokio::sync::mpsc::channel::<NormalizedCandle>(1000);
    let (candle_bcast_tx, candle_bcast_rx) =
        tokio::sync::broadcast::channel::<NormalizedCandle>(1200);

    tokio::spawn(async move {
        while let Some(candle) = candle_fwd_rx.recv().await {
            let _ = candle_bcast_tx.send(candle);
        }
    });

    let (agg_4h_tx, mut agg_4h_rx) =
        tokio::sync::mpsc::channel::<crate::candle_aggregator::AggregatedCandle>(200);
    let (agg_1d_tx, mut agg_1d_rx) =
        tokio::sync::mpsc::channel::<crate::candle_aggregator::AggregatedCandle>(200);
    let agg_symbol = internal_symbol.to_string();
    tokio::spawn(crate::candle_aggregator::spawn_candle_aggregator(
        agg_symbol.clone(),
        candle_bcast_rx,
        agg_4h_tx,
        agg_1d_tx,
    ));

    let logger_agg_telemetry = state.telemetry_tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(c4h) = agg_4h_rx.recv() => {
                    let _ = logger_agg_telemetry.send(db::TelemetryMsg::ConsoleLog(format!(
                        "🕯️  [{}] 4h Candle Aggregated | Close: ${:.4} | Sources: {}",
                        agg_symbol, c4h.candle.close, c4h.source_count
                    ))).await;
                }
                Some(c1d) = agg_1d_rx.recv() => {
                    let _ = logger_agg_telemetry.send(db::TelemetryMsg::ConsoleLog(format!(
                        "🕯️  [{}] 1d Candle Aggregated | Close: ${:.4} | Sources: {}",
                        agg_symbol, c1d.candle.close, c1d.source_count
                    ))).await;
                }
                else => break,
            }
        }
    });

    if let Some(ref ws) = warmed_states {
        for c in &ws.0.history {
            let _ = candle_fwd_tx.send(c.clone()).await;
        }
    }

    // Spawn 4 pipeline tasks
    let macro_secs = macro_cfg.candles.duration_seconds;
    let slow_secs = slow_cfg.candles.duration_seconds;
    let fast_secs = fast_cfg.candles.duration_seconds;
    let micro_secs = micro_cfg.candles.duration_seconds;
    let (w_micro, w_fast, w_slow, w_macro) = match &warmed_states {
        Some((m, s, med, l)) => (Some(m.clone()), Some(s.clone()), Some(med.clone()), Some(l.clone())),
        None => (None, None, None, None),
    };

    #[allow(clippy::type_complexity)]
    let pipeline_specs: Vec<(
        mpsc::Receiver<NormalizedEvent>,
        TimeframeConfig,
        Arc<RwLock<VecDeque<NormalizedCandle>>>,
        Arc<RwLock<Option<MarketSnapshot>>>,
        Arc<RwLock<VecDeque<MarketSnapshot>>>,
        &str,
        u64,
        tokio::sync::broadcast::Sender<MarketSnapshot>,
        Arc<tokio::sync::Mutex<DivergenceDetector>>,
        Option<tokio::sync::mpsc::Sender<NormalizedCandle>>,
        Option<analyzer::WarmedPipelineState>,
    )> = vec![
        (
            micro_chan_rx,
            micro_cfg.clone(),
            micro_history.clone(),
            micro_latest.clone(),
            micro_snapshot_history.clone(),
            "Micro",
            micro_secs,
            micro_broadcast_tx.clone(),
            active_pair.micro.divergence_detector.clone(),
            Some(candle_fwd_tx.clone()),
            w_micro,
        ),
        (
            fast_chan_rx,
            fast_cfg.clone(),
            fast_history.clone(),
            fast_latest.clone(),
            fast_snapshot_history.clone(),
            "Fast",
            fast_secs,
            fast_broadcast_tx.clone(),
            active_pair.fast.divergence_detector.clone(),
            None,
            w_fast,
        ),
        (
            slow_chan_rx,
            slow_cfg.clone(),
            slow_history.clone(),
            slow_latest.clone(),
            slow_snapshot_history.clone(),
            "Slow",
            slow_secs,
            slow_broadcast_tx.clone(),
            active_pair.slow.divergence_detector.clone(),
            None,
            w_slow,
        ),
        (
            macro_chan_rx,
            macro_cfg.clone(),
            macro_history.clone(),
            macro_latest.clone(),
            macro_snapshot_history.clone(),
            "Macro",
            macro_secs,
            macro_broadcast_tx.clone(),
            active_pair.r#macro.divergence_detector.clone(),
            None,
            w_macro,
        ),
    ];

    for (rx, tf_cfg, hist, snap, snap_hist, label, tf_secs, bcast, div_det, candle_fwd, warmed) in
        pipeline_specs
    {
        let a_symbol = internal_symbol.to_string();
        let a_pair_key = pair_key.to_string();
        let a_telemetry = state.telemetry_tx.clone();
        let a_cancel = cancel.clone();
        let a_fib = fib_config.clone();
        let a_pool = state.pool.clone();
        let a_latest_oi = active_pair.latest_oi.clone();
        let a_latest_funding = active_pair.latest_funding.clone();
        let a_latest_mark = active_pair.latest_mark_px.clone();
        let a_latest_index = active_pair.latest_index_px.clone();
        let a_cluster_matrix = active_pair.cluster_matrix.clone();
        tokio::spawn(async move {
            analyzer::run_single(
                rx,
                a_telemetry,
                bcast,
                tf_cfg,
                a_fib,
                shared::statistics::StatisticsConfig::default(),
                div_det,
                hist,
                snap,
                snap_hist,
                a_symbol,
                a_pair_key,
                tf_secs,
                label,
                a_cancel,
                candle_fwd,
                warmed,
                Some(a_pool),
                a_latest_oi,
                a_latest_funding,
                a_latest_mark,
                a_latest_index,
                a_cluster_matrix,
                crate::config::OrderBookConfig::default(),
            )
            .await;
        });
    }

    // WebSocket adapter (perpetual futures on all exchanges)
    let ws_symbol = exchange_choice.raw_symbol(base, &quote);
    let ws_product_type = exchange_choice
        .bitget_product_type(&quote)
        .unwrap_or("")
        .to_string();
    let ws_internal = internal_symbol.to_string();
    let ws_tx = active_pair.snapshot_tx.clone();
    let ws_cancel = cancel.clone();
    let ws_url = if exchange_choice == ExchangeChoice::Bitget {
        state.bitget_ws_url.clone()
    } else {
        state.ws_url.clone()
    };
    let exchange_for_spawn = exchange_choice.clone();
    tokio::spawn(async move {
        if exchange_for_spawn == ExchangeChoice::Bitget {
            crate::adapters::bitget::run_for_symbol(ws_symbol, ws_internal, ws_product_type, ws_tx, ws_cancel, &ws_url).await;
        } else {
            crate::adapters::hyperliquid::run_for_symbol(ws_symbol, ws_internal, ws_tx, ws_cancel, &ws_url).await;
        }
    });

    // Phase 0: HL derivatives poller (mark price + OI + funding).
    // Bitget already pushes these natively on the WS adapter above.
    if liquidity_config.enabled && exchange_choice != ExchangeChoice::Bitget {
        let hl_info_url = state.ws_url
            .replace("wss://", "https://")
            .replace("ws://", "http://")
            .replace("/ws", "/info");
        let poller_raw = exchange_choice.raw_symbol(base, &quote);
        let poller_internal = internal_symbol.to_string();
        let poller_tx = std::sync::Arc::new(active_pair.snapshot_tx.clone());
        let poller_cancel = cancel.clone();
        let poll_ms = liquidity_config.mark_price_poll_ms;
        crate::adapters::hl_derivatives_poller::spawn_hl_derivatives_poller(
            poller_raw,
            poller_internal,
            hl_info_url,
            poller_tx,
            poller_cancel,
            poll_ms,
        );
    }

    // Phase 2: Liquidation cluster-matrix refresh task. Runs every
    // 5 minutes (configurable), computes an estimated cluster matrix
    // from current OI + funding + price history, and writes it to the
    // shared handle on the active pair.
    if liquidity_config.enabled {
        let cluster_handle = active_pair.cluster_matrix.clone();
        let active_pair_clone = active_pair.clone();
        let refresh_config = liquidity_config.clone();
        let cancel_for_refresh = cancel.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                refresh_config.cluster_refresh_secs.max(30),
            ));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await; // skip the immediate first tick
            loop {
                tokio::select! {
                    biased;
                    _ = cancel_for_refresh.cancelled() => break,
                    _ = interval.tick() => {}
                }
                if let Some(m) = compute_cluster_from_active_pair(
                    &active_pair_clone,
                    &refresh_config,
                ).await {
                    *cluster_handle.write().await = Some(m);
                }
            }
        });
    }
}

/// Compute a cluster matrix from an active pair's micro buffer (no
/// full `Instance` needed). Used by the cluster refresh task spawned
/// in `spawn_tasks`.
async fn compute_cluster_from_active_pair(
    active_pair: &Arc<analyzer::ActivePair>,
    config: &crate::config::LiquidityConfig,
) -> Option<shared::liquidity::LiquidationClusterMatrix> {
    use shared::liquidity::{estimate_clusters, ClusterEstimateInput};

    let micro = active_pair.micro.latest_snapshot.read().await.clone()?;
    let mid = micro.mid_price.to_f64()?;
    if mid <= 0.0 { return None; }
    let funding = micro.funding_rate.and_then(|d| d.to_f64()).unwrap_or(0.0);
    let oi = micro.open_interest.and_then(|d| d.to_f64()).unwrap_or(0.0);
    if oi <= 0.0 { return None; }

    let history_handle = active_pair.micro.history.read().await;
    let price_history: Vec<f64> = history_handle
        .iter()
        .rev()
        .take(200)
        .filter_map(|c| c.close.to_f64())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    drop(history_handle);

    let symbol = micro.symbol.clone();
    let input = ClusterEstimateInput {
        symbol: &symbol,
        mid_price: mid,
        price_history: &price_history,
        total_oi_usd: oi,
        funding_rate: funding,
        long_oi_pct: None,
        maintenance_margin_rate: config.maintenance_margin_rate,
        funding_extreme_pct: config.funding_extreme_pct,
        funding_modulation_active: true,
        leverage_buckets: &[1, 3, 5, 10, 20, 50, 100],
        leverage_weights: &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
        min_cluster_notional_usd: 50_000.0,
    };
    Some(estimate_clusters(&input))
}

fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:016x}", ts)
}
