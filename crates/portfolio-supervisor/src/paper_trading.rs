//! v7 TAE — compatibility shim.
//!
//! The v6 paper-trading engine was absorbed into the unified
//! [`ExecutionEngine`](crate::execution::engine::ExecutionEngine)
//! (`crates/portfolio-supervisor/src/execution/engine.rs`). The simulation
//! matching lives in [`PaperSimulation`](crate::execution::backend::PaperSimulation).
//! This module re-exports the historical surface (name, fee config, position
//! shape) so existing callers and configs keep working; it will be removed
//! in the v7 cleaning pass.

use config_models::Direction;
use rust_decimal::Decimal;

pub use crate::execution::engine::ExecutionEngine as PaperTradingEngine;
pub use crate::execution::engine::{ExecutionEngine, ReplayTrade};

/// Fee / cost configuration shared by the unified engine (both modes).
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

/// One open paper/live position.
#[derive(Debug, Clone)]
pub struct PaperPosition {
    pub symbol: String,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub direction: Direction,
    /// Per-position unrealized PnL (Decimal).
    pub unrealized_pnl: Decimal,
    /// Per-position realized PnL (Decimal).
    pub realized_pnl: Decimal,
    /// v7: entry timestamp (ms) — powers hold-time analytics and persistence.
    pub opened_at_ms: u64,
}

/// Mock snapshot builder used by backtest/replay tooling.
pub fn build_mock_snapshot(
    symbol: &str,
    mid_price: Decimal,
) -> core_domain::models::MarketSnapshot {
    let mut snapshot = core_domain::models::MarketSnapshot::default();
    snapshot.symbol = symbol.to_string();
    snapshot.mid_price = mid_price;
    snapshot.bid_price = mid_price;
    snapshot.ask_price = mid_price;
    snapshot.close = Some(mid_price);
    snapshot
}
