//! v7 TAE — Unified ExecutionEngine.
//!
//! ONE engine for paper and live. It owns all trading state — orders,
//! positions, the equity ledger, fee/slippage/funding accounting — and the
//! only mode-dependent part is the `ExecutionBackend` (PaperSimulation now;
//! a LiveBroker later) that answers "does this order fill at this price?".
//!
//! This engine replaces the v6 `ExecutionEngine` (gates/process_trigger)
//! and absorbs the v6 `PaperTradingEngine` accounting. Persistence writes
//! through the canonical `database_storage` schemas.

use config_models::{ExecutionMode, OrderPacket, OrderSide, OrderStatus, OrderType};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::execution::backend::{ExecutionBackend, Fill, PaperSimulation};
use crate::execution::state_machine::OrderLifecycle;
use crate::paper_trading::{FeesConfig, PaperPosition};

/// One closed-trade record returned by replay/backtest helpers.
#[derive(Debug, Clone)]
pub struct ReplayTrade {
    pub timestamp: u64,
    pub symbol: String,
    pub direction: String,
    pub size: Decimal,
    pub fill_price: Decimal,
    pub order_id: String,
}

/// Activity-log entry (in-memory ring buffer + persisted to
/// `automation_activity`).
#[derive(Debug, Clone)]
pub struct ActivityEntry {
    pub instance_id: String,
    pub symbol: String,
    pub ts_ms: u64,
    pub event: String,
    pub detail: String,
}

/// Outcome of the most recent position close per symbol — feeds the PME
/// safety manager's consecutive-loss ladder.
#[derive(Debug, Clone)]
pub struct CloseOutcome {
    pub is_loss: bool,
    pub exit_reason: String,
    pub pnl: Decimal,
}

pub struct ExecutionEngine {
    pub orders: Arc<RwLock<HashMap<String, OrderLifecycle>>>,
    pub fee_config: FeesConfig,
    pub positions: Arc<RwLock<HashMap<String, PaperPosition>>>,
    /// Master equity ledger in `Decimal`.
    pub equity: Arc<RwLock<Decimal>>,
    pub next_order_id: Arc<RwLock<u64>>,
    pub pool: Option<Arc<SqlitePool>>,
    /// v10: the session id stamped on every persisted row (None = legacy
    /// or standalone paths).
    pub session_id: RwLock<Option<i64>>,
    /// The mode switch — lives at the very end of the execution path.
    pub mode: RwLock<ExecutionMode>,
    pub backend: tokio::sync::RwLock<Box<dyn ExecutionBackend>>,
    pub cross_leverage: RwLock<u32>,
    /// Ring buffer of executor events (capped).
    pub activity: RwLock<Vec<ActivityEntry>>,
    /// Most recent close outcome per symbol (single-consume via
    /// `take_last_close`).
    pub last_close: RwLock<HashMap<String, CloseOutcome>>,
}

