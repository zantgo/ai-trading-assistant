//! # Domain Data Models
//!
//! This module defines the common data structures representing market telemetry.
//! It includes raw ticker prices, generic EMAs, and consolidated candle bars.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::normalized::Exchange;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<Exchange>,
    pub timeframe_secs: u64,
    pub timestamp: u64,
    pub symbol: String,
    pub is_completed: Option<bool>,
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
    pub average_volume: Option<Decimal>,
    pub rvol: Option<Decimal>,
    
    // Bollinger Bands
    pub bb_upper: Option<Decimal>,
    pub bb_middle: Option<Decimal>,
    pub bb_lower: Option<Decimal>,
    
    // Standalone ATR, VWAP and ADX lines (adx_plus and adx_minus added)
    pub atr_14: Option<Decimal>,
    pub atr_slope: Option<Decimal>,
    pub atr_volatility_regime: Option<String>,
    pub atr_stop_loss_level: Option<Decimal>,
    pub atr_take_profit_level: Option<Decimal>,
    pub vwap: Option<Decimal>,
    pub vwap_bias: Option<String>,
    pub adx_14: Option<Decimal>,
    pub adx_plus: Option<Decimal>,
    pub adx_minus: Option<Decimal>,
    
    // Generic-named Exponential Moving Averages
    pub ema_fast: Option<Decimal>,
    pub ema_medium: Option<Decimal>,
    pub ema_slow: Option<Decimal>,
    pub ema_long: Option<Decimal>,
    pub ema_stack_state: Option<String>,
    
    // Relative Strength Index
    pub rsi_14: Option<Decimal>,
    
    // MACD (Line, Signal, Histogram)
    pub macd_line: Option<Decimal>,
    pub macd_signal: Option<Decimal>,
    pub macd_hist: Option<Decimal>,
    
    // Squeeze Momentum Indicator (State & Histogram Value)
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<Decimal>,
    pub squeeze_duration: Option<u32>,
    pub squeeze_release_trigger: Option<bool>,
    pub squeeze_momentum_direction: Option<String>,

    // BBWP (Bollinger Band Width Percentile)
    pub bbwp: Option<Decimal>,

    // Support & Resistance Levels (JSON arrays serialized)
    pub support_levels: Option<String>,
    pub resistance_levels: Option<String>,
    pub sr_flip_events: Option<String>,
    pub chart_pattern: Option<String>,
    pub chart_pattern_confidence: Option<Decimal>,

    // Fibonacci Retracement & Extension Levels
    pub fib_golden_pocket_low: Option<Decimal>,
    pub fib_golden_pocket_high: Option<Decimal>,
    pub fib_extension_1618: Option<Decimal>,
    pub fib_extension_2618: Option<Decimal>,
    pub swing_high: Option<Decimal>,
    pub swing_low: Option<Decimal>,

    // ─── RSI Divergence Coordinates ────────────────────────────────────
    // Serialized JSON arrays of [price, indicator_value, index] for
    // the first and second extrema. Present only when a potential RSI
    // divergence is active. Empty/null when no divergence detected.
    pub rsi_divergence_status: Option<String>,
    pub rsi_divergence_coords: Option<String>,

    // ─── MACD Divergence Coordinates ───────────────────────────────────
    pub macd_divergence_status: Option<String>,
    pub macd_divergence_coords: Option<String>,

    // ─── MACD Momentum State ──────────────────────────────────────────
    pub macd_histogram_peak: Option<Decimal>,
    pub macd_trend_state: Option<String>,
    pub macd_crossover_detected: Option<bool>,
    pub macd_crossover_direction: Option<String>,

    // ─── ADX Trend Strength State ─────────────────────────────────────
    pub adx_slope: Option<Decimal>,
    pub adx_peak: Option<Decimal>,
    pub adx_regime: Option<String>,
    pub adx_di_crossover_detected: Option<bool>,
    pub adx_di_crossover_direction: Option<String>,
}
