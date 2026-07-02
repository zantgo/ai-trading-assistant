use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::analyzer;
use crate::automation;
use crate::config::{FibonacciConfig, IntervalsConfig, SafetyConfig, TimeframeConfig};
use crate::db;
use crate::instance::{Instance, TimeframeBuffers};
use crate::llm::LlmClient;
use crate::portfolio_risk::PortfolioRiskState;
use crate::sr_engine::SrRoleTracker;
use crate::workspace::Workspace;
use shared::indicators::DivergenceDetector;
use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedCandle, NormalizedEvent};
use tokio_util::sync::CancellationToken;

pub struct PipelineContext {
    pub base: String,
    pub pair_key: String,
    pub micro_cfg: TimeframeConfig,
    pub fast_cfg: TimeframeConfig,
    pub slow_cfg: TimeframeConfig,
    pub macro_cfg: TimeframeConfig,
    pub fib_config: FibonacciConfig,
    pub safety_config: SafetyConfig,
    pub intervals_config: IntervalsConfig,
    pub cancel: CancellationToken,
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
    workspace: &Arc<Workspace>,
    llm_client: Arc<LlmClient>,
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
        symbol: ctx.base.clone(),
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
        },
        snapshot_tx: snapshot_tx.clone(),
        cancel: cancel.clone(),
    });

    spawn_tasks(
        snapshot_rx,
        &ctx.base,
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
        workspace,
        warmed_states,
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
        (ctx.base.clone(), "USDT".to_string()),
        active_pair.clone(),
        workspace.pool.clone(),
        workspace.config.clone(),
        ctx.intervals_config.clone(),
        ctx.safety_config.clone(),
        micro_buf.clone(),
        fast_buf.clone(),
        slow_buf.clone(),
        macro_buf.clone(),
    ));

    let auto_ctx = automation::AutomationContext {
        pair_key: ctx.pair_key.clone(),
        symbol: ctx.base.clone(),
        micro_history: micro_history.clone(),
        fast_history: fast_history.clone(),
        slow_history: slow_history.clone(),
        macro_history: macro_history.clone(),
        micro_latest: micro_latest.clone(),
        fast_latest: fast_latest.clone(),
        slow_latest: slow_latest.clone(),
        macro_latest: macro_latest.clone(),
        micro_snapshot_history: micro_snapshot_history.clone(),
        fast_snapshot_history: fast_snapshot_history.clone(),
        slow_snapshot_history: slow_snapshot_history.clone(),
        macro_snapshot_history: macro_snapshot_history.clone(),
        config: workspace.config.clone(),
        pool: workspace.pool.clone(),
        llm_client,
        telemetry_tx: workspace.telemetry_tx.clone(),
        cancel: cancel.clone(),
        api_key_configured: workspace.api_key_configured.clone(),
        portfolio_risk: Arc::new(PortfolioRiskState::default()),
        pair_close_histories: Arc::new(RwLock::new(HashMap::new())),
        safety: instance.safety.clone(),
        intervals: instance.config_state.read().await.intervals.clone(),
        next_interval_override: Arc::new(RwLock::new(None)),
    };
    *instance.automation_ctx.write().await = Some(auto_ctx);

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
    workspace: &Arc<Workspace>,
    warmed_states: Option<(
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
    )>,
) {
    let (micro_chan_tx, micro_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (fast_chan_tx, fast_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (slow_chan_tx, slow_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (macro_chan_tx, macro_chan_rx) = mpsc::channel::<NormalizedEvent>(200);

    let router_symbol = base.to_string();
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
    let agg_symbol = base.to_string();
    tokio::spawn(crate::candle_aggregator::spawn_candle_aggregator(
        agg_symbol.clone(),
        candle_bcast_rx,
        agg_4h_tx,
        agg_1d_tx,
    ));

    let logger_agg_telemetry = workspace.telemetry_tx.clone();
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
        let a_symbol = base.to_string();
        let a_pair_key = pair_key.to_string();
        let a_telemetry = workspace.telemetry_tx.clone();
        let a_cancel = cancel.clone();
        let a_fib = fib_config.clone();
        let a_pool = workspace.pool.clone();
        tokio::spawn(async move {
            analyzer::run_single(
                rx,
                a_telemetry,
                bcast,
                tf_cfg,
                a_fib,
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
            )
            .await;
        });
    }

    // WebSocket adapter
    let ws_symbol = base.to_string();
    let ws_tx = active_pair.snapshot_tx.clone();
    let ws_cancel = cancel.clone();
    let ws_url = workspace.ws_url.clone();
    tokio::spawn(async move {
        crate::adapters::hyperliquid::run_for_symbol(ws_symbol, ws_tx, ws_cancel, &ws_url).await;
    });
}

fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:016x}", ts)
}
