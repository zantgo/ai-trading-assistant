//! # Domain Data Models
//!
//! This module defines the common data structures representing market telemetry.
//! It includes raw ticker prices, consolidated candle bars, and the unified
//! dual-representation normalized indicator map (v2.0).

use crate::indicators::normalized::NormalizedIndicatorValue;
use crate::normalized::Exchange;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    // Consolidated Candle OHLC Bars (core, non-indicator telemetry)
    pub open: Option<Decimal>,
    pub high: Option<Decimal>,
    pub low: Option<Decimal>,
    pub close: Option<Decimal>,
    pub volume: Option<Decimal>,
    pub average_volume: Option<Decimal>,

    /// Unified dual-representation indicator map.
    ///
    /// Each entry pairs a raw value, a `[-1.0, 1.0]` normalized score, and a
    /// context-aware state label. Keys: `rsi`, `macd`, `squeeze`, `adx`,
    /// `bbwp`, `rvol`, `ema_stack`, `vwap`, `fibonacci`, `patterns`,
    /// `support_resistance` (plus auxiliary chart series carried in `values`).
    #[serde(default)]
    pub indicators: HashMap<String, NormalizedIndicatorValue>,

    /// Synthesized higher-level market context (trend/momentum/volatility/
    /// volume/liquidity/regime + overall). Populated for completed snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<crate::market_context::MarketContext>,
}

/// Legacy-compatible read accessors that reconstruct flat indicator values
/// from the nested [`MarketSnapshot::indicators`] map. These bridge existing
/// consumers (CLI, server pipeline, DB persistence) during the transition to
/// the fully nested dual-representation model.
impl MarketSnapshot {
    /// Fetch a normalized indicator entry by key.
    pub fn ind(&self, key: &str) -> Option<&NormalizedIndicatorValue> {
        self.indicators.get(key)
    }

    /// Fetch an indicator's primary raw scalar.
    pub fn ind_raw(&self, key: &str) -> Option<f64> {
        self.indicators.get(key).map(|v| v.raw_value)
    }

    /// Fetch an indicator's normalized `[-1.0, 1.0]` score.
    pub fn ind_norm(&self, key: &str) -> Option<f64> {
        self.indicators.get(key).map(|v| v.normalized)
    }

    /// Fetch an indicator's state label.
    pub fn ind_label(&self, key: &str) -> Option<&str> {
        self.indicators.get(key).map(|v| v.state_label.as_str())
    }

    /// Fetch an auxiliary raw sub-component (macd line/signal, bollinger bands).
    pub fn ind_sub(&self, key: &str, sub: &str) -> Option<f64> {
        self.indicators
            .get(key)
            .and_then(|v| v.values.as_ref())
            .and_then(|m| m.get(sub))
            .copied()
    }

    fn dec(x: Option<f64>) -> Option<Decimal> {
        x.and_then(Decimal::from_f64_retain)
    }

