mod bootstrap;
pub mod pipelines;

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use config_models::{Stance, TimeframeConfig};
use crate::instance::{ConfigState, Instance, InstanceStatus};
use crate::lifecycle::LifecycleManager;
use crate::registry_context::RegistryContext;
use crate::session::{Currency, ExchangeChoice};
use core_domain::normalized::Exchange;

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstanceSummary {
    pub id: String,
    pub pair: String,
    pub status: String,
    pub symbol: String,
    pub initial_capital: f64,
    pub current_equity: f64,
    pub consecutive_losses: u32,
    pub safety_state: String,
}

/// Add a new instance to the state, starting all pipeline tasks.
pub async fn add_instance(
    state: &RegistryContext,
    pair: (String, String),
) -> Result<Arc<Instance>, String> {
    // Session-first gate: no pipelines may be spawned until the user has
    // initialized a session (exchange) via the Welcome Gate.
    if !state
        .session
        .active
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err(
            "No active session. Initialize a session (select exchange) before adding pairs."
                .to_string(),
        );
    }

    // Resolve the active exchange and its settlement/quote currency from the
    // session. The quote is forced to the session currency so that frontend and
    // backend pair keys / native symbols always agree.
    let exchange_choice = state
        .session
        .exchange
        .read()
        .await
        .clone()
        .unwrap_or(ExchangeChoice::Hyperliquid);
    let quote = state
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
        if state.workspace.get(&pair_key).await.is_some() {
            return Err(format!("Instance for pair {} already exists", pair_key));
        }
    }

    let raw_symbol = exchange_choice.raw_symbol(&base, &quote);

    // Verify the symbol is actually tradeable on the selected exchange's
    // perpetual futures market before spawning any pipelines.
    {
        let (bitget_ticker_url, hl_info_url) = {
            let cfg = state.platform.read().await;
            (cfg.bitget.ticker_url(), cfg.hyperliquid.rest_url())
        };
        let availability = match exchange_choice {
            ExchangeChoice::Bitget => {
                let pt = exchange_choice
                    .bitget_product_type(&quote)
                    .unwrap_or("USDT-FUTURES");
                network_adapters::adapters::bitget_rest::symbol_exists(&raw_symbol, pt, &bitget_ticker_url)
                    .await
            }
            ExchangeChoice::Hyperliquid => {
                network_adapters::adapters::hyperliquid_rest::symbol_exists(&base, &hl_info_url).await
            }
        };
        match availability {
            Ok(true) => {}
            Ok(false) => {
                return Err(format!(
                    "'{}' isn't available on {} ({} perpetual futures). Check the symbol (e.g. BTC, ETH) and try again.",
                    base,
                    exchange_choice.as_str(),
                    quote.as_str()
                ));
            }
            Err(e) => {
                eprintln!(
                    "⚠️  Symbol availability check failed for {} on {}: {}",
                    base,
                    exchange_choice.as_str(),
                    e
                );
                return Err(format!(
                    "Couldn't verify '{}' on {} right now (network issue). Please try again.",
                    base,
                    exchange_choice.as_str()
                ));
            }
        }
    }

    // Register symbol mapping (native <-> unified)
    let exchange_enum = match exchange_choice {
        ExchangeChoice::Bitget => Exchange::Bitget,
        ExchangeChoice::Hyperliquid => Exchange::Hyperliquid,
    };
    state
        .symbol_mapper
        .register(exchange_enum, &raw_symbol, &normalized)
        .await;

    // Build pipeline configs
    let config_guard = state.workspace.config().await;
    let pair_cfg = config_guard
        .instances
        .iter()
        .find(|i| i.symbol == pair_key)
        .cloned();
    let default_indicators = config_guard.indicators.clone();
    let fib_config = config_guard.fibonacci.clone();
    let safety_config = config_guard.safety.clone();
    let intervals_config = config_guard.intervals.clone();

    let micro_cfg = pair_cfg.as_ref().map(|p| p.micro_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(60, default_indicators.clone()));
    let fast_cfg = pair_cfg.as_ref().map(|p| p.fast_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(180, default_indicators.clone()));
    let slow_cfg = pair_cfg.as_ref().and_then(|p| p.slow_term.clone())
        .unwrap_or_else(|| {
            TimeframeConfig::new(
                config_guard.slow_timeframe.duration_seconds,
                default_indicators.clone(),
            )
        });
    let macro_cfg = pair_cfg.as_ref().and_then(|p| p.macro_term.clone())
        .unwrap_or_else(|| {
            TimeframeConfig::new(
                config_guard.macro_timeframe.duration_seconds,
                default_indicators.clone(),
            )
        });
    let rest_url = match exchange_choice {
        ExchangeChoice::Bitget => state.platform.read().await.bitget.rest_url(),
        _ => state.platform.read().await.hyperliquid.rest_url(),
    };
    let drop_fib = fib_config.clone();
    let operational_mode = pair_cfg
        .as_ref()
        .map(|p| p.operational_mode.clone())
        .unwrap_or_default();
    let weight_overrides = pair_cfg.as_ref().and_then(|p| p.weight_overrides.clone());
    let position_scaling = pair_cfg.as_ref().and_then(|p| p.position_scaling.clone());
    let liquidity_config_first = config_guard.liquidity.clone();
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
        pool: state.pool.clone(),
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
        reliability: Some(state.reliability.clone()),
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
        operational_mode: operational_mode.clone(),
        weight_overrides: weight_overrides.clone(),
        position_scaling: position_scaling.clone(),
        liquidity_config: liquidity_config_first,
    };

    let artifacts =
        pipelines::build_pipelines(&pipeline_ctx, state, warmed_states.as_ref().ok().cloned())
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

    // Register instance: insert into both the live map AND the persisted
    // workspace config so a subsequent `recharge_instance` (or any other
    // reader of `workspace.config()`) can find the InstanceEntry. The legacy
    // implementation only updated the live map, which broke save→recharge
    // cycles because the in-memory WorkspaceConfig snapshot stayed empty
    // until the daemon was restarted and the TOML was reloaded.
    state.workspace.insert(pair_key.clone(), Arc::clone(&artifacts.instance)).await;

    {
        let mut config = state.workspace.config().await;
        if let Some(slot) = config
            .instances
            .iter_mut()
            .find(|i| i.symbol == pair_key)
        {
            // Re-adding an existing pair (rare). Refresh the UUID in case the
            // disk copy is stale and accept the live configs in memory.
            slot.id = artifacts.instance.id.clone();
        } else {
            let entry = config_models::InstanceEntry {
                id: artifacts.instance.id.clone(),
                symbol: pair_key.clone(),
                quote: quote.as_str().to_string(),
                initial_capital_usd: 1000.0,
                status: config_models::InstanceStatus::Running,
                micro_term: micro_cfg.clone(),
                fast_term: fast_cfg.clone(),
                slow_term: Some(slow_cfg.clone()),
                macro_term: Some(macro_cfg.clone()),
                automation: config_models::AutomationConfig::default(),
                operational_mode: operational_mode.clone(),
                weight_overrides: weight_overrides.clone(),
                position_scaling: position_scaling.clone(),
                activation: None,
            };
            config.instances.push(entry);
        }
        if let Err(e) = config_models::save_workspace(&config) {
            eprintln!("⚠️  Failed to persist workspace after add: {}", e);
        }
        state.workspace.set_config(config).await;
    }

    sync_exchange_status_active_pairs(state).await;

    println!(
        "✅ Instance created: {} ({})",
        artifacts.instance.pair_display(),
        artifacts.instance.id
    );
    Ok(artifacts.instance)
}

