//! # Domain Data Models
//!
//! This module defines the common data structures representing market telemetry.
//! These structures act as the formal interface between different layers of the 
//! system, such as WebSocket consumers, database adapters, and indicator calculators.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Represents a unified price and liquidity snapshot of an asset's order book at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    /// Epoch timestamp in seconds.
    pub timestamp: u64,
    /// The ticker asset symbol (e.g., "ETH").
    pub symbol: String,
    /// The midpoint price between best bid and best ask.
    pub mid_price: Decimal,
    /// The highest price a buyer is willing to pay.
    pub bid_price: Decimal,
    /// The lowest price a seller is willing to accept.
    pub ask_price: Decimal,
    /// The current 8-hour funding rate on the swap contract, if applicable.
    pub funding_rate: Option<Decimal>,
}
