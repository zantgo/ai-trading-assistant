use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{RwLock, mpsc};
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::config::{AppConfig, AutomationConfig, IntervalsConfig};
use crate::db;
use crate::llm::LlmClient;
use crate::paper_trading;
use crate::profile_evaluation::{SnapshotValues, classify_market_regime};
use crate::safety::SafetyManager;

fn indicator_to_snapshot(snap: &crate::server::IndicatorSnapshot) -> SnapshotValues {
    SnapshotValues {
        rsi: snap.rsi,
        squeeze_on: snap.squeeze_on,
        squeeze_momentum: snap.squeeze_momentum,
        squeeze_duration: snap.squeeze_duration,
        squeeze_release_trigger: snap.squeeze_release_trigger,
        squeeze_momentum_direction: snap.squeeze_momentum_direction.clone(),
        chart_pattern: snap.chart_pattern.clone(),
        chart_pattern_confidence: snap.chart_pattern_confidence,
        bbwp: snap.bbwp,
        macd_line: snap.macd_line,
        macd_signal: snap.macd_signal,
        macd_hist: snap.macd_histogram,
        adx: snap.adx,
        adx_plus: snap.adx_plus,
        adx_minus: snap.adx_minus,
        bb_upper: snap.bb_upper,
        bb_middle: snap.bb_middle,
        bb_lower: snap.bb_lower,
        atr: snap.atr,
        ema_fast: snap.ema_fast,
        ema_medium: snap.ema_medium,
        ema_slow: snap.ema_slow,
        ema_long: snap.ema_long,
        ema_stack_state: snap.ema_stack_state.clone(),
        vwap: snap.vwap,
        vwap_bias: snap.vwap_bias.clone(),
        close: snap.current_price,
        volume: snap.volume,
        average_volume: snap.average_volume,
        rvol: snap.rvol,
        current_price: snap.current_price.unwrap_or(0.0),
        rsi_divergence_status: None,
        macd_divergence_status: None,
        macd_trend_state: snap.macd_trend_state.clone(),
        macd_crossover_detected: snap.macd_crossover_detected,
        macd_crossover_direction: snap.macd_crossover_direction.clone(),
        macd_histogram_peak: snap.macd_histogram_peak,
        atr_volatility_regime: snap.atr_volatility_regime.clone(),
        adx_slope: None,
        adx_regime: None,
        adx_di_crossover_detected: None,
        adx_di_crossover_direction: None,
    }
}
use shared::models::MarketSnapshot;
use shared::normalized::NormalizedCandle;
use shared::TriggerType;
use crate::portfolio_risk::PortfolioRiskState;

pub struct AutomationContext {
    pub pair_key: String,
    pub symbol: String,
    pub micro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub short_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub medium_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub large_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub micro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub short_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub medium_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub large_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub micro_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub short_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub medium_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub large_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
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
                if elapsed >= self.interval_seconds {
                    0
                } else {
                    self.interval_seconds - elapsed
                }
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
        let auto_cfg = fresh_config
            .instances
            .get(&ctx.pair_key)
            .map(|p| &p.automation)
            .cloned()
            .unwrap_or_default();

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

