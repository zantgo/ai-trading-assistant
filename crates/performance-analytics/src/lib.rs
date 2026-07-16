//! # Performance Analytics Crate
//!
//! Performance Analytics Engine (PAE): dashboard statistics compilation,
//! regime-strategy optimizer, and the (currently dormant) forward-testing
//! performance evaluator. Reads from `database-storage` and produces
//! JSON-shaped reports for the API gateway.

pub mod performance_evaluator;
pub mod stats_compiler;
pub mod strategy_optimizer;
