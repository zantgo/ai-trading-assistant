//! # Shared Crate Entrypoint
//!
//! This module registers and exports the shared domain logic, making
//! data models and technical analysis indicator modules accessible
//! to the main trading engine or other workspace consumers.

pub mod indicators;
pub mod jsonrpc;
pub mod jsonrpc_methods;
pub mod market_context;
pub mod models;
pub mod normalized;
pub mod alignment;
pub mod analysis;
pub mod risk;
pub mod advisory;
pub mod overview;
pub mod statistics;
pub mod decision_context;

pub use normalized::TriggerType;
