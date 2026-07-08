use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{RwLock, mpsc};
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use rust_decimal::prelude::ToPrimitive;

use crate::config::{
    AppConfig, AutomationConfig, IntervalsConfig, OperationalMode, TriggerMode,
    PositionScalingConfig,
};
use crate::db;
use crate::event_detector;
use crate::llm::LlmClient;
use crate::paper_trading;
use crate::profile_evaluation::{SnapshotValues, classify_market_regime, regime_allows_entry};
use crate::safety::SafetyManager;

fn indicator_to_snapshot(snap: &crate::server::IndicatorSnapshot) -> SnapshotValues {
    crate::profile_evaluation::snapshot_values_from_flat(snap)
}
use shared::models::MarketSnapshot;
use shared::normalized::NormalizedCandle;
use shared::TriggerType;
use crate::portfolio_risk::PortfolioRiskState;

#[derive(Debug, Clone)]
pub struct TriggerMessage {
    pub reason: String,
    pub trigger_type_detail: String,
}

pub struct AutomationContext {
    pub pair_key: String,
    pub symbol: String,
    pub micro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub fast_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub slow_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub macro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub micro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub fast_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub slow_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub macro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub micro_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub fast_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub slow_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub macro_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub config: Arc<RwLock<AppConfig>>,
    pub pool: SqlitePool,
    pub llm_client: Arc<LlmClient>,
    pub telemetry_tx: mpsc::Sender<db::TelemetryMsg>,
    pub cancel: CancellationToken,
    pub api_key_configured: Arc<AtomicBool>,
    pub portfolio_risk: Arc<PortfolioRiskState>,
    pub pair_close_histories: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    pub safety: Arc<SafetyManager>,
    pub intervals: IntervalsConfig,
    pub next_interval_override: Arc<RwLock<Option<u64>>>,
    pub operational_mode: OperationalMode,
    pub weight_overrides: Arc<RwLock<Option<HashMap<String, i32>>>>,
    pub position_scaling: Arc<RwLock<Option<PositionScalingConfig>>>,
    pub candle_counters: Arc<RwLock<HashMap<String, u32>>>,
    pub prev_indicators: Arc<RwLock<Option<crate::server::IndicatorSnapshot>>>,
    pub trigger_tx: mpsc::Sender<TriggerMessage>,
    pub trigger_rx: Arc<tokio::sync::Mutex<Option<mpsc::Receiver<TriggerMessage>>>>,
}

#[derive(Debug, Clone)]
pub struct AutomationState {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub last_run: Option<std::time::Instant>,
}

impl AutomationState {
    pub fn from_config(cfg: &AutomationConfig) -> Self {
        Self {
            enabled: cfg.enabled,
            interval_seconds: cfg.interval_seconds,
            last_run: None,
        }
    }

    pub fn next_remaining_secs(&self) -> u64 {
        match self.last_run {
            Some(last) => {
                let elapsed = last.elapsed().as_secs();
                self.interval_seconds.saturating_sub(elapsed)
            }
            None if self.enabled => self.interval_seconds,
            None => 0,
        }
    }
}

