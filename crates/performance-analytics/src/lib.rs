//! # Performance Analytics Crate
//!
//! Performance Analytics Engine (PAE): dashboard statistics compilation,
//! trade reconstruction, strategy analytics (NHST), risk-adjusted metrics,
//! regime-strategy optimizer, and the performance evaluator.
//! Reads from `database-storage` and produces JSON-shaped reports for the API gateway.

pub mod backtest;
pub mod performance_evaluator;
pub mod performance_layer;
pub mod risk_analytics;
pub mod stats_compiler;
pub mod strategy_analytics;
pub mod strategy_optimizer;
pub mod trade_analytics;
