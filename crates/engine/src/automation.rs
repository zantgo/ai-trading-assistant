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
use crate::profile_evaluation::{SnapshotValues, classify_market_regime};
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
    micro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    config: Arc<RwLock<AppConfig>>,
    pool: SqlitePool,
    llm_client: Arc<LlmClient>,
    telemetry_tx: mpsc::Sender<db::TelemetryMsg>,
    portfolio_risk: Arc<PortfolioRiskState>,
    pair_close_histories: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    safety: Arc<SafetyManager>,
    intervals: IntervalsConfig,
    next_interval_override: Arc<RwLock<Option<u64>>>,
    operational_mode: OperationalMode,
    weight_overrides: Arc<RwLock<Option<HashMap<String, i32>>>>,
}

fn ctx_to_clone(ctx: &AutomationContext) -> AutomationContextLight {
    AutomationContextLight {
        pair_key: ctx.pair_key.clone(),
        symbol: ctx.symbol.clone(),
        micro_history: ctx.micro_history.clone(),
        micro_latest: ctx.micro_latest.clone(),
        config: ctx.config.clone(),
        pool: ctx.pool.clone(),
        llm_client: ctx.llm_client.clone(),
        telemetry_tx: ctx.telemetry_tx.clone(),
        portfolio_risk: ctx.portfolio_risk.clone(),
        pair_close_histories: ctx.pair_close_histories.clone(),
        safety: ctx.safety.clone(),
        intervals: ctx.intervals.clone(),
        next_interval_override: ctx.next_interval_override.clone(),
        operational_mode: ctx.operational_mode.clone(),
        weight_overrides: ctx.weight_overrides.clone(),
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
    );

    let weight_overrides_guard = ctx.weight_overrides.read().await;
    let weight_map = weight_overrides_guard.as_ref();

    let multi_agent_results = crate::server::run_multi_agent_pipeline(
        llm.clone(),
        ctx.pool.clone(),
        &ctx.symbol,
        &indicators_micro,
        &indicators_fast,
        &indicators_slow,
        &indicators_macro,
        prices,
        master_id,
        &telemetry,
    )
    .await
    .map_err(|e| format!("Parallel agents failed: {}", e))?;

    let legacy_signals = multi_agent_results.to_legacy_signals();
    let phase_one_json = serde_json::to_string(&legacy_signals).unwrap_or_else(|_| "[]".into());

    let journal_context =
        db::query_recent_journal_for_context(&ctx.pool, &ctx.symbol, 10).await;
    let journal_opt: Option<&str> =
        if journal_context.is_empty() {
            None
        } else {
            Some(&journal_context)
        };

    let indicators_json = serde_json::to_string(&indicators_micro.indicators).ok();
    let phase_two = llm
        .run_multi_timeframe_orchestrator(
            "None",
            "",
            &ctx.symbol,
            &phase_one_json,
            &support_strings,
            &resistance_strings,
            journal_opt,
            Some(&ctx.symbol),
            telemetry.total_confluence_score,
            None,
            indicators_json.as_deref(),
        )
        .await
        .map_err(|e| format!("Orchestrator failed: {}", e))?;

    let _ = ctx
        .telemetry_tx
        .send(db::TelemetryMsg::UpdateMasterRecord {
            master_id,
            general_trend: phase_two.general_trend.clone(),
            support_levels: serde_json::to_string(
                &phase_two.support_and_resistance.detected_support_levels,
            )
            .unwrap_or_default(),
            resistance_levels: serde_json::to_string(
                &phase_two.support_and_resistance.detected_resistance_levels,
            )
            .unwrap_or_default(),
            indicator_synthesis_summary: phase_two.indicator_synthesis.summary_count.clone(),
            indicator_synthesis_evaluation: phase_two.indicator_synthesis.evaluation.clone(),
            recommended_action: phase_two.position_recommendation.action.clone(),
            recommendation_rationale: phase_two.position_recommendation.rationale.clone(),
            score_points: Some(phase_two.eight_factor_score),
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
        "🤖 Automation: {} analysis complete. Action: {} | Trend: {} | Interval: {}",
        ctx.pair_key,
        phase_two.position_recommendation.action,
        phase_two.general_trend,
        phase_two.position_recommendation.next_interval.as_deref().unwrap_or("normal"),
    );

    if let Some(ref next) = phase_two.position_recommendation.next_interval {
        let new_secs = match next.as_str() {
            "slow" => ctx.intervals.slow_seconds,
            "fast" => ctx.intervals.fast_seconds,
            _ => ctx.intervals.normal_seconds,
        };
        *ctx.next_interval_override.write().await = Some(new_secs);
    }

    let local_snap = indicator_to_snapshot(&indicators_micro);
    let regime = classify_market_regime(&local_snap);

    let _ = sqlx::query(
        "UPDATE master_assistant_records SET market_regime = ?2, portfolio_allocation_pct = ?3 WHERE id = ?1",
    )
    .bind(master_id)
    .bind(regime.as_str())
    .bind(phase_two.allocation_pct)
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
        &phase_two.position_recommendation.action,
        85,
        phase_two.eight_factor_score,
        2.0,
    )
    .await;

    let balance = db::paper_get_balance(&ctx.pool, &ctx.symbol).await;
    if !balance.auto_execute {
        return Ok(());
    }

    let action = phase_two.position_recommendation.action.as_str();
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
        phase_two.allocation_pct,
        &existing_positions,
        &ctx.pair_close_histories,
    )
    .await;
    if !validation.allowed
        && phase_two.allocation_pct > 0.0
        && (action == "Open Long" || action == "Open Short")
    {
        println!(
            "🛑 Auto Paper: {} portfolio risk check failed: {}",
            ctx.pair_key, validation.reason
        );
        return Ok(());
    }

    let eight_factor_score = if use_scoring {
        Some(phase_two.allocation_pct)
    } else {
        None
    };

    if use_scoring {
        let pos = db::paper_get_active_position(&ctx.pool, &ctx.symbol).await;
        if let Some(ref p) = pos {
            let macro_trend = phase_two.general_trend.as_str();
            let snap_values = indicator_to_snapshot(&indicators_micro);
            let (should_exit, opposite_count) = paper_trading::evaluate_opposite_exit(
                &p.direction,
                &snap_values,
                &sr_levels_f64.0,
                &sr_levels_f64.1,
                macro_trend,
                max_opposite,
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

    let micro_trend = &legacy_signals
        .iter()
        .filter(|r| r.indicator_name.starts_with("micro-"))
        .map(|r| r.signal.as_str())
        .collect::<Vec<_>>();
    let fast_trend = &legacy_signals
        .iter()
        .filter(|r| r.indicator_name.starts_with("fast-"))
        .map(|r| r.signal.as_str())
        .collect::<Vec<_>>();
    let slow_trend_signals = &legacy_signals
        .iter()
        .filter(|r| r.indicator_name.starts_with("slow-"))
        .map(|r| r.signal.as_str())
        .collect::<Vec<_>>();
    let macro_trend_signals = &legacy_signals
        .iter()
        .filter(|r| r.indicator_name.starts_with("macro-"))
        .map(|r| r.signal.as_str())
        .collect::<Vec<_>>();

    let micro_consensus = if micro_trend.iter().filter(|&&s| s == "BULLISH").count() >= micro_trend.len() / 2 { "BULLISH" } else if micro_trend.iter().filter(|&&s| s == "BEARISH").count() >= micro_trend.len() / 2 { "BEARISH" } else { "SIDEWAYS" };
    let fast_consensus = if fast_trend.iter().filter(|&&s| s == "BULLISH").count() >= fast_trend.len() / 2 { "BULLISH" } else if fast_trend.iter().filter(|&&s| s == "BEARISH").count() >= fast_trend.len() / 2 { "BEARISH" } else { "SIDEWAYS" };
    let slow_consensus = if slow_trend_signals.iter().filter(|&&s| s == "BULLISH").count() >= slow_trend_signals.len() / 2 { "BULLISH" } else if slow_trend_signals.iter().filter(|&&s| s == "BEARISH").count() >= slow_trend_signals.len() / 2 { "BEARISH" } else { "SIDEWAYS" };
    let macro_consensus = if macro_trend_signals.iter().filter(|&&s| s == "BULLISH").count() >= macro_trend_signals.len() / 2 { "BULLISH" } else if macro_trend_signals.iter().filter(|&&s| s == "BEARISH").count() >= macro_trend_signals.len() / 2 { "BEARISH" } else { "SIDEWAYS" };

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
