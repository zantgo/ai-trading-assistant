use config_models::{LifecycleState, OrderPacket, Stance};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub enum GateResult {
    Approved,
    Blocked {
        gate: u8,
        reason: String,
    },
    HeldForReview {
        gate: u8,
        reason: String,
    },
    Clipped {
        gate: u8,
        reason: String,
        adjusted_size: Option<rust_decimal::Decimal>,
    },
}

pub fn evaluate_gates(
    order: &OrderPacket,
    lifecycle_state: LifecycleState,
    stance: Stance,
    available_margin: f64,
    margin_usage_ratio: f64,
    trade_readiness: &str,
    slippage_ceiling_pct: f64,
    max_leverage: u32,
    max_position_size_usd: Option<f64>,
    bid_price: Option<Decimal>,
    ask_price: Option<Decimal>,
    active_position_count: usize,
    total_equity: f64,
    safety_state: &str,
) -> GateResult {
    let is_exit = order.reduce_only || order.is_emergency_liquidation;
    let is_emergency = order.is_emergency_liquidation;

    // ── Gate 0: Lifecycle ────────────────────────────────────────
    if !is_exit && lifecycle_state != LifecycleState::Running {
        return GateResult::Blocked {
            gate: 0,
            reason: format!(
                "Lifecycle state is {}, not RUNNING",
                lifecycle_state.as_str()
            ),
        };
    }

    // ── Gate 1: Symbol Stance ────────────────────────────────────
    if !is_emergency && stance == Stance::Avoid {
        return GateResult::Blocked {
            gate: 1,
            reason: "Symbol stance is AVOID".into(),
        };
    }

    if !order.reduce_only && stance == Stance::CloseOnly {
        return GateResult::Blocked {
            gate: 1,
            reason: "Symbol stance is CLOSE_ONLY and order is not reduce_only".into(),
        };
    }

    // Emergency orders skip Gates 1 (checked above with bypass), 2, 4, 5, 6, 7
    // but NOT Gate 3 (capital). Per 08-02 §3.1.
    if is_emergency {
        // ── Gate 3: Capital (still applies for emergency) ─────────
        if available_margin <= 0.0 {
            return GateResult::Blocked {
                gate: 3,
                reason: "Insufficient margin for emergency liquidation".into(),
            };
        }
        return GateResult::Approved;
    }

    // ── Gate 2: Decision Guard (trade readiness) ─────────────────
    match trade_readiness {
        "STAND_ASIDE" => {
            return GateResult::HeldForReview {
                gate: 2,
                reason: "Trade readiness is STAND_ASIDE".into(),
            };
        }
        "WATCH" => {
            eprintln!("TAE: Gate 2 soft warning — trade_readiness=WATCH, continuing");
        }
        _ => {}
    }

    // ── Gate 3: Capital Query ─────────────────────────────────────
    if available_margin <= 0.0 {
        return GateResult::Blocked {
            gate: 3,
            reason: "Insufficient available margin".into(),
        };
    }
    if margin_usage_ratio >= 0.95 {
        return GateResult::Blocked {
            gate: 3,
            reason: format!("Margin usage ratio {:.2} >= 0.95", margin_usage_ratio),
        };
    }

    // ── Gate 4: Position Sizing ──────────────────────────────────
    if !order.reduce_only {
        if let Some(max_usd) = max_position_size_usd {
            if max_usd > 0.0 {
                let size_f64 = order.size.to_f64().unwrap_or(0.0);
                if size_f64 > max_usd {
                    return GateResult::Clipped {
                        gate: 4,
                        reason: format!("Position size {} exceeds max {}", size_f64, max_usd),
                        adjusted_size: Some(
                            rust_decimal::Decimal::from_f64_retain(max_usd).unwrap_or(order.size),
                        ),
                    };
                }
            }
        }

        if max_leverage > 0 {
            let leverage = if available_margin > 0.0 {
                order.size.to_f64().unwrap_or(0.0) / available_margin
            } else {
                f64::MAX
            };
            if leverage > max_leverage as f64 {
                return GateResult::Blocked {
                    gate: 4,
                    reason: format!("Leverage {:.2} exceeds max {}", leverage, max_leverage),
                };
            }
        }
    }

    // ── Gate 5: Slippage Ceiling ─────────────────────────────────
    let estimated_slippage = compute_estimated_slippage(order, bid_price, ask_price);
    if estimated_slippage > slippage_ceiling_pct {
        return GateResult::HeldForReview {
            gate: 5,
            reason: format!(
                "Estimated slippage {:.2}% exceeds ceiling {:.2}%",
                estimated_slippage, slippage_ceiling_pct
            ),
        };
    }

    // ── Gate 6: Exposure Concentration ───────────────────────────
    if !order.reduce_only {
        let concentration = compute_concentration(order, active_position_count, total_equity);
        if concentration > 0.50 {
            return GateResult::Blocked {
                gate: 6,
                reason: format!(
                    "Portfolio exposure concentration {:.2} exceeds limit 0.50",
                    concentration
                ),
            };
        }
        if concentration > 0.20 {
            eprintln!(
                "TAE: Gate 6 warning — single-pair concentration {:.2} > 0.20",
                concentration
            );
        }
    }

    // ── Gate 7: PME Safety Veto ──────────────────────────────────
    // Fresh re-validation against PME authority (per 08-02 §3: Gate 7 is the
    // active-veto check that re-validates against most recent PME authority,
    // catching veto events that may have raced with the policy trigger).
    match safety_state {
        "DRAWDOWN_STOP" => {
            return GateResult::Blocked {
                gate: 7,
                reason: "PME safety veto: capital drawdown limit exceeded".into(),
            };
        }
        "SUSPENDED" => {
            return GateResult::Blocked {
                gate: 7,
                reason: "PME safety veto: trading suspended (loss streak)".into(),
            };
        }
        _ => {}
    }

    if stance == Stance::Avoid {
        return GateResult::Blocked {
            gate: 7,
            reason: "PME safety veto: stance is AVOID".into(),
        };
    }

    GateResult::Approved
}

