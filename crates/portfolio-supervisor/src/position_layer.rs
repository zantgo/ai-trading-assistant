use core_domain::portfolio::{PositionMatrix, PositionState};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub fn compute_position_matrix(
    symbol: &str,
    direction: &str,
    entry_price: Decimal,
    size: Decimal,
    current_price: Decimal,
    entry_timestamp: u64,
    position_id: u64,
) -> PositionMatrix {
    compute_position_matrix_with_config(
        symbol,
        direction,
        entry_price,
        size,
        current_price,
        entry_timestamp,
        position_id,
        0.02,
        0.06,
    )
}

pub fn compute_position_matrix_with_config(
    symbol: &str,
    direction: &str,
    entry_price: Decimal,
    size: Decimal,
    current_price: Decimal,
    entry_timestamp: u64,
    position_id: u64,
    maker_fee_pct: f64,
    taker_fee_pct: f64,
) -> PositionMatrix {
    let direction_sign = match direction {
        "LONG" | "Long" => dec!(1),
        "SHORT" | "Short" => dec!(-1),
        _ => dec!(1),
    };

    let unrealized_pnl = direction_sign * (current_price - entry_price) * size;
    let allocated_usd = entry_price * size;
    let roi_pct = if allocated_usd > dec!(0) {
        (unrealized_pnl / allocated_usd) * dec!(100)
    } else {
        dec!(0)
    };

    let fee_pct =
        Decimal::from_f64_retain((maker_fee_pct + taker_fee_pct) / 100.0).unwrap_or(dec!(0));
    let unrealized_pnl_after_fees = unrealized_pnl - (allocated_usd * fee_pct);

    PositionMatrix {
        position_id,
        symbol: symbol.to_string(),
        direction: direction.to_string(),
        entry_price,
        size,
        allocated_usd,
        entry_timestamp,
        current_price,
        unrealized_pnl,
        roi_pct,
        unrealized_pnl_after_fees,
        stop_loss_price: None,
        take_profit_price: None,
        invalidation_level: None,
        target_profit_ratio: None,
        position_state: PositionState::Opening,
    }
}

pub fn mark_to_market(
    positions: &[PositionMatrix],
    mark_prices: &std::collections::HashMap<String, Decimal>,
) -> Vec<PositionMatrix> {
    positions
        .iter()
        .map(|pos| {
            let current_price = mark_prices
                .get(&pos.symbol)
                .copied()
                .unwrap_or(pos.current_price);

            let direction_sign = match pos.direction.as_str() {
                "LONG" | "Long" => dec!(1),
                "SHORT" | "Short" => dec!(-1),
                _ => dec!(1),
            };

            let unrealized_pnl = direction_sign * (current_price - pos.entry_price) * pos.size;
            let roi_pct = if pos.allocated_usd > dec!(0) {
                (unrealized_pnl / pos.allocated_usd) * dec!(100)
            } else {
                dec!(0)
            };

            PositionMatrix {
                current_price,
                unrealized_pnl,
                roi_pct,
                unrealized_pnl_after_fees: unrealized_pnl - (pos.allocated_usd * dec!(0.0008)),
                ..pos.clone()
            }
        })
        .collect()
}

pub fn check_invalidation_breach(
    position: &PositionMatrix,
    candle_close: Decimal,
) -> Option<LiquidateCommand> {
    if let Some(inval) = position.invalidation_level {
        let breached = match position.direction.as_str() {
            "LONG" | "Long" => candle_close <= inval,
            "SHORT" | "Short" => candle_close >= inval,
            _ => false,
        };

        if breached {
            return Some(LiquidateCommand {
                symbol: position.symbol.clone(),
                size: position.size,
                reason: format!(
                    "Candle close {} breached invalidation level {}",
                    candle_close, inval
                ),
            });
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct LiquidateCommand {
    pub symbol: String,
    pub size: Decimal,
    pub reason: String,
}

pub fn apply_dynamic_stop(
    position: &PositionMatrix,
    new_distance_pct: f64,
    new_invalidation_level: Option<Decimal>,
) -> PositionMatrix {
    let mut updated = position.clone();

    if new_distance_pct > 0.0 && new_distance_pct < 100.0 {
        let distance_frac = Decimal::from_f64_retain(new_distance_pct / 100.0).unwrap_or(dec!(0));
        let candidate_stop = match position.direction.as_str() {
            "LONG" | "Long" => position.current_price * (dec!(1) - distance_frac),
            "SHORT" | "Short" => position.current_price * (dec!(1) + distance_frac),
            _ => position.current_price,
        };

        if let Some(current_stop) = position.stop_loss_price {
            match position.direction.as_str() {
                "LONG" | "Long" if candidate_stop > current_stop => {
                    updated.stop_loss_price = Some(candidate_stop);
                }
                "SHORT" | "Short" if candidate_stop < current_stop => {
                    updated.stop_loss_price = Some(candidate_stop);
                }
                _ => {}
            }
        } else {
            updated.stop_loss_price = Some(candidate_stop);
        }
    }

    if let Some(new_inval) = new_invalidation_level {
        updated.invalidation_level = Some(new_inval);
    }

    updated
}

