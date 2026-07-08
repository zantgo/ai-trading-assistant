use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Pending,
    Open,
    PartialFill,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderState {
    pub cloid: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: f64,
    pub size: f64,
    pub filled: f64,
    pub status: OrderStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResult {
    pub success: bool,
    pub cloid: Option<String>,
    pub filled_size: Option<f64>,
    pub avg_price: Option<f64>,
    pub error: Option<String>,
}

pub struct RateLimiter {
    pub max_per_second: u32,
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(max_per_second: u32) -> Self {
        Self { max_per_second, tokens: max_per_second as f64, last_refill: Instant::now() }
    }

    pub fn check(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.max_per_second as f64).min(self.max_per_second as f64);
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

pub struct OrderManager {
    pub active_orders: HashMap<String, OrderState>,
    nonce: AtomicU64,
    rate_limiter: RateLimiter,
}

impl OrderManager {
    pub fn new(rate_limit: u32) -> Self {
        Self {
            active_orders: HashMap::new(),
            nonce: AtomicU64::new(1),
            rate_limiter: RateLimiter::new(rate_limit),
        }
    }

    pub fn create_order(&mut self, symbol: &str, side: &str, order_type: &str, price: f64, size: f64) -> Result<OrderState, String> {
        if !self.rate_limiter.check() {
            return Err("Rate limit exceeded".into());
        }
        let nonce = self.nonce.fetch_add(1, Ordering::Relaxed);
        let cloid = format!("ord_{}", nonce);
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        let order = OrderState {
            cloid: cloid.clone(),
            symbol: symbol.to_string(),
            side: side.to_string(),
            order_type: order_type.to_string(),
            price,
            size,
            filled: 0.0,
            status: OrderStatus::Pending,
            created_at: ts,
            updated_at: ts,
        };
        self.active_orders.insert(cloid, order.clone());
        Ok(order)
    }

    pub fn update_fill(&mut self, cloid: &str, fill_price: f64, fill_size: f64) -> Option<OrderState> {
        if let Some(order) = self.active_orders.get_mut(cloid) {
            order.filled += fill_size;
            order.updated_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
            if order.filled >= order.size {
                order.status = OrderStatus::Filled;
            } else if order.filled > 0.0 {
                order.status = OrderStatus::PartialFill;
            }
            return Some(order.clone());
        }
        None
    }

    pub fn cancel_order(&mut self, cloid: &str) -> Option<OrderState> {
        if let Some(order) = self.active_orders.get_mut(cloid) {
            if order.status == OrderStatus::Pending || order.status == OrderStatus::Open || order.status == OrderStatus::PartialFill {
                order.status = OrderStatus::Cancelled;
                order.updated_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
                return Some(order.clone());
            }
        }
        None
    }

    pub fn get_order(&self, cloid: &str) -> Option<&OrderState> {
        self.active_orders.get(cloid)
    }

    pub fn orders_for_symbol(&self, symbol: &str) -> Vec<&OrderState> {
        self.active_orders.values().filter(|o| o.symbol == symbol).collect()
    }

    pub fn all_orders(&self) -> Vec<&OrderState> {
        self.active_orders.values().collect()
    }

    pub fn pending_order_count(&self) -> usize {
        self.active_orders.values().filter(|o| o.status == OrderStatus::Pending || o.status == OrderStatus::Open).count()
    }

    pub fn remaining_nonce(&self) -> u64 {
        self.nonce.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_fill_order() {
        let mut om = OrderManager::new(100);
        let order = om.create_order("BTC", "long", "limit", 50000.0, 0.1).unwrap();
        assert_eq!(order.status, OrderStatus::Pending);
        assert_eq!(order.size, 0.1);

        let updated = om.update_fill(&order.cloid, 50000.0, 0.1).unwrap();
        assert_eq!(updated.status, OrderStatus::Filled);
    }

    #[test]
    fn test_cancel_order() {
        let mut om = OrderManager::new(100);
        let order = om.create_order("ETH", "short", "market", 3000.0, 1.0).unwrap();
        let cancelled = om.cancel_order(&order.cloid).unwrap();
        assert_eq!(cancelled.status, OrderStatus::Cancelled);
    }

    #[test]
    fn test_rate_limit() {
        let mut om = OrderManager::new(1);
        let _ = om.create_order("BTC", "long", "limit", 50000.0, 0.1).unwrap();
        let result = om.create_order("BTC", "long", "limit", 50001.0, 0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_partial_fill() {
        let mut om = OrderManager::new(100);
        let order = om.create_order("BTC", "long", "limit", 50000.0, 0.5).unwrap();
        let updated = om.update_fill(&order.cloid, 50000.0, 0.2).unwrap();
        assert_eq!(updated.status, OrderStatus::PartialFill);
        assert_eq!(updated.filled, 0.2);
    }
}