pub async fn run_pair_automation_loop(ctx: AutomationContext) {
    println!("🤖 Automation Task: Started scheduler for {} ({})...", ctx.symbol, ctx.pair_key);

    let mut state = {
        let cfg = ctx.config.read().await;
        let pair_cfg = cfg.instances.get(&ctx.pair_key).map(|p| &p.automation);
        match pair_cfg {
            Some(auto_cfg) => AutomationState::from_config(auto_cfg),
            None => AutomationState::from_config(&AutomationConfig::default()),
        }
    };

    let (trigger_listener_tx, mut trigger_listener_rx) = mpsc::channel::<TriggerMessage>(32);

    let trigger_ctx = TriggerListenerCtx {
        automation_ctx: ctx_to_clone(&ctx),
        cancel: ctx.cancel.clone(),
    };
    tokio::spawn(async move {
        trigger_listener_loop(trigger_ctx, &mut trigger_listener_rx).await;
    });

    loop {
        tokio::select! {
            biased;
            _ = ctx.cancel.cancelled() => {
                println!("🛑 Automation Task: {} scheduler cancelled, shutting down.", ctx.pair_key);
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {}
        }

        let fresh_config = ctx.config.read().await.clone();
        let pair_cfg = fresh_config.instances.get(&ctx.pair_key);
        let auto_cfg = pair_cfg
            .map(|p| &p.automation)
            .cloned()
            .unwrap_or_default();
        let op_mode = pair_cfg
            .map(|p| p.operational_mode.clone())
            .unwrap_or(OperationalMode::HybridAiCopilot);
        let trigger_cfg = pair_cfg
            .map(|p| p.ai_trigger.trigger.clone())
            .unwrap_or(TriggerMode::Interval { seconds: 900 });

        if auto_cfg.enabled != state.enabled {
            state.enabled = auto_cfg.enabled;
            if auto_cfg.enabled {
                state.last_run = None;
                println!("🤖 Automation: {} activated.", ctx.pair_key);
            } else {
                println!("🤖 Automation: {} deactivated.", ctx.pair_key);
            }
        }
        if auto_cfg.interval_seconds != state.interval_seconds {
            state.interval_seconds = auto_cfg.interval_seconds;
            println!(
                "🤖 Automation: {} interval changed to {}s.",
                ctx.pair_key, auto_cfg.interval_seconds
            );
            state.last_run = None;
        }

        if !state.enabled {
            continue;
        }

        match op_mode {
            OperationalMode::ManualOnly => {
                update_heuristics_state(&ctx).await;
                continue;
            }
            OperationalMode::DeterministicHeuristics => {
                update_heuristics_state(&ctx).await;
                continue;
            }
            OperationalMode::HybridAiCopilot => {}
        }

        // ─── Safety Check ────────────────────────────────────────────
        if let Err(reason) = ctx.safety.check_allow_trade().await {
            println!("🛑 Automation: {} safety block: {}", ctx.pair_key, reason);
            continue;
        }
        if let Err(reason) = ctx.safety.check_capital_drawdown().await {
            eprintln!("🛑 Automation: {} drawdown stop triggered: {}", ctx.pair_key, reason);
            if db::paper_get_active_position(&ctx.pool, &ctx.symbol).await.is_some() {
                let price = ctx.micro_latest.read().await.as_ref()
                    .and_then(|s| s.mid_price.to_f64())
                    .unwrap_or(0.0);
                if price > 0.0 {
                    let _ = paper_trading::close_paper_position(
                        &ctx.pool, &ctx.telemetry_tx,
                        &ctx.symbol, price, "DRAWDOWN_STOP",
                    ).await;
                    eprintln!("🛑 Automation: {} closed all positions at ${:.2} due to drawdown stop", ctx.pair_key, price);
                }
            }
            continue;
        }

        if !ctx.api_key_configured.load(std::sync::atomic::Ordering::Relaxed) {
            println!("🤖 Automation: No API Key configured for {}. Skipping cycle...", ctx.pair_key);
            continue;
        }

        // ─── Dynamic Interval Override ───────────────────────────────
        if let Some(new_secs) = *ctx.next_interval_override.read().await {
            if new_secs != state.interval_seconds {
                println!("🔄 Automation: {} interval changed {}s → {}s (AI selection)", ctx.pair_key, state.interval_seconds, new_secs);
                state.interval_seconds = new_secs;
                state.last_run = None;
            }
            *ctx.next_interval_override.write().await = None;
        }

        let should_trigger = match &trigger_cfg {
            TriggerMode::Interval { seconds: _ } => {
                state.next_remaining_secs() == 0
            }
            TriggerMode::CandleClose { timeframe, count } => {
                let counters = ctx.candle_counters.read().await;
                let current = counters.get(timeframe).copied().unwrap_or(0);
                current >= *count
            }
            TriggerMode::EventDriven { events } => {
                if events.is_empty() {
                    false
                } else {
                    let curr = build_indicator_snapshot_from_latest(&ctx.micro_latest).await;
                    let prev_guard = ctx.prev_indicators.read().await;
                    let triggered = event_detector::evaluate_trigger_events(
                        prev_guard.as_ref(),
                        &curr,
                        events,
                    );
                    if !triggered.is_empty() {
                        println!(
                            "🎯 Event Trigger: {} fired events: {:?}",
                            ctx.pair_key, triggered
                        );
                        true
                    } else {
                        false
                    }
                }
            }
        };

        if !should_trigger {
            update_heuristics_state(&ctx).await;
            continue;
        }

        let history_guard = ctx.micro_history.read().await;
        let candle_count = history_guard.len();
        if candle_count < 10 {
            drop(history_guard);
            continue;
        }
        let prices: Vec<f64> = history_guard.iter().map(|c| {
            c.close.to_f64().unwrap_or(0.0)
        }).collect();
        drop(history_guard);

        let last_close_rate = prices.last().copied().unwrap_or(0.0);

        {
            let mut hist_map = ctx.pair_close_histories.write().await;
            let entry = hist_map.entry(ctx.symbol.clone()).or_default();
            *entry = prices.clone();
        }

        if ctx.llm_client.api_key.read().await.is_empty() {
            continue;
        }

        let trigger_detail = format_trigger_detail(&trigger_cfg);
        let master_id = db::insert_master_placeholder(
            &ctx.pool,
            "None",
            "",
            &format!("{}", last_close_rate),
            &ctx.symbol,
            TriggerType::Automated,
        )
        .await;

        db::insert_automated_performance_baseline(
            &ctx.pool,
            master_id,
            &ctx.symbol,
            &format!("{}", last_close_rate),
        )
        .await;

        // Log trigger detail
        let _ = sqlx::query(
            "UPDATE master_assistant_records SET trigger_type_detail = ?2, operational_mode = ?3 WHERE id = ?1",
        )
        .bind(master_id)
        .bind(&trigger_detail)
        .bind(ctx.operational_mode.as_str())
        .execute(&ctx.pool)
        .await;

        state.last_run = Some(std::time::Instant::now());

        // Reset candle counters on any trigger dispatch
        if let TriggerMode::CandleClose { timeframe, .. } = &trigger_cfg {
            ctx.candle_counters.write().await.insert(timeframe.clone(), 0);
        }

        let trigger_msg = TriggerMessage {
            reason: trigger_detail.clone(),
            trigger_type_detail: trigger_detail,
        };
        let _ = trigger_listener_tx.send(trigger_msg).await;
    }

    println!("🛑 Automation Task: {} scheduler terminated.", ctx.pair_key);
}

struct TriggerListenerCtx {
    automation_ctx: AutomationContextLight,
    cancel: CancellationToken,
}

#[derive(Clone)]
struct AutomationContextLight {
    pair_key: String,
    symbol: String,
    micro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    config: Arc<RwLock<AppConfig>>,
    pool: SqlitePool,
    llm_client: Arc<LlmClient>,
    telemetry_tx: mpsc::Sender<db::TelemetryMsg>,
    portfolio_risk: Arc<PortfolioRiskState>,
    pair_close_histories: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    intervals: IntervalsConfig,
    next_interval_override: Arc<RwLock<Option<u64>>>,
    operational_mode: OperationalMode,
    weight_overrides: Arc<RwLock<Option<HashMap<String, i32>>>>,
    safety: Arc<SafetyManager>,
}

fn ctx_to_clone(ctx: &AutomationContext) -> AutomationContextLight {
    AutomationContextLight {
        pair_key: ctx.pair_key.clone(),
        symbol: ctx.symbol.clone(),
        micro_history: ctx.micro_history.clone(),
        config: ctx.config.clone(),
        pool: ctx.pool.clone(),
        llm_client: ctx.llm_client.clone(),
        telemetry_tx: ctx.telemetry_tx.clone(),
        portfolio_risk: ctx.portfolio_risk.clone(),
        pair_close_histories: ctx.pair_close_histories.clone(),
        intervals: ctx.intervals.clone(),
        next_interval_override: ctx.next_interval_override.clone(),
        operational_mode: ctx.operational_mode.clone(),
        weight_overrides: ctx.weight_overrides.clone(),
        safety: ctx.safety.clone(),
    }
}

async fn trigger_listener_loop(
    ctx: TriggerListenerCtx,
    rx: &mut mpsc::Receiver<TriggerMessage>,
) {
    while let Some(msg) = rx.recv().await {
        if ctx.cancel.is_cancelled() {
            break;
        }
        if ctx.automation_ctx.operational_mode != OperationalMode::HybridAiCopilot {
            continue;
        }
        println!(
            "🎯 Trigger Listener: {} executing AI cycle (reason: {})",
            ctx.automation_ctx.pair_key, msg.reason
        );

        let history_guard = ctx.automation_ctx.micro_history.read().await;
        let prices: Vec<f64> = history_guard.iter().map(|c| c.close.to_f64().unwrap_or(0.0)).collect();
        drop(history_guard);

        let last_close = prices.last().copied().unwrap_or(0.0);

        {
            let mut hist_map = ctx.automation_ctx.pair_close_histories.write().await;
            let entry = hist_map.entry(ctx.automation_ctx.symbol.clone()).or_default();
            *entry = prices.clone();
        }

        let master_id = db::insert_master_placeholder(
            &ctx.automation_ctx.pool,
            "None",
            "",
            &format!("{}", last_close),
            &ctx.automation_ctx.symbol,
            TriggerType::Automated,
        )
        .await;

        db::insert_automated_performance_baseline(
            &ctx.automation_ctx.pool,
            master_id,
            &ctx.automation_ctx.symbol,
            &format!("{}", last_close),
        )
        .await;

        let _ = sqlx::query(
            "UPDATE master_assistant_records SET trigger_type_detail = ?2, operational_mode = ?3 WHERE id = ?1",
        )
        .bind(master_id)
        .bind(&msg.trigger_type_detail)
        .bind(ctx.automation_ctx.operational_mode.as_str())
        .execute(&ctx.automation_ctx.pool)
        .await;

        if let Err(e) = execute_automation_cycle_light(&ctx.automation_ctx, master_id, &prices).await {
            eprintln!(
                "Automation cycle error for {}: {}",
                ctx.automation_ctx.pair_key, e
            );
        }
    }
}

/// Update the heuristics state (prev indicators, candle counters) on each tick.
async fn update_heuristics_state(ctx: &AutomationContext) {
    let curr = build_indicator_snapshot_from_latest(&ctx.micro_latest).await;
    *ctx.prev_indicators.write().await = Some(curr);
}

async fn build_indicator_snapshot_from_latest(
    latest: &Arc<RwLock<Option<MarketSnapshot>>>,
) -> crate::server::IndicatorSnapshot {
    let guard = latest.read().await;
    match guard.as_ref() {
        Some(s) => {
            let current_price = s.mid_price.to_string().parse::<f64>().ok();
            let mut snap =
                crate::server::IndicatorSnapshot::new(s.indicators.clone(), current_price);
            snap.volume = s.volume.and_then(|d| d.to_string().parse::<f64>().ok());
            snap.average_volume = s.average_volume.and_then(|d| d.to_string().parse::<f64>().ok());
            snap
        }
        None => crate::server::IndicatorSnapshot::default(),
    }
}

fn format_trigger_detail(cfg: &TriggerMode) -> String {
    match cfg {
        TriggerMode::Interval { seconds } => format!("interval:{}s", seconds),
        TriggerMode::CandleClose { timeframe, count } => {
            format!("candle:{}:{}", timeframe, count)
        }
        TriggerMode::EventDriven { events } => {
            format!("event:{}", events.join(","))
        }
    }
}

async fn execute_automation_cycle_light(
    ctx: &AutomationContextLight,
    master_id: i64,
    prices: &[f64],
) -> Result<(), String> {
    let llm = ctx.llm_client.clone();
    if llm.api_key.read().await.is_empty() {
        return Ok(());
    }

    let last_close = prices.last().copied().unwrap_or(0.0);

    let config_guard = ctx.config.read().await;
    let slow_tf_secs = config_guard.slow_timeframe.duration_seconds;
    let macro_tf_secs = config_guard.macro_timeframe.duration_seconds;
    let _scoring_cfg = config_guard.scoring.clone();
    let regime_mult = (!config_guard.scoring.regime_weight_multipliers.is_empty())
        .then(|| config_guard.scoring.regime_weight_multipliers.clone());
    drop(config_guard);

    let snapshot_micro = db::query_latest_snapshot(&ctx.pool, &ctx.symbol, 60).await;
    let snapshot_fast = db::query_latest_snapshot(&ctx.pool, &ctx.symbol, 180).await;
    let snapshot_slow = db::query_latest_snapshot(&ctx.pool, &ctx.symbol, slow_tf_secs).await;
    let snapshot_macro = db::query_latest_snapshot(&ctx.pool, &ctx.symbol, macro_tf_secs).await;

    let indicators_micro = build_indicator_snapshot(&snapshot_micro);
    let indicators_fast = build_indicator_snapshot(&snapshot_fast);
    let indicators_slow = build_indicator_snapshot(&snapshot_slow);
    let indicators_macro = build_indicator_snapshot(&snapshot_macro);

    let (support_levels, resistance_levels) =
        crate::server::compute_support_resistance(prices, last_close);

    let support_strings: Vec<String> = support_levels.iter().map(|s| s.to_string()).collect();
    let resistance_strings: Vec<String> = resistance_levels.iter().map(|s| s.to_string()).collect();
    let telemetry = crate::server::compile_deterministic_telemetry(
        &indicators_micro,
        &support_strings,
        &resistance_strings,
        None,
        regime_mult.as_ref(),
    );

    let weight_overrides_guard = ctx.weight_overrides.read().await;
    let weight_map = weight_overrides_guard.as_ref();

    // Build indicator DTO array from normalized indicator map
    let indicators_dto = build_indicators_dto(&indicators_micro);

    // Build decision context
    let decision_context = build_decision_context_json(&indicators_micro);

    // Build market context
    let market_context_json = serde_json::to_string(
        &shared::market_context::MarketContext::synthesize(&indicators_micro.indicators),
    )
    .unwrap_or_else(|_| "{}".to_string());

    // Step 1: Analyst Agent — prepare structured market document
    let analyst_document = llm
        .run_analyst_agent(
            &ctx.symbol,
            last_close,
            &indicators_dto,
            &decision_context,
            &market_context_json,
            &support_strings,
            &resistance_strings,
            prices,
            Some(&ctx.symbol),
        )
        .await
        .map_err(|e| format!("Analyst agent failed: {}", e))?;

    let analyst_json = serde_json::to_string(&analyst_document)
        .map_err(|e| format!("Failed to serialize analyst document: {}", e))?;

    // Institutional Risk Management Layer — deterministic advisory risk profile.
    let risk_profile_json = {
        let cfg = ctx.config.read().await;
        let risk_cfg = cfg.risk.clone();
        let suspend = cfg.safety.consecutive_loss_suspend;
        let drawdown_limit = cfg.safety.capital_drawdown_pct;
        let timeframe_secs = cfg.candles.duration_seconds as i64;
        drop(cfg);

        let indicators = &indicators_micro.indicators;
        let market = shared::market_context::MarketContext::synthesize(indicators);
        let atr_val = indicators.get("atr").map(|v| v.raw_value).unwrap_or(0.0);
        let mut sum = 0.0f64;
        let mut wgt = 0.0f64;
        for meta in shared::indicators::registry::INDICATORS {
            if meta.directional {
                if let Some(v) = indicators.get(meta.key) {
                    sum += meta.default_weight * v.normalized;
                    wgt += meta.default_weight;
                }
            }
        }
        let confluence = if wgt > 0.0 { (sum / wgt * 100.0).clamp(-100.0, 100.0) } else { 0.0 };
        let decision = shared::decision_context::DecisionContext::compute(
            indicators, last_close, atr_val, confluence,
        );
        let engine = crate::risk_engine::RiskEngine::new(risk_cfg, suspend, drawdown_limit);
        let profile = engine
            .evaluate(
                &ctx.pool,
                &ctx.symbol,
                &ctx.symbol,
                timeframe_secs,
                indicators,
                Some(&market),
                Some(&decision),
                None,
                Some(&ctx.safety),
            )
            .await;
        serde_json::to_string(&profile).unwrap_or_default()
    };

    // Step 2: Trader Agent — make final trading decision
    let trader_decision = llm
        .run_trader_agent(
            &analyst_json,
            "None",
            "",
            &ctx.symbol,
            &risk_profile_json,
            Some(&ctx.symbol),
        )
        .await
        .map_err(|e| format!("Trader agent failed: {}", e))?;

    let trend = if analyst_document.market_summary.to_lowercase().contains("bullish") {
        "UPWARD"
    } else if analyst_document.market_summary.to_lowercase().contains("bearish") {
        "DOWNWARD"
    } else {
        "SIDEWAYS"
    };

    let _ = ctx
        .telemetry_tx
        .send(db::TelemetryMsg::UpdateMasterRecord {
            master_id,
            general_trend: trend.to_string(),
            support_levels: serde_json::to_string(&support_strings).unwrap_or_default(),
            resistance_levels: serde_json::to_string(&resistance_strings).unwrap_or_default(),
            indicator_synthesis_summary: format!(
                "Action: {} (confidence: {})",
                trader_decision.action, trader_decision.confidence
            ),
            indicator_synthesis_evaluation: analyst_document.confluence_summary.clone(),
            recommended_action: trader_decision.action.clone(),
            recommendation_rationale: trader_decision.rationale.clone(),
            score_points: Some(trader_decision.confidence as i32),
            signals_json: None,
        })
        .await;

    if let Some(wm) = weight_map {
        if let Ok(json) = serde_json::to_string(wm) {
            let _ = sqlx::query(
                "UPDATE master_assistant_records SET indicator_weights_json = ?2 WHERE id = ?1",
            )
            .bind(master_id)
            .bind(&json)
            .execute(&ctx.pool)
            .await;
        }
    }

    println!(
        "🤖 Automation: {} analysis complete. Action: {} (confidence: {})",
        ctx.pair_key,
        trader_decision.action,
        trader_decision.confidence,
    );

    let local_snap = indicator_to_snapshot(&indicators_micro);
    let regime = classify_market_regime(&local_snap);

    let _ = sqlx::query(
        "UPDATE master_assistant_records SET market_regime = ?2, portfolio_allocation_pct = ?3 WHERE id = ?1",
    )
    .bind(master_id)
    .bind(regime.as_str())
    .bind(trader_decision.confidence as f64)
    .execute(&ctx.pool)
    .await;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    db::insert_decision_memory_buffer(
        &ctx.pool,
        &ctx.symbol,
        now,
        regime.as_str(),
        &trader_decision.action,
        85,
        trader_decision.confidence as i32,
        2.0,
    )
    .await;

    let balance = db::paper_get_balance(&ctx.pool, &ctx.symbol).await;
    if !balance.auto_execute {
        return Ok(());
    }

    let action = trader_decision.action.as_str();
    let current_price = prices.last().copied().unwrap_or(0.0);

    let sr_levels_f64: (Vec<f64>, Vec<f64>) = {
        let supports: Vec<f64> = support_levels.iter().filter_map(|s| s.parse::<f64>().ok()).collect();
        let resistances: Vec<f64> = resistance_levels.iter().filter_map(|s| s.parse::<f64>().ok()).collect();
        (supports, resistances)
    };

    let auto_cfg_light = {
        let cfg = ctx.config.read().await;
        cfg.instances
            .get(&ctx.pair_key)
            .map(|p| p.automation.clone())
            .unwrap_or_default()
    };
    let use_scoring = auto_cfg_light.use_scoring_allocation;
    let max_opposite = auto_cfg_light.max_opposite_exit_signals as u32;

    let existing_positions = crate::portfolio_risk::query_all_active_positions(&ctx.pool).await;
    let validation = crate::portfolio_risk::validate_new_position(
        &ctx.portfolio_risk,
        &ctx.pool,
        &ctx.symbol,
        trader_decision.confidence as f64,
        &existing_positions,
        &ctx.pair_close_histories,
    )
    .await;
    if !validation.allowed
        && trader_decision.confidence > 0
        && (action == "Open Long" || action == "Open Short")
    {
        println!(
            "🛑 Auto Paper: {} portfolio risk check failed: {}",
            ctx.pair_key, validation.reason
        );
        return Ok(());
    }

    if (action == "Open Long" || action == "Open Short") && !regime_allows_entry(&regime) {
        println!(
            "🛑 Auto Paper: {} blocked by regime {} — entries not permitted in this market state",
            ctx.pair_key, regime.as_str()
        );
        return Ok(());
    }

    let eight_factor_score = if use_scoring {
        Some(trader_decision.confidence as f64)
    } else {
        None
    };

    if use_scoring {
        let pos = db::paper_get_active_position(&ctx.pool, &ctx.symbol).await;
        if let Some(ref p) = pos {
            let macro_trend = trend;
            let snap_values = indicator_to_snapshot(&indicators_micro);
            let regime_mult = {
                let cfg = ctx.config.read().await;
                (!cfg.scoring.regime_weight_multipliers.is_empty())
                    .then(|| cfg.scoring.regime_weight_multipliers.clone())
            };
            let (should_exit, opposite_count) = paper_trading::evaluate_opposite_exit(
                &p.direction,
                &snap_values,
                &sr_levels_f64.0,
                &sr_levels_f64.1,
                macro_trend,
                max_opposite,
                regime_mult.as_ref(),
            );
            if should_exit {
                println!(
                    "🛑 Auto Paper: {} Opposite-signal exit triggered! {} opposite signals (limit: {})",
                    ctx.pair_key, opposite_count, max_opposite
                );
                let _ = paper_trading::close_paper_position(
                    &ctx.pool,
                    &ctx.telemetry_tx,
                    &ctx.symbol,
                    current_price,
                    &format!("OPPOSITE_EXIT:{}", opposite_count),
                )
                .await;
                return Ok(());
            }
        }
    }

    if use_scoring {
        paper_trading::check_break_even_trail(&ctx.pool, &ctx.symbol, current_price).await;
    }

    let slow_trend_direction = determine_slow_trend_direction(&indicators_slow);

    let micro_consensus = determine_slow_trend_direction(&indicators_micro);
    let fast_consensus = determine_slow_trend_direction(&indicators_fast);
    let slow_consensus = determine_slow_trend_direction(&indicators_slow);
    let macro_consensus = determine_slow_trend_direction(&indicators_macro);

    let (confluence, count) = evaluate_confluence_mtf(
        micro_consensus,
        fast_consensus,
        slow_consensus,
        macro_consensus,
    );

    match action {
        "Open Long" => {
            if slow_trend_direction != "BULLISH" {
                println!(
                    "📄 Auto Paper: {} skipping Open Long — slow trend is {} (15m chart)",
                    ctx.pair_key, slow_trend_direction
                );
            } else if confluence == "BULLISH" {
                let res = paper_trading::verify_margin_and_open_with_alloc(
                    &ctx.pool,
                    &ctx.telemetry_tx,
                    &ctx.symbol,
                    "LONG",
                    current_price,
                    eight_factor_score,
                )
                .await;
                if res.success {
                    try_set_fibonacci_take_profits(
                        &ctx.pool, &ctx.symbol, "LONG", current_price,
                        &indicators_micro, &regime,
                    ).await;
                }
                println!(
                    "📄 Auto Paper: {} {} (confluence {}/4, slow {})",
                    ctx.pair_key, res.message, count, slow_trend_direction
                );
            } else {
                println!(
                    "📄 Auto Paper: {} skipping Open Long — no confluence ({}/4 aligned)",
                    ctx.pair_key, count
                );
            }
        }
        "Open Short" => {
            if slow_trend_direction != "BEARISH" {
                println!(
                    "📄 Auto Paper: {} skipping Open Short — slow trend is {} (15m chart)",
                    ctx.pair_key, slow_trend_direction
                );
            } else if confluence == "BEARISH" {
                let res = paper_trading::verify_margin_and_open_with_alloc(
                    &ctx.pool,
                    &ctx.telemetry_tx,
                    &ctx.symbol,
                    "SHORT",
                    current_price,
                    eight_factor_score,
                )
                .await;
                if res.success {
                    try_set_fibonacci_take_profits(
                        &ctx.pool, &ctx.symbol, "SHORT", current_price,
                        &indicators_micro, &regime,
                    ).await;
                }
                println!(
                    "📄 Auto Paper: {} {} (confluence {}/4, slow {})",
                    ctx.pair_key, res.message, count, slow_trend_direction
                );
            } else {
                println!(
                    "📄 Auto Paper: {} skipping Open Short — no confluence ({}/4 aligned)",
                    ctx.pair_key, count
                );
            }
        }
        "Close" => {
            let pos = db::paper_get_active_position(&ctx.pool, &ctx.symbol).await;
            if pos.is_some() {
                let res = paper_trading::close_paper_position(
                    &ctx.pool,
                    &ctx.telemetry_tx,
                    &ctx.symbol,
                    current_price,
                    "AUTOMATED",
                )
                .await;
                println!("📄 Auto Paper: {} {}", ctx.pair_key, res.message);

                if let Some(ref p) = pos {
                    let pnl = if p.direction == "LONG" {
                        (current_price - p.entry_price) * p.size
                    } else {
                        (p.entry_price - current_price) * p.size
                    };
                    let roi = if p.allocated_usd > 0.0 {
                        (pnl / p.allocated_usd) * 100.0
                    } else {
                        0.0
                    };
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64;
                    db::trade_telemetry_insert(
                        &ctx.pool,
                        "Hyperliquid",
                        &ctx.symbol,
                        &p.direction,
                        p.entry_timestamp,
                        now,
                        p.entry_price,
                        current_price,
                        p.size,
                        p.allocated_usd * 0.0006,
                        0.0,
                        pnl,
                        roi,
                        "AUTOMATED",
                    )
                    .await;

                    let _ = ctx
                        .telemetry_tx
                        .send(db::TelemetryMsg::JournalTrade {
                            symbol: ctx.symbol.clone(),
                            direction: p.direction.clone(),
                            entry_price: p.entry_price,
                            exit_price: current_price,
                            entry_timestamp: p.entry_timestamp,
                            exit_timestamp: now,
                            size: p.size,
                            realized_pnl: pnl,
                            roi_pct: roi,
                            allocated_usd: p.allocated_usd,
                            trigger: "AUTOMATED".to_string(),
                        })
                        .await;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

/// Determine the medium trend direction from the 15-minute chart (Section 3.2).
/// Returns "BULLISH" if price > 200 EMA on medium timeframe, "BEARISH" otherwise.
fn determine_slow_trend_direction(indicators: &crate::server::IndicatorSnapshot) -> &'static str {
    match (indicators.ema_long(), indicators.current_price) {
        (Some(ema), Some(price)) if price > ema => "BULLISH",
        (Some(_ema), Some(_price)) => "BEARISH",
        _ => {
            match (indicators.ema_fast(), indicators.ema_slow()) {
                (Some(fast), Some(slow)) if fast > slow => "BULLISH",
                _ => "BEARISH",
            }
        }
    }
}

fn evaluate_confluence_mtf(micro_signal: &str, fast_signal: &str, slow_signal: &str, macro_signal: &str) -> (&'static str, usize) {
    let bullish = ["BULLISH", "UPWARD"];
    let bearish = ["BEARISH", "DOWNWARD"];

    let micro_bull = bullish.iter().any(|&s| micro_signal.to_uppercase().contains(s));
    let micro_bear = bearish.iter().any(|&s| micro_signal.to_uppercase().contains(s));
    let fast_bull = bullish.iter().any(|&s| fast_signal.to_uppercase().contains(s));
    let fast_bear = bearish.iter().any(|&s| fast_signal.to_uppercase().contains(s));
    let slow_bull = bullish.iter().any(|&s| slow_signal.to_uppercase().contains(s));
    let slow_bear = bearish.iter().any(|&s| slow_signal.to_uppercase().contains(s));
    let macro_bull = bullish.iter().any(|&s| macro_signal.to_uppercase().contains(s));
    let macro_bear = bearish.iter().any(|&s| macro_signal.to_uppercase().contains(s));

    let bull_count = [micro_bull, fast_bull, slow_bull, macro_bull].iter().filter(|&&x| x).count();
    let bear_count = [micro_bear, fast_bear, slow_bear, macro_bear].iter().filter(|&&x| x).count();

    // Require at least 3 of 4 timeframes aligned
    if bull_count >= 3 {
        ("BULLISH", bull_count)
    } else if bear_count >= 3 {
        ("BEARISH", bear_count)
    } else {
        ("SIDEWAYS", 0)
    }
}



fn build_indicator_snapshot(
    snapshot: &Option<shared::models::MarketSnapshot>,
) -> crate::server::IndicatorSnapshot {
    match snapshot {
        Some(s) => {
            let current_price = s.mid_price.to_string().parse::<f64>().ok();
            let mut snap = crate::server::IndicatorSnapshot::new(s.indicators.clone(), current_price);
            snap.volume = s.volume.and_then(|d| d.to_string().parse::<f64>().ok());
            snap.average_volume = s.average_volume.and_then(|d| d.to_string().parse::<f64>().ok());
            snap
        }
        None => crate::server::IndicatorSnapshot::default(),
    }
}

/// Build a compact JSON array of indicator DTOs from the normalized indicator map.
fn build_indicators_dto(snap: &crate::server::IndicatorSnapshot) -> String {
    let arr: Vec<serde_json::Value> = snap
        .indicators
        .iter()
        .map(|(key, v)| {
            let mut obj = serde_json::json!({
                "indicator_name": key,
                "normalized": v.normalized,
                "state_label": v.state_label,
            });
            if let Some(vals) = &v.values {
                obj["values"] = serde_json::to_value(vals).unwrap_or(serde_json::json!({}));
            }
            obj
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

/// Build decision context JSON from snapshot indicators.
fn build_decision_context_json(snap: &crate::server::IndicatorSnapshot) -> String {
    let total = snap.indicators.len().max(1) as f64;
    let bullish_count = snap.indicators.values().filter(|v| v.normalized > 0.0).count() as f64;
    let bearish_count = snap.indicators.values().filter(|v| v.normalized < 0.0).count() as f64;
    let active_count = snap.indicators.values().filter(|v| v.normalized.abs() > 0.2).count() as f64;

    let directional_sum: f64 = snap.indicators.values().map(|v| v.normalized).sum();
    let bias = (directional_sum / total).clamp(-1.0, 1.0);

    serde_json::to_string(&serde_json::json!({
        "bullish_probability": bullish_count / total,
        "bearish_probability": bearish_count / total,
        "directional_bias": bias,
        "consensus": active_count / total,
        "confluence": (bias * 100.0) as i32,
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

async fn try_set_fibonacci_take_profits(
    pool: &SqlitePool,
    symbol: &str,
    direction: &str,
    entry_price: f64,
    snap: &crate::server::IndicatorSnapshot,
    regime: &crate::profile_evaluation::MarketRegime,
) {
    let fib = match snap.indicators.get("fibonacci") {
        Some(v) => v,
        None => return,
    };
    let vals = match &fib.values {
        Some(m) => m,
        None => return,
    };
    let ext_1618 = vals.get("ext_1618").copied().unwrap_or(0.0);
    let ext_2618 = vals.get("ext_2618").copied().unwrap_or(0.0);

    if ext_1618 <= 0.0 {
        return;
    }

    let mut targets: Vec<(f64, f64)> = Vec::new();

    if direction == "LONG" {
        if ext_1618 > entry_price {
            targets.push((50.0, ext_1618));
        }
        if ext_2618 > ext_1618 {
            let close_pct = if *regime == crate::profile_evaluation::MarketRegime::Expansion {
                100.0
            } else {
                50.0
            };
            targets.push((close_pct, ext_2618));
        }
    } else {
        if ext_1618 < entry_price {
            targets.push((50.0, ext_1618));
        }
        if ext_2618 < ext_1618 {
            let close_pct = if *regime == crate::profile_evaluation::MarketRegime::Expansion {
                100.0
            } else {
                50.0
            };
            targets.push((close_pct, ext_2618));
        }
    }

    if !targets.is_empty() {
        let _ = paper_trading::set_take_profit_targets(pool, symbol, &targets).await;
    }
}
