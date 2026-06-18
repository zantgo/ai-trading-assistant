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
    let pair_cfg = config_guard.pairs.get(&pair_key);
    let default_indicators = config_guard.indicators.clone();
    let fib_config = config_guard.fibonacci.clone();
    let safety_config = config_guard.safety.clone();
    let intervals_config = config_guard.intervals.clone();

    let mid_cfg = pair_cfg
        .map(|p| p.mid_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(60, default_indicators.clone()));
    let long_cfg = pair_cfg
        .map(|p| p.long_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(300, default_indicators.clone()));
    let macro_cfg = pair_cfg
        .and_then(|p| p.macro_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(
            config_guard.macro_timeframe.duration_seconds,
            default_indicators.clone(),
        ));
    let supermacro_cfg = pair_cfg
        .and_then(|p| p.supermacro_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(
            config_guard.supermacro_timeframe.duration_seconds,
            default_indicators.clone(),
        ));
    drop(config_guard);

    let (snapshot_tx, snapshot_rx) = mpsc::channel::<NormalizedEvent>(500);
    let cancel = CancellationToken::new();

    let (mid_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (long_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (macro_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (supermacro_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);

    let mid_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(mid_cfg.candles.analysis_limit)));
    let long_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(long_cfg.candles.analysis_limit)));
    let macro_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(macro_cfg.candles.analysis_limit)));
    let supermacro_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(supermacro_cfg.candles.analysis_limit)));

    let mid_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let long_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let macro_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let supermacro_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));

    let active_pair = Arc::new(analyzer::ActivePair {
        symbol: base.clone(),
        mid: analyzer::TimeframePipeline {
            history: mid_history.clone(),
            broadcast_tx: mid_broadcast_tx.clone(),
            latest_snapshot: mid_latest.clone(),
            timeframe_secs: 60,
            timeframe_label: "Mid",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: fib_config.clone(),
        },
        long: analyzer::TimeframePipeline {
            history: long_history.clone(),
            broadcast_tx: long_broadcast_tx.clone(),
            latest_snapshot: long_latest.clone(),
            timeframe_secs: 300,
            timeframe_label: "Long",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: fib_config.clone(),
        },
        r#macro: analyzer::TimeframePipeline {
            history: macro_history.clone(),
            broadcast_tx: macro_broadcast_tx.clone(),
            latest_snapshot: macro_latest.clone(),
            timeframe_secs: macro_cfg.candles.duration_seconds,
            timeframe_label: "Macro",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: fib_config.clone(),
        },
        supermacro: analyzer::TimeframePipeline {
            history: supermacro_history.clone(),
            broadcast_tx: supermacro_broadcast_tx.clone(),
            latest_snapshot: supermacro_latest.clone(),
            timeframe_secs: supermacro_cfg.candles.duration_seconds,
            timeframe_label: "SuperMacro",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
            fibonacci: fib_config.clone(),
        },
        snapshot_tx: snapshot_tx.clone(),
        cancel: cancel.clone(),
    });

    // Spawn event router (fan out to 4 timeframes)
    let (mid_chan_tx, mid_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (long_chan_tx, long_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (macro_chan_tx, macro_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (supermacro_chan_tx, supermacro_chan_rx) = mpsc::channel::<NormalizedEvent>(200);

    let router_symbol = base.clone();
    let router_cancel = cancel.clone();
    tokio::spawn(async move {
        analyzer::run_event_router(
            snapshot_rx,
            mid_chan_tx,
            long_chan_tx,
            macro_chan_tx,
            supermacro_chan_tx,
            router_symbol,
            router_cancel,
        ).await;
    });

    // Spawn 4 pipeline tasks
    let supermacro_secs = supermacro_cfg.candles.duration_seconds;
    let macro_secs = macro_cfg.candles.duration_seconds;
    let (candle_fwd_tx, mut candle_fwd_rx) = tokio::sync::mpsc::unbounded_channel::<NormalizedCandle>();

    let pipeline_specs: Vec<(
        mpsc::Receiver<NormalizedEvent>,
        TimeframeConfig,
        Arc<RwLock<VecDeque<NormalizedCandle>>>,
        Arc<RwLock<Option<MarketSnapshot>>>,
        &str,
        u64,
        tokio::sync::broadcast::Sender<MarketSnapshot>,
        Arc<tokio::sync::Mutex<DivergenceDetector>>,
        Option<tokio::sync::mpsc::UnboundedSender<NormalizedCandle>>,
    )> = vec![
        (mid_chan_rx, mid_cfg.clone(), mid_history.clone(), mid_latest.clone(), "Mid", 60u64, mid_broadcast_tx.clone(), active_pair.mid.divergence_detector.clone(), Some(candle_fwd_tx.clone())),
        (long_chan_rx, long_cfg.clone(), long_history.clone(), long_latest.clone(), "Long", 300u64, long_broadcast_tx.clone(), active_pair.long.divergence_detector.clone(), None),
        (macro_chan_rx, macro_cfg, macro_history.clone(), macro_latest.clone(), "Macro", macro_secs, macro_broadcast_tx.clone(), active_pair.r#macro.divergence_detector.clone(), None),
        (supermacro_chan_rx, supermacro_cfg, supermacro_history.clone(), supermacro_latest.clone(), "SuperMacro", supermacro_secs, supermacro_broadcast_tx.clone(), active_pair.supermacro.divergence_detector.clone(), None),
    ];

    for (rx, tf_cfg, hist, snap, label, tf_secs, bcast, div_det, candle_fwd) in pipeline_specs {
        let a_symbol = base.clone();
        let a_pair_key = pair_key.clone();
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
                a_symbol,
                a_pair_key,
                tf_secs,
                label,
                a_cancel,
                candle_fwd,
            ).await;
        });
    }

    // Candle aggregator
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

    // WebSocket adapter
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
        mid_history.clone(),
        long_history.clone(),
        macro_history.clone(),
        supermacro_history.clone(),
        mid_latest.clone(),
        long_latest.clone(),
        macro_latest.clone(),
        supermacro_latest.clone(),
    ));

    // Build the automation context and store it on the instance
    let auto_ctx = automation::AutomationContext {
        pair_key: pair_key.clone(),
        symbol: base.clone(),
        mid_history: mid_history.clone(),
        long_history: long_history.clone(),
        macro_history: macro_history.clone(),
        supermacro_history: supermacro_history.clone(),
        mid_latest: mid_latest.clone(),
        long_latest: long_latest.clone(),
        macro_latest: macro_latest.clone(),
        supermacro_latest: supermacro_latest.clone(),
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
        let symbol_entry = format!("Hyperliquid:{}", base);
        if !config.symbols.contains(&symbol_entry) {
            config.symbols.push(symbol_entry);
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
        let symbol_entry = format!("Hyperliquid:{}", instance.pair.0);
        if let Some(pos) = config.symbols.iter().position(|s| s == &symbol_entry) {
            config.symbols.remove(pos);
            if let Ok(toml_str) = toml::to_string_pretty(&*config) {
                let _ = std::fs::write("config.toml", toml_str);
            }
        }
        config.pairs.remove(&pair_key);
        crate::config::save_pairs(&config.pairs);
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
            pair: inst.pair_display(),
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
