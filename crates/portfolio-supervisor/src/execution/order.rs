use config_models::{Direction, OrderPacket, OrderSide, OrderType, Stance};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::policy::engine::PolicyTrigger;

pub fn construct_order(
    trigger: &PolicyTrigger,
    available_margin: Decimal,
    entry_price: Decimal,
    stop_loss_distance_pct: f64,
    stance: Stance,
    is_emergency: bool,
    position_size_from_matrix: Option<Decimal>,
) -> Result<OrderPacket, String> {
    let reduce_only = stance == Stance::CloseOnly || is_emergency;
    let client_order_id = format!(
        "{}_{}_{}",
        trigger.policy_id,
        trigger.trigger_timestamp,
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>()
    );

    let side = match trigger.direction {
        Direction::Long => OrderSide::Buy,
        Direction::Short => OrderSide::Sell,
    };

    let order_type = if is_emergency {
        OrderType::Market
    } else {
        OrderType::Limit
    };

    let price = if order_type == OrderType::Market {
        None
    } else {
        Some(entry_price)
    };

    let size = if reduce_only {
        position_size_from_matrix.unwrap_or(dec!(0))
    } else {
        // D_sl resolution priority:
        // 1. fixed_stop_loss_pct from execution policy
        // 2. stop_loss_distance_pct from MME Decision Matrix (passed as param)
        // 3. system default 2.0%
        let d_sl_raw = trigger
            .risk_parameters
            .fixed_stop_loss_pct
            .unwrap_or(stop_loss_distance_pct);
        let d_sl_frac = if d_sl_raw <= 0.0 || d_sl_raw > 100.0 {
            0.02 // 2% default if invalid
        } else {
            d_sl_raw / 100.0
        };
        let risk_fraction = trigger.risk_parameters.risk_per_trade_pct / 100.0;
        let d_sl_dec = Decimal::from_f64_retain(d_sl_frac).unwrap_or(dec!(0));
        let risk_dec = Decimal::from_f64_retain(risk_fraction).unwrap_or(dec!(0));

        if d_sl_dec == dec!(0) {
            return Err("Stop-loss distance is zero".into());
        }

        let size_quote = (available_margin * risk_dec) / d_sl_dec;
        let size_base = if entry_price > dec!(0) {
            size_quote / entry_price
        } else {
            dec!(0)
        };

        if size_base <= dec!(0) {
            return Err("Computed position size is zero or negative".into());
        }

        if let Some(max_usd) = trigger.risk_parameters.max_position_size_usd {
            let max_size = Decimal::from_f64_retain(max_usd).unwrap_or(size_base);
            if size_base > max_size {
                return Ok(OrderPacket {
                    client_order_id,
                    symbol: trigger.symbol.clone(),
                    side,
                    order_type,
                    price,
                    size: max_size,
                    reduce_only,
                    is_emergency_liquidation: is_emergency,
                    associated_position_id: None,
                });
            }
        }

        size_base
    };

    if size <= dec!(0) && !is_emergency {
        return Err("Position size is zero or negative".into());
    }

    Ok(OrderPacket {
        client_order_id,
        symbol: trigger.symbol.clone(),
        side,
        order_type,
        price,
        size,
        reduce_only,
        is_emergency_liquidation: is_emergency,
        associated_position_id: None,
    })
}
