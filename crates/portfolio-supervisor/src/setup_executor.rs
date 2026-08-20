//! v7 TAE — Setup Executor.
//!
//! Consumes the MME's top setup (the best Actionable/READY profile across
//! the 4 latest completed snapshots per symbol) and manages the trade
//! lifecycle: Idle → PendingEntry (limit @ zone midpoint) → PositionOpen
//! (TP/SL bracket) → Closed.
//!
//! Invalidation semantics (see docs/engines/trade-automation-engine/
//! 03-03-01-tae-overview-spec.md §6):
//!   - LEVEL:    price crossed the SL/invalidation level → pending cancelled /
//!     SL stop fills.
//!   - SIGNAL:   MME direction flipped opposite on a completed candle →
//!     pending cancelled / position closed at market.
//!   - REPLACED: a different setup type tops the ranking → pending cancelled.
//!
//! Neutral / STAND_ASIDE never invalidates an open position.

use config_models::{Direction, MinimalTaeConfig, OrderPacket, OrderSide, OrderType};
use core_domain::analysis::{MarketBias, OpportunityType, TradeViability};
use core_domain::models::MarketSnapshot;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::execution::engine::ExecutionEngine;
use crate::risk_calculator::{compute_risk, RiskCalculation, RiskCalculationInput};

/// The accepted setup — everything the executor needs to trade it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupPlan {
    pub symbol: String,
    pub direction: String, // "LONG" | "SHORT"
    pub setup_type: String,
    pub score: f64,
    pub source_tf: String,
    pub source_tf_secs: u64,
    pub entry_mid: Decimal,
    pub entry_zone_low: Decimal,
    pub entry_zone_high: Decimal,
    pub sl: Decimal,
    pub tp: Decimal,
    pub net_rr: f64,
    pub time_horizon: String,
    /// Idempotency key: symbol:direction:setup_type:candle_timestamp.
    pub fingerprint: String,
}

impl SetupPlan {}

/// Risk projection for the active setup (mirrors the RecommendationPanel's
/// "Projected Risk and Return" drawer, computed automatically).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupProjection {
    pub risk_capital: Decimal,
    pub position_size_units: Decimal,
    pub position_notional: Decimal,
    pub margin_required: Decimal,
    pub liquidation_price: Decimal,
    pub entry_fee_usd: Decimal,
    pub exit_fee_usd: Decimal,
    pub total_fees: Decimal,
    pub net_profit_usd: Decimal,
    pub roi_pct: Decimal,
    pub net_rr: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorPhase {
    Idle,
    PendingEntry,
    PositionOpen,
}

#[derive(Debug, Clone)]
pub struct SymbolState {
    pub phase: ExecutorPhase,
    pub fingerprint: String,
    pub tracked_setup: Option<SetupPlan>,
    pub projection: Option<SetupProjection>,
    pub entry_order_id: Option<String>,
    pub tp_order_id: Option<String>,
    pub sl_order_id: Option<String>,
    /// Candle timestamp that produced the last close — re-entry guard.
    pub last_closed_candle_ts: u64,
}

impl Default for SymbolState {
    fn default() -> Self {
        Self {
            phase: ExecutorPhase::Idle,
            fingerprint: String::new(),
            tracked_setup: None,
            projection: None,
            entry_order_id: None,
            tp_order_id: None,
            sl_order_id: None,
            last_closed_candle_ts: 0,
        }
    }
}

/// Per-tick context supplied by the daemon (reads instance state).
#[derive(Clone)]
pub struct TickContext {
    pub safety_allows_entry: bool,
    pub lifecycle_running: bool,
    /// Candle timestamp of the top snapshot (re-entry guard).
    pub candle_ts: u64,
    /// Per-instance SafetyManager (informational PME state). When present,
    /// the executor feeds `record_trade_outcome` on position close so the
    /// CAUTIOUS / SUSPENDED ladder stays accurate.
    pub safety: Option<Arc<crate::safety::SafetyManager>>,
    /// When `false` the executor evaluates setups and populates
    /// `tracked_setup` / `projection` (the would-be preview) but never
    /// submits, cancels, or closes orders. The daemon sets this to `false`
    /// for observe instances — the "ghost" radar.
    pub dispatch: bool,
}

/// Extract the top setup from the latest completed snapshots of the 4 TFs.
///
/// Selection: candidates = profiles with `preconditions_met > 0`,
/// `trade_viability == Actionable`, geometry consistent for the active side
/// (from `analysis.bias`); then the RR filter; then the highest score across
/// all timeframes (ties → faster TF wins).
pub fn extract_top_setup(snapshots: &[&MarketSnapshot], min_net_rr: f64) -> Option<SetupPlan> {
    let mut best: Option<SetupPlan> = None;

    for snap in snapshots {
        if snap.is_completed != Some(true) {
            continue;
        }
        let Some(decision) = snap.decision_context.as_ref() else {
            continue;
        };
        let Some(analysis) = snap.analysis.as_ref() else {
            continue;
        };
        let Some(opp) = snap.opportunity.as_ref() else {
            continue;
        };

        let direction = match analysis.bias {
            MarketBias::StrongBullish | MarketBias::Bullish => "LONG",
            MarketBias::StrongBearish | MarketBias::Bearish => "SHORT",
            MarketBias::Neutral => continue,
        };

        for profile in &opp.profiles {
            if profile.preconditions_met == 0 {
                continue;
            }
            if profile.trade_viability != Some(TradeViability::Actionable) {
                continue;
            }

            let (entry, target, invalidation, geometry_ok) = if direction == "LONG" {
                (
                    profile.long_entry_zone.as_ref(),
                    profile.long_target_zone.as_ref(),
                    profile.long_invalidation_level.unwrap_or(0.0),
                    profile.long_geometry_consistent,
                )
            } else {
                (
                    profile.short_entry_zone.as_ref(),
                    profile.short_target_zone.as_ref(),
                    profile.short_invalidation_level.unwrap_or(0.0),
                    profile.short_geometry_consistent,
                )
            };
            if !geometry_ok {
                continue;
            }
            let (Some(entry), Some(target)) = (entry, target) else {
                continue;
            };
            if entry.low <= 0.0 || entry.high <= 0.0 || invalidation <= 0.0 {
                continue;
            }

            let entry_mid = Decimal::from_f64_retain((entry.low + entry.high) / 2.0)?;
            let tp = Decimal::from_f64_retain((target.low + target.high) / 2.0)?;
            let sl = Decimal::from_f64_retain(invalidation)?;

            let net_rr = decision.expected_reward_risk_ratio.max(0.0);
            if net_rr < min_net_rr {
                continue;
            }

            let score = profile.display_score.unwrap_or(profile.score);
            let setup_type = opportunity_type_str(&profile.opportunity_type);
            let source_tf = snap
                .timeframe_slot
                .as_ref()
                .map(|s| s.as_str().to_string())
                .unwrap_or_else(|| snap.timeframe_secs.to_string());
            let fingerprint = format!(
                "{}:{}:{}:{}",
                snap.symbol, direction, setup_type, snap.timestamp
            );

            let plan = SetupPlan {
                symbol: snap.symbol.clone(),
                direction: direction.to_string(),
                setup_type,
                score,
                source_tf,
                source_tf_secs: snap.timeframe_secs,
                entry_mid,
                entry_zone_low: Decimal::from_f64_retain(entry.low)?,
                entry_zone_high: Decimal::from_f64_retain(entry.high)?,
                sl,
                tp,
                net_rr,
                time_horizon: opp.time_horizon.clone(),
                fingerprint,
            };

            // Best wins; ties → faster TF wins.
            let replace = match &best {
                Some(b) => {
                    b.score < score || (b.score == score && b.source_tf_secs > plan.source_tf_secs)
                }
                None => true,
            };
            if replace {
                best = Some(plan);
            }
        }
    }

    best
}