    // ── Raw scalar accessors (Option<Decimal>) ──
    pub fn rsi_14(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("rsi"))
    }
    pub fn stoch_k(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("stochastic", "k_line"))
    }
    pub fn stoch_d(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("stochastic", "d_line"))
    }
    pub fn chandemo(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("chandemo"))
    }
    pub fn supertrend_line(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("supertrend", "line"))
    }
    pub fn keltner_middle(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("keltner", "middle"))
    }
    pub fn donchian_upper(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("donchian", "upper"))
    }
    pub fn obv(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("obv"))
    }
    pub fn cmf(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("cmf"))
    }
    pub fn mfi(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("mfi"))
    }
    pub fn hv(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("hv"))
    }
    pub fn aroon_oscillator(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("aroon"))
    }
    pub fn choppiness(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("choppiness"))
    }
    pub fn linreg_slope(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("linreg_slope"))
    }
    pub fn zscore(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("zscore"))
    }
    pub fn macd_line(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("macd", "line"))
    }
    pub fn macd_signal(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("macd", "signal"))
    }
    pub fn macd_hist(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("macd", "histogram"))
    }
    pub fn macd_histogram_peak(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("macd", "histogram_peak"))
    }
    pub fn adx_14(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("adx", "adx"))
    }
    pub fn adx_plus(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("adx", "plus_di"))
    }
    pub fn adx_minus(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("adx", "minus_di"))
    }
    pub fn adx_slope(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("adx", "adx_slope"))
    }
    pub fn atr_14(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("atr", "atr_14"))
    }
    pub fn atr_slope(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("atr", "atr_slope"))
    }
    pub fn bb_upper(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("bollinger", "upper"))
    }
    pub fn bb_middle(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("bollinger", "middle"))
    }
    pub fn bb_lower(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("bollinger", "lower"))
    }
    pub fn bbwp(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("bbwp"))
    }
    pub fn rvol(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("rvol"))
    }
    pub fn vwap(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("vwap", "vwap"))
    }
    pub fn squeeze_momentum(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("squeeze"))
    }
    pub fn ema_fast(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("ema_stack", "fast"))
    }
    pub fn ema_medium(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("ema_stack", "medium"))
    }
    pub fn ema_slow(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("ema_stack", "slow"))
    }
    pub fn ema_long(&self) -> Option<Decimal> {
        Self::dec(self.ind_sub("ema_stack", "long"))
    }

    // ── Boolean accessors ──
    pub fn squeeze_on(&self) -> Option<bool> {
        self.ind_label("squeeze").map(|l| l == "COMPRESSION_COILING")
    }
    pub fn squeeze_release_trigger(&self) -> Option<bool> {
        self.ind_label("squeeze")
            .map(|l| l.ends_with("VOLATILITY_RELEASE"))
    }
    pub fn macd_crossover_detected(&self) -> Option<bool> {
        self.ind_label("macd").map(|l| l.contains("CROSSOVER"))
    }

    // ── State-string accessors (legacy vocabulary) ──
    pub fn ema_stack_state(&self) -> Option<String> {
        self.ind_label("ema_stack").map(|l| {
            if l.contains("BULLISH") {
                "bullish".to_string()
            } else if l.contains("BEARISH") {
                "bearish".to_string()
            } else {
                "tangled".to_string()
            }
        })
    }
    pub fn vwap_bias(&self) -> Option<String> {
        self.ind_label("vwap").map(|l| {
            if l.contains("PREMIUM") {
                "premium".to_string()
            } else if l.contains("DISCOUNT") {
                "discount".to_string()
            } else {
                "equilibrium".to_string()
            }
        })
    }
    pub fn adx_regime(&self) -> Option<String> {
        self.ind_label("adx").map(|l| {
            if l.contains("CONGESTION") {
                "congestion".to_string()
            } else if l.contains("EMERGING") {
                "emerging".to_string()
            } else if l.contains("CLIMACTIC") {
                "extreme".to_string()
            } else if l.contains("STRONG") {
                "strong".to_string()
            } else {
                "congestion".to_string()
            }
        })
    }
    pub fn squeeze_momentum_direction(&self) -> Option<String> {
        self.ind("squeeze").map(|v| {
            let l = v.state_label.as_str();
            if l.contains("BULLISH") && v.normalized >= 0.5 {
                "BullishAcceleration".to_string()
            } else if l.contains("BULLISH") {
                "BullishDeceleration".to_string()
            } else if l.contains("BEARISH") && v.normalized <= -0.5 {
                "BearishAcceleration".to_string()
            } else if l.contains("BEARISH") {
                "BearishDeceleration".to_string()
            } else {
                "Flat".to_string()
            }
        })
    }
    pub fn macd_trend_state(&self) -> Option<String> {
        let hist = self.ind_sub("macd", "histogram")?.abs();
        let peak = self.ind_sub("macd", "histogram_peak")?.abs();
        Some(if peak > 0.0 && hist < peak {
            "decelerating".to_string()
        } else {
            "accelerating".to_string()
        })
    }
    pub fn macd_crossover_direction(&self) -> Option<String> {
        let v = self.ind("macd")?;
        if !v.state_label.contains("CROSSOVER") {
            return None;
        }
        Some(if v.normalized >= 0.0 { "BULLISH" } else { "BEARISH" }.to_string())
    }
    pub fn chart_pattern(&self) -> Option<String> {
        self.ind("patterns").and_then(|v| {
            if v.normalized > 0.0 {
                Some("BullishPattern".to_string())
            } else if v.normalized < 0.0 {
                Some("BearishPattern".to_string())
            } else {
                None
            }
        })
    }
    pub fn chart_pattern_confidence(&self) -> Option<Decimal> {
        Self::dec(self.ind_raw("patterns"))
    }

    // ── Fibonacci resting-level accessors (raw prices) ──
    pub fn fib_gp_top(&self) -> Option<f64> {
        self.ind_sub("fibonacci", "gp_top")
    }
    pub fn fib_gp_bottom(&self) -> Option<f64> {
        self.ind_sub("fibonacci", "gp_bottom")
    }
    pub fn fib_ext_1618(&self) -> Option<f64> {
        self.ind_sub("fibonacci", "ext_1618")
    }
    pub fn fib_ext_2618(&self) -> Option<f64> {
        self.ind_sub("fibonacci", "ext_2618")
    }
}
