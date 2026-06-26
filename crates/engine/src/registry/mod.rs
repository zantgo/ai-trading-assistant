mod bootstrap;
mod pipelines;

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::config::TimeframeConfig;
use crate::db;
use crate::instance::{Instance, InstanceStatus};
use crate::workspace::Workspace;

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
    llm_client: Arc<crate::llm::LlmClient>,
) -> Result<Arc<Instance>, String> {
    let current_count = workspace.instance_count().await;
    let max_count = workspace.max_instances().await;
    if current_count >= max_count {
        return Err(format!(
            "Maximum instance count reached ({}/{}). Remove an instance first.",
            current_count, max_count
        ));
    }

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
    workspace
        .symbol_mapper
        .register(exchange_enum, &base, &normalized)
        .await;

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
        .unwrap_or_else(|| {
            TimeframeConfig::new(
                config_guard.medium_timeframe.duration_seconds,
                default_indicators.clone(),
            )
        });
    let large_cfg = pair_cfg
        .and_then(|p| p.large_term.clone())
        .unwrap_or_else(|| {
            TimeframeConfig::new(
                config_guard.large_timeframe.duration_seconds,
                default_indicators.clone(),
            )
        });
    let rest_url = config_guard.hyperliquid.rest_url();
    let drop_fib = fib_config.clone();
    drop(config_guard);

    let cancel = CancellationToken::new();

    let micro_secs = micro_cfg.candles.duration_seconds;
    let short_secs = short_cfg.candles.duration_seconds;
    let medium_secs = medium_cfg.candles.duration_seconds;
    let large_secs = large_cfg.candles.duration_seconds;

    let micro_limit = micro_cfg.candles.analysis_limit as u64;
    let short_limit = short_cfg.candles.analysis_limit as u64;
    let medium_limit = medium_cfg.candles.analysis_limit as u64;
    let large_limit = large_cfg.candles.analysis_limit as u64;

    // ── Historical Bootstrap FIRST ──
    let bootstrap_input = bootstrap::BootstrapInput {
        base: base.clone(),
        rest_url,
        micro_cfg: micro_cfg.clone(),
        short_cfg: short_cfg.clone(),
        medium_cfg: medium_cfg.clone(),
        large_cfg: large_cfg.clone(),
        fib_config: drop_fib.clone(),
        micro_secs,
        short_secs,
        medium_secs,
        large_secs,
        micro_limit,
        short_limit,
        medium_limit,
        large_limit,
    };

    let warmed_states = bootstrap::fetch_and_warm_bootstrap(&bootstrap_input).await;

    // ── Build pipelines (creates channels, buffers, ActivePair) ──
    let pipeline_ctx = pipelines::PipelineContext {
        base: base.clone(),
        pair_key: pair_key.clone(),
        micro_cfg: micro_cfg.clone(),
        short_cfg: short_cfg.clone(),
        medium_cfg: medium_cfg.clone(),
        large_cfg: large_cfg.clone(),
        fib_config: drop_fib,
        safety_config,
        intervals_config: intervals_config.clone(),
        cancel: cancel.clone(),
    };

    let artifacts = pipelines::build_pipelines(
        &pipeline_ctx,
        workspace,
        llm_client,
        warmed_states.as_ref().ok().cloned(),
    )
    .await;

    // Populates buffers directly if warmed states are present
    if let Ok((ref wm, ref ws, ref wmed, ref wl)) = warmed_states {
        bootstrap::populate_buffers(
            &Some(wm.clone()),
            &Some(ws.clone()),
            &Some(wmed.clone()),
            &Some(wl.clone()),
            &artifacts.micro.history,
            &artifacts.short.history,
            &artifacts.medium.history,
            &artifacts.large.history,
            &artifacts.micro.latest,
            &artifacts.short.latest,
            &artifacts.medium.latest,
            &artifacts.large.latest,
            &artifacts.micro.snapshot_history,
            &artifacts.short.snapshot_history,
            &artifacts.medium.snapshot_history,
            &artifacts.large.snapshot_history,
        )
        .await;
    }

    // Persist symbol to config
    {
        let mut config = workspace.config.write().await;
        if !config.symbols.contains(&base) {
            config.symbols.push(base.clone());
            if let Ok(toml_str) = toml::to_string_pretty(&*config) {
                let _ = tokio::fs::write("config.toml", toml_str).await;
            }
        }
    }

    // Register instance
    workspace
        .instances
        .write()
        .await
        .insert(pair_key, Arc::clone(&artifacts.instance));

    println!(
        "✅ Instance created: {} ({})",
        artifacts.instance.pair_display(),
        artifacts.instance.id
    );
    Ok(artifacts.instance)
}

