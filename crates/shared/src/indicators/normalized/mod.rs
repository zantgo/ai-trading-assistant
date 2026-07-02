//! # Fractional Normalized Indicator Model (v2.0)
//!
//! Centralized, stateless normalization layer. The low-level indicator
//! calculators under `crates/shared/src/indicators/` remain pure math
//! producers of raw metrics. This module maps those raw metrics — together
//! with a [`NormalizationContext`] carrying directional/positional state —
//! into a unified `[-1.0, 1.0]` scale paired with a context-aware label.
//!
//! `-1.0` = absolute bearish conviction, `0.0` = equilibrium/compression,
//! `+1.0` = absolute bullish conviction.
//!
//! Self-contained mappers (RSI/MACD/Squeeze) live here; context-dependent
//! mappers live in [`context`]; the [`NormalizationEngine::normalize_all`]
//! consolidation lives in [`all`].

mod all;
mod context;

pub use all::IndicatorInputs;

use super::squeeze::MomentumDirection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified dual-representation indicator value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedIndicatorValue {
    /// Primary raw scalar (native indicator units).
    pub raw_value: f64,
    /// Continuous normalized score in `[-1.0, 1.0]`.
    pub normalized: f64,
    /// Context-aware level string for frontend rendering / logging.
    pub state_label: String,
    /// Auxiliary raw components for multi-line indicators (macd line/signal,
    /// bollinger bands, adx/di). `None` for single-line indicators.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<HashMap<String, f64>>,
}

impl NormalizedIndicatorValue {
    /// Build a single-line normalized value.
    pub fn scalar(raw_value: f64, normalized: f64, state_label: impl Into<String>) -> Self {
        Self {
            raw_value,
            normalized: clamp_unit(normalized),
            state_label: state_label.into(),
            values: None,
        }
    }

    /// Build a normalized value carrying auxiliary raw component lines.
    pub fn with_values(
        raw_value: f64,
        normalized: f64,
        state_label: impl Into<String>,
        values: HashMap<String, f64>,
    ) -> Self {
        Self {
            raw_value,
            normalized: clamp_unit(normalized),
            state_label: state_label.into(),
            values: Some(values),
        }
    }

    /// Neutral/equilibrium value used for missing data or defaults.
    pub fn neutral(label: impl Into<String>) -> Self {
        Self::scalar(0.0, 0.0, label)
    }
}

/// Divergence classification input for RSI/MACD normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DivergenceState {
    #[default]
    None,
    PotentialBullish,
    PotentialBearish,
    ConfirmedBullish,
    ConfirmedBearish,
}

/// Stateful context bridging the pure calculators to signed normalization.
#[derive(Debug, Clone, Default)]
pub struct NormalizationContext {
    /// Prevailing trend bias: +1 bull, -1 bear, 0 neutral.
    pub trend_bias: i8,
    /// Current reference price (typically the completed candle close).
    pub price: f64,
    /// Session VWAP, if available.
    pub vwap: Option<f64>,
    /// Active position direction: Some(+1) long, Some(-1) short, None flat.
    pub active_position: Option<i8>,
    /// EMA stack classification string ("bullish"/"bearish"/"tangled").
    pub ema_stack_state: Option<String>,
    /// Medium EMA value for dynamic support/resistance retest detection.
    pub ema_medium: Option<f64>,
    /// Sorted support price levels.
    pub support_levels: Vec<f64>,
    /// Sorted resistance price levels.
    pub resistance_levels: Vec<f64>,
    /// Relative volume for breakout confirmation gates.
    pub rvol: Option<f64>,
    /// True when ADX has decelerated (negative slope) for >= 2 consecutive
    /// bars while in the extreme regime — triggers the hard hook exit. Tracked
    /// statefully by the analyzer from the historical ADX buffer.
    pub adx_consecutive_deceleration: bool,
}

/// Clamp a value into the `[-1.0, 1.0]` unit interval.
#[inline]
pub fn clamp_unit(x: f64) -> f64 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(-1.0, 1.0)
    }
}

/// Soft saturation helper for unbounded raw magnitudes into `[0, 1)`.
#[inline]
pub(crate) fn saturate(x: f64) -> f64 {
    x.abs().tanh()
}

/// Signed-label selector helper.
#[inline]
pub(crate) fn pick(sign: f64, bull: &str, bear: &str) -> String {
    if sign > 0.0 {
        bull.to_string()
    } else {
        bear.to_string()
    }
}

/// Pure, static normalization mappers. Context-dependent mappers are
/// implemented in the [`context`] submodule; consolidation in [`all`].
pub struct NormalizationEngine;

