use core_domain::portfolio::{CapitalMatrix, PositionMatrix};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub fn available_margin(
    initial_balance: Decimal,
    realized_pnl: Decimal,
    unrealized_pnl: Decimal,
    committed_margin: Decimal,
) -> Decimal {
    let unrealized_deduction = if unrealized_pnl < dec!(0) {
        unrealized_pnl
    } else {
        dec!(0)
    };

    (initial_balance + realized_pnl + unrealized_deduction) - committed_margin
}

pub fn margin_required(position_notional: Decimal, cross_leverage: Decimal) -> Decimal {
    if cross_leverage > dec!(0) {
        position_notional / cross_leverage
    } else {
        position_notional
    }
}

pub fn compute_trading_fees(
    position_notional: Decimal,
    maker_fee_pct: f64,
    taker_fee_pct: f64,
) -> Decimal {
    let fee_rate =
        Decimal::from_f64_retain((maker_fee_pct + taker_fee_pct) / 100.0).unwrap_or(dec!(0));
    position_notional * fee_rate
}

pub fn compute_funding_cost(
    position_notional: Decimal,
    funding_rate_8h: f64,
    funding_intervals: u32,
) -> Decimal {
    let rate = Decimal::from_f64_retain(funding_rate_8h / 100.0).unwrap_or(dec!(0));
    if funding_intervals > 0 {
        position_notional * rate * Decimal::from(funding_intervals)
    } else {
        position_notional * rate
    }
}

pub fn compute_capital_matrix(
    initial_balance: Decimal,
    realized_pnl: Decimal,
    positions: &[PositionMatrix],
    cross_leverage: Decimal,
    starting_session_equity: Decimal,
    daily_pnl: Decimal,
    max_daily_drawdown_pct: Decimal,
) -> CapitalMatrix {
    let unrealized_pnl: Decimal = positions.iter().map(|p| p.unrealized_pnl).sum();
    let committed_margin: Decimal = positions
        .iter()
        .map(|p| margin_required(p.allocated_usd, cross_leverage))
        .sum();

    let current_equity = initial_balance + realized_pnl + unrealized_pnl;
    let available = available_margin(initial_balance, realized_pnl, unrealized_pnl, committed_margin);

    let margin_usage_ratio = if current_equity > dec!(0) {
        committed_margin / current_equity
    } else {
        dec!(0)
    };

    let leverage_ratio = if current_equity > dec!(0) {
        let gross_exposure: Decimal = positions.iter().map(|p| p.allocated_usd).sum();
        gross_exposure / current_equity
    } else {
        dec!(0)
    };

    CapitalMatrix {
        initial_balance,
        current_equity,
        available_margin: available,
        committed_margin,
        realized_pnl,
        unrealized_pnl,
        margin_usage_ratio,
        leverage_ratio,
        max_daily_drawdown_pct,
        daily_pnl,
        starting_session_equity,
    }
}

#[derive(Debug, Clone)]
pub enum MarginAlert {
    Warning,
    CloseOnly,
    Emergency,
}

pub fn check_margin_alerts(margin_usage_ratio: Decimal) -> Option<MarginAlert> {
    if margin_usage_ratio >= dec!(1) {
        Some(MarginAlert::Emergency)
    } else if margin_usage_ratio >= dec!(0.95) {
        Some(MarginAlert::CloseOnly)
    } else if margin_usage_ratio >= dec!(0.80) {
        Some(MarginAlert::Warning)
    } else {
        None
    }
}