fn opportunity_type_str(ot: &OpportunityType) -> String {
    match ot {
        OpportunityType::TrendContinuation => "TrendContinuation",
        OpportunityType::Breakout => "Breakout",
        OpportunityType::Pullback => "Pullback",
        OpportunityType::MeanReversion => "MeanReversion",
        OpportunityType::Reversal => "Reversal",
        OpportunityType::LiquiditySqueeze => "LiquiditySqueeze",
        OpportunityType::Scalp => "Scalp",
        OpportunityType::NoClearOpportunity => "NoClearOpportunity",
    }
    .to_string()
}

/// The v7 setup executor — one instance per daemon, one `SymbolState` per
/// symbol. All effects go through the unified `ExecutionEngine`.
pub struct SetupExecutor {
    pub min_net_rr: f64,
    pub risk_per_trade_pct: f64,
    pub max_position_size_usd: Option<f64>,
    pub max_open_positions: u32,
    pub engine: Arc<ExecutionEngine>,
    state: RwLock<HashMap<String, SymbolState>>,
}

impl SetupExecutor {
    pub fn new(engine: Arc<ExecutionEngine>, cfg: &MinimalTaeConfig) -> Self {
        Self {
            min_net_rr: cfg.min_net_rr,
            risk_per_trade_pct: cfg.risk_per_trade_pct,
            max_position_size_usd: cfg.max_position_size_usd,
            max_open_positions: cfg.max_open_positions,
            engine,
            state: RwLock::new(HashMap::new()),
        }
    }

    pub async fn state(&self, symbol: &str) -> SymbolState {
        self.state
            .read()
            .await
            .get(symbol)
            .cloned()
            .unwrap_or_default()
    }

    /// One executor tick per instance: evaluate the current top setup and
    /// advance the per-symbol state machine.
    pub async fn tick(
        &self,
        instance_id: &str,
        symbol: &str,
        snapshots: Vec<&MarketSnapshot>,
        mid: Decimal,
        ctx: TickContext,
    ) {
        let top = extract_top_setup(&snapshots, self.min_net_rr);
        let mut state = self.state.write().await;
        let entry = state.entry(symbol.to_string()).or_default();

        match entry.phase {
            ExecutorPhase::Idle => {
                self.tick_idle(instance_id, symbol, &top, mid, ctx, entry)
                    .await
            }
            ExecutorPhase::PendingEntry => {
                self.tick_pending(instance_id, symbol, &top, mid, entry)
                    .await
            }
            ExecutorPhase::PositionOpen => {
                self.tick_position(instance_id, symbol, &top, mid, ctx, entry)
                    .await
            }
        }
    }