impl NormalizationEngine {
    /// RSI (14): piecewise sigmoid-style compression with divergence override.
    pub fn normalize_rsi(rsi: f64, divergence: DivergenceState) -> NormalizedIndicatorValue {
        // Confirmed divergence hard-overrides the entire score.
        match divergence {
            DivergenceState::ConfirmedBullish => {
                return NormalizedIndicatorValue::scalar(rsi, 1.0, "OVERSOLD_ACCUMULATION")
            }
            DivergenceState::ConfirmedBearish => {
                return NormalizedIndicatorValue::scalar(rsi, -1.0, "OVERBOUGHT_DISTRIBUTION")
            }
            _ => {}
        }

        let base = if rsi <= 30.0 {
            0.7 + ((30.0 - rsi) / 30.0) * 0.3
        } else if rsi >= 70.0 {
            -0.7 - ((rsi - 70.0) / 30.0) * 0.3
        } else if rsi <= 50.0 {
            ((50.0 - rsi) / 20.0) * 0.7
        } else {
            -((rsi - 50.0) / 20.0) * 0.7
        };

        // Potential (unconfirmed) divergence additive boost, capped at ±0.90.
        let boosted = match divergence {
            DivergenceState::PotentialBullish => (base + 0.15).min(0.90),
            DivergenceState::PotentialBearish => (base - 0.15).max(-0.90),
            _ => base,
        };

        let norm = clamp_unit(boosted);
        NormalizedIndicatorValue::scalar(rsi, norm, rsi_label(norm))
    }

    /// MACD: crossover / zero-line filtering + histogram exhaustion warnings.
    pub fn normalize_macd(
        macd_line: f64,
        signal_line: f64,
        histogram: f64,
        histogram_peak: f64,
        crossover: Option<i8>,
    ) -> NormalizedIndicatorValue {
        let (norm, label) = match crossover {
            Some(c) if c > 0 => {
                if macd_line < 0.0 {
                    (
                        0.8 + 0.2 * saturate(macd_line),
                        "BULLISH_CROSSOVER_ACCELERATING",
                    )
                } else {
                    (0.2, "FOMO_BULLISH_CROSSOVER_REJECTED")
                }
            }
            Some(c) if c < 0 => {
                if macd_line > 0.0 {
                    (
                        -(0.8 + 0.2 * saturate(macd_line)),
                        "BEARISH_CROSSOVER_ACCELERATING",
                    )
                } else {
                    (-0.2, "PANIC_BEARISH_CROSSOVER_REJECTED")
                }
            }
            _ => {
                // No crossover — momentum regime from line/signal ordering.
                let contracting = histogram_peak.abs() > f64::EPSILON
                    && (histogram_peak.abs() - histogram.abs()) / histogram_peak.abs() >= 0.30;
                if histogram.abs() < 1e-9 {
                    (0.0, "MOMENTUM_FLATLINE")
                } else if macd_line > signal_line {
                    if contracting {
                        (0.3, "BULLISH_MOMENTUM_EXHAUSTION_WARNING")
                    } else {
                        (0.4 + 0.3 * saturate(histogram), "BULLISH_MOMENTUM_EXPANDING")
                    }
                } else if contracting {
                    (-0.3, "BEARISH_MOMENTUM_EXHAUSTION_WARNING")
                } else {
                    (
                        -(0.4 + 0.3 * saturate(histogram)),
                        "BEARISH_MOMENTUM_EXPANDING",
                    )
                }
            }
        };

        let mut values = HashMap::new();
        values.insert("line".to_string(), macd_line);
        values.insert("signal".to_string(), signal_line);
        values.insert("histogram".to_string(), histogram);
        values.insert("histogram_peak".to_string(), histogram_peak);
        NormalizedIndicatorValue::with_values(histogram, clamp_unit(norm), label, values)
    }

    /// Squeeze Momentum: coiling compression + 4-phase release direction.
    pub fn normalize_squeeze(
        squeeze_on: bool,
        release_trigger: bool,
        momentum_value: f64,
        direction: MomentumDirection,
    ) -> NormalizedIndicatorValue {
        if squeeze_on {
            return NormalizedIndicatorValue::scalar(momentum_value, 0.0, "COMPRESSION_COILING");
        }
        if release_trigger {
            return if momentum_value >= 0.0 {
                NormalizedIndicatorValue::scalar(momentum_value, 1.0, "BULLISH_VOLATILITY_RELEASE")
            } else {
                NormalizedIndicatorValue::scalar(momentum_value, -1.0, "BEARISH_VOLATILITY_RELEASE")
            };
        }
        let (norm, label) = match direction {
            MomentumDirection::BullishAcceleration => (
                0.5 + 0.4 * saturate(momentum_value),
                "BULLISH_EXPANSION_ACCELERATING",
            ),
            MomentumDirection::BullishDeceleration => (0.2, "BULLISH_MOMENTUM_EXHAUSTING"),
            MomentumDirection::BearishAcceleration => (
                -(0.5 + 0.4 * saturate(momentum_value)),
                "BEARISH_EXPANSION_ACCELERATING",
            ),
            MomentumDirection::BearishDeceleration => (-0.2, "BEARISH_MOMENTUM_EXHAUSTING"),
            MomentumDirection::Flat => (0.0, "COMPRESSION_COILING"),
        };
        NormalizedIndicatorValue::scalar(momentum_value, clamp_unit(norm), label)
    }
}

/// RSI level label derived from its normalized score band.
fn rsi_label(norm: f64) -> &'static str {
    if norm >= 0.70 {
        "OVERSOLD_ACCUMULATION"
    } else if norm >= 0.10 {
        "BULLISH_DISCOUNT"
    } else if norm > -0.10 {
        "EQUILIBRIUM"
    } else if norm > -0.70 {
        "BEARISH_PREMIUM"
    } else {
        "OVERBOUGHT_DISTRIBUTION"
    }
}
