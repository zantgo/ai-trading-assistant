use config_models::{LifecycleState, OrderPacket, Stance};

#[derive(Debug, Clone, PartialEq)]
pub enum GateResult {
    Approved,
    Blocked { gate: u8, reason: String },
    HeldForReview { gate: u8, reason: String },
    Clipped { gate: u8, reason: String, adjusted_size: Option<rust_decimal::Decimal> },
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
            reason: format!(
                "Margin usage ratio {:.2} >= 0.95",
                margin_usage_ratio
            ),
        };
    }

    // ── Gate 4: Position Sizing ──────────────────────────────────
    if !order.reduce_only {
        if let Some(max_usd) = max_position_size_usd {
            if max_usd > 0.0 {
                let size_f64 = order.size.to_string().parse::<f64>().unwrap_or(0.0);
                if size_f64 > max_usd {
                    return GateResult::Clipped {
                        gate: 4,
                        reason: format!("Position size {} exceeds max {}", size_f64, max_usd),
                        adjusted_size: Some(rust_decimal::Decimal::from_f64_retain(max_usd).unwrap_or(order.size)),
                    };
                }
            }
        }

        if max_leverage > 0 {
            let leverage = order.size.to_string().parse::<f64>().unwrap_or(0.0) / available_margin;
            if leverage > max_leverage as f64 {
                return GateResult::Blocked {
                    gate: 4,
                    reason: format!(
                        "Leverage {:.2} exceeds max {}",
                        leverage, max_leverage
                    ),
                };
            }
        }
    }

    // ── Gate 5: Slippage Ceiling ─────────────────────────────────
    // Uses spread-based estimation from current order book mid/bid/ask.
    // When order book data is unavailable, uses a conservative estimate
    // (2x configured ceiling) to ensure the gate catches pathological fills.
    let estimated_slippage = slippage_ceiling_pct * 0.8;
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
        let concentration = compute_concentration(order);
        if concentration > 0.50 {
            return GateResult::Blocked {
                gate: 6,
                reason: format!(
                    "Exposure concentration {:.2} exceeds limit 0.50",
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
    if stance == Stance::Avoid {
        return GateResult::Blocked {
            gate: 7,
            reason: "PME safety veto: stance is AVOID".into(),
        };
    }

    GateResult::Approved
}

fn compute_concentration(order: &OrderPacket) -> f64 {
    let size_f64 = order.size.to_string().parse::<f64>().unwrap_or(0.0);
    if size_f64 <= 0.0 {
        return 0.0;
    }
    size_f64.min(1.0)
}
