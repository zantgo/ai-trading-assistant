//! # Portfolio Supervisor Crate
//!
//! Portfolio Management Engine (PME) and Trade Automation Engine (TAE):
//! instance lifecycle, the v7 setup executor, the unified execution engine,
//! position sizing, capital constraints, session state, risk/commission
//! math, and pipeline orchestration. Owns the in-memory transactional sync
//! between the cold-path Decimal ledger and the indicator pipeline.

#![allow(clippy::too_many_arguments)]

pub use workspace_state::WorkspaceState;

pub mod capital_layer;
pub mod commission;
pub mod execution;
pub mod exposure_layer;
pub mod instance;
pub mod lifecycle;
pub mod paper_trading;
pub mod portfolio_equity;
pub mod portfolio_layer;
pub mod position_layer;
pub mod profile_evaluation;
pub mod registry;
pub mod registry_context;
pub mod risk_calculator;
pub mod safety;
pub mod session;
pub mod setup_executor;
pub mod workspace_state;
