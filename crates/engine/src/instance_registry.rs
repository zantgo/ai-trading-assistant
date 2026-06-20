use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;

use crate::analyzer;
use crate::automation;
use crate::config::{TimeframeConfig};
use crate::instance::{Instance, InstanceStatus};
use crate::workspace::Workspace;
use crate::portfolio_risk::PortfolioRiskState;
use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedEvent, NormalizedCandle};
use shared::indicators::DivergenceDetector;
use crate::sr_engine::SrRoleTracker;
use crate::adapters;
use crate::db;

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstanceSummary {
    pub id: String,
    pub pair: String,
    pub status: String,
    pub symbol: String,
    pub initial_capital: f64,
    pub current_equity: f64,
    pub consecutive_losses: u32,
    pub caution_level: String,
}

/// Add a new instance to the workspace, starting all pipeline tasks.
pub async fn add_instance(
    workspace: &Arc<Workspace>,
    pair: (String, String),
    llm_client: Arc<RwLock<crate::llm::LlmClient>>,
) -> Result<Arc<Instance>, String> {
    let current_count = workspace.instance_count().await;
    let max_count = workspace.max_instances().await;
    if current_count >= max_count {
        return Err(format!(
            "Maximum instance count reached ({}/{}). Remove an instance first.",
            current_count, max_count
        ));
    }

    let id = format!("inst_{}", uuid_v4_simple());
    let (base, quote) = (pair.0.clone(), pair.1.clone());
    let pair_key = format!("{}-{}", base, quote);
    let normalized = format!("{}-{}", base, quote);

    // Check for duplicate
    {
        let instances = workspace.instances.read().await;
        if instances.contains_key(&pair_key) {
            return Err(format!("Instance for pair {} already exists", pair_key));
        }
    }

    // Register symbol mapping
    let exchange_enum = shared::normalized::Exchange::Hyperliquid;
    workspace.symbol_mapper.register(exchange_enum, &base, &normalized).await;

    // Build pipeline configs
    let config_guard = workspace.config.read().await;
    let pair_cfg = config_guard.instances.get(&pair_key);
    let default_indicators = config_guard.indicators.clone();
    let fib_config = config_guard.fibonacci.clone();
    let safety_config = config_guard.safety.clone();
    let intervals_config = config_guard.intervals.clone();

    let micro_cfg = pair_cfg
        .map(|p| p.micro_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(60, default_indicators.clone()));
    let short_cfg = pair_cfg
        .map(|p| p.short_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(300, default_indicators.clone()));
    let medium_cfg = pair_cfg
        .and_then(|p| p.medium_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(
            config_guard.medium_timeframe.duration_seconds,
            default_indicators.clone(),
        ));
    let large_cfg = pair_cfg
        .and_then(|p| p.large_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(
            config_guard.large_timeframe.duration_seconds,
            default_indicators.clone(),
        ));
    let drop_fib = fib_config.clone();
    let rest_url = config_guard.hyperliquid.rest_url();
    drop(config_guard);

    let (snapshot_tx, snapshot_rx) = mpsc::channel::<NormalizedEvent>(500);
    let cancel = CancellationToken::new();

    let (micro_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (short_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (medium_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (large_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);

    let micro_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(micro_cfg.candles.analysis_limit)));
    let short_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(short_cfg.candles.analysis_limit)));
    let medium_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(medium_cfg.candles.analysis_limit)));
    let large_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(large_cfg.candles.analysis_limit)));

    let micro_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let short_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let medium_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let large_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));

    let micro_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(analyzer::HIST_BUFFER_MAX)));
    let short_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(analyzer::HIST_BUFFER_MAX)));
    let medium_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(analyzer::HIST_BUFFER_MAX)));
    let large_snapshot_history = Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::with_capacity(analyzer::HIST_BUFFER_MAX)));

    // ── Historical Bootstrap ──────────────────────────────────────────
    // Fetch historical candles per timeframe from the Hyperliquid REST API,
    // warm all technical indicators, and pre-populate the in-memory buffers so
    // the frontend renders fully formed charts on first load.

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let micro_secs = micro_cfg.candles.duration_seconds;
    let short_secs = short_cfg.candles.duration_seconds;
    let medium_secs = medium_cfg.candles.duration_seconds;
    let large_secs = large_cfg.candles.duration_seconds;

    let micro_limit = micro_cfg.candles.analysis_limit as u64;
    let short_limit = short_cfg.candles.analysis_limit as u64;
    let medium_limit = medium_cfg.candles.analysis_limit as u64;
    let large_limit = large_cfg.candles.analysis_limit as u64;

    let micro_start = now_ms.saturating_sub(micro_secs * micro_limit * 1000);
    let short_start = now_ms.saturating_sub(short_secs * short_limit * 1000);
    let medium_start = now_ms.saturating_sub(medium_secs * medium_limit * 1000);
    let large_start = now_ms.saturating_sub(large_secs * large_limit * 1000);

    let fetch_base = base.clone();
    let fetch_rest = rest_url.clone();

    let bootstrap_result: Result<
        (Vec<NormalizedCandle>, Vec<NormalizedCandle>, Vec<NormalizedCandle>, Vec<NormalizedCandle>),
        String,
    > = tokio::try_join!(
        adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            adapters::hyperliquid_rest::timeframe_secs_to_interval(micro_secs),
            micro_start, now_ms, &fetch_rest,
        ),
        adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            adapters::hyperliquid_rest::timeframe_secs_to_interval(short_secs),
            short_start, now_ms, &fetch_rest,
        ),
        adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            adapters::hyperliquid_rest::timeframe_secs_to_interval(medium_secs),
            medium_start, now_ms, &fetch_rest,
        ),
        adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            adapters::hyperliquid_rest::timeframe_secs_to_interval(large_secs),
            large_start, now_ms, &fetch_rest,
        ),
    );

    let (warmed_micro, warmed_short, warmed_medium, warmed_large, historical_micro) = match bootstrap_result {
        Ok((mc, sc, mc2, lc)) => {
            println!("📡 Historical Bootstrap [{}]: Fetched {}/{}/{}/{} candles (1m/5m/15m/1h)",
                base, mc.len(), sc.len(), mc2.len(), lc.len());

            let w_micro = analyzer::warm_indicators_for_timeframe(
                mc, &micro_cfg, &drop_fib, &base, micro_secs,
            );
            let w_short = analyzer::warm_indicators_for_timeframe(
                sc, &short_cfg, &drop_fib, &base, short_secs,
            );
            let w_medium = analyzer::warm_indicators_for_timeframe(
                mc2.clone(), &medium_cfg, &drop_fib, &base, medium_secs,
            );
            let w_large = analyzer::warm_indicators_for_timeframe(
                lc, &large_cfg, &drop_fib, &base, large_secs,
            );

            (Some(w_micro), Some(w_short), Some(w_medium), Some(w_large), Some(mc2))
        }
        Err(e) => {
            eprintln!("⚠️  Historical Bootstrap [{}]: REST fetch failed — {}. Falling back to live-only data.", base, e);
            (None, None, None, None, None)
        }
    };

    // Pre-populate history deques and latest snapshots from warmed state
    if let Some(ref w) = warmed_micro {
        {
            let mut hist = micro_history.write().await;
            for c in &w.history { hist.push_back(c.clone()); }
        }
        if let Some(ref snap) = w.latest_snapshot {
            *micro_latest.write().await = Some(snap.clone());
        }
        {
            let mut sh = micro_snapshot_history.write().await;
            for snap in &w.snapshot_history { sh.push_back(snap.clone()); }
        }
    }
    if let Some(ref w) = warmed_short {
        {
            let mut hist = short_history.write().await;
            for c in &w.history { hist.push_back(c.clone()); }
        }
        if let Some(ref snap) = w.latest_snapshot {
            *short_latest.write().await = Some(snap.clone());
        }
        {
            let mut sh = short_snapshot_history.write().await;
            for snap in &w.snapshot_history { sh.push_back(snap.clone()); }
        }
    }
    if let Some(ref w) = warmed_medium {
        {
            let mut hist = medium_history.write().await;
            for c in &w.history { hist.push_back(c.clone()); }
        }
        if let Some(ref snap) = w.latest_snapshot {
            *medium_latest.write().await = Some(snap.clone());
        }
        {
            let mut sh = medium_snapshot_history.write().await;
            for snap in &w.snapshot_history { sh.push_back(snap.clone()); }
        }
    }
    if let Some(ref w) = warmed_large {
        {
            let mut hist = large_history.write().await;
            for c in &w.history { hist.push_back(c.clone()); }
        }
        if let Some(ref snap) = w.latest_snapshot {
            *large_latest.write().await = Some(snap.clone());
        }
        {
            let mut sh = large_snapshot_history.write().await;
            for snap in &w.snapshot_history { sh.push_back(snap.clone()); }
        }
    }

    let active_pair = Arc::new(analyzer::ActivePair {
        symbol: base.clone(),
        micro: analyzer::TimeframePipeline {
            history: micro_history.clone(),
            broadcast_tx: micro_broadcast_tx.clone(),
            latest_snapshot: micro_latest.clone(),
            snapshot_history: micro_snapshot_history.clone(),
            timeframe_secs: 60,
            timeframe_label: "Micro",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: drop_fib.clone(),
        },
        short: analyzer::TimeframePipeline {
            history: short_history.clone(),
            broadcast_tx: short_broadcast_tx.clone(),
            latest_snapshot: short_latest.clone(),
            snapshot_history: short_snapshot_history.clone(),
            timeframe_secs: 300,
            timeframe_label: "Small",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: drop_fib.clone(),
        },
        medium: analyzer::TimeframePipeline {
            history: medium_history.clone(),
            broadcast_tx: medium_broadcast_tx.clone(),
            latest_snapshot: medium_latest.clone(),
            snapshot_history: medium_snapshot_history.clone(),
            timeframe_secs: medium_cfg.candles.duration_seconds,
            timeframe_label: "Medium",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: drop_fib.clone(),
        },
        large: analyzer::TimeframePipeline {
            history: large_history.clone(),
            broadcast_tx: large_broadcast_tx.clone(),
            latest_snapshot: large_latest.clone(),
            snapshot_history: large_snapshot_history.clone(),
            timeframe_secs: large_cfg.candles.duration_seconds,
            timeframe_label: "Large",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: drop_fib.clone(),
        },
        snapshot_tx: snapshot_tx.clone(),
        cancel: cancel.clone(),
    });

    // Spawn event router (fan out to 4 timeframes)
    let (micro_chan_tx, micro_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (short_chan_tx, short_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (medium_chan_tx, medium_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (large_chan_tx, large_chan_rx) = mpsc::channel::<NormalizedEvent>(200);

    let router_symbol = base.clone();
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
        ).await;
    });

    // ── Candle Forwarder & Aggregator (spawned early to receive historical micro candles) ──
    let (candle_fwd_tx, mut candle_fwd_rx) = tokio::sync::mpsc::unbounded_channel::<NormalizedCandle>();
    let (candle_bcast_tx, candle_bcast_rx) = tokio::sync::broadcast::channel::<NormalizedCandle>(1200);

    tokio::spawn(async move {
        loop {
            match candle_fwd_rx.recv().await {
                Some(candle) => { let _ = candle_bcast_tx.send(candle); }
                None => break,
            }
        }
    });

    let (agg_4h_tx, mut agg_4h_rx) = tokio::sync::mpsc::channel::<crate::candle_aggregator::AggregatedCandle>(200);
    let (agg_1d_tx, mut agg_1d_rx) = tokio::sync::mpsc::channel::<crate::candle_aggregator::AggregatedCandle>(200);
    let agg_symbol = base.clone();
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

    // Route historical micro candles through the aggregator for 4h/1d bootstrap
    if let Some(ref hist_micro) = historical_micro {
        for c in hist_micro {
            let _ = candle_fwd_tx.send(c.clone());
        }
    }

    // ── Spawn 4 pipeline tasks (with pre-warmed state if available) ──
    let large_secs = large_cfg.candles.duration_seconds;
    let medium_secs = medium_cfg.candles.duration_seconds;

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
        Option<tokio::sync::mpsc::UnboundedSender<NormalizedCandle>>,
        Option<analyzer::WarmedPipelineState>,
    )> = vec![
        (micro_chan_rx, micro_cfg.clone(), micro_history.clone(), micro_latest.clone(), micro_snapshot_history.clone(), "Micro", 60u64, micro_broadcast_tx.clone(), active_pair.micro.divergence_detector.clone(), Some(candle_fwd_tx.clone()), warmed_micro),
        (short_chan_rx, short_cfg.clone(), short_history.clone(), short_latest.clone(), short_snapshot_history.clone(), "Small", 300u64, short_broadcast_tx.clone(), active_pair.short.divergence_detector.clone(), None, warmed_short),
        (medium_chan_rx, medium_cfg, medium_history.clone(), medium_latest.clone(), medium_snapshot_history.clone(), "Medium", medium_secs, medium_broadcast_tx.clone(), active_pair.medium.divergence_detector.clone(), None, warmed_medium),
        (large_chan_rx, large_cfg, large_history.clone(), large_latest.clone(), large_snapshot_history.clone(), "Large", large_secs, large_broadcast_tx.clone(), active_pair.large.divergence_detector.clone(), None, warmed_large),
    ];

    for (rx, tf_cfg, hist, snap, snap_hist, label, tf_secs, bcast, div_det, candle_fwd, warmed) in pipeline_specs {
        let a_symbol = base.clone();
        let a_pair_key = pair_key.clone();
        let a_telemetry = workspace.telemetry_tx.clone();
        let a_cancel = cancel.clone();
        let a_fib = drop_fib.clone();
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
            ).await;
        });
    }

    // WebSocket adapter (connect only after historical data is loaded)
    let ws_symbol = base.clone();
    let ws_tx = snapshot_tx.clone();
    let ws_cancel = cancel.clone();
    let ws_url = workspace.ws_url.clone();
    tokio::spawn(async move {
        adapters::hyperliquid::run_for_symbol(ws_symbol, ws_tx, ws_cancel, &ws_url).await;
    });

    let instance = Arc::new(Instance::new(
        id.clone(),
        pair,
        active_pair,
        workspace.pool.clone(),
        workspace.config.clone(),
        intervals_config,
        safety_config,
        micro_history.clone(),
        short_history.clone(),
        medium_history.clone(),
        large_history.clone(),
        micro_latest.clone(),
        short_latest.clone(),
        medium_latest.clone(),
        large_latest.clone(),
        micro_snapshot_history.clone(),
        short_snapshot_history.clone(),
        medium_snapshot_history.clone(),
        large_snapshot_history.clone(),
    ));

    // Build the automation context and store it on the instance
    let auto_ctx = automation::AutomationContext {
        pair_key: pair_key.clone(),
        symbol: base.clone(),
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
        intervals: instance.intervals.read().await.clone(),
        next_interval_override: Arc::new(RwLock::new(None)),
    };
    *instance.automation_ctx.write().await = Some(auto_ctx);

    // Persist symbol to config
    {
        let mut config = workspace.config.write().await;
        if !config.symbols.contains(&base) {
            config.symbols.push(base.clone());
            if let Ok(toml_str) = toml::to_string_pretty(&*config) {
                let _ = std::fs::write("config.toml", toml_str);
            }
        }
    }

    // Register instance
    workspace.instances.write().await.insert(pair_key, Arc::clone(&instance));

    println!("✅ Instance created: {} ({})", instance.pair_display(), instance.id);
    Ok(instance)
}

