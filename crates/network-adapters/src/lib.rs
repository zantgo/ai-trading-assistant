//! # Network Adapters Crate
//!
//! All exchange WebSocket / REST I/O, candle gap reconstruction, NTP clock
//! monitoring, and connection-quality tracking. Depends on `core-domain`
//! (transport types) and `config-models` (endpoint URLs, candle config) but
//! **never** on the database or the Axum gateway.

pub mod adapters;
pub mod clock_monitor;
pub mod connection_quality_tracker;
pub mod exchange_status_tracker;
pub mod median_filter;
pub mod orchestrator;
pub mod pipeline_reliability;
