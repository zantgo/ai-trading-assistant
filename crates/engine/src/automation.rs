use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{RwLock, mpsc};
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use crate::config::{AppConfig, AutomationConfig};
use crate::db;
use crate::llm::LlmClient;
use crate::paper_trading;
use shared::normalized::NormalizedCandle;
use shared::TriggerType;

pub struct AutomationContext {
    pub pair_key: String,
    pub symbol: String,
    pub history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub config: Arc<RwLock<AppConfig>>,
    pub pool: SqlitePool,
    pub llm_client: Arc<RwLock<LlmClient>>,
    pub telemetry_tx: mpsc::Sender<db::TelemetryMsg>,
    pub cancel: CancellationToken,
    pub api_key_configured: Arc<AtomicBool>,
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
        let pair_cfg = cfg.pairs.get(&ctx.pair_key).map(|p| &p.automation);
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
            .pairs
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

        if !ctx.api_key_configured.load(std::sync::atomic::Ordering::Relaxed) {
            continue;
        }

        let remaining = state.next_remaining_secs();
        if remaining > 0 {
            continue;
        }

        let history_guard = ctx.history.read().await;
        let candle_count = history_guard.len();
        if candle_count < 10 {
            drop(history_guard);
            continue;
        }
        let prices: Vec<f64> = history_guard.iter().map(|c| {
            c.close.to_string().parse::<f64>().unwrap_or(0.0)
        }).collect();
        drop(history_guard);

        let last_close = prices.last().copied().unwrap_or(0.0);

        let llm = ctx.llm_client.read().await;
        if llm.api_key.is_empty() {
            drop(llm);
            continue;
        }

        let snapshot = db::query_latest_snapshot(&ctx.pool, &ctx.symbol).await;
        let indicators = build_indicator_snapshot(&snapshot);

        let (support_levels, resistance_levels) =
            crate::server::compute_support_resistance(&prices, last_close);

        let atr_trend = crate::server::determine_atr_trend(&ctx.pool, indicators.atr).await;

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

        let phase_one_results = crate::server::run_phase_one_agents(
            &llm,
            &indicators,
            &prices,
            &atr_trend,
            master_id,
            &ctx.telemetry_tx,
        )
        .await;

        let phase_one_json = serde_json::to_string(&phase_one_results).unwrap_or_else(|_| "[]".into());

        let phase_two = match llm.run_master_orchestrator(
            "None",
            "",
            &prices,
            &ctx.symbol,
            &phase_one_json,
            &support_levels.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            &resistance_levels.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        ).await {
            Ok(master_result) => {
                let _ = ctx.telemetry_tx.send(db::TelemetryMsg::UpdateMasterRecord {
                    master_id,
                    general_trend: master_result.general_trend.clone(),
                    support_levels: serde_json::to_string(&master_result.support_and_resistance.detected_support_levels).unwrap_or_default(),
                    resistance_levels: serde_json::to_string(&master_result.support_and_resistance.detected_resistance_levels).unwrap_or_default(),
                    indicator_synthesis_summary: master_result.indicator_synthesis.summary_count.clone(),
                    indicator_synthesis_evaluation: master_result.indicator_synthesis.evaluation.clone(),
                    recommended_action: master_result.position_recommendation.action.clone(),
                    recommendation_rationale: master_result.position_recommendation.rationale.clone(),
                }).await;
                master_result
            }
            Err(e) => {
                eprintln!(
                    "⚠️  Automation: Master orchestrator failed for {}: {}",
                    ctx.pair_key, e
                );
                let heuristic = crate::server::heuristic_master_synthesis(
                    "None",
                    &prices,
                    &indicators,
                    &support_levels,
                    &resistance_levels,
                    &phase_one_results,
                );
                let _ = ctx.telemetry_tx.send(db::TelemetryMsg::UpdateMasterRecord {
                    master_id,
                    general_trend: heuristic.general_trend.clone(),
                    support_levels: serde_json::to_string(&heuristic.support_and_resistance.detected_support_levels).unwrap_or_default(),
                    resistance_levels: serde_json::to_string(&heuristic.support_and_resistance.detected_resistance_levels).unwrap_or_default(),
                    indicator_synthesis_summary: heuristic.indicator_synthesis.summary_count.clone(),
                    indicator_synthesis_evaluation: heuristic.indicator_synthesis.evaluation.clone(),
                    recommended_action: heuristic.position_recommendation.action.clone(),
                    recommendation_rationale: heuristic.position_recommendation.rationale.clone(),
                }).await;
                heuristic
            }
        };
        drop(llm);

