use config_models::{
    ExecutionPolicy, OrderPacket, OrderSide, OrderStatus, OrderType,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::execution::state_machine::OrderLifecycle;
use crate::policy::engine::PolicyTrigger;

pub struct PaperTradingEngine {
    pub orders: Arc<RwLock<HashMap<String, OrderLifecycle>>>,
    pub fee_config: FeesConfig,
    pub positions: Arc<RwLock<HashMap<String, PaperPosition>>>,
    pub equity: Arc<RwLock<f64>>,
    pub next_order_id: Arc<RwLock<u64>>,
    pub pool: Option<Arc<SqlitePool>>,
}

#[derive(Debug, Clone)]
pub struct FeesConfig {
    pub maker_fee_pct: f64,
    pub taker_fee_pct: f64,
    pub funding_rate_8h: f64,
    pub simulated_spread_pct: f64,
}

impl Default for FeesConfig {
    fn default() -> Self {
        Self {
            maker_fee_pct: 0.02,
            taker_fee_pct: 0.06,
            funding_rate_8h: 0.01,
            simulated_spread_pct: 0.01,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaperPosition {
    pub symbol: String,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub direction: config_models::Direction,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
}

impl PaperTradingEngine {
    pub fn new(fee_config: FeesConfig) -> Self {
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
            fee_config,
            positions: Arc::new(RwLock::new(HashMap::new())),
            equity: Arc::new(RwLock::new(10000.0)),
            next_order_id: Arc::new(RwLock::new(1)),
            pool: None,
        }
    }

    pub fn set_db(&mut self, pool: Arc<SqlitePool>) {
        self.pool = Some(pool);
    }

    async fn persist_paper_trade(
        &self,
        symbol: &str,
        direction: &str,
        entry_price: Decimal,
        exit_price: Decimal,
        size: Decimal,
        pnl: f64,
        fee: f64,
    ) {
        if let Some(ref pool) = self.pool {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let entry_str = entry_price.to_string();
            let exit_str = exit_price.to_string();
            let size_str = size.to_string();
            let _ = sqlx::query(
                "INSERT INTO paper_trades (symbol, direction, entry_price, exit_price, size, pnl, fee, closed_at_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(symbol)
            .bind(direction)
            .bind(&entry_str)
            .bind(&exit_str)
            .bind(&size_str)
            .bind(pnl)
            .bind(fee)
            .bind(now)
            .execute(pool.as_ref())
            .await;
        }
    }

    async fn persist_trade_telemetry(
        &self,
        symbol: &str,
        direction: &str,
        entry_price: Decimal,
        exit_price: Decimal,
        size: Decimal,
        pnl: f64,
        fee: f64,
        trigger_source: &str,
    ) {
        if let Some(ref pool) = self.pool {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let entry_str = entry_price.to_string();
            let exit_str = exit_price.to_string();
            let size_str = size.to_string();
            let roi = if let Ok(entry_f64) = entry_price.to_string().parse::<f64>() {
                if entry_f64 > 0.0 {
                    (pnl / (entry_f64 * size.to_string().parse::<f64>().unwrap_or(0.0))) * 100.0
                } else { 0.0 }
            } else { 0.0 };
            let _ = sqlx::query(
                "INSERT INTO trade_telemetry_history (symbol, direction, entry_price, exit_price, size, pnl, fee, roi_pct, trigger_source, closed_at_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(symbol)
            .bind(direction)
            .bind(&entry_str)
            .bind(&exit_str)
            .bind(&size_str)
            .bind(pnl)
            .bind(fee)
            .bind(roi)
            .bind(trigger_source)
            .bind(now)
            .execute(pool.as_ref())
            .await;
        }
    }

    async fn persist_equity_snapshot(&self, equity: f64) {
        if let Some(ref pool) = self.pool {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let _ = sqlx::query(
                "INSERT INTO portfolio_equity_history (equity, timestamp_ms) VALUES (?, ?)"
            )
            .bind(equity)
            .bind(now)
            .execute(pool.as_ref())
            .await;
        }
    }

    pub async fn submit_order(
        &self,
        packet: OrderPacket,
        current_mid_price: Decimal,
    ) -> Result<String, String> {
        let exchange_id;
        let is_market;
        {
            let mut orders = self.orders.write().await;
            let mut next_id = self.next_order_id.write().await;
            exchange_id = format!("paper_{:06}", *next_id);
            *next_id += 1;

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let mut lifecycle = OrderLifecycle::new(packet, now);
            lifecycle.exchange_order_id = Some(exchange_id.clone());

            let order_type = lifecycle.packet.order_type;
            is_market = order_type == OrderType::Market;

            if is_market {
                self.fill_market_order(&mut lifecycle, current_mid_price)?;
            } else {
                lifecycle.status = OrderStatus::Open;
                lifecycle.transitions.push(crate::execution::state_machine::OrderTransition {
                    from: OrderStatus::Pending,
                    to: OrderStatus::Open,
                    timestamp_ms: now * 1000,
                    metadata: None,
                });
            }

            orders.insert(exchange_id.clone(), lifecycle);
        }

        if is_market {
            self.update_position(&exchange_id).await?;
        }

        Ok(exchange_id)
    }

    pub async fn evaluate_order_fills(
        &self,
        current_mid_price: Decimal,
    ) -> Vec<(String, Decimal)> {
        let mut fills = Vec::new();

        let orders_to_check: Vec<(String, OrderLifecycle)> = {
            let orders = self.orders.read().await;
            orders
                .iter()
                .filter(|(_, o)| o.status == OrderStatus::Open)
                .map(|(id, o)| (id.clone(), o.clone()))
                .collect()
        };

        {
            let mut orders = self.orders.write().await;
            for (order_id, lifecycle) in orders_to_check {
                let should_fill = match lifecycle.packet.order_type {
                    OrderType::Limit => match lifecycle.packet.price {
                        Some(limit_price) => match lifecycle.packet.side {
                            OrderSide::Buy => current_mid_price <= limit_price,
                            OrderSide::Sell => current_mid_price >= limit_price,
                        },
                        None => false,
                    },
                    OrderType::Stop => match lifecycle.packet.price {
                        Some(stop_price) => match lifecycle.packet.side {
                            OrderSide::Sell => current_mid_price <= stop_price,
                            OrderSide::Buy => current_mid_price >= stop_price,
                        },
                        None => false,
                    },
                    _ => false,
                };

                if should_fill {
                    if let Some(o) = orders.get_mut(&order_id) {
                        self.fill_market_order(o, current_mid_price).ok();
                        fills.push((order_id.clone(), current_mid_price));
                    }
                }
            }
        }

        for (order_id, _) in &fills {
            let _ = self.update_position(order_id).await;
        }

        fills
    }

    fn fill_market_order(
        &self,
        lifecycle: &mut OrderLifecycle,
        mid_price: Decimal,
    ) -> Result<(), String> {
        let spread_half = Decimal::from_f64_retain(self.fee_config.simulated_spread_pct / 100.0 / 2.0)
            .unwrap_or(dec!(0));
        let fill_price = if lifecycle.packet.side == OrderSide::Buy {
            mid_price * (dec!(1) + spread_half)
        } else {
            mid_price * (dec!(1) - spread_half)
        };

        let fill_qty = lifecycle.packet.size;
        lifecycle.filled_size = fill_qty;
        lifecycle.fill_price = Some(fill_price);

        let slippage = if fill_price > dec!(0) {
            let mid_f64 = mid_price.to_string().parse::<f64>().unwrap_or(0.0);
            let fill_f64 = fill_price.to_string().parse::<f64>().unwrap_or(0.0);
            if mid_f64 > 0.0 {
                Some(((fill_f64 - mid_f64).abs() / mid_f64) * 10000.0)
            } else {
                None
            }
        } else {
            None
        };
        lifecycle.slippage_bps = slippage;

        lifecycle.status = OrderStatus::Closed;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        lifecycle.transitions.push(crate::execution::state_machine::OrderTransition {
            from: OrderStatus::Open,
            to: OrderStatus::Closed,
            timestamp_ms: now * 1000,
            metadata: Some(format!("filled {} @ {}", fill_qty, fill_price)),
        });

        Ok(())
    }

    async fn update_position(&self, order_id: &str) -> Result<(), String> {
        let orders = self.orders.read().await;
        let lifecycle = orders.get(order_id).ok_or("Order not found")?;
        if lifecycle.status != OrderStatus::Closed {
            return Ok(());
        }

        let symbol = lifecycle.packet.symbol.clone();
        let fill_price = lifecycle.fill_price.ok_or("No fill price")?;
        let fill_qty = lifecycle.filled_size;

        let is_market = lifecycle.packet.order_type == OrderType::Market;
        let fee_pct = if is_market {
            self.fee_config.taker_fee_pct / 100.0
        } else {
            self.fee_config.maker_fee_pct / 100.0
        };
        let notional = fill_qty * fill_price;
        let notional_f64 = notional.to_string().parse::<f64>().unwrap_or(0.0);
        let fee = notional_f64 * fee_pct;

        let mut positions = self.positions.write().await;
        let mut equity = self.equity.write().await;

        if lifecycle.packet.reduce_only {
            if let Some(pos) = positions.remove(&symbol) {
                let pnl = match pos.direction {
                    config_models::Direction::Long => {
                        let entry_f64 = pos.entry_price.to_string().parse::<f64>().unwrap_or(0.0);
                        let fill_f64 = fill_price.to_string().parse::<f64>().unwrap_or(0.0);
                        (fill_f64 - entry_f64) * pos.size.to_string().parse::<f64>().unwrap_or(0.0)
                    }
                    config_models::Direction::Short => {
                        let entry_f64 = pos.entry_price.to_string().parse::<f64>().unwrap_or(0.0);
                        let fill_f64 = fill_price.to_string().parse::<f64>().unwrap_or(0.0);
                        (entry_f64 - fill_f64) * pos.size.to_string().parse::<f64>().unwrap_or(0.0)
                    }
                };
                *equity += pnl - fee;

                let dir_str = match pos.direction {
                    config_models::Direction::Long => "LONG",
                    config_models::Direction::Short => "SHORT",
                };
                let trigger_src = if lifecycle.packet.is_emergency_liquidation {
                    "hard_exit"
                } else if lifecycle.packet.reduce_only {
                    "reduce_only"
                } else {
                    "policy"
                };
                self.persist_paper_trade(&symbol, dir_str, pos.entry_price, fill_price, pos.size, pnl, fee).await;
                self.persist_trade_telemetry(&symbol, dir_str, pos.entry_price, fill_price, pos.size, pnl, fee, trigger_src).await;
                self.persist_equity_snapshot(*equity).await;
            }
        } else {
            let direction = match lifecycle.packet.side {
                OrderSide::Buy => config_models::Direction::Long,
                OrderSide::Sell => config_models::Direction::Short,
            };
            positions.insert(
                symbol.clone(),
                PaperPosition {
                    symbol,
                    size: fill_qty,
                    entry_price: fill_price,
                    direction,
                    unrealized_pnl: 0.0,
                    realized_pnl: 0.0,
                },
            );
            *equity -= fee;
        }

        Ok(())
    }

    pub async fn get_position(&self, symbol: &str) -> Option<PaperPosition> {
        self.positions.read().await.get(symbol).cloned()
    }

    pub async fn get_equity(&self) -> f64 {
        *self.equity.read().await
    }

    pub async fn cancel_all_orders(&self) {
        let mut orders = self.orders.write().await;
        orders.clear();
    }

    pub async fn close_position(&self, symbol: &str, current_mid_price: Decimal) {
        let pos_size = {
            let positions = self.positions.read().await;
            positions.get(symbol).map(|p| p.size).unwrap_or(dec!(0))
        };

        if pos_size == dec!(0) {
            return;
        }

        let packet = OrderPacket {
            client_order_id: format!("manual_close_{}", symbol),
            symbol: symbol.to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Market,
            price: None,
            size: pos_size,
            reduce_only: true,
            is_emergency_liquidation: true,
            associated_position_id: None,
        };

        let _ = self.submit_order(packet, current_mid_price).await;
    }

    pub async fn settle_funding(&self) {
        let positions = self.positions.read().await;
        if positions.is_empty() {
            return;
        }

        let rate = self.fee_config.funding_rate_8h / 100.0;
        let mut total_settlement: f64 = 0.0;

        let settlement_details: Vec<(String, Decimal, f64)> = positions
            .iter()
            .map(|(sym, pos)| {
                let notional = pos.size * pos.entry_price;
                let notional_f64 = notional.to_string().parse::<f64>().unwrap_or(0.0);
                let payment = notional_f64 * rate;
                (sym.clone(), pos.size, payment)
            })
            .collect();

        drop(positions);

        for (_sym, _size, payment) in &settlement_details {
            total_settlement -= *payment;
        }

        let mut equity = self.equity.write().await;
        *equity += total_settlement;

        if total_settlement.abs() > 0.0001 {
            eprintln!(
                "💰 PAPER: Funding settlement applied — {:.4} (rate={:.4}%)",
                total_settlement, self.fee_config.funding_rate_8h
            );
        }

        self.persist_equity_snapshot(*equity).await;
    }

    pub async fn replay(
        &self,
        price_sequence: &[(u64, Decimal)],
        policies: &[ExecutionPolicy],
    ) -> Vec<ReplayTrade> {
        let mut trades = Vec::new();

        for &(timestamp, mid_price) in price_sequence {
            let _fills = self.evaluate_order_fills(mid_price).await;

            for policy in policies {
                if !policy.enabled {
                    continue;
                }

                let _mock_snapshot = build_mock_snapshot(&policy.symbol, mid_price);
                let _mock_trigger = PolicyTrigger {
                    policy_id: policy.policy_id.clone(),
                    symbol: policy.symbol.clone(),
                    direction: policy.direction,
                    trigger_timestamp: timestamp,
                    decision_context_snapshot: serde_json::json!({
                        "score": 80.0,
                        "bias": "BULLISH",
                        "confidence": 0.80,
                        "trade_readiness": "READY",
                    }),
                    stance: config_models::Stance::Active,
                    risk_parameters: policy.risk.clone(),
                };

                let order = OrderPacket {
                    client_order_id: format!("replay_{}_{}", policy.policy_id, timestamp),
                    symbol: policy.symbol.clone(),
                    side: match policy.direction {
                        config_models::Direction::Long => OrderSide::Buy,
                        config_models::Direction::Short => OrderSide::Sell,
                    },
                    order_type: OrderType::Market,
                    price: Some(mid_price),
                    size: Decimal::from(1),
                    reduce_only: false,
                    is_emergency_liquidation: false,
                    associated_position_id: None,
                };

                let order_id = match self.submit_order(order.clone(), mid_price).await {
                    Ok(id) => id,
                    Err(_) => continue,
                };

                let orders = self.orders.read().await;
                if let Some(lifecycle) = orders.get(&order_id) {
                    if lifecycle.status == OrderStatus::Closed {
                        trades.push(ReplayTrade {
                            timestamp,
                            symbol: policy.symbol.clone(),
                            direction: format!("{:?}", policy.direction),
                            size: order.size,
                            fill_price: lifecycle.fill_price.unwrap_or(mid_price),
                            order_id,
                        });
                    }
                }
            }
        }

        trades
    }
}

#[derive(Debug, Clone)]
pub struct ReplayTrade {
    pub timestamp: u64,
    pub symbol: String,
    pub direction: String,
    pub size: Decimal,
    pub fill_price: Decimal,
    pub order_id: String,
}

fn build_mock_snapshot(symbol: &str, mid_price: Decimal) -> core_domain::models::MarketSnapshot {
    use std::collections::HashMap;
    core_domain::models::MarketSnapshot {
        timeframe_slot: Some(core_domain::models::TimeframeSlot::Micro),
        exchange: None,
        timeframe_secs: 60,
        timestamp: 0,
        symbol: symbol.to_string(),
        is_completed: Some(true),
        mid_price,
        bid_price: mid_price - Decimal::from(1),
        ask_price: mid_price + Decimal::from(1),
        bid_size: None,
        ask_size: None,
        funding_rate: None,
        open: None,
        high: None,
        low: None,
        close: None,
        volume: None,
        average_volume: None,
        indicators: HashMap::new(),
        context: None,
        alignment: None,
        analysis: None,
        risk: None,
        advisory: None,
        open_interest: None,
        oi_delta_1h: None,
        mark_price: None,
        index_price: None,
        mark_index_spread_pct: None,
        prev_day_px: None,
        statistical_context: None,
        decision_context: None,
        risk_profile: None,
        liquidity: None,
        cluster: None,
        volume_profile: None,
        liquidity_signals: vec![],
        metrics_config: None,
        opportunity: None,
        quality_envelope: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_market_order_fill() {
        let engine = PaperTradingEngine::new(FeesConfig::default());
        let packet = OrderPacket {
            client_order_id: "test1".into(),
            symbol: "BTC-USDT".into(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: None,
            size: dec!(1),
            reduce_only: false,
            is_emergency_liquidation: false,
            associated_position_id: None,
        };

        let order_id = engine
            .submit_order(packet, Decimal::from(50000))
            .await
            .unwrap();
        assert!(!order_id.is_empty());

        let orders = engine.orders.read().await;
        let order = orders.get(&order_id).unwrap();
        assert_eq!(order.status, OrderStatus::Closed);
        assert!(order.fill_price.is_some());
        assert_eq!(order.filled_size, dec!(1));
    }

    #[tokio::test]
    async fn test_limit_order_fills_when_price_crosses() {
        let engine = PaperTradingEngine::new(FeesConfig::default());
        let packet = OrderPacket {
            client_order_id: "limit1".into(),
            symbol: "BTC-USDT".into(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Some(Decimal::from(49000)),
            size: dec!(1),
            reduce_only: false,
            is_emergency_liquidation: false,
            associated_position_id: None,
        };

        let order_id = engine
            .submit_order(packet, Decimal::from(50000))
            .await
            .unwrap();

        {
            let orders = engine.orders.read().await;
            let order = orders.get(&order_id).unwrap();
            assert_eq!(order.status, OrderStatus::Open);
        }

        let fills = engine.evaluate_order_fills(Decimal::from(48900)).await;
        assert_eq!(fills.len(), 1);

        let orders = engine.orders.read().await;
        let order = orders.get(&order_id).unwrap();
        assert_eq!(order.status, OrderStatus::Closed);
    }

    #[tokio::test]
    async fn test_close_position_sells() {
        let engine = PaperTradingEngine::new(FeesConfig::default());

        let buy_packet = OrderPacket {
            client_order_id: "buy1".into(),
            symbol: "BTC-USDT".into(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: None,
            size: dec!(2),
            reduce_only: false,
            is_emergency_liquidation: false,
            associated_position_id: None,
        };
        let _ = engine
            .submit_order(buy_packet, Decimal::from(50000))
            .await
            .unwrap();

        let pos = engine.get_position("BTC-USDT").await;
        assert!(pos.is_some());
        assert_eq!(pos.unwrap().size, dec!(2));

        engine
            .close_position("BTC-USDT", Decimal::from(51000))
            .await;

        let pos_after = engine.get_position("BTC-USDT").await;
        assert!(pos_after.is_none());
    }
}