/// Pause an instance (no new trades, keep open positions for TP/SL).
pub async fn pause_instance(state: &RegistryContext, instance_id: &str) -> Result<(), String> {
    let instance = state.workspace.list().await
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;
    {
        let mut lifecycle = instance.lifecycle.write().await;
        lifecycle.pause("operator", Some("Manual pause".into())).await?;
    }
    let mut config_state = instance.config_state.write().await;
    config_state.status = InstanceStatus::Paused;
    println!(
        "⏸️  Instance paused: {} ({})",
        instance.pair_display(),
        instance_id
    );
    Ok(())
}

/// Start an instance (from STOPPED or lifecycle PAUSED).
pub async fn start_instance(state: &RegistryContext, instance_id: &str) -> Result<(), String> {
    let instance = state.workspace.list().await
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;
    {
        let mut lifecycle = instance.lifecycle.write().await;
        lifecycle.start("operator", Some("Manual start".into())).await?;
    }
    let mut config_state = instance.config_state.write().await;
    config_state.status = InstanceStatus::Running;
    println!(
        "▶️  Instance started: {} ({})",
        instance.pair_display(),
        instance_id
    );
    Ok(())
}

/// Stop an instance (close all positions immediately, transition STOPPING -> STOPPED).
pub async fn stop_instance(state: &RegistryContext, instance_id: &str) -> Result<(), String> {
    let instance = state.workspace.list().await
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;
    {
        let mut lifecycle = instance.lifecycle.write().await;
        lifecycle.stop("operator", Some("Manual stop".into())).await?;
    }
    instance.cancel.cancel();

    tokio::spawn({
        let inst = Arc::clone(&instance);
        async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let mut lifecycle = inst.lifecycle.write().await;
            let _ = lifecycle.complete_stop().await;
            let mut config_state = inst.config_state.write().await;
            config_state.status = InstanceStatus::Stopped;
        }
    });

    println!(
        "🛑 Instance stopping: {} ({})",
        instance.pair_display(),
        instance_id
    );
    Ok(())
}