fn compute_estimated_slippage(
    order: &OrderPacket,
    bid_price: Option<Decimal>,
    ask_price: Option<Decimal>,
) -> f64 {
    match (bid_price, ask_price) {
        (Some(bid), Some(ask)) if bid > Decimal::ZERO && ask > Decimal::ZERO => {
            let spread = ask - bid;
            let mid = (bid + ask) / Decimal::from(2);
            if mid > Decimal::ZERO {
                let spread_pct = (spread / mid * Decimal::from(100)).to_f64().unwrap_or(1.0);
                let order_size_f64 = order.size.to_f64().unwrap_or(0.0);
                if order_size_f64 > 0.0 {
                    spread_pct * (1.0 + (order_size_f64 / 100_000.0).min(5.0))
                } else {
                    spread_pct
                }
            } else {
                1.0
            }
        }
        _ => {
            let order_size_f64 = order.size.to_f64().unwrap_or(0.0);
            if order_size_f64 > 10_000.0 {
                0.5
            } else {
                0.1
            }
        }
    }
}

fn compute_concentration(
    order: &OrderPacket,
    active_position_count: usize,
    total_equity: f64,
) -> f64 {
    let order_size_f64 = order.size.to_f64().unwrap_or(0.0);
    if order_size_f64 <= 0.0 || total_equity <= 0.0 {
        return 0.0;
    }
    let base_concentration = order_size_f64 / total_equity;
    let position_multiplier = 1.0 + (active_position_count as f64 * 0.15);
    (base_concentration * position_multiplier).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use config_models::{OrderSide, OrderType};
    use rust_decimal_macros::dec;

    fn make_order(reduce_only: bool, is_emergency: bool, size: Decimal) -> OrderPacket {
        OrderPacket {
            client_order_id: "test".into(),
            symbol: "BTC-USDT".into(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Some(dec!(50000)),
            size,
            reduce_only,
            is_emergency_liquidation: is_emergency,
            associated_position_id: None,
        }
    }

    #[test]
    fn test_gate0_blocks_entry_when_not_running() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::LifecyclePaused,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Blocked { gate: 0, .. }));
    }

    #[test]
    fn test_gate0_allows_exit_when_paused() {
        let order = make_order(true, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::LifecyclePaused,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Approved));
    }

    #[test]
    fn test_gate1_blocks_avoid_stance() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Avoid,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Blocked { gate: 1, .. }));
    }

    #[test]
    fn test_gate1_allows_emergency_during_avoid() {
        let order = make_order(false, true, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Avoid,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Approved));
    }

    #[test]
    fn test_gate2_holds_stand_aside() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "STAND_ASIDE",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::HeldForReview { gate: 2, .. }));
    }

    #[test]
    fn test_gate3_blocks_margin_usage_ratio_high() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            100.0,
            0.96,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Blocked { gate: 3, .. }));
    }

    #[test]
    fn test_gate3_allows_margin_usage_ratio_ok() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            5000.0,
            0.5,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Approved));
    }

    #[test]
    fn test_gate4_clips_excessive_size() {
        let order = make_order(false, false, dec!(2000));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            Some(500.0),
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Clipped { gate: 4, .. }));
    }

    #[test]
    fn test_gate5_computes_real_spread() {
        let order = make_order(false, false, dec!(100));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.01,
            20,
            None,
            Some(dec!(49950)),
            Some(dec!(50050)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::HeldForReview { gate: 5, .. }));
    }

    #[test]
    fn test_gate5_approves_normal_spread() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Approved));
    }

    #[test]
    fn test_gate6_blocks_high_concentration() {
        let order = make_order(false, false, dec!(7000));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Blocked { gate: 6, .. }));
    }

    #[test]
    fn test_gate6_skips_reduce_only() {
        let order = make_order(true, false, dec!(7000));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Approved));
    }

    #[test]
    fn test_gate7_blocks_drawdown_stop() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "DRAWDOWN_STOP",
        );
        assert!(matches!(result, GateResult::Blocked { gate: 7, .. }));
    }

    #[test]
    fn test_gate7_blocks_suspended() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "SUSPENDED",
        );
        assert!(matches!(result, GateResult::Blocked { gate: 7, .. }));
    }

    #[test]
    fn test_gate7_allows_normal_safety() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert!(matches!(result, GateResult::Approved));
    }

    #[test]
    fn test_gate7_receives_safety_state_as_str() {
        // Audit regression (M1): the daemon previously forwarded
        // `format!("{:?}", state)` (PascalCase "DrawdownStop"), which Gate 7
        // never matched — the direct safety-state guard was dead code. The
        // canonical `SafetyState::as_str()` (SCREAMING_SNAKE) must feed the
        // gate and block.
        use core_domain::portfolio::SafetyState;
        let order = make_order(false, false, dec!(1));
        for state in [SafetyState::DrawdownStop, SafetyState::Suspended] {
            let result = evaluate_gates(
                &order,
                LifecycleState::Running,
                Stance::Active,
                10000.0,
                0.1,
                "READY",
                0.5,
                20,
                None,
                Some(dec!(49999)),
                Some(dec!(50001)),
                0,
                10000.0,
                state.as_str(),
            );
            assert!(
                matches!(result, GateResult::Blocked { gate: 7, .. }),
                "Gate 7 must block on {} (as_str = {})",
                state.as_str(),
                state.as_str(),
            );
        }
        // The Debug spelling must NOT be what the gate consumes — pin that
        // the canonical wire form differs from `{:?}`.
        assert_ne!(format!("{:?}", SafetyState::DrawdownStop), "DRAWDOWN_STOP");
    }

    #[test]
    fn test_full_approval_path() {
        let order = make_order(false, false, dec!(1));
        let result = evaluate_gates(
            &order,
            LifecycleState::Running,
            Stance::Active,
            10000.0,
            0.1,
            "READY",
            0.5,
            20,
            None,
            Some(dec!(49999)),
            Some(dec!(50001)),
            0,
            10000.0,
            "NORMAL",
        );
        assert_eq!(result, GateResult::Approved);
    }
}
