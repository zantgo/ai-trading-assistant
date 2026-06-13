use crate::db;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RiskCalculation {
    pub risk_capital: f64,
    pub price_distance: f64,
    pub position_size_units: f64,
    pub position_notional: f64,
    pub leverage_required: f64,
    pub leverage_selected: i32,
    pub margin_required: f64,
    pub liquidation_price: f64,
    pub risk_reward_ratio: Option<f64>,
    pub estimated_profit: f64,
    pub total_fees: f64,
    pub net_pnl: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RiskCalculationInput {
    pub capital: f64,
    pub max_risk_pct: f64,
    pub leverage: i32,
    pub direction: String,
    pub entry_price: f64,
    pub stop_loss_price: f64,
    pub take_profit_price: f64,
    pub commission_pct: f64,
    pub funding_rate_8h: f64,
    pub spread: f64,
}

pub fn compute_risk(input: &RiskCalculationInput) -> Result<RiskCalculation, String> {
    if input.entry_price <= 0.0 {
        return Err("Entry price must be greater than zero".into());
    }
    if input.capital <= 0.0 {
        return Err("Capital must be greater than zero".into());
    }
    if input.stop_loss_price <= 0.0 {
        return Err("Stop loss price must be greater than zero".into());
    }
    if input.take_profit_price <= 0.0 {
        return Err("Take profit price must be greater than zero".into());
    }

    let is_long = input.direction.to_uppercase() == "LONG";
    if is_long {
        if input.stop_loss_price >= input.entry_price {
            return Err("For LONG position, stop loss must be below entry price".into());
        }
        if input.take_profit_price <= input.entry_price {
            return Err("For LONG position, take profit must be above entry price".into());
        }
    } else {
        if input.stop_loss_price <= input.entry_price {
            return Err("For SHORT position, stop loss must be above entry price".into());
        }
        if input.take_profit_price >= input.entry_price {
            return Err("For SHORT position, take profit must be below entry price".into());
        }
    }

    let risk_capital = input.capital * (input.max_risk_pct / 100.0);
    let price_distance = (input.entry_price - input.stop_loss_price).abs();
    let position_size_units = risk_capital / price_distance;

    let position_notional = position_size_units * input.entry_price;
    let leverage_required = if input.capital > 0.0 {
        position_notional / input.capital
    } else {
        0.0
    };

    let leverage_selected = input.leverage;
    let margin_required = if leverage_selected > 0 {
        position_notional / leverage_selected as f64
    } else {
        position_notional
    };

    let liquidation_distance = if is_long {
        input.entry_price / (leverage_selected as f64)
    } else {
        input.entry_price / (leverage_selected as f64)
    };
    let liquidation_price = if is_long {
        input.entry_price - liquidation_distance
    } else {
        input.entry_price + liquidation_distance
    };

    let risk_reward_ratio = if risk_capital > 0.0 {
        let profit_distance = (input.take_profit_price - input.entry_price).abs();
        let potential_profit = profit_distance * position_size_units;
        Some(potential_profit / risk_capital)
    } else {
        None
    };

    let estimated_profit = if is_long {
        (input.take_profit_price - input.entry_price) * position_size_units
    } else {
        (input.entry_price - input.take_profit_price) * position_size_units
    };

    let total_fees = (input.commission_pct / 100.0) * position_notional * 2.0
        + (input.funding_rate_8h / 100.0) * position_notional
        + input.spread;

    let net_pnl = estimated_profit - total_fees;

    Ok(RiskCalculation {
        risk_capital,
        price_distance,
        position_size_units,
        position_notional,
        leverage_required,
        leverage_selected,
        margin_required,
        liquidation_price,
        risk_reward_ratio,
        estimated_profit,
        total_fees,
        net_pnl,
    })
}

pub fn compute_risk_from_profile(
    profile: &db::RiskProfile,
    direction: &str,
    entry_price: f64,
    stop_loss_price: f64,
    take_profit_price: f64,
) -> Result<RiskCalculation, String> {
    let input = RiskCalculationInput {
        capital: profile.capital,
        max_risk_pct: profile.max_risk_pct,
        leverage: profile.leverage,
        direction: direction.to_string(),
        entry_price,
        stop_loss_price,
        take_profit_price,
        commission_pct: profile.commission_pct,
        funding_rate_8h: profile.funding_rate_8h,
        spread: profile.spread,
    };
    compute_risk(&input)
}
