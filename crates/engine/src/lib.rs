// DB persistence and pipeline layers legitimately take many
// positional parameters (column binds, multi-timeframe context). Refactoring
// them into parameter structs adds churn without clarity, so the lint is
// accepted crate-wide.
#![allow(clippy::too_many_arguments)]

pub mod adapters;
pub mod analyzer;
pub mod candle_aggregator;
pub mod clock_monitor;
pub mod cluster_refresh;
pub mod commission;
pub mod config;
pub mod connection_quality;
pub mod db;
pub mod event_detector;
pub mod instance;
pub mod paper_trading;
pub mod performance_evaluator;
pub mod portfolio_equity;
pub mod portfolio_risk;
pub mod profile_evaluation;
pub mod registry;
pub mod risk_calculator;
pub mod safety;
pub mod server;
pub mod session;
pub mod sr_engine;
pub mod stats_compiler;
pub mod strategy_optimizer;
pub mod trigger_engine;
