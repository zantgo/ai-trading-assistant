use core_domain::portfolio::{PositionMatrix, PositionState};
use rust_decimal::prelude::ToPrimitive;
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
        average_entry_price: entry_price,
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
        current_portions: 1,
        max_portions: 4,
        position_state: PositionState::Opening,
        initial_allocated_margin: allocated_usd / dec!(20),
        realized_pnl_accumulator: dec!(0),
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

            let unrealized_pnl =
                direction_sign * (current_price - pos.average_entry_price) * pos.size;
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

pub fn apply_scaled_entry(
    positions: &[PositionMatrix],
    slot_index: u32,
    entry_price: Decimal,
    size: Decimal,
    max_slots: u32,
) -> Result<PositionMatrix, String> {
    let total_existing = positions.iter().find(|p| !p.symbol.is_empty());
    if let Some(existing) = total_existing {
        if existing.current_portions >= max_slots {
            return Err(format!(
                "Position slot limit reached: {}/{} portions filled",
                existing.current_portions, max_slots
            ));
        }

        let new_state = if slot_index.saturating_add(1) >= max_slots {
            PositionState::Managing
        } else {
            PositionState::Opening
        };

        let total_size = existing.size + size;
        let vwap = if total_size > dec!(0) {
            ((existing.average_entry_price * existing.size) + (entry_price * size)) / total_size
        } else {
            entry_price
        };
        let new_allocated = existing.allocated_usd + (entry_price * size);

        Ok(PositionMatrix {
            position_id: existing.position_id,
            symbol: existing.symbol.clone(),
            direction: existing.direction.clone(),
            average_entry_price: vwap,
            size: total_size,
            allocated_usd: new_allocated,
            current_portions: slot_index.saturating_add(1),
            position_state: new_state,
            entry_price: existing.entry_price,
            ..existing.clone()
        })
    } else {
        Ok(compute_position_matrix(
            "",
            "Long",
            entry_price,
            size,
            entry_price,
            0,
            0,
        ))
    }
}

pub fn compute_scaled_portion_size(
    base_size: Decimal,
    scale_pct: f64,
    allocation_curve: &str,
    slot_index: u32,
) -> Decimal {
    let base = Decimal::from_f64_retain(base_size.to_f64().unwrap_or(0.0) * scale_pct / 100.0)
        .unwrap_or(dec!(0));
    match allocation_curve {
        "Linear" => base * Decimal::from(slot_index + 1),
        "Exponential" => base * Decimal::from(2u32.pow(slot_index)),
        "Logarithmic" => {
            let factor = ((slot_index + 1) as f64).ln() + 1.0;
            let dec_factor = Decimal::from_f64_retain(factor).unwrap_or(dec!(1));
            base * dec_factor
        }
        "Sigmoid" => {
            let x = slot_index as f64 - 1.5;
            let factor = 1.0 / (1.0 + (-x).exp());
            let dec_factor = Decimal::from_f64_retain(factor).unwrap_or(dec!(1));
            base * dec_factor
        }
        "Power" => {
            let factor = ((slot_index + 1) as f64).powf(1.5);
            let dec_factor = Decimal::from_f64_retain(factor).unwrap_or(dec!(1));
            base * dec_factor
        }
        _ => base,
    }
}