    async fn tick_idle(
        &self,
        instance_id: &str,
        symbol: &str,
        top: &Option<SetupPlan>,
        mid: Decimal,
        ctx: TickContext,
        entry: &mut SymbolState,
    ) {
        let Some(plan) = top else { return };
        if !ctx.lifecycle_running || !ctx.safety_allows_entry {
            return;
        }
        // No re-entry on the candle that produced the last close.
        if ctx.candle_ts > 0 && ctx.candle_ts == entry.last_closed_candle_ts {
            return;
        }
        // One position per symbol + global cap.
        let positions = self.engine.positions.read().await;
        if positions.contains_key(symbol) {
            return;
        }
        let open_count = positions.len() as u32;
        drop(positions);
        if open_count >= self.max_open_positions {
            return;
        }

        // Sizing via the canonical risk calculator (same function as
        // POST /api/risk/calculate — the drawer's engine).
        let Some(projection) = self.project(plan, mid).await else {
            return;
        };
        let size = projection.position_size_units;
        if size <= dec!(0) {
            return;
        }

        let side = if plan.direction == "LONG" {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("trigger_source".to_string(), plan.setup_type.clone());

        let packet = OrderPacket {
            client_order_id: format!("setup_entry_{}", symbol),
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Limit,
            price: Some(plan.entry_mid),
            size,
            reduce_only: false,
            is_emergency_liquidation: false,
            associated_position_id: None,
            metadata,
        };

        if !ctx.dispatch {
            // Ghost (observe) evaluation: record the would-be setup and
            // projection without dispatching any order. No entry_order_id
            // is stored, so tick_pending never sees a fill; the LEVEL /
            // SIGNAL / REPLACED invalidation logic keeps re-evaluating the
            // candidate until it dies or is replaced.
            entry.phase = ExecutorPhase::PendingEntry;
            entry.fingerprint = plan.fingerprint.clone();
            entry.tracked_setup = Some(plan.clone());
            entry.projection = Some(projection);
            self.log(
                instance_id,
                symbol,
                "setup_accepted",
                &format!(
                    "GHOST {} {} entry={} sl={} tp={} rr={:.2} score={:.0} tf={}",
                    plan.direction,
                    plan.setup_type,
                    plan.entry_mid,
                    plan.sl,
                    plan.tp,
                    plan.net_rr,
                    plan.score,
                    plan.source_tf
                ),
            )
            .await;
        } else {
            match self.engine.submit_order(packet, mid).await {
                Ok(order_id) => {
                    entry.phase = ExecutorPhase::PendingEntry;
                    entry.fingerprint = plan.fingerprint.clone();
                    entry.tracked_setup = Some(plan.clone());
                    entry.projection = Some(projection);
                    entry.entry_order_id = Some(order_id);
                    self.log(
                        instance_id,
                        symbol,
                        "setup_accepted",
                        &format!(
                            "{} {} entry={} sl={} tp={} rr={:.2} score={:.0} tf={}",

                            plan.direction,
                            plan.setup_type,
                            plan.entry_mid,
                            plan.sl,
                            plan.tp,
                            plan.net_rr,
                            plan.score,
                            plan.source_tf
                        ),
                    )
                    .await;
                }
                Err(e) => {
                    self.log(instance_id, symbol, "entry_rejected", &e).await;
                }
            }
        }
    }

    async fn tick_pending(
        &self,
        instance_id: &str,
        symbol: &str,
        top: &Option<SetupPlan>,
        mid: Decimal,
        entry: &mut SymbolState,
    ) {
        // ── Entry filled? ──
        let has_position = self.engine.get_position(symbol).await.is_some();
        if has_position {
            entry.phase = ExecutorPhase::PositionOpen;
            self.arm_bracket(instance_id, symbol, entry).await;
            self.log(instance_id, symbol, "entry_filled", &entry.fingerprint)
                .await;
            return;
        }

        // ── LEVEL invalidation: price crossed the SL while pending ──
        if let Some(plan) = &entry.tracked_setup {
            let breached = if plan.direction == "LONG" {
                mid < plan.sl
            } else {
                mid > plan.sl
            };
            if breached {
                if let Some(id) = entry.entry_order_id.take() {
                    let _ = self.engine.cancel_order(&id, symbol).await;
                }
                self.log(
                    instance_id,
                    symbol,
                    "invalidated_level",
                    &format!("pending entry cancelled — price crossed SL {}", plan.sl),
                )
                .await;
                self.reset(entry);
                return;
            }
        }

        // ── SIGNAL invalidation: direction flipped on a completed candle ──
        if let Some(plan) = top {
            if let Some(tracked) = &entry.tracked_setup {
                if plan.direction != tracked.direction {
                    if let Some(id) = entry.entry_order_id.take() {
                        let _ = self.engine.cancel_order(&id, symbol).await;
                    }
                    self.log(
                        instance_id,
                        symbol,
                        "invalidated_signal",
                        &format!(
                            "pending entry cancelled — recommendation flipped to {}",
                            plan.direction
                        ),
                    )
                    .await;
                    self.reset(entry);
                    return;
                }
                // ── REPLACED: a different setup type now tops the ranking ──
                if plan.setup_type != tracked.setup_type {
                    if let Some(id) = entry.entry_order_id.take() {
                        let _ = self.engine.cancel_order(&id, symbol).await;
                    }
                    self.log(
                        instance_id,
                        symbol,
                        "cancelled_replaced",
                        &format!(
                            "pending entry cancelled — setup replaced by {}",
                            plan.setup_type
                        ),
                    )
                    .await;
                    self.reset(entry);
                }
            }
        }
    }

    async fn tick_position(
        &self,
        instance_id: &str,
        symbol: &str,
        top: &Option<SetupPlan>,
        mid: Decimal,
        ctx: TickContext,
        entry: &mut SymbolState,
    ) {
        let has_position = self.engine.get_position(symbol).await.is_some();
        if !has_position {
            // Closed by TP/SL/manual — feed the PME safety ladder, then
            // back to Idle, armed against the candle that produced the close.
            if let Some(outcome) = self.engine.take_last_close(symbol).await {
                if let Some(safety) = &ctx.safety {
                    safety.record_trade_outcome(symbol, outcome.is_loss).await;
                }
            }
            let closed_on = if ctx.candle_ts > 0 {
                ctx.candle_ts
            } else {
                entry.last_closed_candle_ts
            };
            entry.last_closed_candle_ts = closed_on;
            entry.tp_order_id = None;
            entry.sl_order_id = None;
            entry.entry_order_id = None;
            self.log(instance_id, symbol, "position_closed", &entry.fingerprint)
                .await;
            self.reset(entry);
            return;
        }

        // ── SIGNAL invalidation while open: close at market ──
        if let Some(plan) = top {
            if let Some(tracked) = &entry.tracked_setup {
                if plan.direction != tracked.direction {
                    match self
                        .engine
                        .close_position(symbol, mid, "invalidated_signal")
                        .await
                    {
                        Ok(_) => {
                            self.log(
                                instance_id,
                                symbol,
                                "invalidated_signal",
                                &format!(
                                    "position closed at market — recommendation flipped to {}",
                                    plan.direction
                                ),
                            )
                            .await;
                        }
                        Err(e) => {
                            self.log(instance_id, symbol, "close_error", &e).await;
                        }
                    }
                }
            }
        }
    }

    /// Arm the TP limit + SL stop bracket after an entry fill.
    async fn arm_bracket(&self, instance_id: &str, symbol: &str, entry: &mut SymbolState) {
        let Some(plan) = entry.tracked_setup.clone() else {
            return;
        };
        if entry.tp_order_id.is_some() && entry.sl_order_id.is_some() {
            return;
        }
        let pos = match self.engine.get_position(symbol).await {
            Some(p) => p,
            None => return,
        };
        let exit_side = match pos.direction {
            Direction::Long => OrderSide::Sell,
            Direction::Short => OrderSide::Buy,
        };

        let mut tp_meta = std::collections::HashMap::new();
        tp_meta.insert("exit_reason".to_string(), "tp".to_string());
        tp_meta.insert("trigger_source".to_string(), plan.setup_type.clone());
        let tp_packet = OrderPacket {
            client_order_id: format!("tp_{}_{}", symbol, entry.fingerprint),
            symbol: symbol.to_string(),
            side: exit_side,
            order_type: OrderType::Limit,
            price: Some(plan.tp),
            size: pos.size,
            reduce_only: true,
            is_emergency_liquidation: false,
            associated_position_id: None,
            metadata: tp_meta,
        };

        let mut sl_meta = std::collections::HashMap::new();
        sl_meta.insert("exit_reason".to_string(), "sl".to_string());
        sl_meta.insert("trigger_source".to_string(), plan.setup_type.clone());
        let sl_packet = OrderPacket {
            client_order_id: format!("sl_{}_{}", symbol, entry.fingerprint),
            symbol: symbol.to_string(),
            side: exit_side,
            order_type: OrderType::Stop,
            price: Some(plan.sl),
            size: pos.size,
            reduce_only: true,
            is_emergency_liquidation: false,
            associated_position_id: None,
            metadata: sl_meta,
        };

        let mid = pos.entry_price;
        if entry.tp_order_id.is_none() {
            if let Ok(id) = self.engine.submit_order(tp_packet, mid).await {
                entry.tp_order_id = Some(id);
            }
        }
        if entry.sl_order_id.is_none() {
            if let Ok(id) = self.engine.submit_order(sl_packet, mid).await {
                entry.sl_order_id = Some(id);
            }
        }
        self.log(
            instance_id,
            symbol,
            "bracket_armed",
            &format!("TP {} / SL {}", plan.tp, plan.sl),
        )
        .await;
    }

    /// Canonical sizing + projection via `compute_risk` (risk capital =
    /// equity × risk_per_trade_pct). Notional clamped to the configured cap.
    async fn project(&self, plan: &SetupPlan, _mid: Decimal) -> Option<SetupProjection> {
        let equity = self.engine.get_equity_decimal().await;
        let risk_capital = equity * Decimal::from_f64_retain(self.risk_per_trade_pct / 100.0)?;
        let leverage = *self.engine.cross_leverage.read().await;
        let taker_pct = self.engine.fee_config.taker_fee_pct;

        let input = RiskCalculationInput {
            capital: risk_capital,
            max_risk_pct: dec!(100),
            leverage: leverage as i32,
            direction: plan.direction.clone(),
            entry_price: plan.entry_mid,
            stop_loss_price: plan.sl,
            take_profit_price: plan.tp,
            commission_pct: Decimal::from_f64_retain(taker_pct)?,
            funding_rate_8h: Decimal::from_f64_retain(self.engine.fee_config.funding_rate_8h)?,
            spread: Decimal::from_f64_retain(self.engine.fee_config.simulated_spread_pct)?,
            atr_value: None,
            atr_multiplier: None,
            atr_target_rr: None,
            use_dynamic_atr: false,
            min_tick_size: None,
        };

        let calc: RiskCalculation = compute_risk(&input).ok()?;
        let mut size = calc.position_size_units;
        if let Some(cap) = self.max_position_size_usd {
            let notional = size * plan.entry_mid;
            if notional > Decimal::from_f64_retain(cap)? {
                size = Decimal::from_f64_retain(cap)? / plan.entry_mid;
            }
        }
        let notional = size * plan.entry_mid;
        let entry_fee = notional * Decimal::from_f64_retain(taker_pct / 100.0)?;
        let exit_fee = entry_fee;
        let net_profit = calc.net_pnl;
        let margin = calc.margin_required;
        let roi = if margin > dec!(0) {
            net_profit / margin * dec!(100)
        } else {
            dec!(0)
        };

        Some(SetupProjection {
            risk_capital,
            position_size_units: size,
            position_notional: notional,
            margin_required: margin,
            liquidation_price: calc.liquidation_price,
            entry_fee_usd: entry_fee,
            exit_fee_usd: exit_fee,
            total_fees: entry_fee + exit_fee,
            net_profit_usd: net_profit,
            roi_pct: roi,
            net_rr: calc.risk_reward_ratio,
        })
    }

    fn reset(&self, entry: &mut SymbolState) {
        entry.phase = ExecutorPhase::Idle;
        entry.fingerprint = String::new();
        entry.tracked_setup = None;
        entry.projection = None;
        entry.entry_order_id = None;
        entry.tp_order_id = None;
        entry.sl_order_id = None;
    }

    async fn log(&self, instance_id: &str, symbol: &str, event: &str, detail: &str) {
        self.engine
            .log_activity(instance_id, symbol, event, detail)
            .await;
    }

    /// Restart recovery: persist the tracked setup fingerprint so the daemon
    /// can log a recovery-flatten on boot. Pending entries are cancelled and
    /// positions are flattened at the last known mark (recovery flatten).
    pub async fn persist_open_state(&self, instance_id: &str, symbol: &str) {
        let state = self.state.read().await;
        if let Some(entry) = state.get(symbol) {
            if entry.phase != ExecutorPhase::Idle {
                let payload = serde_json::to_string(&entry.tracked_setup).unwrap_or_default();
                self.engine.persist_open_state(instance_id, &payload).await;
                return;
            }
        }
        self.engine.clear_open_state(instance_id).await;
    }

    /// On boot: if a tracked setup was persisted, log a recovery-flatten
    /// event and start fresh (equity is restored separately by the daemon).
    pub async fn recover(&self, instance_id: &str, symbol: &str) {
        if let Some((payload, _saved_at)) = self.engine.load_open_state(instance_id).await {
            if !payload.is_empty() {
                self.log(
                    instance_id,
                    symbol,
                    "recovery_flatten",
                    "daemon restarted with open state — pending entries cancelled, \
                     positions flattened at last known mark; equity restored",
                )
                .await;
            }
            self.engine.clear_open_state(instance_id).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config_models::{Direction, ExecutionMode, OrderStatus, OrderType};
    use core_domain::analysis::{
        AnalysisMatrix, MarketBias, OpportunityProfile, OpportunityType, PriceRange, SetupQuality,
        TradeViability,
    };
    use core_domain::decision_context::DecisionContext;
    use core_domain::models::MarketSnapshot;
    use core_domain::opportunity::OpportunityMatrix;
    use core_domain::risk::{RiskDimension, RiskLevel, RiskState};
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    // ── builders ───────────────────────────────────────────────────────

    fn rr_dim(score: f64) -> RiskDimension {
        RiskDimension {
            score,
            level: RiskLevel::Low,
            state: RiskState::Stable,
            confidence: 80.0,
            evidence: vec![],
            volatility_to_spread_ratio: None,
        }
    }

    fn decision(rr: f64, readiness: &str) -> DecisionContext {
        DecisionContext {
            score: 60.0,
            bias: "Bullish".to_string(),
            score_confidence: 0.6,
            entry_danger: rr_dim(20.0),
            expected_reward_risk_ratio: rr,
            trade_readiness: readiness.to_string(),
            contributing_indicators: vec![],
            long_probability: 60.0,
            short_probability: 30.0,
            hold_probability: 10.0,
            net_bias_pct: 30.0,
            lean_floor_applied: false,
        }
    }

    fn analysis(bias: MarketBias) -> AnalysisMatrix {
        let mut a = AnalysisMatrix::empty("BTC-USDC");
        a.bias = bias;
        a
    }

    fn long_profile(score: f64, rr: f64, viability: TradeViability) -> OpportunityProfile {
        OpportunityProfile {
            opportunity_type: OpportunityType::TrendContinuation,
            score,
            preconditions_met: 4,
            preconditions_total: 4,
            notes: String::new(),
            direction_family: None,
            long_entry_zone: Some(PriceRange {
                low: 90.0,
                high: 100.0,
            }),
            long_target_zone: Some(PriceRange {
                low: 120.0,
                high: 130.0,
            }),
            long_invalidation_level: Some(85.0),
            short_entry_zone: None,
            short_target_zone: None,
            short_invalidation_level: None,
            long_expected_rr_internal: rr,
            short_expected_rr_internal: 0.0,
            trade_viability: Some(viability),
            long_geometry_consistent: true,
            short_geometry_consistent: false,
            scoring_factors: None,
            display_score: Some(score),
        }
    }

    fn short_profile(score: f64) -> OpportunityProfile {
        OpportunityProfile {
            opportunity_type: OpportunityType::Pullback,
            score,
            preconditions_met: 4,
            preconditions_total: 4,
            notes: String::new(),
            direction_family: None,
            long_entry_zone: None,
            long_target_zone: None,
            long_invalidation_level: None,
            short_entry_zone: Some(PriceRange {
                low: 110.0,
                high: 120.0,
            }),
            short_target_zone: Some(PriceRange {
                low: 80.0,
                high: 90.0,
            }),
            short_invalidation_level: Some(125.0),
            long_expected_rr_internal: 0.0,
            short_expected_rr_internal: 2.0,
            trade_viability: Some(TradeViability::Actionable),
            long_geometry_consistent: false,
            short_geometry_consistent: true,
            scoring_factors: None,
            display_score: Some(score),
        }
    }

    fn snapshot(
        tf_secs: u64,
        bias: MarketBias,
        profiles: Vec<OpportunityProfile>,
        rr: f64,
        ts: u64,
        mid: f64,
    ) -> MarketSnapshot {
        MarketSnapshot {
            symbol: "BTC-USDC".to_string(),
            timeframe_secs: tf_secs,
            timestamp: ts,
            is_completed: Some(true),
            mid_price: Decimal::from_f64_retain(mid).unwrap(),
            bid_price: Decimal::from_f64_retain(mid).unwrap(),
            ask_price: Decimal::from_f64_retain(mid).unwrap(),
            close: Some(Decimal::from_f64_retain(mid).unwrap()),
            analysis: Some(analysis(bias)),
            decision_context: Some(decision(rr, "READY")),
            opportunity: Some(OpportunityMatrix {
                symbol: "BTC-USDC".to_string(),
                primary_opportunity: OpportunityType::TrendContinuation,
                opportunity_score: 60.0,
                setup_quality: SetupQuality::Strong,
                profiles,
                forecast_confidence: 0.7,
                contributing_signals: vec![],
                invalidation_note: String::new(),
                entry_zone: PriceRange {
                    low: 90.0,
                    high: 100.0,
                },
                target_zone: PriceRange {
                    low: 120.0,
                    high: 130.0,
                },
                time_horizon: "SWING".to_string(),
                long_entry_zone: PriceRange {
                    low: 90.0,
                    high: 100.0,
                },
                long_target_zone: PriceRange {
                    low: 120.0,
                    high: 130.0,
                },
                long_invalidation_level: 85.0,
                short_entry_zone: PriceRange {
                    low: 0.0,
                    high: 0.0,
                },
                short_target_zone: PriceRange {
                    low: 0.0,
                    high: 0.0,
                },
                short_invalidation_level: 0.0,
                long_expected_rr_internal: 2.0,
                short_expected_rr_internal: 0.0,
                long_gross_rr_internal: 2.0,
                short_gross_rr_internal: 0.0,
                invalidation_level: 85.0,
                direction_family: None,
                long_geometry_consistent: true,
                short_geometry_consistent: false,
                neutral_reference_bracket: None,
                confluent_entry_levels: vec![],
                confluent_target_levels: vec![],
                confluent_invalidation_levels: vec![],
            }),
            ..MarketSnapshot::default()
        }
    }
    fn executor_with(min_rr: f64, max_pos: u32) -> (Arc<ExecutionEngine>, SetupExecutor) {
        let engine = Arc::new(ExecutionEngine::new(
            crate::paper_trading::FeesConfig::default(),
        ));
        let cfg = config_models::MinimalTaeConfig {
            enabled: true,
            risk_per_trade_pct: 1.0,
            min_net_rr: min_rr,
            max_position_size_usd: None,
            max_open_positions: max_pos,
            entry_mode: "zone_midpoint".to_string(),
            invalidate_on: "direction_flip".to_string(),
        };
        let ex = SetupExecutor::new(engine.clone(), &cfg);
        (engine, ex)
    }

    fn ctx(ts: u64) -> TickContext {
        TickContext {
            safety_allows_entry: true,
            lifecycle_running: true,
            candle_ts: ts,
            safety: None,
            dispatch: true,
        }
    }

    fn snap_refs<'a>(v: &'a [&'a MarketSnapshot]) -> Vec<&'a MarketSnapshot> {
        v.to_vec()
    }

    // ── extract_top_setup ──────────────────────────────────────────────

    #[test]
    fn extracts_best_profile_across_timeframes() {
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(55.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        let macro_snap = snapshot(
            3600,
            MarketBias::Bullish,
            vec![long_profile(88.0, 3.0, TradeViability::Actionable)],
            3.0,
            2000,
            105.0,
        );
        let plan = extract_top_setup(&snap_refs(&[&micro, &macro_snap]), 1.0).unwrap();
        assert_eq!(plan.score, 88.0);
        assert_eq!(plan.setup_type, "TrendContinuation");
        assert_eq!(plan.direction, "LONG");
        assert_eq!(plan.entry_mid, dec!(95));
        assert_eq!(plan.sl, dec!(85));
        assert_eq!(plan.tp, dec!(125));
    }

    #[test]
    fn rejects_sub_floor_rr() {
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(90.0, 0.5, TradeViability::Actionable)],
            0.5,
            1000,
            105.0,
        );
        assert!(extract_top_setup(&snap_refs(&[&micro]), 1.0).is_none());
    }

    #[test]
    fn rejects_neutral_bias() {
        let micro = snapshot(
            60,
            MarketBias::Neutral,
            vec![long_profile(90.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        assert!(extract_top_setup(&snap_refs(&[&micro]), 1.0).is_none());
    }

    #[test]
    fn rejects_non_actionable() {
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(90.0, 0.5, TradeViability::Qualifying)],
            0.5,
            1000,
            105.0,
        );
        assert!(extract_top_setup(&snap_refs(&[&micro]), 1.0).is_none());
    }

    #[test]
    fn picks_short_when_bias_bearish() {
        let micro = snapshot(
            60,
            MarketBias::Bearish,
            vec![short_profile(70.0)],
            2.0,
            1000,
            105.0,
        );
        let plan = extract_top_setup(&snap_refs(&[&micro]), 1.0).unwrap();
        assert_eq!(plan.direction, "SHORT");
        assert_eq!(plan.entry_mid, dec!(115));
    }

    // ── executor state machine ─────────────────────────────────────────

    #[tokio::test]
    async fn accept_places_limit_entry_at_midpoint() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;

        let st = ex.state("BTC-USDC").await;
        assert_eq!(st.phase, ExecutorPhase::PendingEntry);
        let id = st.entry_order_id.unwrap();
        let orders = engine.orders.read().await;
        let order = orders.get(&id).unwrap();
        assert_eq!(order.packet.order_type, OrderType::Limit);
        assert_eq!(order.packet.price, Some(dec!(95)));
        assert_eq!(order.status, OrderStatus::Open);
    }

    #[tokio::test]
    async fn ghost_dispatch_false_populates_preview_but_never_dispatches() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );

        let mut ghost_ctx = ctx(1000);
        ghost_ctx.dispatch = false;
        ex.tick(
            "i1",
            "BTC-USDC",
            snap_refs(&[&micro]),
            dec!(105),
            ghost_ctx.clone(),
        )
        .await;

        // Ghost state: pending entry with the would-be setup + projection,
        // but no order was ever submitted.
        let st = ex.state("BTC-USDC").await;
        assert_eq!(st.phase, ExecutorPhase::PendingEntry);
        assert!(st.tracked_setup.is_some());
        assert!(st.projection.is_some());
        assert_eq!(st.entry_order_id, None);
        assert!(engine.orders.read().await.is_empty());

        // No fills can open a position — there is no order to fill.
        engine.evaluate_order_fills(dec!(94)).await;
        ex.tick(
            "i1",
            "BTC-USDC",
            snap_refs(&[&micro]),
            dec!(94),
            ghost_ctx.clone(),
        )
        .await;
        assert!(engine.get_position("BTC-USDC").await.is_none());
        assert_eq!(ex.state("BTC-USDC").await.entry_order_id, None);

        // LEVEL invalidation still resets the ghost candidate.
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(80), ghost_ctx)
            .await;
        assert_eq!(ex.state("BTC-USDC").await.phase, ExecutorPhase::Idle);
    }

