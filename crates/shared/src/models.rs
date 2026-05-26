//! # Domain Data Models
//!
//! This module defines the common data structures representing market telemetry.
//! It includes raw ticker prices, generic EMAs, and consolidated candle bars.

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
    pub bid_size: Option<Decimal>,
    pub ask_size: Option<Decimal>,
    pub funding_rate: Option<Decimal>,
    
    // Consolidated Candle OHLC Bars
    pub open: Option<Decimal>,
    pub high: Option<Decimal>,
    pub low: Option<Decimal>,
    pub close: Option<Decimal>,
    pub volume: Option<Decimal>,
    
    // Bollinger Bands
    pub bb_upper: Option<Decimal>,
    pub bb_middle: Option<Decimal>,
    pub bb_lower: Option<Decimal>,
    
    // Standalone ATR, VWAP and ADX lines (adx_plus and adx_minus added)
    pub atr_14: Option<Decimal>,
    pub vwap: Option<Decimal>,
    pub adx_14: Option<Decimal>,
    pub adx_plus: Option<Decimal>,
    pub adx_minus: Option<Decimal>,
    
    // Generic-named Exponential Moving Averages
    pub ema_fast: Option<Decimal>,
    pub ema_medium: Option<Decimal>,
    pub ema_slow: Option<Decimal>,
    pub ema_long: Option<Decimal>,
    
    // Relative Strength Index
    pub rsi_14: Option<Decimal>,
    
    // MACD (Line, Signal, Histogram)
    pub macd_line: Option<Decimal>,
    pub macd_signal: Option<Decimal>,
    pub macd_hist: Option<Decimal>,
    
    // Squeeze Momentum Indicator (State & Histogram Value)
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<Decimal>,
}