        println!(
            "🤖 Automation: {} analysis complete. Action: {} | Trend: {}",
            ctx.pair_key,
            phase_two.position_recommendation.action,
            phase_two.general_trend,
        );

        {
            let balance = db::paper_get_balance(&ctx.pool, &ctx.symbol).await;
            if balance.auto_execute {
                let action = phase_two.position_recommendation.action.as_str();
                let current_price = prices.last().copied().unwrap_or(0.0);

                match action {
                    "Open Long" => {
                        let res = paper_trading::verify_margin_and_open(
                            &ctx.pool, &ctx.telemetry_tx,
                            &ctx.symbol, "LONG", current_price,
                        ).await;
                        println!("📄 Auto Paper: {} {}", ctx.pair_key, res.message);
                    }
                    "Open Short" => {
                        let res = paper_trading::verify_margin_and_open(
                            &ctx.pool, &ctx.telemetry_tx,
                            &ctx.symbol, "SHORT", current_price,
                        ).await;
                        println!("📄 Auto Paper: {} {}", ctx.pair_key, res.message);
                    }
                    "Close" => {
                        let pos = db::paper_get_active_position(&ctx.pool, &ctx.symbol).await;
                        if pos.is_some() {
                            let res = paper_trading::close_paper_position(
                                &ctx.pool, &ctx.telemetry_tx,
                                &ctx.symbol, current_price, "AUTOMATED",
                            ).await;
                            println!("📄 Auto Paper: {} {}", ctx.pair_key, res.message);
                        }
                    }
                    _ => {}
                }
            }
        }

        state.last_run = Some(std::time::Instant::now());
    }

    println!("🛑 Automation Task: {} scheduler terminated.", ctx.pair_key);
}

fn build_indicator_snapshot(snapshot: &Option<shared::models::MarketSnapshot>) -> crate::server::IndicatorSnapshot {
    match snapshot {
        Some(s) => crate::server::IndicatorSnapshot {
            rsi: s.rsi_14.and_then(|d| d.to_string().parse::<f64>().ok()),
            squeeze_on: s.squeeze_on,
            squeeze_momentum: s.squeeze_momentum.and_then(|d| d.to_string().parse::<f64>().ok()),
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
            current_price: Some(s.mid_price.to_string().parse::<f64>().unwrap_or(0.0)),
            volume: s.volume.and_then(|d| d.to_string().parse::<f64>().ok()),
            average_volume: s.average_volume.and_then(|d| d.to_string().parse::<f64>().ok()),
            ema_fast: s.ema_fast.and_then(|d| d.to_string().parse::<f64>().ok()),
            ema_medium: s.ema_medium.and_then(|d| d.to_string().parse::<f64>().ok()),
            ema_slow: s.ema_slow.and_then(|d| d.to_string().parse::<f64>().ok()),
            ema_long: s.ema_long.and_then(|d| d.to_string().parse::<f64>().ok()),
            vwap: s.vwap.and_then(|d| d.to_string().parse::<f64>().ok()),
        },
        None => crate::server::IndicatorSnapshot {
            rsi: None, squeeze_on: None, squeeze_momentum: None,
            macd_line: None, macd_signal: None, macd_histogram: None,
            macd_histogram_trend: None, adx: None, adx_plus: None, adx_minus: None,
            bb_upper: None, bb_middle: None, bb_lower: None,
            atr: None, atr_trend: None, current_price: None,
            volume: None, average_volume: None,
            ema_fast: None, ema_medium: None, ema_slow: None, ema_long: None,
            vwap: None,
        },
    }
}
