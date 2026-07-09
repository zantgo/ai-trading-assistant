use crate::config::ExecutionConfig;
use crate::execution::order_manager::OrderState;

pub struct ExecutionGuard {
    pub max_slippage_pct: f64,
    pub min_balance_for_order: f64,
    pub max_order_size: f64,
    pub max_position_value: f64,
    pub max_order_timeout_secs: u64,
}

impl ExecutionGuard {
    pub fn new(config: &ExecutionConfig) -> Self {
        Self {
            max_slippage_pct: config.max_slippage_pct,
            min_balance_for_order: 1.0,
            max_order_size: config.max_order_size_usd,
            max_position_value: config.max_position_value_usd,
            max_order_timeout_secs: config.order_timeout_secs,
        }
    }

    pub fn validate_order(
        &self,
        order: &OrderState,
        balance: f64,
        current_position: f64,
    ) -> Result<(), String> {
        let order_value = order.price * order.size;

        if balance < self.min_balance_for_order {
            return Err("Insufficient balance to place orders".into());
        }

        if order_value > self.max_order_size {
            return Err(format!(
                "Order value ${:.2} exceeds max ${:.2}",
                order_value, self.max_order_size
            ));
        }

        let new_position = current_position.abs() + order_value;
        if new_position > self.max_position_value {
            return Err(format!(
                "Total position ${:.2} would exceed max ${:.2}",
                new_position, self.max_position_value
            ));
        }

        if order.size <= 0.0 {
            return Err("Order size must be positive".into());
        }

        if order.price <= 0.0 {
            return Err("Order price must be positive".into());
        }

        Ok(())
    }

    pub fn is_price_stale(&self, last_price_ts: i64, current_ts: i64) -> bool {
        (current_ts - last_price_ts).abs() > self.max_order_timeout_secs as i64
    }

    pub fn expected_slippage(&self, order_size: f64, market_depth: f64) -> f64 {
        if market_depth <= 0.0 {
            return self.max_slippage_pct;
        }
        (order_size / market_depth * 100.0).min(self.max_slippage_pct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ExecutionConfig {
        ExecutionConfig {
            mode: "paper".into(),
            max_slippage_pct: 0.5,
            order_timeout_secs: 30,
            rate_limit_orders_per_sec: 5,
            max_order_size_usd: 50000.0,
            max_position_value_usd: 200000.0,
        }
    }

    #[test]
    fn test_validate_valid_order() {
        let guard = ExecutionGuard::new(&test_config());
        let order = OrderState {
            cloid: "test".into(), symbol: "BTC".into(), side: "long".into(),
            order_type: "limit".into(), price: 50000.0, size: 0.1,
            filled: 0.0, status: super::super::order_manager::OrderStatus::Pending,
            created_at: 0, updated_at: 0,
        };
        assert!(guard.validate_order(&order, 10000.0, 0.0).is_ok());
    }

    #[test]
    fn test_validate_order_exceeds_max() {
        let guard = ExecutionGuard::new(&test_config());
        let order = OrderState {
            cloid: "big".into(), symbol: "BTC".into(), side: "long".into(),
            order_type: "limit".into(), price: 100000.0, size: 5.0,
            filled: 0.0, status: super::super::order_manager::OrderStatus::Pending,
            created_at: 0, updated_at: 0,
        };
        assert!(guard.validate_order(&order, 1000000.0, 0.0).is_err());
    }

    #[test]
    fn test_price_stale_detection() {
        let guard = ExecutionGuard::new(&test_config());
        assert!(guard.is_price_stale(100, 200));
        assert!(!guard.is_price_stale(100, 120));
    }

    #[test]
    fn test_slippage_estimate() {
        let guard = ExecutionGuard::new(&test_config());
        let slip = guard.expected_slippage(1.0, 100.0);
        assert!(slip > 0.0 && slip <= 0.5);
    }
}
