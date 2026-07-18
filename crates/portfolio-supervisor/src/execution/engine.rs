use config_models::{
    Direction, LifecycleState, OrderPacket, OrderStatus, OrderType, OrderSide, Stance,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::execution::gates::evaluate_gates;
use crate::execution::state_machine::OrderLifecycle;
use crate::policy::engine::PolicyTrigger;
use crate::capital_layer;

#[derive(Debug, Clone)]
pub struct CapitalState {
    pub available_margin: Decimal,
    pub reserved_margin: Decimal,
    pub total_equity: Decimal,
}

impl Default for CapitalState {
    fn default() -> Self {
        Self {
            available_margin: dec!(10000),
            reserved_margin: dec!(0),
            total_equity: dec!(10000),
        }
    }
}

pub struct ExecutionEngine {
    pub capital: Arc<RwLock<CapitalState>>,
    pub orders: Arc<RwLock<HashMap<String, OrderLifecycle>>>,
    pub positions: Arc<RwLock<HashMap<String, Decimal>>>,
    pub config_capital: Arc<RwLock<f64>>,
    pub slippage_ceiling_pct: RwLock<f64>,
    pub pool: Option<Arc<SqlitePool>>,
    pub cross_leverage: RwLock<u32>,
    pub maker_fee_pct: RwLock<f64>,
    pub taker_fee_pct: RwLock<f64>,
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            capital: Arc::new(RwLock::new(CapitalState::default())),
            orders: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
            config_capital: Arc::new(RwLock::new(10000.0)),
            slippage_ceiling_pct: RwLock::new(0.5),
            pool: None,
            cross_leverage: RwLock::new(20),
            maker_fee_pct: RwLock::new(0.02),
            taker_fee_pct: RwLock::new(0.06),
        }
    }

    pub async fn set_fee_config(&self, maker: f64, taker: f64, leverage: u32) {
        *self.maker_fee_pct.write().await = maker;
        *self.taker_fee_pct.write().await = taker;
        *self.cross_leverage.write().await = leverage;
    }

    pub fn set_db(&mut self, pool: Arc<SqlitePool>) {
        self.pool = Some(pool);
    }

    async fn persist_gate_event(
        &self,
        symbol: &str,
        gate_id: u8,
        decision: &str,
        reason: &str,
    ) {
        if let Some(ref pool) = self.pool {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let _ = sqlx::query(
                "INSERT INTO risk_control_events (instance_id, symbol, gate_id, decision, reason, timestamp_ms) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind("tae-core")
            .bind(symbol)
            .bind(gate_id as i64)
            .bind(decision)
            .bind(reason)
            .bind(now)
            .execute(pool.as_ref())
            .await;
        }
    }

    pub async fn set_config_capital(&self, amount: f64) {
        *self.config_capital.write().await = amount;
        let mut cap = self.capital.write().await;
        cap.total_equity = Decimal::from_f64_retain(amount).unwrap_or(dec!(10000));
        cap.available_margin = crate::capital_layer::available_margin(
            cap.total_equity,
            dec!(0),
            dec!(0),
            cap.reserved_margin,
        );
    }

    pub async fn process_trigger(
        &self,
        trigger: &PolicyTrigger,
        snapshot: &core_domain::models::MarketSnapshot,
        lifecycle_state: LifecycleState,
        stance: Stance,
    ) -> Result<Option<String>, String> {
        let cap = self.capital.read().await;
        let available_margin = cap.available_margin;
        drop(cap);

        let entry_price = snapshot.mid_price;

        // D_sl resolution priority (highest first):
        // 1. fixed_stop_loss_pct from execution policy
        // 2. stop_loss_distance_pct from MME Advisory Matrix
        // 3. system default 2.0%
        let stop_loss_distance_pct = trigger
            .risk_parameters
            .fixed_stop_loss_pct
            .or_else(|| {
                snapshot.advisory.as_ref().and_then(|adv| {
                    if adv.stop_loss_distance_pct > 0.0 {
                        Some(adv.stop_loss_distance_pct)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(2.0);

        let is_emergency = false;

        let position_size = if stance == Stance::CloseOnly {
            let positions = self.positions.read().await;
            positions.get(&trigger.symbol).copied()
        } else {
            None
        };

        let order = crate::execution::order::construct_order(
            trigger,
            available_margin,
            entry_price,
            stop_loss_distance_pct,
            stance,
            is_emergency,
            position_size,
        )?;

        let trade_readiness = snapshot
            .decision_context
            .as_ref()
            .map(|d| d.trade_readiness.as_str())
            .unwrap_or("WATCH");
        let slippage_ceiling = *self.slippage_ceiling_pct.read().await;

        let gate_result = evaluate_gates(
            &order,
            lifecycle_state,
            stance,
            available_margin.to_string().parse().unwrap_or(0.0),
            0.0,
            trade_readiness,
            slippage_ceiling,
            20,
            trigger.risk_parameters.max_position_size_usd,
        );

        let final_order = match gate_result {
            crate::execution::gates::GateResult::Approved => order,
            crate::execution::gates::GateResult::Blocked { gate, ref reason } => {
                self.persist_gate_event(&trigger.symbol, gate, "BLOCKED", reason).await;
                return Err(format!("Gate {} blocked: {}", gate, reason));
            }
            crate::execution::gates::GateResult::HeldForReview { gate, ref reason } => {
                eprintln!("TAE: Gate {} held for review: {}", gate, reason);
                self.persist_gate_event(&trigger.symbol, gate, "HELD_FOR_REVIEW", reason).await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let mut lifecycle = OrderLifecycle::new(order.clone(), now);
                lifecycle.status = OrderStatus::PreDispatch;
                lifecycle.transitions.push(crate::execution::state_machine::OrderTransition {
                    from: OrderStatus::PreDispatch,
                    to: OrderStatus::PreDispatch,
                    timestamp_ms: now * 1000,
                    metadata: Some(format!("Gate {}: {}", gate, reason)),
                });
                let client_order_id = lifecycle.packet.client_order_id.clone();
                let mut orders = self.orders.write().await;
                orders.insert(client_order_id.clone(), lifecycle);
                return Ok(Some(client_order_id));
            }
            crate::execution::gates::GateResult::Clipped { gate, ref reason, adjusted_size } => {
                eprintln!("TAE: Gate {} clipped: {}", gate, reason);
                self.persist_gate_event(&trigger.symbol, gate, "CLIP_AND_CONTINUE", reason).await;
                if let Some(adjusted) = adjusted_size {
                    OrderPacket { size: adjusted, ..order.clone() }
                } else {
                    order.clone()
                }
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let lifecycle = OrderLifecycle::new(final_order.clone(), now);
        let client_order_id = lifecycle.packet.client_order_id.clone();

        let mut orders = self.orders.write().await;
        orders.insert(client_order_id.clone(), lifecycle);

        if !final_order.reduce_only {
            let mut cap = self.capital.write().await;
            let leverage = Decimal::from(*self.cross_leverage.read().await);
            let margin_used = if leverage > dec!(0) {
                final_order.size * entry_price / leverage
            } else {
                final_order.size * entry_price
            };
            cap.reserved_margin += margin_used;
            cap.available_margin -= margin_used;
        }

        Ok(Some(client_order_id))
    }

    pub async fn dispatch_hard_exit(
        &self,
        symbol: &str,
    ) -> Result<Option<String>, String> {
        let pos_size = {
            let positions = self.positions.read().await;
            positions.get(symbol).copied().unwrap_or(dec!(0))
        };

        if pos_size == dec!(0) {
            return Ok(None);
        }

        let client_order_id = format!("hard_exit_{}_{}", symbol, std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());

        let order = OrderPacket {
            client_order_id: client_order_id.clone(),
            symbol: symbol.to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Market,
            price: None,
            size: pos_size,
            reduce_only: true,
            is_emergency_liquidation: true,
            associated_position_id: None,
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let lifecycle = OrderLifecycle::new(order, now);
        let mut orders = self.orders.write().await;
        orders.insert(client_order_id.clone(), lifecycle);

        Ok(Some(client_order_id))
    }

    pub async fn cancel_all_orders(&self, symbol: &str) {
        let mut orders = self.orders.write().await;
        orders.retain(|_id, order| order.packet.symbol != *symbol);
    }

    pub async fn hard_exit_for_symbol(&self, symbol: &str) {
        let pos_size = {
            let positions = self.positions.read().await;
            positions.get(symbol).copied().unwrap_or(dec!(0))
        };
        if pos_size > dec!(0) {
            let client_order_id = format!("hard_exit_{}_{}", symbol,
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            let order = OrderPacket {
                client_order_id: client_order_id.clone(),
                symbol: symbol.to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::Market,
                price: None,
                size: pos_size,
                reduce_only: true,
                is_emergency_liquidation: true,
                associated_position_id: None,
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            let lifecycle = OrderLifecycle::new(order, now);
            let mut orders = self.orders.write().await;
            orders.insert(client_order_id, lifecycle);
        }
    }
}