/// Pause an instance (no new trades, keep open positions for TP/SL).
pub async fn pause_instance(workspace: &Arc<Workspace>, instance_id: &str) -> Result<(), String> {
    let instances = workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id)
        .cloned()
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;
    drop(instances);

    let mut status = instance.status.write().await;
    if *status == InstanceStatus::Stopped {
        return Err("Cannot pause a stopped instance".to_string());
    }
    *status = InstanceStatus::Paused;
    println!("⏸️  Instance paused: {} ({})", instance.pair_display(), instance_id);
    Ok(())
}

/// Stop an instance (close all positions immediately).
pub async fn stop_instance(workspace: &Arc<Workspace>, instance_id: &str) -> Result<(), String> {
    let instances = workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id)
        .cloned()
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;
    drop(instances);

    // Close any open paper positions
    let symbol = instance.symbol();
    let _ = crate::db::paper_get_active_position(&instance.pool, &symbol).await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let _ = workspace.telemetry_tx.send(db::TelemetryMsg::PaperClosePosition {
        symbol: symbol.clone(),
        exit_price: 0.0,
        exit_timestamp: now,
        trigger: "STOP".to_string(),
    }).await;

    let mut status = instance.status.write().await;
    *status = InstanceStatus::Stopped;
    instance.cancel.cancel();
    println!("🛑 Instance stopped: {} ({})", instance.pair_display(), instance_id);
    Ok(())
}