/// Pause an instance (no new trades, keep open positions for TP/SL).
pub async fn pause_instance(workspace: &Arc<Workspace>, instance_id: &str) -> Result<(), String> {
    let instances = workspace.instances.read().await;
    let instance = instances
        .values()
        .find(|i| i.id == instance_id)
        .cloned()
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;
    drop(instances);

    let mut config_state = instance.config_state.write().await;
    if config_state.status == InstanceStatus::Stopped {
        return Err("Cannot pause a stopped instance".to_string());
    }
    config_state.status = InstanceStatus::Paused;
    println!(
        "⏸️  Instance paused: {} ({})",
        instance.pair_display(),
        instance_id
    );
    Ok(())
}

/// Stop an instance (close all positions immediately).
pub async fn stop_instance(workspace: &Arc<Workspace>, instance_id: &str) -> Result<(), String> {
    let instances = workspace.instances.read().await;
    let instance = instances
        .values()
        .find(|i| i.id == instance_id)
        .cloned()
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;
    drop(instances);

    let symbol = instance.symbol();
    let _ = crate::db::paper_get_active_position(&instance.pool, &symbol).await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let _ = workspace
        .telemetry_tx
        .send(db::TelemetryMsg::PaperClosePosition {
            symbol: symbol.clone(),
            exit_price: 0.0,
            exit_timestamp: now,
            trigger: "STOP".to_string(),
        })
        .await;

    instance.config_state.write().await.status = InstanceStatus::Stopped;
    instance.cancel.cancel();
    println!(
        "🛑 Instance stopped: {} ({})",
        instance.pair_display(),
        instance_id
    );
    Ok(())
}

/// Delete an instance (stop first, then remove from registry).
pub async fn delete_instance(workspace: &Arc<Workspace>, instance_id: &str) -> Result<(), String> {
    let (pair_key, instance) = {
        let instances = workspace.instances.read().await;
        let (pk, inst) = instances
            .iter()
            .find(|(_, i)| i.id == instance_id)
            .map(|(k, i)| (k.clone(), Arc::clone(i)))
            .ok_or_else(|| format!("Instance {} not found", instance_id))?;
        (pk, inst)
    };

    {
        let is_stopped = instance.config_state.read().await.status == InstanceStatus::Stopped;
        if !is_stopped {
            stop_instance(workspace, instance_id).await?;
        }
    }

    workspace.instances.write().await.remove(&pair_key);

    {
        let mut config = workspace.config.write().await;
        let base_symbol = &instance.pair.0;
        if let Some(pos) = config.symbols.iter().position(|s| s == base_symbol) {
            config.symbols.remove(pos);
            if let Ok(toml_str) = toml::to_string_pretty(&*config) {
                let _ = tokio::fs::write("config.toml", toml_str).await;
            }
        }
        config.instances.remove(&pair_key);
        crate::config::save_instances(&config.instances).await;
    }

    println!(
        "🗑️  Instance deleted: {} ({})",
        instance.pair_display(),
        instance_id
    );
    Ok(())
}

pub async fn list_instances(workspace: &Arc<Workspace>) -> Vec<InstanceSummary> {
    let instances = workspace.instances.read().await;
    let mut summaries = Vec::new();
    for (_, inst) in instances.iter() {
        summaries.push(InstanceSummary {
            id: inst.id.clone(),
            pair: inst.pair_key(),
            status: inst.config_state.read().await.status.as_str().to_string(),
            symbol: inst.symbol(),
            initial_capital: inst.trading.read().await.initial_capital,
            current_equity: inst.trading.read().await.current_equity,
            consecutive_losses: inst
                .safety
                .consecutive_losses
                .load(std::sync::atomic::Ordering::Relaxed),
            caution_level: inst.safety.caution_level.read().await.as_str().to_string(),
        });
    }
    summaries
}
