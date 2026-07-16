use database_storage;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCalculation {
    #[serde(with = "rust_decimal::serde::str")]
    pub risk_capital: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub price_distance: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub position_size_units: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub position_notional: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub leverage_required: Decimal,
    pub leverage_selected: i32,
    #[serde(with = "rust_decimal::serde::str")]
    pub margin_required: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub liquidation_price: Decimal,
    #[serde(with = "rust_decimal::serde::str_option", default)]
    pub risk_reward_ratio: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub estimated_profit: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_fees: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub net_pnl: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCalculationInput {
    #[serde(with = "rust_decimal::serde::str")]
    pub capital: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub max_risk_pct: Decimal,
    pub leverage: i32,
    pub direction: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub entry_price: Decimal,
    #[serde(default, with = "rust_decimal::serde::str")]
    pub stop_loss_price: Decimal,
    #[serde(default, with = "rust_decimal::serde::str")]
    pub take_profit_price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub commission_pct: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub funding_rate_8h: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub spread: Decimal,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub atr_value: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub atr_multiplier: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub atr_target_rr: Option<Decimal>,
    #[serde(default)]
    pub use_dynamic_atr: bool,
    /// Minimum order size increment (in base asset units). When provided,
    /// `position_size_units` is quantized to the nearest multiple of this tick.
    /// Required to satisfy exchange-native minimum order sizes.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub min_tick_size: Option<Decimal>,
}

pub fn compute_risk(input: &RiskCalculationInput) -> Result<RiskCalculation, String> {
    let zero = dec!(0);
    let hundred = dec!(100);

    if input.entry_price <= zero {
        return Err("Entry price must be greater than zero".into());
    }
    if input.capital <= zero {
        return Err("Capital must be greater than zero".into());
    }
    if input.stop_loss_price <= zero {
        return Err("Stop loss price must be greater than zero".into());
    }
    if input.take_profit_price <= zero {
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

    let risk_capital = input.capital * input.max_risk_pct / hundred;
    let price_distance = (input.entry_price - input.stop_loss_price).abs();

    let mut position_size_units = risk_capital / price_distance;

    if let Some(tick) = input.min_tick_size {
        if tick > zero {
            position_size_units = (position_size_units / tick).floor() * tick;
        }
    }

    let position_notional = position_size_units * input.entry_price;
    let leverage_required = if input.capital > zero {
        position_notional / input.capital
    } else {
        zero
    };

    let leverage_selected = input.leverage;
    let margin_required = if leverage_selected > 0 {
        position_notional / Decimal::from(leverage_selected)
    } else {
        position_notional
    };

    let liquidation_distance = input.entry_price / Decimal::from(leverage_selected);
    let liquidation_price = if is_long {
        input.entry_price - liquidation_distance
    } else {
        input.entry_price + liquidation_distance
    };

    let risk_reward_ratio = if risk_capital > zero {
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

    let total_fees = (input.commission_pct / hundred) * position_notional * dec!(2)
        + (input.funding_rate_8h / hundred) * position_notional
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
    profile: &database_storage::RiskProfile,
    direction: &str,
    entry_price: Decimal,
    stop_loss_price: Decimal,
    take_profit_price: Decimal,
    min_tick_size: Option<Decimal>,
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
        atr_value: None,
        atr_multiplier: None,
        atr_target_rr: None,
        use_dynamic_atr: false,
        min_tick_size,
    };
    compute_risk(&input)
}

pub fn compute_risk_with_atr(input: &RiskCalculationInput) -> Result<RiskCalculation, String> {
    let atr = input.atr_value.unwrap_or(dec!(0));
    let multiplier = input.atr_multiplier.unwrap_or(dec!(2));
    let target_rr = input.atr_target_rr.unwrap_or(dec!(2.5));
    let is_long = input.direction.to_uppercase() == "LONG";

    let (stop_loss, take_profit) = if input.use_dynamic_atr && atr > dec!(0) {
        let sl_distance = atr * multiplier;
        let tp_distance = atr * multiplier * target_rr;
        if is_long {
            (input.entry_price - sl_distance, input.entry_price + tp_distance)
        } else {
            (input.entry_price + sl_distance, input.entry_price - tp_distance)
        }
    } else {
        (input.stop_loss_price, input.take_profit_price)
    };

    let resolved_input = RiskCalculationInput {
        capital: input.capital,
        max_risk_pct: input.max_risk_pct,
        leverage: input.leverage,
        direction: input.direction.clone(),
        entry_price: input.entry_price,
        stop_loss_price: stop_loss,
        take_profit_price: take_profit,
        commission_pct: input.commission_pct,
        funding_rate_8h: input.funding_rate_8h,
        spread: input.spread,
        atr_value: input.atr_value,
        atr_multiplier: input.atr_multiplier,
        atr_target_rr: input.atr_target_rr,
        use_dynamic_atr: false,
        min_tick_size: input.min_tick_size,
    };

    compute_risk(&resolved_input)
}
