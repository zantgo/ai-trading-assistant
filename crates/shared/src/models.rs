//! # Domain Data Models
//!
//! This module defines the common data structures representing market telemetry.
//! It includes the raw ticker prices and all sliding-window calculated technical indicators.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Represents a unified price snapshot alongside all computed technical indicators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub timestamp: u64,
    pub symbol: String,
    pub mid_price: Decimal,
    pub bid_price: Decimal,
    pub ask_price: Decimal,
    pub funding_rate: Option<Decimal>,
    
    // Exponential Moving Averages
    pub ema_10: Option<Decimal>,
    pub ema_50: Option<Decimal>,
    pub ema_100: Option<Decimal>,
    pub ema_200: Option<Decimal>,
    
    // Relative Strength Index
    pub rsi_14: Option<Decimal>,
    
    // MACD (Line, Signal, Histogram)
    pub macd_line: Option<Decimal>,
    pub macd_signal: Option<Decimal>,
    pub macd_hist: Option<Decimal>,
    
    // Average Directional Index
    pub adx_14: Option<Decimal>,
    
    // Squeeze Momentum Indicator (State & Histogram Value)
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<Decimal>,
}