/// Delete an instance (stop first, then remove from registry).
pub async fn delete_instance(workspace: &Arc<Workspace>, instance_id: &str) -> Result<(), String> {
    // Find the pair key for this instance
    let (pair_key, instance) = {
        let instances = workspace.instances.read().await;
        let (pk, inst) = instances.iter()
            .find(|(_, i)| i.id == instance_id)
            .map(|(k, i)| (k.clone(), Arc::clone(i)))
            .ok_or_else(|| format!("Instance {} not found", instance_id))?;
        (pk, inst)
    };

    // Stop if still running
    {
        let status = instance.status.read().await;
        if *status != InstanceStatus::Stopped {
            drop(status);
            stop_instance(workspace, instance_id).await?;
        }
    }

    // Remove from registry
    workspace.instances.write().await.remove(&pair_key);

    // Remove symbol from config
    {
        let mut config = workspace.config.write().await;
        let base_symbol = &instance.pair.0;
        if let Some(pos) = config.symbols.iter().position(|s| s == base_symbol) {
            config.symbols.remove(pos);
            if let Ok(toml_str) = toml::to_string_pretty(&*config) {
                let _ = std::fs::write("config.toml", toml_str);
            }
        }
        config.instances.remove(&pair_key);
        crate::config::save_instances(&config.instances);
    }

    println!("🗑️  Instance deleted: {} ({})", instance.pair_display(), instance_id);
    Ok(())
}

pub async fn list_instances(workspace: &Arc<Workspace>) -> Vec<InstanceSummary> {
    let instances = workspace.instances.read().await;
    let mut summaries = Vec::new();
    for (_, inst) in instances.iter() {
        summaries.push(InstanceSummary {
            id: inst.id.clone(),
            pair: inst.pair_key(),
            status: inst.status.read().await.as_str().to_string(),
            symbol: inst.symbol(),
            initial_capital: *inst.initial_capital.read().await,
            current_equity: *inst.current_equity.read().await,
            consecutive_losses: inst.safety.consecutive_losses.load(std::sync::atomic::Ordering::Relaxed),
            caution_level: inst.safety.caution_level.read().await.as_str().to_string(),
        });
    }
    summaries
}

fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("{:016x}", ts)
}
