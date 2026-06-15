use crate::config::FeesConfig;

#[derive(Debug, Clone, serde::Serialize)]
pub struct FeeTableRow {
    pub exchange_fee_pct: f64,
    pub leverage: u32,
    pub capital: f64,
    pub min_profit_pct_to_cover_fees: f64,
    pub fees_in_dollars: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FeeBreakdown {
    pub maker_fee_pct: f64,
    pub taker_fee_pct: f64,
    pub order_type: String,
    pub effective_fee_pct: f64,
    pub entry_1_fees: f64,
    pub entry_2_fees: f64,
    pub total_fees: f64,
    pub funding_rate_8h: f64,
    pub funding_cost: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EntryMetrics {
    pub entry_number: u8,
    pub entry_price: f64,
    pub stop_loss_price: f64,
    pub take_profit_price: f64,
    pub capital_allocated: f64,
    pub capital_pct: f64,
    pub position_size_units: f64,
    pub position_notional: f64,
    pub margin_required: f64,
    pub risk_amount: f64,
    pub potential_profit: f64,
    pub fees: f64,
    pub net_profit: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommissionProjection {
    pub direction: String,
    pub leverage: i32,
    pub total_capital: f64,
    pub total_position_notional: f64,
    pub total_margin_required: f64,
    pub weighted_avg_entry: f64,
    pub effective_stop_loss: f64,
    pub effective_take_profit: f64,
    pub total_risk_amount: f64,
    pub fee_breakdown: FeeBreakdown,
    pub entry_1: EntryMetrics,
    pub entry_2: EntryMetrics,
    pub max_gain_scenario: f64,
    pub max_loss_scenario: f64,
    pub max_gain_net_after_fees: f64,
    pub max_loss_net_after_fees: f64,
    pub trade_viable: bool,
    pub viability_reason: String,
    pub min_profit_pct_to_cover_fees: f64,
    pub required_price_move_pct: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CommissionProjectionRequest {
    pub direction: String,
    pub entry_1: f64,
    pub entry_2: f64,
    pub stop_loss_1: f64,
    pub stop_loss_2: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,
    pub capital: f64,
    pub leverage: i32,
    pub max_risk_pct: f64,
    pub capital_entry_1_pct: f64,
    #[serde(default = "default_order_type")]
    pub order_type: String,
    pub commission_pct: Option<f64>,
    pub funding_rate_8h: Option<f64>,
}

fn default_order_type() -> String {
    "taker".to_string()
}

pub fn generate_fee_table(
    config: &FeesConfig,
    leverages: &[u32],
    capitals: &[f64],
    order_type: &str,
) -> Vec<FeeTableRow> {
    let effective_fee = if order_type == "maker" {
        config.maker_fee_pct
    } else {
        config.taker_fee_pct
    };

    let mut rows = Vec::new();
    for &leverage in leverages {
        for &capital in capitals {
            let position_notional = capital * leverage as f64;
            let fees_in_dollars = (effective_fee / 100.0) * position_notional * 2.0;
            let min_profit_pct = if capital > 0.0 {
                (fees_in_dollars / capital) * 100.0
            } else {
                0.0
            };
            rows.push(FeeTableRow {
                exchange_fee_pct: effective_fee,
                leverage,
                capital,
                min_profit_pct_to_cover_fees: (min_profit_pct * 100.0).round() / 100.0,
                fees_in_dollars: (fees_in_dollars * 100.0).round() / 100.0,
            });
        }
    }
    rows
}

pub fn compute_commission_projection(
    input: &CommissionProjectionRequest,
    fees_config: &FeesConfig,
) -> Result<CommissionProjection, String> {
    if input.entry_1 <= 0.0 || input.entry_2 <= 0.0 {
        return Err("Entry prices must be greater than zero".into());
    }
    if input.capital <= 0.0 {
        return Err("Capital must be greater than zero".into());
    }
    if input.leverage <= 0 {
        return Err("Leverage must be greater than zero".into());
    }
    if input.capital_entry_1_pct < 0.0 || input.capital_entry_1_pct > 100.0 {
        return Err("Capital allocation for Entry 1 must be between 0 and 100".into());
    }

    let is_long = input.direction.to_uppercase() == "LONG";
    if is_long {
        if input.stop_loss_1 >= input.entry_1 || input.stop_loss_2 >= input.entry_2 {
            return Err("For LONG, stop losses must be below entry prices".into());
        }
        if input.take_profit_1 <= input.entry_1 || input.take_profit_2 <= input.entry_2 {
            return Err("For LONG, take profits must be above entry prices".into());
        }
    } else {
        if input.stop_loss_1 <= input.entry_1 || input.stop_loss_2 <= input.entry_2 {
            return Err("For SHORT, stop losses must be above entry prices".into());
        }
        if input.take_profit_1 >= input.entry_1 || input.take_profit_2 >= input.entry_2 {
            return Err("For SHORT, take profits must be below entry prices".into());
        }
    }

    let cap_1 = input.capital * (input.capital_entry_1_pct / 100.0);
    let cap_2 = input.capital - cap_1;

    let order_type = if input.order_type == "maker" { "maker" } else { "taker" };
    let effective_fee_pct = if order_type == "maker" {
        fees_config.maker_fee_pct
    } else {
        fees_config.taker_fee_pct
    };
    let commission_override = input.commission_pct.unwrap_or(effective_fee_pct);
    let funding_rate = input.funding_rate_8h.unwrap_or(0.0);

    let entry_1 = compute_entry_metrics(
        1, input.entry_1, input.stop_loss_1, input.take_profit_1,
        cap_1, input.leverage, input.max_risk_pct, commission_override,
        funding_rate, is_long,
    )?;

    let entry_2 = compute_entry_metrics(
        2, input.entry_2, input.stop_loss_2, input.take_profit_2,
        cap_2, input.leverage, input.max_risk_pct, commission_override,
        funding_rate, is_long,
    )?;

    let total_notional = entry_1.position_notional + entry_2.position_notional;
    let total_margin = entry_1.margin_required + entry_2.margin_required;
    let total_cap = cap_1 + cap_2;

    let weighted_avg_entry = if total_notional > 0.0 {
        (entry_1.entry_price * entry_1.position_notional + entry_2.entry_price * entry_2.position_notional) / total_notional
    } else {
        0.0
    };

    let effective_sl = if total_notional > 0.0 {
        (input.stop_loss_1 * entry_1.position_notional + input.stop_loss_2 * entry_2.position_notional) / total_notional
    } else {
        0.0
    };

    let effective_tp = if total_notional > 0.0 {
        (input.take_profit_1 * entry_1.position_notional + input.take_profit_2 * entry_2.position_notional) / total_notional
    } else {
        0.0
    };

    let total_risk = entry_1.risk_amount + entry_2.risk_amount;

    let total_fees = entry_1.fees + entry_2.fees;
    let entry1_funding = (funding_rate / 100.0) * entry_1.position_notional;
    let entry2_funding = (funding_rate / 100.0) * entry_2.position_notional;
    let total_funding = entry1_funding + entry2_funding;

    let fee_breakdown = FeeBreakdown {
        maker_fee_pct: fees_config.maker_fee_pct,
        taker_fee_pct: fees_config.taker_fee_pct,
        order_type: order_type.to_string(),
        effective_fee_pct: commission_override,
        entry_1_fees: entry_1.fees,
        entry_2_fees: entry_2.fees,
        total_fees,
        funding_rate_8h: funding_rate,
        funding_cost: total_funding,
    };

    let max_gain = entry_1.potential_profit + entry_2.potential_profit;
    let max_loss = entry_1.risk_amount + entry_2.risk_amount;

    let all_in_costs = total_fees + total_funding;
    let max_gain_net = max_gain - all_in_costs;
    let max_loss_net = max_loss + all_in_costs;

    let min_profit_to_cover = if input.capital > 0.0 {
        (all_in_costs / input.capital) * 100.0
    } else {
        0.0
    };

    let required_move = if total_notional > 0.0 {
        (all_in_costs / total_notional) * 100.0
    } else {
        0.0
    };

    let (trade_viable, viability_reason) = if max_gain_net <= 0.0 {
        (
            false,
            format!(
                "Trade is NOT viable: Maximum net gain (${:.2}) is negative or zero after ${:.2} in fees. The projected profit does not cover the round-trip commission costs.",
                max_gain_net, all_in_costs
            ),
        )
    } else if max_gain_net < all_in_costs * 0.5 {
        (
            true,
            format!(
                "Trade is MARGINALLY viable: Net gain (${:.2}) barely exceeds fees (${:.2}). Consider widening take-profit targets or reducing position size.",
                max_gain_net, all_in_costs
            ),
        )
    } else {
        (
            true,
            format!(
                "Trade is VIABLE: Projected net gain of ${:.2} comfortably exceeds ${:.2} in total fees.",
                max_gain_net, all_in_costs
            ),
        )
    };

    Ok(CommissionProjection {
        direction: input.direction.clone(),
        leverage: input.leverage,
        total_capital: total_cap,
        total_position_notional: total_notional,
        total_margin_required: total_margin,
        weighted_avg_entry,
        effective_stop_loss: effective_sl,
        effective_take_profit: effective_tp,
        total_risk_amount: total_risk,
        fee_breakdown,
        entry_1,
        entry_2,
        max_gain_scenario: max_gain,
        max_loss_scenario: max_loss,
        max_gain_net_after_fees: max_gain_net,
        max_loss_net_after_fees: max_loss_net,
        trade_viable,
        viability_reason,
        min_profit_pct_to_cover_fees: (min_profit_to_cover * 100.0).round() / 100.0,
        required_price_move_pct: (required_move * 100.0).round() / 100.0,
    })
}

fn compute_entry_metrics(
    entry_number: u8,
    entry_price: f64,
    stop_loss: f64,
    take_profit: f64,
    capital: f64,
    leverage: i32,
    max_risk_pct: f64,
    commission_pct: f64,
    funding_rate_8h: f64,
    is_long: bool,
) -> Result<EntryMetrics, String> {
    let risk_capital = capital * (max_risk_pct / 100.0);
    let price_distance = (entry_price - stop_loss).abs();
    if price_distance <= 0.0 {
        return Err(format!(
            "Entry {}: Stop loss distance is zero — stop loss must not equal entry price",
            entry_number
        ));
    }

    let position_size_units = risk_capital / price_distance;
    let position_notional = position_size_units * entry_price;
    let margin_required = if leverage > 0 {
        position_notional / leverage as f64
    } else {
        position_notional
    };

    let risk_amount = (entry_price - stop_loss).abs() * position_size_units;

    let potential_profit = if is_long {
        (take_profit - entry_price) * position_size_units
    } else {
        (entry_price - take_profit) * position_size_units
    };

    let fees = (commission_pct / 100.0) * position_notional * 2.0
        + (funding_rate_8h / 100.0) * position_notional;

    let net_profit = potential_profit - fees;

    Ok(EntryMetrics {
        entry_number,
        entry_price,
        stop_loss_price: stop_loss,
        take_profit_price: take_profit,
        capital_allocated: capital,
        capital_pct: 0.0,
        position_size_units,
        position_notional,
        margin_required,
        risk_amount,
        potential_profit,
        fees,
        net_profit,
    })
}

pub fn is_trade_viable(projection: &CommissionProjection) -> bool {
    projection.trade_viable && projection.max_gain_net_after_fees > 0.0
}
