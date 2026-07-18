use config_models::{
    Direction, LifecycleState, OrderPacket, OrderStatus, OrderType, OrderSide, Stance,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, RwLock as StdRwLock};
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

impl CapitalState {
    pub fn margin_usage_ratio(&self) -> f64 {
        if self.total_equity > dec!(0) {
            (self.reserved_margin / self.total_equity)
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0)
        } else {
            0.0
        }
    }
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

#[derive(Debug, Clone, PartialEq)]
pub struct PositionRecord {
    pub size: Decimal,
    pub direction: Direction,
    pub entry_price: Decimal,
}

pub struct ExecutionEngine {
    pub capital: Arc<RwLock<CapitalState>>,
    pub orders: Arc<RwLock<HashMap<String, OrderLifecycle>>>,
    pub positions: Arc<RwLock<HashMap<String, PositionRecord>>>,
    pub config_capital: Arc<RwLock<f64>>,
    pub slippage_ceiling_pct: RwLock<f64>,
    pub pool: Option<Arc<SqlitePool>>,
    pub cross_leverage: RwLock<u32>,
    pub maker_fee_pct: RwLock<f64>,
    pub taker_fee_pct: RwLock<f64>,
    pub safety_state: Arc<RwLock<String>>,
    pub dispatched_ids: StdRwLock<HashMap<String, u64>>,
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
            safety_state: Arc::new(RwLock::new("NORMAL".to_string())),
            dispatched_ids: StdRwLock::new(HashMap::new()),
        }
    }

    fn build_idempotency_key(&self, trigger: &PolicyTrigger) -> String {
        format!("{}:{}:{}", trigger.policy_id, trigger.symbol, trigger.trigger_timestamp)
    }

    fn check_and_mark_dedup(&self, key: &str, ttl_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut dispatched = self.dispatched_ids.write().unwrap();
        dispatched.retain(|_, ts| now - *ts < ttl_secs);
        if dispatched.contains_key(key) {
            return false;
        }
        dispatched.insert(key.to_string(), now);
        true
    }

    pub async fn set_fee_config(&self, maker: f64, taker: f64, leverage: u32) {
        *self.maker_fee_pct.write().await = maker;
        *self.taker_fee_pct.write().await = taker;
        *self.cross_leverage.write().await = leverage;
    }

    pub fn set_db(&mut self, pool: Arc<SqlitePool>) {
        self.pool = Some(pool);
    }

    pub async fn set_safety_state(&self, state: &str) {
        *self.safety_state.write().await = state.to_string();
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

    pub async fn set_position(&self, symbol: &str, size: Decimal, direction: Direction, entry_price: Decimal) {
        let mut positions = self.positions.write().await;
        if size > dec!(0) {
            positions.insert(symbol.to_string(), PositionRecord {
                size,
                direction,
                entry_price,
            });
        } else {
            positions.remove(symbol);
        }
    }

    pub async fn process_trigger(
        &self,
        trigger: &PolicyTrigger,
        snapshot: &core_domain::models::MarketSnapshot,
        lifecycle_state: LifecycleState,
        stance: Stance,
    ) -> Result<Option<String>, String> {
        let dedup_key = self.build_idempotency_key(trigger);
        if !self.check_and_mark_dedup(&dedup_key, 600) {
            return Ok(None);
        }

        let cap = self.capital.read().await;
        let available_margin = cap.available_margin;
        let margin_usage_ratio = cap.margin_usage_ratio();
        let total_equity = cap.total_equity.to_string().parse::<f64>().unwrap_or(0.0);
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
            positions.get(&trigger.symbol).map(|p| p.size)
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
        let active_position_count = self.positions.read().await.len();
        let safety_state_str = self.safety_state.read().await.clone();

        let available_margin_f64 = available_margin.to_string().parse().unwrap_or(0.0);

        let gate_result = evaluate_gates(
            &order,
            lifecycle_state,
            stance,
            available_margin_f64,
            margin_usage_ratio,
            trade_readiness,
            slippage_ceiling,
            *self.cross_leverage.read().await,
            trigger.risk_parameters.max_position_size_usd,
            Some(snapshot.bid_price),
            Some(snapshot.ask_price),
            active_position_count,
            total_equity,
            &safety_state_str,
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
        let (pos_size, pos_direction) = {
            let positions = self.positions.read().await;
            positions.get(symbol)
                .map(|p| (p.size, p.direction))
                .unwrap_or((dec!(0), Direction::Long))
        };

        if pos_size == dec!(0) {
            return Ok(None);
        }

        let client_order_id = format!("hard_exit_{}_{}", symbol, std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());

        // Close side depends on position direction:
        // Long position → Sell to close; Short position → Buy to close
        let side = match pos_direction {
            Direction::Long => OrderSide::Sell,
            Direction::Short => OrderSide::Buy,
        };

        let order = OrderPacket {
            client_order_id: client_order_id.clone(),
            symbol: symbol.to_string(),
            side,
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
        let _ = self.dispatch_hard_exit(symbol).await;
    }

    pub fn position_count(&self) -> usize {
        self.positions.try_read().map(|p| p.len()).unwrap_or(0)
    }

    pub fn open_order_count(&self) -> usize {
        self.orders.try_read().map(|o| {
            o.values().filter(|l| {
                l.status == OrderStatus::Open
                    || l.status == OrderStatus::Submitted
                    || l.status == OrderStatus::Pending
                    || l.status == OrderStatus::PreDispatch
            }).count()
        }).unwrap_or(0)
    }
}
