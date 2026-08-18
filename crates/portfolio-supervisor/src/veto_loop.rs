use core_domain::overview::OverviewMatrix;
use core_domain::portfolio::PositionMatrix;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::capital_layer;
use crate::exposure_layer;
use crate::paper_trading::PaperTradingEngine;
use crate::position_layer;
use crate::WorkspaceState;
use config_models::Stance;

#[derive(Debug, Clone)]
pub struct VetoEvent {
    pub instance_id: String,
    pub symbol: String,
    pub target_stance: Stance,
    pub reason: String,
    pub hard_exit: bool,
    pub timestamp_ms: u64,
}

pub fn spawn_veto_loop(
    workspace_state: WorkspaceState,
    paper_engine: Arc<PaperTradingEngine>,
    overview: Arc<RwLock<Option<OverviewMatrix>>>,
    tx: mpsc::Sender<VetoEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;

            let instances = workspace_state.list().await;
            let paper_positions = paper_engine.positions.read().await;
            let paper_equity_val = *paper_engine.equity.read().await;
            let systemic_risk_score = overview
                .read()
                .await
                .as_ref()
                .map(|o| o.systemic_risk_score)
                .unwrap_or(0.0);
            let equity_dec = paper_equity_val;

            for inst in &instances {
                if inst.cancel.is_cancelled() {
                    continue;
                }

                let symbol = inst.symbol();

                let paper_pos = paper_positions.get(&symbol);
                let positions: Vec<PositionMatrix> = if let Some(pos) = paper_pos {
                    let latest_price_val = inst.latest_price().await.unwrap_or(0.0);
                    let price_dec = Decimal::from_f64_retain(latest_price_val).unwrap_or(dec!(0));

                    let direction_str = match pos.direction {
                        config_models::Direction::Long => "Long",
                        config_models::Direction::Short => "Short",
                    };

                    vec![position_layer::compute_position_matrix_with_config(
                        &symbol,
                        direction_str,
                        pos.entry_price,
                        pos.size,
                        price_dec,
                        0,
                        0,
                        paper_engine.fee_config.maker_fee_pct,
                        paper_engine.fee_config.taker_fee_pct,
                    )]
                } else {
                    vec![]
                };

                let exposure = exposure_layer::compute_exposure_matrix(&positions, equity_dec);

                let cross_leverage = Decimal::from(20u32);
                let initial_cap =
                    Decimal::from_f64_retain(inst.trading.read().await.initial_capital)
                        .unwrap_or(dec!(0));
                let capital = capital_layer::compute_capital_matrix(
                    initial_cap,
                    dec!(0),
                    &positions,
                    cross_leverage,
                    equity_dec,
                    dec!(0),
                    Decimal::from_f64_retain(inst.safety_config.max_daily_drawdown_pct)
                        .unwrap_or(dec!(5)),
                );

                inst.safety.set_current_equity(equity_dec).await;
                inst.safety.evaluate_daily_drawdown_warn().await;

                let exposure_limits = exposure_layer::ConcentrationLimits::default();
                let exposure_breached = if !positions.is_empty() {
                    exposure_layer::validate_concentration(
                        &symbol,
                        dec!(0),
                        equity_dec,
                        &exposure,
                        &exposure_limits,
                    )
                    .is_err()
                } else {
                    false
                };

                let mut triggers = inst
                    .safety
                    .evaluate_all(
                        &symbol,
                        capital.margin_usage_ratio,
                        systemic_risk_score,
                        exposure_breached,
                    )
                    .await;

                for pos in &positions {
                    if let Some(latest_close) = inst
                        .latest_close_str()
                        .await
                        .and_then(|s| s.parse::<f64>().ok())
                    {
                        let close_dec = Decimal::from_f64_retain(latest_close).unwrap_or(dec!(0));
                        if let Some(liquidate) =
                            position_layer::check_invalidation_breach(pos, close_dec)
                        {
                            triggers.push(core_domain::portfolio::VetoTrigger {
                                condition: "invalidation_breach".into(),
                                target_stance: "AVOID".into(),
                                reason: liquidate.reason,
                                hard_exit: true,
                            });
                        }
                    }
                }

                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                for trigger in triggers {
                    let target_stance = match trigger.target_stance.as_str() {
                        "AVOID" => Stance::Avoid,
                        "CLOSE_ONLY" => Stance::CloseOnly,
                        _ => continue,
                    };

                    let event = VetoEvent {
                        instance_id: inst.id.clone(),
                        symbol: symbol.clone(),
                        target_stance,
                        reason: trigger.reason,
                        hard_exit: trigger.hard_exit,
                        timestamp_ms: now_ms,
                    };

                    eprintln!(
                        "🛑 PME VETO: {} (instance={}) {} — reason: {}",
                        symbol, inst.id, trigger.condition, event.reason
                    );

                    if tx.send(event).await.is_err() {
                        eprintln!("PME: veto channel closed, stopping veto loop");
                        return;
                    }
                }
            }
        }
    })
}
