//! # Portfolio Supervisor Crate
//!
//! Portfolio Management Engine (PME) and Trade Automation Engine (TAE):
//! instance lifecycle, position sizing, safety vetoes, capital constraints,
//! session state, risk/commission math, decision-profile evaluation, and
//! pipeline orchestration. Owns the in-memory transactional sync between
//! the cold-path Decimal ledger and the indicator pipeline.

#![allow(clippy::too_many_arguments)]

pub use workspace_state::WorkspaceState;

pub mod capital_layer;
pub mod cluster_refresh;
pub mod commission;
pub mod execution;
pub mod exposure_layer;
pub mod instance;
pub mod lifecycle;
pub mod paper_trading;
pub mod policy;
pub mod portfolio_equity;
pub mod portfolio_layer;
pub mod portfolio_risk;
pub mod position_layer;
pub mod profile_evaluation;
pub mod registry;
pub mod registry_context;
pub mod risk_calculator;
pub mod safety;
pub mod session;
pub mod trigger_engine;
pub mod veto_loop;
pub mod workspace_state;