    #[tokio::test]
    async fn entry_fills_then_bracket_armed() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;

        // Price pulls into the zone → fill.
        engine.evaluate_order_fills(dec!(94)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(94), ctx(1000))
            .await;

        let st = ex.state("BTC-USDC").await;
        assert_eq!(st.phase, ExecutorPhase::PositionOpen);
        let pos = engine.get_position("BTC-USDC").await.unwrap();
        assert_eq!(pos.direction, Direction::Long);
        assert!(st.tp_order_id.is_some());
        assert!(st.sl_order_id.is_some());

        // Bracket prices: TP limit sell @125, SL stop sell @85.
        let orders = engine.orders.read().await;
        let tp = orders.get(st.tp_order_id.as_ref().unwrap()).unwrap();
        assert_eq!(tp.packet.order_type, OrderType::Limit);
        assert_eq!(tp.packet.price, Some(dec!(125)));
        assert!(tp.packet.reduce_only);
        let sl = orders.get(st.sl_order_id.as_ref().unwrap()).unwrap();
        assert_eq!(sl.packet.order_type, OrderType::Stop);
        assert_eq!(sl.packet.price, Some(dec!(85)));
        assert!(sl.packet.reduce_only);
    }

    #[tokio::test]
    async fn tp_hit_closes_position_with_profit() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;
        engine.evaluate_order_fills(dec!(94)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(94), ctx(1000))
            .await;
        assert!(engine.get_position("BTC-USDC").await.is_some());

        // Price runs to TP.
        engine.evaluate_order_fills(dec!(126)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(126), ctx(1000))
            .await;

        assert!(engine.get_position("BTC-USDC").await.is_none());
        let st = ex.state("BTC-USDC").await;
        assert_eq!(st.phase, ExecutorPhase::Idle);
    }

    #[tokio::test]
    async fn level_invalidation_cancels_pending_entry() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;
        assert_eq!(
            ex.state("BTC-USDC").await.phase,
            ExecutorPhase::PendingEntry
        );

        // Price crashes below SL before filling.
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(84), ctx(1000))
            .await;

        let st = ex.state("BTC-USDC").await;
        assert_eq!(st.phase, ExecutorPhase::Idle);
        assert!(st.entry_order_id.is_none());
        let activity = engine.activity_for("i1").await;
        assert!(activity.iter().any(|a| a.event == "invalidated_level"));
    }

    #[tokio::test]
    async fn signal_flip_cancels_pending_entry() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;

        // Next candle: recommendation flips to SHORT.
        let flipped = snapshot(
            60,
            MarketBias::Bearish,
            vec![short_profile(70.0)],
            2.0,
            1001,
            105.0,
        );
        ex.tick(
            "i1",
            "BTC-USDC",
            snap_refs(&[&flipped]),
            dec!(105),
            ctx(1001),
        )
        .await;

        let st = ex.state("BTC-USDC").await;
        assert_eq!(st.phase, ExecutorPhase::Idle);
        let activity = engine.activity_for("i1").await;
        assert!(activity.iter().any(|a| a.event == "invalidated_signal"));
    }

    #[tokio::test]
    async fn replaced_setup_cancels_pending_entry() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;

        // New candle: same direction but a different setup type wins.
        let mut replaced = micro.clone();
        replaced.timestamp = 1001;
        let mut profile = long_profile(90.0, 2.5, TradeViability::Actionable);
        profile.opportunity_type = OpportunityType::Breakout;
        replaced.opportunity.as_mut().unwrap().profiles = vec![profile];
        replaced.opportunity.as_mut().unwrap().primary_opportunity = OpportunityType::Breakout;
        ex.tick(
            "i1",
            "BTC-USDC",
            snap_refs(&[&replaced]),
            dec!(105),
            ctx(1001),
        )
        .await;

        let st = ex.state("BTC-USDC").await;
        assert_eq!(st.phase, ExecutorPhase::Idle);
        let activity = engine.activity_for("i1").await;
        assert!(activity.iter().any(|a| a.event == "cancelled_replaced"));
    }

    #[tokio::test]
    async fn signal_flip_closes_open_position_at_market() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;
        engine.evaluate_order_fills(dec!(94)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(94), ctx(1000))
            .await;
        assert!(engine.get_position("BTC-USDC").await.is_some());

        // Flip to SHORT while open → close at market.
        let flipped = snapshot(
            60,
            MarketBias::Bearish,
            vec![short_profile(70.0)],
            2.0,
            1001,
            94.0,
        );
        ex.tick(
            "i1",
            "BTC-USDC",
            snap_refs(&[&flipped]),
            dec!(94),
            ctx(1001),
        )
        .await;

        assert!(engine.get_position("BTC-USDC").await.is_none());
        let activity = engine.activity_for("i1").await;
        assert!(activity.iter().any(|a| a.event == "invalidated_signal"));
    }

    #[tokio::test]
    async fn neutral_holds_open_position() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;
        engine.evaluate_order_fills(dec!(94)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(94), ctx(1000))
            .await;

        // Neutral recommendation → hold.
        let neutral = snapshot(60, MarketBias::Neutral, vec![], 0.0, 1001, 94.0);
        ex.tick(
            "i1",
            "BTC-USDC",
            snap_refs(&[&neutral]),
            dec!(94),
            ctx(1001),
        )
        .await;

        assert!(engine.get_position("BTC-USDC").await.is_some());
        let st = ex.state("BTC-USDC").await;
        assert_eq!(st.phase, ExecutorPhase::PositionOpen);
    }

    #[tokio::test]
    async fn instant_fill_when_price_beyond_zone() {
        let (engine, ex) = executor_with(1.0, 1);
        // Price already below the entry zone (mid 80 < zone.low 90) —
        // the limit buy at 95 is marketable and fills immediately.
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            80.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(80), ctx(1000))
            .await;
        engine.evaluate_order_fills(dec!(80)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(80), ctx(1000))
            .await;

        let pos = engine.get_position("BTC-USDC").await;
        assert!(pos.is_some(), "marketable limit must fill immediately");
        let st = ex.state("BTC-USDC").await;
        assert_eq!(st.phase, ExecutorPhase::PositionOpen);
    }

    #[tokio::test]
    async fn gap_through_sl_closes_at_market() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;
        engine.evaluate_order_fills(dec!(94)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(94), ctx(1000))
            .await;
        assert!(engine.get_position("BTC-USDC").await.is_some());

        // Gap: mid opens far below SL (85).
        engine.evaluate_order_fills(dec!(80)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(80), ctx(1000))
            .await;

        assert!(engine.get_position("BTC-USDC").await.is_none());
        assert!(
            engine.get_equity().await < 10000.0,
            "gap SL must realize a loss"
        );
    }

    #[tokio::test]
    async fn safety_gate_blocks_new_entries() {
        let (_engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        let blocked_ctx = TickContext {
            safety_allows_entry: false,
            lifecycle_running: true,
            candle_ts: 1000,
            safety: None,
            dispatch: true,
        };
        ex.tick(
            "i1",
            "BTC-USDC",
            snap_refs(&[&micro]),
            dec!(105),
            blocked_ctx,
        )
        .await;
        assert_eq!(
            ex.state("BTC-USDC").await.phase,
            ExecutorPhase::Idle,
            "safety gate must block entries"
        );
    }

    #[tokio::test]
    async fn global_position_cap_blocks_second_symbol() {
        let (engine, ex) = executor_with(1.0, 1);
        // Open a position on ETH manually.
        engine
            .submit_order(
                OrderPacket {
                    client_order_id: "t1".to_string(),
                    symbol: "ETH-USDC".to_string(),
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

        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;
        assert_eq!(
            ex.state("BTC-USDC").await.phase,
            ExecutorPhase::Idle,
            "global cap of 1 must block the BTC entry"
        );
    }

    #[tokio::test]
    async fn no_reen_try_on_same_candle() {
        let (engine, ex) = executor_with(1.0, 1);
        let micro = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1000,
            105.0,
        );
        // Close a position first (open+fill+TP close on candle 1000).
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;
        engine.evaluate_order_fills(dec!(94)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(94), ctx(1000))
            .await;
        engine.evaluate_order_fills(dec!(126)).await;
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(126), ctx(1000))
            .await;
        assert_eq!(ex.state("BTC-USDC").await.phase, ExecutorPhase::Idle);

        // Same candle timestamp → no re-entry.
        ex.tick("i1", "BTC-USDC", snap_refs(&[&micro]), dec!(105), ctx(1000))
            .await;
        assert_eq!(ex.state("BTC-USDC").await.phase, ExecutorPhase::Idle);

        // Later candle → entry allowed again.
        let later = snapshot(
            60,
            MarketBias::Bullish,
            vec![long_profile(80.0, 2.0, TradeViability::Actionable)],
            2.0,
            1001,
            105.0,
        );
        ex.tick("i1", "BTC-USDC", snap_refs(&[&later]), dec!(105), ctx(1001))
            .await;
        assert_eq!(
            ex.state("BTC-USDC").await.phase,
            ExecutorPhase::PendingEntry
        );
    }

    #[tokio::test]
    async fn mode_is_paper_by_default_and_shared_ledger() {
        let (engine, _ex) = executor_with(1.0, 1);
        assert_eq!(engine.mode().await, ExecutionMode::Paper);
        // Fee/ledger path identical regardless of mode: open a long via
        // market order and verify the equity ledger deducts the fee.
        let before = engine.get_equity_decimal().await;
        engine
            .submit_order(
                OrderPacket {
                    client_order_id: "t2".to_string(),
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
        let after = engine.get_equity_decimal().await;
        assert!(after < before, "entry fee must be deducted from equity");
    }
}

#[cfg(test)]
mod safety_ladder_tests {
    use super::*;
    use core_domain::analysis::{
        MarketBias, OpportunityProfile, OpportunityType, PriceRange, SetupQuality, TradeViability,
    };
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    use crate::safety::SafetyManager;

    fn snap_ts(ts: u64, mid: f64) -> MarketSnapshot {
        let mut s = MarketSnapshot::default();
        s.symbol = "BTC-USDC".to_string();
        s.timeframe_secs = 60;
        s.timestamp = ts;
        s.is_completed = Some(true);
        s.mid_price = Decimal::from_f64_retain(mid).unwrap();
        s.bid_price = s.mid_price;
        s.ask_price = s.mid_price;
        s.close = Some(s.mid_price);
        let mut a = core_domain::analysis::AnalysisMatrix::empty("BTC-USDC");
        a.bias = MarketBias::Bullish;
        s.analysis = Some(a);
        s.decision_context = Some(decision_ctx(2.0));
        s.opportunity = Some(opp_profiles(vec![long_prof()]));
        s
    }

    fn decision_ctx(rr: f64) -> core_domain::decision_context::DecisionContext {
        core_domain::decision_context::DecisionContext {
            score: 60.0,
            bias: "Bullish".to_string(),
            score_confidence: 0.6,
            entry_danger: core_domain::risk::RiskDimension {
                score: 20.0,
                level: core_domain::risk::RiskLevel::Low,
                state: core_domain::risk::RiskState::Stable,
                confidence: 80.0,
                evidence: vec![],
                volatility_to_spread_ratio: None,
            },
            expected_reward_risk_ratio: rr,
            trade_readiness: "READY".to_string(),
            contributing_indicators: vec![],
            long_probability: 60.0,
            short_probability: 30.0,
            hold_probability: 10.0,
            net_bias_pct: 30.0,
            lean_floor_applied: false,
        }
    }

    fn long_prof() -> OpportunityProfile {
        OpportunityProfile {
            opportunity_type: OpportunityType::TrendContinuation,
            score: 80.0,
            preconditions_met: 4,
            preconditions_total: 4,
            notes: String::new(),
            direction_family: None,
            long_entry_zone: Some(PriceRange {
                low: 90.0,
                high: 100.0,
            }),
            long_target_zone: Some(PriceRange {
                low: 120.0,
                high: 130.0,
            }),
            long_invalidation_level: Some(85.0),
            short_entry_zone: None,
            short_target_zone: None,
            short_invalidation_level: None,
            long_expected_rr_internal: 2.0,
            short_expected_rr_internal: 0.0,
            trade_viability: Some(TradeViability::Actionable),
            long_geometry_consistent: true,
            short_geometry_consistent: false,
            scoring_factors: None,
            display_score: Some(80.0),
        }
    }

    fn opp_profiles(
        profiles: Vec<OpportunityProfile>,
    ) -> core_domain::opportunity::OpportunityMatrix {
        core_domain::opportunity::OpportunityMatrix {
            symbol: "BTC-USDC".to_string(),
            primary_opportunity: OpportunityType::TrendContinuation,
            opportunity_score: 60.0,
            setup_quality: SetupQuality::Strong,
            profiles,
            time_horizon: "SWING".to_string(),
            long_entry_zone: core_domain::analysis::PriceRange {
                low: 90.0,
                high: 100.0,
            },
            long_target_zone: core_domain::analysis::PriceRange {
                low: 120.0,
                high: 130.0,
            },
            long_invalidation_level: 85.0,
            long_expected_rr_internal: 2.0,
            ..core_domain::opportunity::OpportunityMatrix::default()
        }
    }

    #[tokio::test]
    async fn sl_close_records_loss_in_safety_ladder() {
        let engine = Arc::new(ExecutionEngine::new(
            crate::paper_trading::FeesConfig::default(),
        ));
        let safety = Arc::new(SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0));
        safety.set_initial_capital(dec!(1000)).await;
        let cfg = config_models::MinimalTaeConfig {
            enabled: true,
            risk_per_trade_pct: 1.0,
            min_net_rr: 1.0,
            max_position_size_usd: None,
            max_open_positions: 1,
            entry_mode: "zone_midpoint".to_string(),
            invalidate_on: "direction_flip".to_string(),
        };
        let ex = SetupExecutor::new(engine.clone(), &cfg);

        // Accept + fill at 94.
        let s1 = snap_ts(1000, 105.0);
        ex.tick(
            "i1",
            "BTC-USDC",
            vec![&s1],
            dec!(105),
            TickContext {
                safety_allows_entry: true,
                lifecycle_running: true,
                candle_ts: 1000,
                safety: Some(safety.clone()),
                dispatch: true,
            },
        )
        .await;
        engine.evaluate_order_fills(dec!(94)).await;
        ex.tick(
            "i1",
            "BTC-USDC",
            vec![&s1],
            dec!(94),
            TickContext {
                safety_allows_entry: true,
                lifecycle_running: true,
                candle_ts: 1000,
                safety: Some(safety.clone()),
                dispatch: true,
            },
        )
        .await;
        assert!(engine.get_position("BTC-USDC").await.is_some());

        // Price crashes through SL (85) → stop fills → executor records the loss.
        engine.evaluate_order_fills(dec!(80)).await;
        ex.tick(
            "i1",
            "BTC-USDC",
            vec![&s1],
            dec!(80),
            TickContext {
                safety_allows_entry: true,
                lifecycle_running: true,
                candle_ts: 1000,
                safety: Some(safety.clone()),
                dispatch: true,
            },
        )
        .await;

        assert!(engine.get_position("BTC-USDC").await.is_none());
        let losses = safety.consecutive_losses.read().await;
        assert_eq!(losses.get("BTC-USDC"), Some(&1));
    }

    #[tokio::test]
    async fn three_losses_raise_cautious_and_block_entries() {
        let engine = Arc::new(ExecutionEngine::new(
            crate::paper_trading::FeesConfig::default(),
        ));
        let safety = Arc::new(SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0));
        safety.set_initial_capital(dec!(1000)).await;
        let cfg = config_models::MinimalTaeConfig {
            enabled: true,
            risk_per_trade_pct: 1.0,
            min_net_rr: 1.0,
            max_position_size_usd: None,
            max_open_positions: 1,
            entry_mode: "zone_midpoint".to_string(),
            invalidate_on: "direction_flip".to_string(),
        };
        let ex = SetupExecutor::new(engine.clone(), &cfg);

        // Three losing round-trips: accept → fill → SL.
        for i in 0..3u64 {
            let s = snap_ts(1000 + i, 105.0);
            ex.tick(
                "i1",
                "BTC-USDC",
                vec![&s],
                dec!(105),
                TickContext {
                    safety_allows_entry: true,
                    lifecycle_running: true,
                    candle_ts: 1000 + i,
                    safety: Some(safety.clone()),
                dispatch: true,
                },
            )
            .await;
            engine.evaluate_order_fills(dec!(94)).await;
            ex.tick(
                "i1",
                "BTC-USDC",
                vec![&s],
                dec!(94),
                TickContext {
                    safety_allows_entry: true,
                    lifecycle_running: true,
                    candle_ts: 1000 + i,
                    safety: Some(safety.clone()),
                dispatch: true,
                },
            )
            .await;
            engine.evaluate_order_fills(dec!(80)).await;
            ex.tick(
                "i1",
                "BTC-USDC",
                vec![&s],
                dec!(80),
                TickContext {
                    safety_allows_entry: true,
                    lifecycle_running: true,
                    candle_ts: 1000 + i,
                    safety: Some(safety.clone()),
                dispatch: true,
                },
            )
            .await;
        }

        assert_eq!(
            *safety.safety_state.read().await,
            core_domain::portfolio::SafetyState::Cautious
        );

        // The executor's soft gate then blocks a new entry.
        let s4 = snap_ts(2000, 105.0);
        ex.tick(
            "i1",
            "BTC-USDC",
            vec![&s4],
            dec!(105),
            TickContext {
                safety_allows_entry: false,
                lifecycle_running: true,
                candle_ts: 2000,
                safety: Some(safety.clone()),
                dispatch: true,
            },
        )
        .await;
        assert_eq!(ex.state("BTC-USDC").await.phase, ExecutorPhase::Idle);
    }
}
