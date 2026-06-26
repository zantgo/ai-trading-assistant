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
    pub short_cfg: TimeframeConfig,
    pub medium_cfg: TimeframeConfig,
    pub large_cfg: TimeframeConfig,
    pub fib_config: FibonacciConfig,
    pub safety_config: SafetyConfig,
    pub intervals_config: IntervalsConfig,
    pub cancel: CancellationToken,
}

pub struct PipelineArtifacts {
    pub instance: Arc<Instance>,
    pub micro: TimeframeBuffers,
    pub short: TimeframeBuffers,
    pub medium: TimeframeBuffers,
    pub large: TimeframeBuffers,
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
    let (short_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (medium_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (large_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);

    let micro_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
        ctx.micro_cfg.candles.analysis_limit,
    )));
    let short_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
        ctx.short_cfg.candles.analysis_limit,
    )));
    let medium_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
        ctx.medium_cfg.candles.analysis_limit,
    )));
    let large_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(
        ctx.large_cfg.candles.analysis_limit,
    )));

    let micro_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let short_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let medium_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let large_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));

    let micro_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(
        analyzer::HIST_BUFFER_MAX,
    )));
    let short_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(
        analyzer::HIST_BUFFER_MAX,
    )));
    let medium_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(
        analyzer::HIST_BUFFER_MAX,
    )));
    let large_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(
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
        short: analyzer::TimeframePipeline {
            history: short_history.clone(),
            broadcast_tx: short_broadcast_tx.clone(),
            latest_snapshot: short_latest.clone(),
            snapshot_history: short_snapshot_history.clone(),
            timeframe_secs: ctx.short_cfg.candles.duration_seconds,
            timeframe_label: "Small",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: ctx.fib_config.clone(),
        },
        medium: analyzer::TimeframePipeline {
            history: medium_history.clone(),
            broadcast_tx: medium_broadcast_tx.clone(),
            latest_snapshot: medium_latest.clone(),
            snapshot_history: medium_snapshot_history.clone(),
            timeframe_secs: ctx.medium_cfg.candles.duration_seconds,
            timeframe_label: "Medium",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: ctx.fib_config.clone(),
        },
        large: analyzer::TimeframePipeline {
            history: large_history.clone(),
            broadcast_tx: large_broadcast_tx.clone(),
            latest_snapshot: large_latest.clone(),
            snapshot_history: large_snapshot_history.clone(),
            timeframe_secs: ctx.large_cfg.candles.duration_seconds,
            timeframe_label: "Large",
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
        &ctx.short_cfg,
        &ctx.medium_cfg,
        &ctx.large_cfg,
        &ctx.fib_config,
        &cancel,
        &micro_broadcast_tx,
        &short_broadcast_tx,
        &medium_broadcast_tx,
        &large_broadcast_tx,
        &micro_history,
        &short_history,
        &medium_history,
        &large_history,
        &micro_latest,
        &short_latest,
        &medium_latest,
        &large_latest,
        &micro_snapshot_history,
        &short_snapshot_history,
        &medium_snapshot_history,
        &large_snapshot_history,
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
    let short_buf = TimeframeBuffers {
        history: short_history.clone(),
        latest: short_latest.clone(),
        snapshot_history: short_snapshot_history.clone(),
    };
    let medium_buf = TimeframeBuffers {
        history: medium_history.clone(),
        latest: medium_latest.clone(),
        snapshot_history: medium_snapshot_history.clone(),
    };
    let large_buf = TimeframeBuffers {
        history: large_history.clone(),
        latest: large_latest.clone(),
        snapshot_history: large_snapshot_history.clone(),
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
        short_buf.clone(),
        medium_buf.clone(),
        large_buf.clone(),
    ));

    let auto_ctx = automation::AutomationContext {
        pair_key: ctx.pair_key.clone(),
        symbol: ctx.base.clone(),
        micro_history: micro_history.clone(),
        short_history: short_history.clone(),
        medium_history: medium_history.clone(),
        large_history: large_history.clone(),
        micro_latest: micro_latest.clone(),
        short_latest: short_latest.clone(),
        medium_latest: medium_latest.clone(),
        large_latest: large_latest.clone(),
        micro_snapshot_history: micro_snapshot_history.clone(),
        short_snapshot_history: short_snapshot_history.clone(),
        medium_snapshot_history: medium_snapshot_history.clone(),
        large_snapshot_history: large_snapshot_history.clone(),
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
        short: short_buf,
        medium: medium_buf,
        large: large_buf,
    }
}

#[allow(clippy::too_many_arguments)]
async fn spawn_tasks(
    snapshot_rx: mpsc::Receiver<NormalizedEvent>,
    base: &str,
    pair_key: &str,
    micro_cfg: &TimeframeConfig,
    short_cfg: &TimeframeConfig,
    medium_cfg: &TimeframeConfig,
    large_cfg: &TimeframeConfig,
    fib_config: &FibonacciConfig,
    cancel: &CancellationToken,
    micro_broadcast_tx: &tokio::sync::broadcast::Sender<MarketSnapshot>,
    short_broadcast_tx: &tokio::sync::broadcast::Sender<MarketSnapshot>,
    medium_broadcast_tx: &tokio::sync::broadcast::Sender<MarketSnapshot>,
    large_broadcast_tx: &tokio::sync::broadcast::Sender<MarketSnapshot>,
    micro_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    short_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    medium_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    large_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    micro_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    short_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    medium_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    large_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    micro_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    short_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    medium_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    large_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
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
    let (short_chan_tx, short_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (medium_chan_tx, medium_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (large_chan_tx, large_chan_rx) = mpsc::channel::<NormalizedEvent>(200);

    let router_symbol = base.to_string();
    let router_cancel = cancel.clone();
    tokio::spawn(async move {
        analyzer::run_event_router(
            snapshot_rx,
            micro_chan_tx,
            short_chan_tx,
            medium_chan_tx,
            large_chan_tx,
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
        loop {
            match candle_fwd_rx.recv().await {
                Some(candle) => {
                    let _ = candle_bcast_tx.send(candle);
                }
                None => break,
            }
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
    let large_secs = large_cfg.candles.duration_seconds;
    let medium_secs = medium_cfg.candles.duration_seconds;
    let short_secs = short_cfg.candles.duration_seconds;
    let micro_secs = micro_cfg.candles.duration_seconds;
    let (w_micro, w_short, w_medium, w_large) = match &warmed_states {
        Some((m, s, med, l)) => (Some(m.clone()), Some(s.clone()), Some(med.clone()), Some(l.clone())),
        None => (None, None, None, None),
    };

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
            short_chan_rx,
            short_cfg.clone(),
            short_history.clone(),
            short_latest.clone(),
            short_snapshot_history.clone(),
            "Small",
            short_secs,
            short_broadcast_tx.clone(),
            active_pair.short.divergence_detector.clone(),
            None,
            w_short,
        ),
        (
            medium_chan_rx,
            medium_cfg.clone(),
            medium_history.clone(),
            medium_latest.clone(),
            medium_snapshot_history.clone(),
            "Medium",
            medium_secs,
            medium_broadcast_tx.clone(),
            active_pair.medium.divergence_detector.clone(),
            None,
            w_medium,
        ),
        (
            large_chan_rx,
            large_cfg.clone(),
            large_history.clone(),
            large_latest.clone(),
            large_snapshot_history.clone(),
            "Large",
            large_secs,
            large_broadcast_tx.clone(),
            active_pair.large.divergence_detector.clone(),
            None,
            w_large,
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