/// Reconcile the `ExchangeStatusTracker` pair counts with the live
/// `WorkspaceState` so the Data Infrastructure panel reflects reality.
///
/// Iterates every `Arc<Instance>` in `state.workspace` and sets each
/// exchange's `active_pairs` to the number of currently running instances
/// for that exchange. The exchange key is the BASE currency (`pair.0`)
/// in the canonical Hyperliquid / Bitget format; the daemon seeds these
/// already so we know they're tracked. Idempotent and safe to call after
/// every workspace mutation.
///
/// Public so the integration test suite can drive the helper directly
/// without spinning up the full per-instance pipeline.
pub async fn sync_exchange_status_active_pairs(state: &RegistryContext) {
    let total = state.workspace.list().await.len() as u32;
    // The current daemon only runs instances on a single exchange
    // (determined by the Welcome Gate / session choice). Every workspace
    // instance is therefore counted under that exchange's `active_pairs`.
    // The exchange label is hardcoded to "Hyperliquid" here because:
    // (1) the per-pipeline register_exchange call in pipelines.rs:577
    //     always passes "Hyperliquid" as the exchange label,
    // (2) the daemon seeds the tracker with Hyperliquid at startup,
    // (3) the user's session starts on Hyperliquid by default.
    // Bitget is not yet used by any prod path; when it is, this helper
    // should be extended to read the per-instance exchange from the
    // fields on `Instance` (currently `Instance` carries no exchange
    // field beyond pair + session).
    state
        .exchange_status
        .update_active_pairs("Hyperliquid", total)
        .await;
}

