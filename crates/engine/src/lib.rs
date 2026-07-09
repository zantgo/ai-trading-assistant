// DB persistence, orchestrator, and pipeline layers legitimately take many
// positional parameters (column binds, multi-timeframe context). Refactoring
// them into parameter structs adds churn without clarity, so the lint is
// accepted crate-wide.
#![allow(clippy::too_many_arguments)]

pub mod adapters;
pub mod analyzer;
pub mod api_failover;
pub mod automation;
pub mod backtest;
pub mod candle_aggregator;
pub mod cli;
pub mod commission;
pub mod config;
pub mod db;
pub mod edges;
pub mod event_detector;
pub mod execution;
pub mod historical_analyst;
pub mod instance;
pub mod registry;
pub mod llm;
pub mod order_matcher;
pub mod paper_trading;
pub mod performance_evaluator;
pub mod portfolio_equity;
pub mod portfolio_optimizer;
pub mod portfolio_risk;
pub mod profile_evaluation;
pub mod risk_calculator;
pub mod risk_engine;
pub mod safety;
pub mod server;
pub mod services;
pub mod sr_engine;
pub mod stats_compiler;
pub mod strategy_optimizer;
pub mod stress_test;
pub mod trigger_engine;
pub mod workspace;
