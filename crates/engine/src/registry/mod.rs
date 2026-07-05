mod bootstrap;
mod pipelines;

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::config::TimeframeConfig;
use crate::db;
use crate::instance::{ConfigState, Instance, InstanceStatus};
use crate::workspace::{Currency, ExchangeChoice, Workspace};
use shared::normalized::Exchange;

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
    // Session-first gate: no pipelines may be spawned until the user has
    // initialized a session (exchange + capital) via the Welcome Gate.
    if !workspace
        .session
        .active
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err(
            "No active session. Initialize a session (select exchange and capital) before adding pairs.".to_string(),
        );
    }

    let current_count = workspace.instance_count().await;
    let max_count = workspace.max_instances().await;
    if current_count >= max_count {
        return Err(format!(
            "Maximum instance count reached ({}/{}). Remove an instance first.",
            current_count, max_count
        ));
    }

    // Resolve the active exchange and its settlement/quote currency from the
    // session. The quote is forced to the session currency so that frontend and
    // backend pair keys / native symbols always agree.
    let exchange_choice = workspace
        .session
        .exchange
        .read()
        .await
        .clone()
        .unwrap_or(ExchangeChoice::Hyperliquid);
    let quote = workspace
        .session
        .base_currency
        .read()
        .await
        .clone()
        .unwrap_or(Currency::USDC);

    if !exchange_choice.supports_currency(&quote) {
        return Err(format!(
            "{} does not support {} settlement.",
            exchange_choice.as_str(),
            quote.as_str()
        ));
    }

    let base = pair.0.clone();
    let pair_key = exchange_choice.internal_symbol(&base, &quote);
    let normalized = pair_key.clone();

    // Check for duplicate
    {
        let instances = workspace.instances.read().await;
        if instances.contains_key(&pair_key) {
            return Err(format!("Instance for pair {} already exists", pair_key));
        }
    }

    // Register symbol mapping (native <-> unified)
    let exchange_enum = match exchange_choice {
        ExchangeChoice::Bitget => Exchange::Bitget,
        ExchangeChoice::Hyperliquid => Exchange::Hyperliquid,
    };
    let raw_symbol = exchange_choice.raw_symbol(&base, &quote);
    workspace
        .symbol_mapper
        .register(exchange_enum, &raw_symbol, &normalized)
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
    let fast_cfg = pair_cfg
        .map(|p| p.fast_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(180, default_indicators.clone()));
    let slow_cfg = pair_cfg
        .and_then(|p| p.slow_term.clone())
        .unwrap_or_else(|| {
            TimeframeConfig::new(
                config_guard.slow_timeframe.duration_seconds,
                default_indicators.clone(),
            )
        });
    let macro_cfg = pair_cfg
        .and_then(|p| p.macro_term.clone())
        .unwrap_or_else(|| {
            TimeframeConfig::new(
                config_guard.macro_timeframe.duration_seconds,
                default_indicators.clone(),
            )
        });
    let rest_url = match exchange_choice {
        ExchangeChoice::Bitget => config_guard.bitget.rest_url(),
        _ => config_guard.hyperliquid.rest_url(),
    };
    let drop_fib = fib_config.clone();
    let operational_mode = pair_cfg
        .map(|p| p.operational_mode.clone())
        .unwrap_or_default();
    let weight_overrides = pair_cfg.and_then(|p| p.weight_overrides.clone());
    let position_scaling = pair_cfg.and_then(|p| p.position_scaling.clone());
    drop(config_guard);

    let cancel = CancellationToken::new();

    let micro_secs = micro_cfg.candles.duration_seconds;
    let fast_secs = fast_cfg.candles.duration_seconds;
    let slow_secs = slow_cfg.candles.duration_seconds;
    let macro_secs = macro_cfg.candles.duration_seconds;

    let micro_limit = micro_cfg.candles.analysis_limit as u64;
    let fast_limit = fast_cfg.candles.analysis_limit as u64;
    let slow_limit = slow_cfg.candles.analysis_limit as u64;
    let macro_limit = macro_cfg.candles.analysis_limit as u64;

    // ── Historical Bootstrap FIRST ──
    let bootstrap_input = bootstrap::BootstrapInput {
        base: base.clone(),
        internal_symbol: normalized.clone(),
        quote: quote.clone(),
        rest_url,
        exchange_choice: exchange_choice.clone(),
        pool: workspace.pool.clone(),
        micro_cfg: micro_cfg.clone(),
        fast_cfg: fast_cfg.clone(),
        slow_cfg: slow_cfg.clone(),
        macro_cfg: macro_cfg.clone(),
        fib_config: drop_fib.clone(),
        micro_secs,
        fast_secs,
        slow_secs,
        macro_secs,
        micro_limit,
        fast_limit,
        slow_limit,
        macro_limit,
    };

    let warmed_states = bootstrap::fetch_and_warm_bootstrap(&bootstrap_input).await;

    // ── Build pipelines (creates channels, buffers, ActivePair) ──
    let pipeline_ctx = pipelines::PipelineContext {
        base: base.clone(),
        internal_symbol: normalized.clone(),
        quote: quote.clone(),
        pair_key: pair_key.clone(),
        exchange_choice: exchange_choice.clone(),
        micro_cfg: micro_cfg.clone(),
        fast_cfg: fast_cfg.clone(),
        slow_cfg: slow_cfg.clone(),
        macro_cfg: macro_cfg.clone(),
        fib_config: drop_fib,
        safety_config,
        intervals_config: intervals_config.clone(),
        cancel: cancel.clone(),
        operational_mode,
        weight_overrides,
        position_scaling,
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
            &artifacts.fast.history,
            &artifacts.slow.history,
            &artifacts.r#macro.history,
            &artifacts.micro.latest,
            &artifacts.fast.latest,
            &artifacts.slow.latest,
            &artifacts.r#macro.latest,
            &artifacts.micro.snapshot_history,
            &artifacts.fast.snapshot_history,
            &artifacts.slow.snapshot_history,
            &artifacts.r#macro.snapshot_history,
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

    if current_count == 0 {
        if let Some(capital) = *workspace.session.initial_capital.read().await {
            let _ = crate::db::paper_set_advanced_config(
                &workspace.pool,
                &pair_key,
                capital,
                10.0,
                false,
                2.0,
                20,
                15,
                10,
                false,
            )
            .await;
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

/// Recharge an existing instance with new timeframe/indicator configurations.
/// Cancels old tasks, flushes buffers, re-bootstraps, and re-spawns pipelines
/// while preserving active paper positions, safety state, and token tracking.
pub async fn recharge_instance(
    workspace: &Arc<Workspace>,
    pair_key: &str,
    llm_client: Arc<crate::llm::LlmClient>,
) -> Result<(), String> {
    let old_instance = {
        let instances = workspace.instances.read().await;
        instances
            .get(pair_key)
            .cloned()
            .ok_or_else(|| format!("Instance for pair {} not found", pair_key))?
    };

    println!("🔄 Recharging instance: {} ({})", old_instance.pair_display(), old_instance.id);

    // Cancel all active tasks for old instance
    old_instance.cancel.cancel();

    // Drain old buffers
    {
        old_instance.micro.history.write().await.clear();
        old_instance.fast.history.write().await.clear();
        old_instance.slow.history.write().await.clear();
        old_instance.r#macro.history.write().await.clear();
        *old_instance.fast.latest.write().await = None;
        *old_instance.slow.latest.write().await = None;
        *old_instance.r#macro.latest.write().await = None;
        old_instance.fast.snapshot_history.write().await.clear();
        old_instance.slow.snapshot_history.write().await.clear();
        old_instance.r#macro.snapshot_history.write().await.clear();
    }

    // Build fresh pipeline configs from saved TOML config
    let (base, _quote) = old_instance.pair.clone();
    let config_guard = workspace.config.read().await;
    let pair_cfg = config_guard
        .instances
        .get(pair_key)
        .cloned()
        .ok_or_else(|| format!("No saved config for pair {}", pair_key))?;
    let default_indicators = config_guard.indicators.clone();
    let fib_config = config_guard.fibonacci.clone();
    let safety_config = config_guard.safety.clone();
    let intervals_config = config_guard.intervals.clone();
    let exchange_choice = workspace.session.exchange.read().await.clone().unwrap_or(ExchangeChoice::Hyperliquid);
    let quote = workspace
        .session
        .base_currency
        .read()
        .await
        .clone()
        .unwrap_or(Currency::USDC);
    let rest_url = match exchange_choice {
        ExchangeChoice::Bitget => config_guard.bitget.rest_url(),
        _ => config_guard.hyperliquid.rest_url(),
    };
    let operational_mode = pair_cfg.operational_mode.clone();
    let weight_overrides = pair_cfg.weight_overrides.clone();
    let position_scaling = pair_cfg.position_scaling.clone();
    drop(config_guard);

    let micro_cfg = pair_cfg.micro_term.clone();
    let fast_cfg = pair_cfg.fast_term.clone();
    let slow_cfg = pair_cfg.slow_term.clone().unwrap_or_else(|| {
        TimeframeConfig::new(300, default_indicators.clone())
    });
    let macro_cfg = pair_cfg.macro_term.clone().unwrap_or_else(|| {
        TimeframeConfig::new(900, default_indicators.clone())
    });

    let micro_secs = micro_cfg.candles.duration_seconds;
    let fast_secs = fast_cfg.candles.duration_seconds;
    let slow_secs = slow_cfg.candles.duration_seconds;
    let macro_secs = macro_cfg.candles.duration_seconds;

    let micro_limit = micro_cfg.candles.analysis_limit as u64;
    let fast_limit = fast_cfg.candles.analysis_limit as u64;
    let slow_limit = slow_cfg.candles.analysis_limit as u64;
    let macro_limit = macro_cfg.candles.analysis_limit as u64;

    // Fresh historical bootstrap
    let bootstrap_input = bootstrap::BootstrapInput {
        base: base.clone(),
        internal_symbol: pair_key.to_string(),
        quote: quote.clone(),
        rest_url,
        exchange_choice: exchange_choice.clone(),
        pool: workspace.pool.clone(),
        micro_cfg: micro_cfg.clone(),
        fast_cfg: fast_cfg.clone(),
        slow_cfg: slow_cfg.clone(),
        macro_cfg: macro_cfg.clone(),
        fib_config: fib_config.clone(),
        micro_secs,
        fast_secs,
        slow_secs,
        macro_secs,
        micro_limit,
        fast_limit,
        slow_limit,
        macro_limit,
    };

    let warmed_states = bootstrap::fetch_and_warm_bootstrap(&bootstrap_input).await;

    // Build fresh pipelines
    let cancel = CancellationToken::new();
    let pipeline_ctx = pipelines::PipelineContext {
        base: base.clone(),
        internal_symbol: pair_key.to_string(),
        quote: quote.clone(),
        pair_key: pair_key.to_string(),
        exchange_choice: exchange_choice.clone(),
        micro_cfg: micro_cfg.clone(),
        fast_cfg: fast_cfg.clone(),
        slow_cfg: slow_cfg.clone(),
        macro_cfg: macro_cfg.clone(),
        fib_config,
        safety_config: safety_config.clone(),
        intervals_config: intervals_config.clone(),
        cancel: cancel.clone(),
        operational_mode,
        weight_overrides,
        position_scaling,
    };

    let artifacts = pipelines::build_pipelines(
        &pipeline_ctx,
        workspace,
        llm_client,
        warmed_states.as_ref().ok().cloned(),
    )
    .await;

    // Populate buffers from warmed states
    if let Ok((ref wm, ref ws, ref wmed, ref wl)) = warmed_states {
        bootstrap::populate_buffers(
            &Some(wm.clone()),
            &Some(ws.clone()),
            &Some(wmed.clone()),
            &Some(wl.clone()),
            &artifacts.micro.history,
            &artifacts.fast.history,
            &artifacts.slow.history,
            &artifacts.r#macro.history,
            &artifacts.micro.latest,
            &artifacts.fast.latest,
            &artifacts.slow.latest,
            &artifacts.r#macro.latest,
            &artifacts.micro.snapshot_history,
            &artifacts.fast.snapshot_history,
            &artifacts.slow.snapshot_history,
            &artifacts.r#macro.snapshot_history,
        )
        .await;
    }

    // Construct new instance shell reusing preserved state from old instance
    let new_instance = Arc::new(Instance {
        id: old_instance.id.clone(),
        pair: old_instance.pair.clone(),
        cancel: cancel.clone(),
        trading: {
            let old_trading = old_instance.trading.read().await;
            tokio::sync::RwLock::new(old_trading.clone())
        },
        config_state: tokio::sync::RwLock::new(ConfigState::new(intervals_config, pair_cfg.operational_mode.clone())),
        safety_config,
        api_key: {
            let old_key = old_instance.api_key.read().await;
            tokio::sync::RwLock::new(old_key.clone())
        },
        api_key_valid: {
            let old_valid = old_instance.api_key_valid.load(std::sync::atomic::Ordering::Relaxed);
            std::sync::atomic::AtomicBool::new(old_valid)
        },
        api_failover: old_instance.api_failover.clone(),
        token_tracker: old_instance.token_tracker.clone(),
        safety: old_instance.safety.clone(),
        active_pair: artifacts.instance.active_pair.clone(),
        automation_ctx: artifacts.instance.automation_ctx.clone(),
        pool: old_instance.pool.clone(),
        config: old_instance.config.clone(),
        micro: artifacts.micro,
        fast: artifacts.fast,
        slow: artifacts.slow,
        r#macro: artifacts.r#macro,
    });

    // Swap in workspace map
    workspace
        .instances
        .write()
        .await
        .insert(pair_key.to_string(), Arc::clone(&new_instance));

    println!(
        "⚡ Instance recharged: {} ({}) — micro={}s fast={}s slow={}s macro={}s",
        new_instance.pair_display(),
        new_instance.id,
        micro_secs,
        fast_secs,
        slow_secs,
        macro_secs,
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