/// Delete an instance (stop first, then remove from registry).
pub async fn delete_instance(state: &RegistryContext, instance_id: &str) -> Result<(), String> {
    let (pair_key, instance) = state
        .workspace
        .list()
        .await
        .into_iter()
        .find(|i| i.id == instance_id)
        .map(|i| (i.pair_key(), Arc::clone(&i)))
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;

    {
        let lifecycle = instance.lifecycle.read().await;
        lifecycle.can_delete().map_err(|e| format!("Cannot delete: {}", e))?;
    }

    state.workspace.remove(&pair_key).await;

    {
        let mut config = state.workspace.config().await;
        config
            .instances
            .retain(|i| i.symbol != pair_key);
        if let Err(e) = config_models::save_workspace(&config) {
            eprintln!("⚠️  Failed to persist workspace after delete: {}", e);
        }
        // Publish the deletion to the in-memory snapshot so the next reader
        // (and in particular `recharge_instance` if it ever runs for a
        // deleted pair) sees the cleared list. Without this, an entry could
        // linger in WorkspaceConfig indefinitely, surviving daemon
        // restarts via disk alone but never being observable to live code.
        state.workspace.set_config(config).await;
    }

    sync_exchange_status_active_pairs(state).await;

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
pub async fn recharge_instance(state: &RegistryContext, pair_key: &str) -> Result<(), String> {
    let old_instance = state
        .workspace
        .get(pair_key)
        .await
        .ok_or_else(|| format!("Instance for pair {} not found", pair_key))?;

    println!(
        "🔄 Recharging instance: {} ({})",
        old_instance.pair_display(),
        old_instance.id
    );

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
    let config_guard = state.workspace.config().await;
    let pair_cfg = config_guard
        .instances
        .iter()
        .find(|i| i.symbol == pair_key)
        .cloned()
        .ok_or_else(|| format!("No saved config for pair {}", pair_key))?;
    let default_indicators = config_guard.indicators.clone();
    let fib_config = config_guard.fibonacci.clone();
    let safety_config = config_guard.safety.clone();
    let intervals_config = config_guard.intervals.clone();
    let exchange_choice = state
        .session
        .exchange
        .read()
        .await
        .clone()
        .unwrap_or(ExchangeChoice::Hyperliquid);
    let quote = state
        .session
        .base_currency
        .read()
        .await
        .clone()
        .unwrap_or(Currency::USDC);
    let rest_url = match exchange_choice {
        ExchangeChoice::Bitget => state.platform.read().await.bitget.rest_url(),
        _ => state.platform.read().await.hyperliquid.rest_url(),
    };
    let operational_mode = pair_cfg.operational_mode.clone();
    let weight_overrides = pair_cfg.weight_overrides.clone();
    let position_scaling = pair_cfg.position_scaling.clone();
    let liquidity_config_recharge = config_guard.liquidity.clone();
    drop(config_guard);

    let micro_cfg = pair_cfg.micro_term.clone();
    let fast_cfg = pair_cfg.fast_term.clone();
    let slow_cfg = pair_cfg
        .slow_term
        .clone()
        .unwrap_or_else(|| TimeframeConfig::new(300, default_indicators.clone()));
    let macro_cfg = pair_cfg
        .macro_term
        .clone()
        .unwrap_or_else(|| TimeframeConfig::new(900, default_indicators.clone()));

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
        pool: state.pool.clone(),
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
        reliability: Some(state.reliability.clone()),
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
        liquidity_config: liquidity_config_recharge,
    };

    let artifacts =
        pipelines::build_pipelines(&pipeline_ctx, state, warmed_states.as_ref().ok().cloned())
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

    let symbol = old_instance.active_pair.symbol.clone();
    let mut stances = std::collections::HashMap::new();
    stances.insert(symbol, Stance::Active);

    let new_instance = Arc::new(Instance {
        id: old_instance.id.clone(),
        pair: old_instance.pair.clone(),
        cancel: cancel.clone(),
        trading: {
            let old_trading = old_instance.trading.read().await;
            tokio::sync::RwLock::new(old_trading.clone())
        },
        config_state: tokio::sync::RwLock::new(ConfigState::new(
            intervals_config,
            pair_cfg.operational_mode.clone(),
        )),
        safety_config,
        safety: old_instance.safety.clone(),
        active_pair: artifacts.instance.active_pair.clone(),
        pool: old_instance.pool.clone(),
        workspace: old_instance.workspace.clone(),
        micro: artifacts.micro,
        fast: artifacts.fast,
        slow: artifacts.slow,
        r#macro: artifacts.r#macro,
        lifecycle: RwLock::new(LifecycleManager::new(None)),
        stances: RwLock::new(stances),
    });

    // Swap in state map
    state.workspace.insert(pair_key.to_string(), Arc::clone(&new_instance)).await;

    sync_exchange_status_active_pairs(state).await;

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

pub async fn list_instances(state: &RegistryContext) -> Vec<InstanceSummary> {
    let mut summaries = Vec::new();
    let instances = state.workspace.list().await;
    for inst in instances {
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
                .read()
                .await
                .values()
                .sum(),
            safety_state: inst.safety.safety_state.read().await.as_str().to_string(),
        });
    }
    summaries
}