impl ExecutionEngine {
    pub fn new(fee_config: FeesConfig) -> Self {
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
            fee_config: fee_config.clone(),
            positions: Arc::new(RwLock::new(HashMap::new())),
            equity: Arc::new(RwLock::new(dec!(10000))),
            next_order_id: Arc::new(RwLock::new(1)),
            pool: None,
            session_id: RwLock::new(None),
            mode: RwLock::new(ExecutionMode::Paper),
            backend: tokio::sync::RwLock::new(Box::new(PaperSimulation::new(fee_config))),
            cross_leverage: RwLock::new(20),
            activity: RwLock::new(Vec::new()),
            last_close: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_mode(fee_config: FeesConfig, mode: ExecutionMode) -> Self {
        let mut engine = Self::new(fee_config);
        *engine.mode.get_mut() = mode;
        engine
    }

    pub fn set_db(&mut self, pool: Arc<SqlitePool>) {
        self.pool = Some(pool);
    }

    /// v10: bind the current session id (stamped on every persisted row).
    pub async fn set_session_id(&self, id: i64) {
        *self.session_id.write().await = Some(id);
    }

    async fn current_session_id(&self) -> Option<i64> {
        *self.session_id.read().await
    }

    pub async fn set_mode(&self, mode: ExecutionMode) {
        *self.mode.write().await = mode;
    }

    /// v7 live trading: swap in a `LiveBroker`/`BitgetLiveBroker` backend.
    pub async fn set_live_backend(&self, backend: Box<dyn ExecutionBackend>) {
        *self.mode.write().await = ExecutionMode::Live;
        *self.backend.write().await = backend;
    }

    /// v7 live trading: restore the paper simulation backend.
    pub async fn set_paper_backend(&self) {
        *self.backend.write().await = Box::new(PaperSimulation::new(self.fee_config.clone()));
        *self.mode.write().await = ExecutionMode::Paper;
    }

    pub async fn mode(&self) -> ExecutionMode {
        *self.mode.read().await
    }

    pub async fn set_initial_equity(&self, amount: Decimal) {
        *self.equity.write().await = amount;
    }

    pub async fn set_cross_leverage(&self, leverage: u32) {
        *self.cross_leverage.write().await = leverage;
    }

    // ── Order submission ──────────────────────────────────────────────

    /// Submit an order. Market orders fill immediately (spread + slippage);
    /// limit/stop orders rest as `Open`. Returns the exchange/paper order id.
    pub async fn submit_order(
        &self,
        packet: OrderPacket,
        current_mid_price: Decimal,
    ) -> Result<String, String> {
        // Live mode: route to the venue; fills arrive via `apply_external_fills`.
        if *self.mode.read().await == ExecutionMode::Live {
            let exchange_id = self.backend.read().await.submit_order(&packet).await?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let mut lifecycle = OrderLifecycle::new(packet, now);
            lifecycle.exchange_order_id = Some(exchange_id.clone());
            lifecycle.status = OrderStatus::Submitted;
            lifecycle
                .transitions
                .push(crate::execution::state_machine::OrderTransition {
                    from: OrderStatus::Pending,
                    to: OrderStatus::Submitted,
                    timestamp_ms: now * 1000,
                    metadata: None,
                });
            self.orders
                .write()
                .await
                .insert(exchange_id.clone(), lifecycle);
            return Ok(exchange_id);
        }

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
                lifecycle
                    .transitions
                    .push(crate::execution::state_machine::OrderTransition {
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

    /// Live mode: apply venue-reported fills to the order book. Marks the
    /// matching order Closed at the reported price and updates positions.
    pub async fn apply_external_fills(&self, fills: Vec<Fill>) {
        for fill in fills {
            let mut orders = self.orders.write().await;
            let Some(order) = orders.get_mut(&fill.order_id) else {
                continue;
            };
            if order.status != OrderStatus::Submitted && order.status != OrderStatus::Open {
                continue;
            }
            let fill_qty = if fill.size > dec!(0) {
                fill.size
            } else {
                order.packet.size
            };
            order.filled_size = fill_qty;
            order.fill_price = Some(fill.price);
            order.status = OrderStatus::Closed;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            order
                .transitions
                .push(crate::execution::state_machine::OrderTransition {
                    from: OrderStatus::Open,
                    to: OrderStatus::Closed,
                    timestamp_ms: now * 1000,
                    metadata: Some(format!(
                        "live filled {} @ {}",
                        order.packet.size, fill.price
                    )),
                });
            drop(orders);
            let _ = self.update_position(&fill.order_id).await;
        }
    }

    // ── Fill evaluation ───────────────────────────────────────────────

    /// Evaluate resting limit/stop orders against the current mid via the
    /// backend. Returns `(order_id, fill_price)` for every fill.
    /// v8.2: evaluates ONLY `symbol`'s open orders — the engine is shared
    /// across instances, and evaluating every symbol's orders at one
    /// symbol's mid cross-fills brackets (e.g. an ETH tick would trigger
    /// BTC's stop at ETH's price).
    pub async fn evaluate_order_fills(
        &self,
        symbol: &str,
        current_mid_price: Decimal,
    ) -> Vec<(String, Decimal)> {
        let mut fills = Vec::new();

        let orders_to_check: Vec<(String, OrderLifecycle)> = {
            let orders = self.orders.read().await;
            orders
                .iter()
                .filter(|(_, o)| o.status == OrderStatus::Open && o.packet.symbol == symbol)
                .map(|(id, o)| (id.clone(), o.clone()))
                .collect()
        };

        for (order_id, lifecycle) in orders_to_check {
            if let Some(fill_price) = self
                .backend
                .read()
                .await
                .evaluate_fill(&lifecycle, current_mid_price)
            {
                let mut orders = self.orders.write().await;
                if let Some(o) = orders.get_mut(&order_id) {
                    self.fill_market_order(o, current_mid_price).ok();
                    fills.push((order_id.clone(), fill_price));
                }
            }
        }

        for (order_id, _) in &fills {
            let _ = self.update_position(order_id).await;
        }

        fills
    }

    /// Fill an order at the given mid: spread applied, slippage recorded,
    /// status → Closed. Limit fills are clamped to the resting limit price.
    fn fill_market_order(
        &self,
        lifecycle: &mut OrderLifecycle,
        mid_price: Decimal,
    ) -> Result<(), String> {
        let spread_half =
            Decimal::from_f64_retain(self.fee_config.simulated_spread_pct / 100.0 / 2.0)
                .unwrap_or(dec!(0));
        let mut fill_price = if lifecycle.packet.side == OrderSide::Buy {
            mid_price * (dec!(1) + spread_half)
        } else {
            mid_price * (dec!(1) - spread_half)
        };

        // Never fill worse than a resting limit price.
        if lifecycle.packet.order_type == OrderType::Limit {
            if let Some(limit) = lifecycle.packet.price {
                fill_price = if lifecycle.packet.side == OrderSide::Buy {
                    fill_price.min(limit)
                } else {
                    fill_price.max(limit)
                };
            }
        }

        let fill_qty = lifecycle.packet.size;
        lifecycle.filled_size = fill_qty;
        lifecycle.fill_price = Some(fill_price);

        let slippage = if fill_price > dec!(0) && mid_price > dec!(0) {
            let abs_diff = if fill_price > mid_price {
                fill_price - mid_price
            } else {
                mid_price - fill_price
            };
            Some((abs_diff / mid_price * dec!(10000)).to_f64().unwrap_or(0.0))
        } else {
            None
        };
        lifecycle.slippage_bps = slippage;

        lifecycle.status = OrderStatus::Closed;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        lifecycle
            .transitions
            .push(crate::execution::state_machine::OrderTransition {
                from: OrderStatus::Open,
                to: OrderStatus::Closed,
                timestamp_ms: now * 1000,
                metadata: Some(format!("filled {} @ {}", fill_qty, fill_price)),
            });

        Ok(())
    }

    // ── Position accounting ───────────────────────────────────────────

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
        let fee_pct_dec = if is_market {
            Decimal::from_f64_retain(self.fee_config.taker_fee_pct / 100.0).unwrap_or(dec!(0))
        } else {
            Decimal::from_f64_retain(self.fee_config.maker_fee_pct / 100.0).unwrap_or(dec!(0))
        };
        let notional = fill_qty * fill_price;
        let fee = notional * fee_pct_dec;

        let mut positions = self.positions.write().await;
        let mut equity = self.equity.write().await;

        if lifecycle.packet.reduce_only {
            if let Some(pos) = positions.remove(&symbol) {
                let pnl = match pos.direction {
                    config_models::Direction::Long => (fill_price - pos.entry_price) * pos.size,
                    config_models::Direction::Short => (pos.entry_price - fill_price) * pos.size,
                };
                *equity += pnl - fee;

                let dir_str = match pos.direction {
                    config_models::Direction::Long => "LONG",
                    config_models::Direction::Short => "SHORT",
                };
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;

                let exit_reason = lifecycle
                    .packet
                    .metadata
                    .get("exit_reason")
                    .cloned()
                    .unwrap_or_default();
                let trigger_source = lifecycle
                    .packet
                    .metadata
                    .get("trigger_source")
                    .cloned()
                    .unwrap_or_else(|| exit_reason.clone());

                // Record the close outcome for the PME safety ladder.
                self.last_close.write().await.insert(
                    symbol.clone(),
                    CloseOutcome {
                        is_loss: pnl < dec!(0),
                        exit_reason: exit_reason.clone(),
                        pnl,
                    },
                );

                self.persist_paper_trade(
                    &symbol,
                    dir_str,
                    pos.entry_price,
                    fill_price,
                    pos.size,
                    pnl,
                    fee,
                    pos.opened_at_ms,
                    now_ms,
                    &trigger_source,
                )
                .await;
                self.persist_trade_telemetry(
                    &symbol,
                    dir_str,
                    pos.entry_price,
                    fill_price,
                    pos.size,
                    pnl,
                    fee,
                    &trigger_source,
                    pos.opened_at_ms,
                    now_ms,
                    exit_reason.as_str(),
                )
                .await;
                self.persist_equity_snapshot().await;
            }
        } else {
            let direction = match lifecycle.packet.side {
                OrderSide::Buy => config_models::Direction::Long,
                OrderSide::Sell => config_models::Direction::Short,
            };
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            positions.insert(
                symbol.clone(),
                PaperPosition {
                    symbol,
                    size: fill_qty,
                    entry_price: fill_price,
                    direction,
                    unrealized_pnl: dec!(0),
                    realized_pnl: dec!(0),
                    opened_at_ms: now_ms,
                    mfe_pct: 0.0,
                    mae_pct: 0.0,
                },
            );
            *equity -= fee;
        }

        Ok(())
    }

    pub async fn get_position(&self, symbol: &str) -> Option<PaperPosition> {
        self.positions.read().await.get(symbol).cloned()
    }

    /// v7 PME: update a position's unrealized PnL against the current mid
    /// (mark-to-market). Called every executor tick for open positions.
    pub async fn mark_to_market(&self, symbol: &str, mid: Decimal) {
        let mut positions = self.positions.write().await;
        if let Some(pos) = positions.get_mut(symbol) {
            pos.unrealized_pnl = match pos.direction {
                config_models::Direction::Long => (mid - pos.entry_price) * pos.size,
                config_models::Direction::Short => (pos.entry_price - mid) * pos.size,
            };
            // v10: MFE/MAE tracking (percent move from entry, signed by
            // direction so favorable is always positive).
            if pos.entry_price > dec!(0) {
                let move_pct = match pos.direction {
                    config_models::Direction::Long => ((mid - pos.entry_price) / pos.entry_price)
                        .to_f64()
                        .unwrap_or(0.0),
                    config_models::Direction::Short => ((pos.entry_price - mid) / pos.entry_price)
                        .to_f64()
                        .unwrap_or(0.0),
                };
                pos.mfe_pct = pos.mfe_pct.max(move_pct * 100.0);
                pos.mae_pct = pos.mae_pct.min(move_pct * 100.0);
            }
        }
    }

    /// v7 PME: take (and clear) the most recent close outcome for a symbol.
    /// Consumed by the setup executor to feed `SafetyManager::record_trade_outcome`.
    pub async fn take_last_close(&self, symbol: &str) -> Option<CloseOutcome> {
        self.last_close.write().await.remove(symbol)
    }

    /// Market-close a position in the correct direction, cancelling any
    /// resting bracket orders first. `exit_reason` is persisted with the trade.
    pub async fn close_position(
        &self,
        symbol: &str,
        current_mid_price: Decimal,
        exit_reason: &str,
    ) -> Result<(), String> {
        let pos = self.get_position(symbol).await.ok_or("No open position")?;

        // Bracket cleanup first — a market close must never leave resting orders.
        self.cancel_orders_for_symbol(symbol).await;

        let side = match pos.direction {
            config_models::Direction::Long => OrderSide::Sell,
            config_models::Direction::Short => OrderSide::Buy,
        };

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("exit_reason".to_string(), exit_reason.to_string());

        let packet = OrderPacket {
            client_order_id: format!("close_{}_{}", symbol, exit_reason),
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Market,
            price: None,
            size: pos.size,
            reduce_only: true,
            is_emergency_liquidation: true,
            associated_position_id: None,
            metadata,
        };

        let _ = self.submit_order(packet, current_mid_price).await?;
        Ok(())
    }

    pub async fn cancel_order(&self, order_id: &str, symbol: &str) -> Result<(), String> {
        // Live mode: also cancel at the venue.
        if *self.mode.read().await == ExecutionMode::Live {
            let _ = self
                .backend
                .read()
                .await
                .cancel_order(order_id, symbol)
                .await;
        }
        let mut orders = self.orders.write().await;
        if let Some(o) = orders.get_mut(order_id) {
            if o.status == OrderStatus::Open || o.status == OrderStatus::Submitted {
                o.status = OrderStatus::Cancelled;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                o.transitions
                    .push(crate::execution::state_machine::OrderTransition {
                        from: OrderStatus::Open,
                        to: OrderStatus::Cancelled,
                        timestamp_ms: now * 1000,
                        metadata: None,
                    });
            }
        }
        Ok(())
    }

    /// Cancel every resting (Open) order for a symbol — bracket cleanup.
    pub async fn cancel_orders_for_symbol(&self, symbol: &str) {
        let ids: Vec<String> = {
            let orders = self.orders.read().await;
            orders
                .iter()
                .filter(|(_, o)| o.packet.symbol == symbol && o.status == OrderStatus::Open)
                .map(|(id, _)| id.clone())
                .collect()
        };
        for id in ids {
            let _ = self.cancel_order(&id, symbol).await;
        }
    }

    pub async fn cancel_all_orders(&self) {
        let mut orders = self.orders.write().await;
        orders.clear();
    }

    /// Backwards-compatible equity reader (f64 wire boundary).
    pub async fn get_equity(&self) -> f64 {
        self.equity.read().await.to_f64().unwrap_or(0.0)
    }

    /// Canonical equity reader (Decimal).
    pub async fn get_equity_decimal(&self) -> Decimal {
        *self.equity.read().await
    }

    /// Total committed margin (sum of position notional / cross leverage).
    pub async fn committed_margin(&self) -> Decimal {
        let positions = self.positions.read().await;
        let leverage = *self.cross_leverage.read().await;
        if leverage == 0 {
            return dec!(0);
        }
        let notional: Decimal = positions.values().map(|p| p.size * p.entry_price).sum();
        notional / Decimal::from(leverage)
    }

    // ── Funding ───────────────────────────────────────────────────────

    /// 8h funding settlement on open positions.
    pub async fn settle_funding(&self) {
        let positions = self.positions.read().await;
        if positions.is_empty() {
            return;
        }

        let rate =
            Decimal::from_f64_retain(self.fee_config.funding_rate_8h / 100.0).unwrap_or(dec!(0));

        let settlement_details: Vec<(String, Decimal)> = positions
            .iter()
            .map(|(sym, pos)| {
                let notional = pos.size * pos.entry_price;
                (sym.clone(), notional * rate)
            })
            .collect();

        drop(positions);

        let total_settlement: Decimal = settlement_details
            .iter()
            .map(|(_, payment)| -(*payment))
            .sum();

        let mut equity = self.equity.write().await;
        *equity += total_settlement;

        if total_settlement.abs() > dec!(0.0001) {
            eprintln!(
                "💰 FUNDING: Settlement applied — {} (rate={:.4}%)",
                total_settlement, self.fee_config.funding_rate_8h
            );
        }

        self.persist_equity_snapshot().await;
    }

    // ── Persistence (canonical schemas) ───────────────────────────────

    async fn persist_paper_trade(
        &self,
        symbol: &str,
        direction: &str,
        entry_price: Decimal,
        exit_price: Decimal,
        size: Decimal,
        pnl: Decimal,
        _fee: Decimal,
        entry_ts_ms: u64,
        exit_ts_ms: i64,
        trigger: &str,
    ) {
        if let Some(ref pool) = self.pool {
            let notional = entry_price * size;
            let roi = if notional > dec!(0) {
                pnl / notional * dec!(100)
            } else {
                dec!(0)
            };
            let _ = sqlx::query(
                "INSERT INTO paper_trades \
                 (symbol, direction, entry_price, exit_price, size, \
                  realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger, session_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )
            .bind(symbol)
            .bind(direction)
            .bind(entry_price.to_string())
            .bind(exit_price.to_string())
            .bind(size.to_string())
            .bind(pnl.to_string())
            .bind(roi.to_string())
            .bind(entry_ts_ms as i64)
            .bind(exit_ts_ms)
            .bind(trigger)
            .bind(self.current_session_id().await)
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
        pnl: Decimal,
        fee: Decimal,
        trigger_source: &str,
        entry_ts_ms: u64,
        exit_ts_ms: i64,
        exit_reason: &str,
    ) {
        if let Some(ref pool) = self.pool {
            let notional = entry_price * size;
            let roi = if notional > dec!(0) {
                pnl / notional * dec!(100)
            } else {
                dec!(0)
            };
            let source = if trigger_source.is_empty() {
                exit_reason.to_string()
            } else {
                trigger_source.to_string()
            };
            let _ = sqlx::query(
                "INSERT INTO trade_telemetry_history \
                 (exchange, symbol, direction, entry_timestamp, exit_timestamp, \
                  entry_price, exit_price, size, commission_fees, funding_fees, \
                  realized_pnl, roi_percentage, trigger_source, session_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            )
            .bind("paper")
            .bind(symbol)
            .bind(direction)
            .bind(entry_ts_ms as i64)
            .bind(exit_ts_ms)
            .bind(entry_price.to_string())
            .bind(exit_price.to_string())
            .bind(size.to_string())
            .bind(fee.to_string())
            .bind("0")
            .bind(pnl.to_string())
            .bind(roi.to_string())
            .bind(&source)
            .bind(self.current_session_id().await)
            .execute(pool.as_ref())
            .await;
        }
    }

    async fn persist_equity_snapshot(&self) {
        if let Some(ref pool) = self.pool {
            let equity = *self.equity.read().await;
            let unrealized: Decimal = {
                let positions = self.positions.read().await;
                positions.values().map(|p| p.unrealized_pnl).sum()
            };
            let cash = equity - unrealized;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let _ = sqlx::query(
                "INSERT INTO portfolio_equity_history \
                 (timestamp, total_value, cash_balance, unrealized_pnl, session_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(now)
            .bind(equity.to_string())
            .bind(cash.to_string())
            .bind(unrealized.to_string())
            .bind(self.current_session_id().await)
            .execute(pool.as_ref())
            .await;
        }
    }

    /// Persist an executor event to `automation_activity` + the in-memory ring.
    pub async fn log_activity(&self, instance_id: &str, symbol: &str, event: &str, detail: &str) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        {
            let mut ring = self.activity.write().await;
            ring.push(ActivityEntry {
                instance_id: instance_id.to_string(),
                symbol: symbol.to_string(),
                ts_ms: ts,
                event: event.to_string(),
                detail: detail.to_string(),
            });
            let len = ring.len();
            if len > 500 {
                ring.drain(0..len - 500);
            }
        }
        if let Some(ref pool) = self.pool {
            let _ = sqlx::query(
                "INSERT INTO automation_activity (instance_id, symbol, ts_ms, event, detail, session_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(instance_id)
            .bind(symbol)
            .bind(ts as i64)
            .bind(event)
            .bind(detail)
            .bind(self.current_session_id().await)
            .execute(pool.as_ref())
            .await;
        }
    }

    /// Activity ring buffer filtered by instance, newest first.
    pub async fn activity_for(&self, instance_id: &str) -> Vec<ActivityEntry> {
        let ring = self.activity.read().await;
        let mut out: Vec<ActivityEntry> = ring
            .iter()
            .filter(|a| a.instance_id == instance_id)
            .cloned()
            .collect();
        out.reverse();
        out
    }

    // ── Open-state persistence (restart recovery) ─────────────────────

    pub async fn persist_open_state(&self, instance_id: &str, payload: &str) {
        if let Some(ref pool) = self.pool {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let _ = sqlx::query(
                "INSERT OR REPLACE INTO tae_open_state \
                 (instance_id, symbol, saved_at_ms, tracked_setup_json, \
                  entry_order_json, bracket_tp_json, bracket_sl_json, \
                  position_json, equity, realized_pnl) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )
            .bind(instance_id)
            .bind("")
            .bind(now)
            .bind(payload)
            .bind("")
            .bind("")
            .bind("")
            .bind("")
            .bind("")
            .bind("")
            .execute(pool.as_ref())
            .await;
        }
    }

    pub async fn load_open_state(&self, instance_id: &str) -> Option<(String, u64)> {
        let pool = self.pool.as_ref()?;
        sqlx::query_as::<_, (String, i64)>(
            "SELECT tracked_setup_json, saved_at_ms FROM tae_open_state WHERE instance_id = ?1",
        )
        .bind(instance_id)
        .fetch_optional(pool.as_ref())
        .await
        .ok()
        .flatten()
        .map(|(json, saved)| (json, saved as u64))
    }

    pub async fn clear_open_state(&self, instance_id: &str) {
        if let Some(ref pool) = self.pool {
            let _ = sqlx::query("DELETE FROM tae_open_state WHERE instance_id = ?1")
                .bind(instance_id)
                .execute(pool.as_ref())
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config_models::Direction;
    use rust_decimal_macros::dec;

    fn engine() -> Arc<ExecutionEngine> {
        Arc::new(ExecutionEngine::new(
            crate::paper_trading::FeesConfig::default(),
        ))
    }

    async fn open_long(e: &ExecutionEngine, symbol: &str, entry: Decimal) {
        e.submit_order(
            OrderPacket {
                client_order_id: format!("t_{}", symbol),
                symbol: symbol.to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Market,
                price: None,
                size: dec!(1),
                reduce_only: false,
                is_emergency_liquidation: false,
                associated_position_id: None,
                metadata: Default::default(),
            },
            entry,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn mark_to_market_updates_unrealized_pnl() {
        let e = engine();
        open_long(&e, "BTC-USDC", dec!(100)).await;

        e.mark_to_market("BTC-USDC", dec!(110)).await;
        let pos = e.get_position("BTC-USDC").await.unwrap();
        // Entry fills at mid + spread half (0.005%): 100.005.
        assert!((pos.unrealized_pnl - dec!(9.995)).abs() < dec!(0.0001));

        e.mark_to_market("BTC-USDC", dec!(95)).await;
        let pos = e.get_position("BTC-USDC").await.unwrap();
        assert!((pos.unrealized_pnl - dec!(-5.005)).abs() < dec!(0.0001));
    }

    #[tokio::test]
    async fn mark_to_market_short_direction() {
        let e = engine();
        e.submit_order(
            OrderPacket {
                client_order_id: "t_short".to_string(),
                symbol: "ETH-USDC".to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::Market,
                price: None,
                size: dec!(2),
                reduce_only: false,
                is_emergency_liquidation: false,
                associated_position_id: None,
                metadata: Default::default(),
            },
            dec!(100),
        )
        .await
        .unwrap();

        e.mark_to_market("ETH-USDC", dec!(90)).await;
        let pos = e.get_position("ETH-USDC").await.unwrap();
        assert_eq!(pos.direction, Direction::Short);
        // Short entry fills at mid - spread half: 99.995.
        assert!((pos.unrealized_pnl - dec!(19.99)).abs() < dec!(0.0001));
    }

    #[tokio::test]
    async fn take_last_close_records_outcome_once() {
        let e = engine();
        open_long(&e, "BTC-USDC", dec!(100)).await;

        let mut meta = std::collections::HashMap::new();
        meta.insert("exit_reason".to_string(), "sl".to_string());
        e.submit_order(
            OrderPacket {
                client_order_id: "close1".to_string(),
                symbol: "BTC-USDC".to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::Market,
                price: None,
                size: dec!(1),
                reduce_only: true,
                is_emergency_liquidation: false,
                associated_position_id: None,
                metadata: meta,
            },
            dec!(90),
        )
        .await
        .unwrap();

        let outcome = e.take_last_close("BTC-USDC").await.unwrap();
        assert!(outcome.is_loss);
        assert_eq!(outcome.exit_reason, "sl");
        // Entry at 100.005, close at 89.9955 → pnl ≈ -10.0095.
        assert!(outcome.pnl < dec!(-10));

        // Single-consume: second take returns None.
        assert!(e.take_last_close("BTC-USDC").await.is_none());
    }

    #[tokio::test]
    async fn take_last_close_win_is_not_loss() {
        let e = engine();
        open_long(&e, "BTC-USDC", dec!(100)).await;
        e.submit_order(
            OrderPacket {
                client_order_id: "close2".to_string(),
                symbol: "BTC-USDC".to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::Market,
                price: None,
                size: dec!(1),
                reduce_only: true,
                is_emergency_liquidation: false,
                associated_position_id: None,
                metadata: Default::default(),
            },
            dec!(120),
        )
        .await
        .unwrap();

        let outcome = e.take_last_close("BTC-USDC").await.unwrap();
        assert!(!outcome.is_loss);
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    type RecordingHandles = (
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        Arc<tokio::sync::RwLock<Vec<Fill>>>,
    );

    /// Records venue calls; fills are injected by the test.
    struct RecordingBackend {
        submits: Arc<AtomicUsize>,
        cancels: Arc<AtomicUsize>,
        fills: Arc<tokio::sync::RwLock<Vec<Fill>>>,
    }

    impl RecordingBackend {
        fn new() -> (Self, RecordingHandles) {
            let submits = Arc::new(AtomicUsize::new(0));
            let cancels = Arc::new(AtomicUsize::new(0));
            let fills = Arc::new(tokio::sync::RwLock::new(Vec::new()));
            let b = Self {
                submits: submits.clone(),
                cancels: cancels.clone(),
                fills: fills.clone(),
            };
            (b, (submits, cancels, fills))
        }
    }

    #[async_trait::async_trait]
    impl ExecutionBackend for RecordingBackend {
        fn mode(&self) -> ExecutionMode {
            ExecutionMode::Live
        }
        fn evaluate_fill(&self, _o: &OrderLifecycle, _m: Decimal) -> Option<Decimal> {
            None
        }
        async fn submit_order(&self, _p: &OrderPacket) -> Result<String, String> {
            self.submits.fetch_add(1, AtomicOrdering::SeqCst);
            Ok("hl_1".to_string())
        }
        async fn cancel_order(&self, _id: &str, _sym: &str) -> Result<(), String> {
            self.cancels.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
        }
        async fn poll_fills(&self) -> Vec<Fill> {
            self.fills.read().await.clone()
        }
        async fn fetch_equity(&self) -> Result<f64, String> {
            Ok(10_000.0)
        }
    }

    fn engine() -> Arc<ExecutionEngine> {
        Arc::new(ExecutionEngine::new(
            crate::paper_trading::FeesConfig::default(),
        ))
    }

    #[tokio::test]
    async fn live_submit_routes_to_backend() {
        let (backend, (submits, _cancels, _fills)) = RecordingBackend::new();
        let e = engine();
        e.set_live_backend(Box::new(backend)).await;

        let id = e
            .submit_order(
                OrderPacket {
                    client_order_id: "t1".to_string(),
                    symbol: "BTC-USDC".to_string(),
                    side: OrderSide::Buy,
                    order_type: OrderType::Limit,
                    price: Some(dec!(95)),
                    size: dec!(1),
                    reduce_only: false,
                    is_emergency_liquidation: false,
                    associated_position_id: None,
                    metadata: Default::default(),
                },
                dec!(100),
            )
            .await
            .unwrap();
        assert_eq!(id, "hl_1");
        assert_eq!(submits.load(AtomicOrdering::SeqCst), 1);
        let orders = e.orders.read().await;
        assert_eq!(orders.get("hl_1").unwrap().status, OrderStatus::Submitted);
    }

    #[tokio::test]
    async fn apply_external_fills_opens_position() {
        let (backend, (_submits, _cancels, fills)) = RecordingBackend::new();
        let e = engine();
        e.set_live_backend(Box::new(backend)).await;
        e.submit_order(
            OrderPacket {
                client_order_id: "t2".to_string(),
                symbol: "BTC-USDC".to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Limit,
                price: Some(dec!(95)),
                size: dec!(1),
                reduce_only: false,
                is_emergency_liquidation: false,
                associated_position_id: None,
                metadata: Default::default(),
            },
            dec!(100),
        )
        .await
        .unwrap();

        *fills.write().await = vec![Fill {
            order_id: "hl_1".to_string(),
            price: dec!(94),
            size: dec!(1),
        }];
        let fills = e.backend.read().await.poll_fills().await;
        e.apply_external_fills(fills).await;

        let orders = e.orders.read().await;
        assert_eq!(orders.get("hl_1").unwrap().status, OrderStatus::Closed);
        drop(orders);
        let pos = e.get_position("BTC-USDC").await.unwrap();
        assert_eq!(pos.entry_price, dec!(94));
        assert_eq!(pos.size, dec!(1));
    }

    #[tokio::test]
    async fn live_cancel_delegates_to_venue() {
        let (backend, (_submits, cancels, _fills)) = RecordingBackend::new();
        let e = engine();
        e.set_live_backend(Box::new(backend)).await;
        e.submit_order(
            OrderPacket {
                client_order_id: "t3".to_string(),
                symbol: "BTC-USDC".to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Limit,
                price: Some(dec!(95)),
                size: dec!(1),
                reduce_only: false,
                is_emergency_liquidation: false,
                associated_position_id: None,
                metadata: Default::default(),
            },
            dec!(100),
        )
        .await
        .unwrap();

        e.cancel_order("hl_1", "BTC-USDC").await.unwrap();
        assert_eq!(cancels.load(AtomicOrdering::SeqCst), 1);
        let orders = e.orders.read().await;
        assert_eq!(orders.get("hl_1").unwrap().status, OrderStatus::Cancelled);
    }

    #[tokio::test]
    async fn set_paper_backend_restores_simulation() {
        let (backend, (_submits, _cancels, _fills)) = RecordingBackend::new();
        let e = engine();
        e.set_live_backend(Box::new(backend)).await;
        assert_eq!(e.mode().await, ExecutionMode::Live);

        e.set_paper_backend().await;
        assert_eq!(e.mode().await, ExecutionMode::Paper);

        // Paper path: market order fills instantly.
        let id = e
            .submit_order(
                OrderPacket {
                    client_order_id: "t4".to_string(),
                    symbol: "BTC-USDC".to_string(),
                    side: OrderSide::Buy,
                    order_type: OrderType::Market,
                    price: None,
                    size: dec!(1),
                    reduce_only: false,
                    is_emergency_liquidation: false,
                    associated_position_id: None,
                    metadata: Default::default(),
                },
                dec!(100),
            )
            .await
            .unwrap();
        let orders = e.orders.read().await;
        assert_eq!(orders.get(&id).unwrap().status, OrderStatus::Closed);
    }

    /// v8.2 regression: the engine is shared across instances — evaluating
    /// fills must only touch the TICKED symbol's orders. An ETH tick at
    /// 3.4k must not fill BTC's stop at 77k (the pre-v8.2 cross-fill bug).
    #[tokio::test]
    async fn evaluate_fills_isolates_symbols() {
        let e = ExecutionEngine::new(FeesConfig::default());
        e.set_initial_equity(dec!(1000)).await;
        // Open a BTC LONG position + arm a BTC stop below it.
        e.submit_order(
            OrderPacket {
                client_order_id: "btc_entry".into(),
                symbol: "BTC-USDC".into(),
                side: OrderSide::Buy,
                order_type: OrderType::Limit,
                price: Some(dec!(78000)),
                size: dec!(1),
                reduce_only: false,
                is_emergency_liquidation: false,
                associated_position_id: None,
                metadata: Default::default(),
            },
            dec!(77900),
        )
        .await
        .unwrap();
        // Fill the entry on its own symbol's tick (fills evaluate per
        // tick, not at submit).
        e.evaluate_order_fills("BTC-USDC", dec!(77900)).await;
        assert!(e.get_position("BTC-USDC").await.is_some());
        e.submit_order(
            OrderPacket {
                client_order_id: "btc_sl".into(),
                symbol: "BTC-USDC".into(),
                side: OrderSide::Sell,
                order_type: OrderType::Stop,
                price: Some(dec!(77000)),
                size: dec!(1),
                reduce_only: true,
                is_emergency_liquidation: false,
                associated_position_id: None,
                metadata: Default::default(),
            },
            dec!(77900),
        )
        .await
        .unwrap();

        // An ETH tick (far below BTC's stop) must NOT fill the BTC stop.
        e.evaluate_order_fills("ETH-USDC", dec!(3400)).await;
        assert!(
            e.get_position("BTC-USDC").await.is_some(),
            "BTC position must survive an unrelated ETH tick"
        );
        // A BTC tick at/below the stop fills it.
        e.evaluate_order_fills("BTC-USDC", dec!(76800)).await;
        assert!(
            e.get_position("BTC-USDC").await.is_none(),
            "BTC stop fills on its own symbol's tick"
        );
    }
}