        // ─── Safety Check ────────────────────────────────────────────
        if let Err(reason) = ctx.safety.check_allow_trade().await {
            println!("🛑 Automation: {} safety block: {}", ctx.pair_key, reason);
            continue;
        }
        if let Err(reason) = ctx.safety.check_capital_drawdown().await {
            eprintln!("🛑 Automation: {} drawdown stop triggered: {}", ctx.pair_key, reason);
            // Immediately close any open positions to protect remaining capital
            if let Some(_) = db::paper_get_active_position(&ctx.pool, &ctx.symbol).await {
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

        let remaining = state.next_remaining_secs();
        if remaining > 0 {
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

        let last_close = prices.last().copied().unwrap_or(0.0);

        {
            let mut hist_map = ctx.pair_close_histories.write().await;
            let entry = hist_map.entry(ctx.symbol.clone()).or_default();
            *entry = prices.clone();
        }

        if ctx.llm_client.api_key.read().await.is_empty() {
            continue;
        }

        let master_id = db::insert_master_placeholder(
            &ctx.pool,
            "None",
            "",
            &format!("{}", last_close),
            &ctx.symbol,
            TriggerType::Automated,
        )
        .await;

        db::insert_automated_performance_baseline(
            &ctx.pool,
            master_id,
            &ctx.symbol,
            &format!("{}", last_close),
        )
        .await;

        state.last_run = Some(std::time::Instant::now());

        if let Err(e) = execute_automation_cycle(&ctx, master_id, &prices).await {
            eprintln!("Automation cycle error for {}: {}", ctx.pair_key, e);
        }
    }

    println!("🛑 Automation Task: {} scheduler terminated.", ctx.pair_key);
}

async fn execute_automation_cycle(
    ctx: &AutomationContext,
    master_id: i64,
    prices: &[f64],
) -> Result<(), String> {
    let llm = ctx.llm_client.clone();
    if llm.api_key.read().await.is_empty() {
        return Ok(());
    }

    let last_close = prices.last().copied().unwrap_or(0.0);

    // Gather snapshots from all 4 timeframes
    let config_guard = ctx.config.read().await;
    let medium_tf_secs = config_guard.medium_timeframe.duration_seconds;
    let large_tf_secs = config_guard.large_timeframe.duration_seconds;
    drop(config_guard);

    let snapshot_micro = db::query_latest_snapshot(&ctx.pool, &ctx.symbol, 60).await;
    let snapshot_small = db::query_latest_snapshot(&ctx.pool, &ctx.symbol, 300).await;
    let snapshot_medium = db::query_latest_snapshot(&ctx.pool, &ctx.symbol, medium_tf_secs).await;
    let snapshot_large = db::query_latest_snapshot(&ctx.pool, &ctx.symbol, large_tf_secs).await;

    let indicators_micro = build_indicator_snapshot(&snapshot_micro);
    let indicators_small = build_indicator_snapshot(&snapshot_small);
    let indicators_medium = build_indicator_snapshot(&snapshot_medium);
    let indicators_large = build_indicator_snapshot(&snapshot_large);

    let (support_levels, resistance_levels) =
        crate::server::compute_support_resistance(prices, last_close);

    let support_strings: Vec<String> = support_levels.iter().map(|s| s.to_string()).collect();
    let resistance_strings: Vec<String> = resistance_levels.iter().map(|s| s.to_string()).collect();
    let telemetry = crate::server::compile_deterministic_telemetry(
        &indicators_micro,
        &support_strings,
        &resistance_strings,
    );

    let multi_agent_results = crate::server::run_multi_agent_pipeline(
        llm.clone(),
        ctx.pool.clone(),
        &ctx.symbol,
        &indicators_micro,
        &indicators_small,
        &indicators_medium,
        &indicators_large,
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

    println!(
        "🤖 Automation: {} analysis complete. Action: {} | Trend: {} | Interval: {}",
        ctx.pair_key,
        phase_two.position_recommendation.action,
        phase_two.general_trend,
        phase_two.position_recommendation.next_interval.as_deref().unwrap_or("normal"),
    );

    // ─── Dynamic Interval Selection ─────────────────────────
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

    // Paper trading evaluation
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

    let auto_cfg = {
        let cfg = ctx.config.read().await;
        cfg.instances
            .get(&ctx.pair_key)
            .map(|p| p.automation.clone())
            .unwrap_or_default()
    };
    let use_scoring = auto_cfg.use_scoring_allocation;
    let max_opposite = auto_cfg.max_opposite_exit_signals as u32;

    // Portfolio risk validation
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

    let mid_hist = ctx.micro_history.read().await;
    let mid_candles: Vec<NormalizedCandle> = mid_hist.iter().cloned().collect();
    drop(mid_hist);
    if let Some(inval_price) =
        check_decisive_close_invalidation(&ctx.pool, &ctx.symbol, &mid_candles).await
    {
        let _ = paper_trading::invalidate_position(
            &ctx.pool,
            &ctx.telemetry_tx,
            &ctx.symbol,
            inval_price,
            "DECISIVE_CLOSE_1M",
        )
        .await;
        println!(
            "🛑 Auto Paper: {} position invalidated by 1m decisive close at ${:.2}",
            ctx.pair_key, inval_price
        );
        return Ok(());
    }

    let medium_trend_direction = determine_medium_trend_direction(&indicators_medium);

    let micro_trend = &legacy_signals
        .iter()
        .filter(|r| r.indicator_name.starts_with("micro-"))
        .map(|r| r.signal.as_str())
        .collect::<Vec<_>>();
    let small_trend = &legacy_signals
        .iter()
        .filter(|r| r.indicator_name.starts_with("small-"))
        .map(|r| r.signal.as_str())
        .collect::<Vec<_>>();
    let medium_trend_signals = &legacy_signals
        .iter()
        .filter(|r| r.indicator_name.starts_with("medium-"))
        .map(|r| r.signal.as_str())
        .collect::<Vec<_>>();
    let large_trend_signals = &legacy_signals
        .iter()
        .filter(|r| r.indicator_name.starts_with("large-"))
        .map(|r| r.signal.as_str())
        .collect::<Vec<_>>();

    let micro_consensus = if micro_trend.iter().filter(|&&s| s == "BULLISH").count() >= micro_trend.len() / 2 { "BULLISH" } else if micro_trend.iter().filter(|&&s| s == "BEARISH").count() >= micro_trend.len() / 2 { "BEARISH" } else { "SIDEWAYS" };
    let small_consensus = if small_trend.iter().filter(|&&s| s == "BULLISH").count() >= small_trend.len() / 2 { "BULLISH" } else if small_trend.iter().filter(|&&s| s == "BEARISH").count() >= small_trend.len() / 2 { "BEARISH" } else { "SIDEWAYS" };
    let medium_consensus = if medium_trend_signals.iter().filter(|&&s| s == "BULLISH").count() >= medium_trend_signals.len() / 2 { "BULLISH" } else if medium_trend_signals.iter().filter(|&&s| s == "BEARISH").count() >= medium_trend_signals.len() / 2 { "BEARISH" } else { "SIDEWAYS" };
    let large_consensus = if large_trend_signals.iter().filter(|&&s| s == "BULLISH").count() >= large_trend_signals.len() / 2 { "BULLISH" } else if large_trend_signals.iter().filter(|&&s| s == "BEARISH").count() >= large_trend_signals.len() / 2 { "BEARISH" } else { "SIDEWAYS" };

    let (confluence, count) = evaluate_confluence_mtf(
        micro_consensus,
        small_consensus,
        medium_consensus,
        large_consensus,
    );

    match action {
        "Open Long" => {
            if medium_trend_direction != "BULLISH" {
                println!(
                    "📄 Auto Paper: {} skipping Open Long — medium trend is {} (15m chart)",
                    ctx.pair_key, medium_trend_direction
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
                    "📄 Auto Paper: {} {} (confluence {}/4, medium {})",
                    ctx.pair_key, res.message, count, medium_trend_direction
                );
            } else {
                println!(
                    "📄 Auto Paper: {} skipping Open Long — no confluence ({}/4 aligned)",
                    ctx.pair_key, count
                );
            }
        }
        "Open Short" => {
            if medium_trend_direction != "BEARISH" {
                println!(
                    "📄 Auto Paper: {} skipping Open Short — medium trend is {} (15m chart)",
                    ctx.pair_key, medium_trend_direction
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
                    "📄 Auto Paper: {} {} (confluence {}/4, medium {})",
                    ctx.pair_key, res.message, count, medium_trend_direction
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
fn determine_medium_trend_direction(indicators: &crate::server::IndicatorSnapshot) -> &'static str {
    match (indicators.ema_long, indicators.current_price) {
        (Some(ema), Some(price)) if price > ema => "BULLISH",
        (Some(_ema), Some(_price)) => "BEARISH",
        _ => {
            match (indicators.ema_fast, indicators.ema_slow) {
                (Some(fast), Some(slow)) if fast > slow => "BULLISH",
                _ => "BEARISH",
            }
        }
    }
}

/// Check if a 1-minute candle has closed decisively beyond the invalidation level.
/// Returns Some(close_price) if invalidated, None otherwise.
async fn check_decisive_close_invalidation(
    pool: &SqlitePool,
    symbol: &str,
    micro_history: &[NormalizedCandle],
) -> Option<f64> {
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = position.as_ref()?;

    let invalidation = Decimal::from_f64(pos.final_invalidation_level?)?;
    let last_candle = micro_history.last()?;
    let close = last_candle.close;

    let tolerance_pct = Decimal::from_f64(0.002).unwrap_or(Decimal::ZERO);
    let buffer = close * tolerance_pct;

    match pos.direction.as_str() {
        "LONG" => {
            if close < invalidation && (invalidation - close) > buffer {
                close.to_f64()
            } else {
                None
            }
        }
        "SHORT" => {
            if close > invalidation && (close - invalidation) > buffer {
                close.to_f64()
            } else {
                None
            }
        }
        _ => None,
    }
}

fn evaluate_confluence_mtf(micro_signal: &str, small_signal: &str, medium_signal: &str, large_signal: &str) -> (&'static str, usize) {
    let bullish = ["BULLISH", "UPWARD"];
    let bearish = ["BEARISH", "DOWNWARD"];

    let micro_bull = bullish.iter().any(|&s| micro_signal.to_uppercase().contains(s));
    let micro_bear = bearish.iter().any(|&s| micro_signal.to_uppercase().contains(s));
    let small_bull = bullish.iter().any(|&s| small_signal.to_uppercase().contains(s));
    let small_bear = bearish.iter().any(|&s| small_signal.to_uppercase().contains(s));
    let medium_bull = bullish.iter().any(|&s| medium_signal.to_uppercase().contains(s));
    let medium_bear = bearish.iter().any(|&s| medium_signal.to_uppercase().contains(s));
    let large_bull = bullish.iter().any(|&s| large_signal.to_uppercase().contains(s));
    let large_bear = bearish.iter().any(|&s| large_signal.to_uppercase().contains(s));

    let bull_count = [micro_bull, small_bull, medium_bull, large_bull].iter().filter(|&&x| x).count();
    let bear_count = [micro_bear, small_bear, medium_bear, large_bear].iter().filter(|&&x| x).count();

    // Require at least 3 of 4 timeframes aligned
    if bull_count >= 3 {
        ("BULLISH", bull_count)
    } else if bear_count >= 3 {
        ("BEARISH", bear_count)
    } else {
        ("SIDEWAYS", 0)
    }
}



fn build_indicator_snapshot(snapshot: &Option<shared::models::MarketSnapshot>) -> crate::server::IndicatorSnapshot {
    match snapshot {
        Some(s) => crate::server::IndicatorSnapshot {
            rsi: s.rsi_14.and_then(|d| d.to_string().parse::<f64>().ok()),
            squeeze_on: s.squeeze_on,
            squeeze_momentum: s.squeeze_momentum.and_then(|d| d.to_string().parse::<f64>().ok()),
            squeeze_duration: s.squeeze_duration,
            squeeze_release_trigger: s.squeeze_release_trigger,
            squeeze_momentum_direction: s.squeeze_momentum_direction.clone(),
            chart_pattern: s.chart_pattern.clone(),
            chart_pattern_confidence: s.chart_pattern_confidence.map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)),
            bbwp: s.bbwp.and_then(|d| d.to_string().parse::<f64>().ok()),
            macd_line: s.macd_line.and_then(|d| d.to_string().parse::<f64>().ok()),
            macd_signal: s.macd_signal.and_then(|d| d.to_string().parse::<f64>().ok()),
            macd_histogram: s.macd_hist.and_then(|d| d.to_string().parse::<f64>().ok()),
            macd_histogram_trend: None,
            adx: s.adx_14.and_then(|d| d.to_string().parse::<f64>().ok()),
            adx_plus: s.adx_plus.and_then(|d| d.to_string().parse::<f64>().ok()),
            adx_minus: s.adx_minus.and_then(|d| d.to_string().parse::<f64>().ok()),
            bb_upper: s.bb_upper.and_then(|d| d.to_string().parse::<f64>().ok()),
            bb_middle: s.bb_middle.and_then(|d| d.to_string().parse::<f64>().ok()),
            bb_lower: s.bb_lower.and_then(|d| d.to_string().parse::<f64>().ok()),
            atr: s.atr_14.and_then(|d| d.to_string().parse::<f64>().ok()),
            atr_trend: None,
            atr_volatility_regime: s.atr_volatility_regime.clone(),
            current_price: Some(s.mid_price.to_string().parse::<f64>().unwrap_or(0.0)),
            volume: s.volume.and_then(|d| d.to_string().parse::<f64>().ok()),
            average_volume: s.average_volume.and_then(|d| d.to_string().parse::<f64>().ok()),
            rvol: s.rvol.and_then(|d| d.to_string().parse::<f64>().ok()),
            ema_fast: s.ema_fast.and_then(|d| d.to_string().parse::<f64>().ok()),
            ema_medium: s.ema_medium.and_then(|d| d.to_string().parse::<f64>().ok()),
            ema_slow: s.ema_slow.and_then(|d| d.to_string().parse::<f64>().ok()),
            ema_long: s.ema_long.and_then(|d| d.to_string().parse::<f64>().ok()),
            ema_stack_state: s.ema_stack_state.clone(),
            vwap: s.vwap.and_then(|d| d.to_string().parse::<f64>().ok()),
            vwap_bias: s.vwap_bias.clone(),
            rsi_divergence_status: s.rsi_divergence_status.clone(),
            macd_divergence_status: s.macd_divergence_status.clone(),
            macd_trend_state: s.macd_trend_state.clone(),
            macd_crossover_detected: s.macd_crossover_detected,
            macd_crossover_direction: s.macd_crossover_direction.clone(),
            macd_histogram_peak: s.macd_histogram_peak.and_then(|d| d.to_string().parse::<f64>().ok()),
            adx_slope: s.adx_slope.and_then(|d| d.to_string().parse::<f64>().ok()),
            adx_regime: s.adx_regime.clone(),
            adx_di_crossover_detected: s.adx_di_crossover_detected,
            adx_di_crossover_direction: s.adx_di_crossover_direction.clone(),
        },
        None => crate::server::IndicatorSnapshot {
            rsi: None, squeeze_on: None, squeeze_momentum: None,
            squeeze_duration: None,             squeeze_release_trigger: None, squeeze_momentum_direction: None,
            chart_pattern: None, chart_pattern_confidence: None, bbwp: None,
            macd_line: None, macd_signal: None, macd_histogram: None,
            macd_histogram_trend: None, adx: None, adx_plus: None, adx_minus: None,
            bb_upper: None, bb_middle: None, bb_lower: None,
            atr: None, atr_trend: None, atr_volatility_regime: None, current_price: None,
            volume: None, average_volume: None, rvol: None,
            ema_fast: None, ema_medium: None, ema_slow: None, ema_long: None, ema_stack_state: None,
            vwap: None,
            vwap_bias: None,
            rsi_divergence_status: None,
            macd_divergence_status: None,
            macd_trend_state: None,
            macd_crossover_detected: None,
            macd_crossover_direction: None,
            macd_histogram_peak: None,
            adx_slope: None,
            adx_regime: None,
            adx_di_crossover_detected: None,
            adx_di_crossover_direction: None,
        },
    }
}
