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
mod extended;
mod signals;

pub use all::IndicatorInputs;

use super::squeeze::MomentumDirection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Discrete signal kind an indicator can emit. Capabilities are declared in the
/// registry (`signal_types`); occurrences are recorded per snapshot in
/// [`NormalizedIndicatorValue::signals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalKind {
    Divergence,
    Crossover,
    Threshold,
    Breakout,
    BandTouch,
    ZeroLineCross,
    CompressionRelease,
    LevelTest,
    TrendFlip,
    VolumeClimax,
    StackChange,
    PatternForming,
}

/// Directional bias of a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalDirection {
    Bullish,
    Bearish,
    Neutral,
}

/// Confirmation status of a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalStatus {
    Potential,
    Confirmed,
    Active,
}

/// A coordinate on the indicator/price series (used for divergence line points).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalPoint {
    pub time: u64,
    pub value: f64,
}

/// A single discrete signal fired by an indicator on a given snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorSignal {
    pub kind: SignalKind,
    pub direction: SignalDirection,
    pub status: SignalStatus,
    pub label: String,
    #[serde(default)]
    pub strength: f64,
    /// Number of completed bars since this signal first appeared (0 = fresh
    /// this bar). Stamped by the analyzer's stateful tracker.
    #[serde(default)]
    pub age_bars: u32,
    /// Pivot coordinates for divergence line drawing (future). Empty otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<Vec<SignalPoint>>,
}

impl IndicatorSignal {
    pub fn new(
        kind: SignalKind,
        direction: SignalDirection,
        status: SignalStatus,
        label: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            direction,
            status,
            label: label.into(),
            strength: 0.0,
            age_bars: 0,
            points: None,
        }
    }

    pub fn with_strength(mut self, strength: f64) -> Self {
        self.strength = strength;
        self
    }

    pub fn with_points(mut self, points: Vec<SignalPoint>) -> Self {
        self.points = Some(points);
        self
    }
}

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
    /// Discrete signals fired on this snapshot (divergence, crossover, breakout,
    /// threshold, etc.). Empty for most snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signals: Vec<IndicatorSignal>,
    /// Conviction of this reading in `[0.0, 1.0]`. Base = `|normalized|`, later
    /// boosted by confirmed signals in the finalization pass.
    #[serde(default)]
    pub confidence: f64,
}

impl NormalizedIndicatorValue {
    /// Build a single-line normalized value.
    pub fn scalar(raw_value: f64, normalized: f64, state_label: impl Into<String>) -> Self {
        let n = clamp_unit(normalized);
        Self {
            raw_value,
            normalized: n,
            state_label: state_label.into(),
            values: None,
            signals: Vec::new(),
            confidence: n.abs(),
        }
    }

    /// Build a normalized value carrying auxiliary raw component lines.
    pub fn with_values(
        raw_value: f64,
        normalized: f64,
        state_label: impl Into<String>,
        values: HashMap<String, f64>,
    ) -> Self {
        let n = clamp_unit(normalized);
        Self {
            raw_value,
            normalized: n,
            state_label: state_label.into(),
            values: Some(values),
            signals: Vec::new(),
            confidence: n.abs(),
        }
    }

    /// Neutral/equilibrium value used for missing data or defaults.
    pub fn neutral(label: impl Into<String>) -> Self {
        Self::scalar(0.0, 0.0, label)
    }

    /// Attach discrete signals (chained builder).
    pub fn with_signals(mut self, signals: Vec<IndicatorSignal>) -> Self {
        self.signals = signals;
        self
    }

    /// Append a single signal (chained builder).
    pub fn push_signal(mut self, signal: IndicatorSignal) -> Self {
        self.signals.push(signal);
        self
    }

    /// Override the computed confidence (chained builder).
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
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

/// Previous completed-bar indicator values, used to detect crossovers and
/// zero-line crosses (state transitions require the prior value as reference).
/// All fields optional so partially-warmed pipelines degrade gracefully.
#[derive(Debug, Clone, Copy, Default)]
pub struct PreviousBarState {
    pub rsi: Option<f64>,
    pub stoch_k: Option<f64>,
    pub stoch_d: Option<f64>,
    pub cmf: Option<f64>,
    pub chandemo: Option<f64>,
    pub aroon_up: Option<f64>,
    pub aroon_down: Option<f64>,
    pub macd_line: Option<f64>,
    pub linreg_slope: Option<f64>,
    pub zscore: Option<f64>,
    pub obv: Option<f64>,
    pub obv_sma: Option<f64>,
    pub mfi: Option<f64>,
    pub adx_plus_di: Option<f64>,
    pub adx_minus_di: Option<f64>,
    pub price: Option<f64>,
    pub ema_fast: Option<f64>,
    pub ema_medium: Option<f64>,
    pub supertrend_line: Option<f64>,
    // ── Deferred-indicator transition state (pivots / ichimoku) ──
    /// Signed position vs the nearest active pivot level: +1 above pivot,
    /// -1 below pivot, 0 unknown — used to detect pivot crossovers.
    pub pivot_active_level: Option<f64>,
    /// Previous Ichimoku Tenkan-sen (conversion line) for TK crossover.
    pub ichimoku_tenkan: Option<f64>,
    /// Previous Ichimoku Kijun-sen (base line) for TK crossover.
    pub ichimoku_kijun: Option<f64>,
    /// Previous price-vs-cloud position: +1 above cloud, -1 below, 0 inside —
    /// used to detect cloud breakouts and price entering/leaving the cloud.
    pub price_vs_cloud: Option<f64>,
    /// Previous future-cloud colour sign (Senkou A − Senkou B) for twist detection.
    pub ichimoku_future_bias: Option<f64>,
    pub hull_ma: Option<f64>,
    pub awesome_oscillator: Option<f64>,
    pub force_index: Option<f64>,
    pub williams_r: Option<f64>,
    pub cci: Option<f64>,
    pub psar_sar: Option<f64>,
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
    /// Previous completed-bar indicator values for crossover/zero-line detection.
    pub prev: PreviousBarState,
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
                        (
                            0.4 + 0.3 * saturate(histogram),
                            "BULLISH_MOMENTUM_EXPANDING",
                        )
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

#[cfg(test)]
mod meta_tests {
    use super::*;

    #[test]
    fn confidence_defaults_to_abs_normalized() {
        let v = NormalizedIndicatorValue::scalar(0.0, -0.8, "X");
        assert!((v.confidence - 0.8).abs() < 1e-9);
        let n = NormalizedIndicatorValue::neutral("N");
        assert_eq!(n.confidence, 0.0);
    }

    #[test]
    fn confidence_override_clamps() {
        let v = NormalizedIndicatorValue::scalar(0.0, 0.2, "X").with_confidence(1.5);
        assert_eq!(v.confidence, 1.0);
    }

    #[test]
    fn signal_carries_age_and_builders() {
        let s = IndicatorSignal::new(
            SignalKind::Divergence,
            SignalDirection::Bullish,
            SignalStatus::Confirmed,
            "CONFIRMED_BULLISH_DIVERGENCE",
        )
        .with_strength(0.9);
        assert_eq!(s.age_bars, 0);
        assert!((s.strength - 0.9).abs() < 1e-9);
        assert_eq!(s.direction, SignalDirection::Bullish);
    }

    #[test]
    fn derive_signals_boosts_confidence() {
        let mut inputs = IndicatorInputs::default();
        inputs.rsi = Some(15.0); // deep oversold → strong normalized + OB/OS threshold signal
        let ctx = NormalizationContext::default();
        let map = NormalizationEngine::normalize_all(&inputs, &ctx);
        let rsi = map.get("rsi").expect("rsi present");
        assert!(
            !rsi.signals.is_empty(),
            "oversold RSI should emit a threshold signal"
        );
        assert!(
            rsi.confidence >= rsi.normalized.abs(),
            "signals should not lower confidence"
        );
    }

    #[test]
    fn inactive_fill_populates_absent_directional_indicators() {
        // Event-driven directional indicators (fibonacci, S/R, patterns) that
        // are not present when no trigger exists must have INACTIVE placeholder.
        let inputs = IndicatorInputs::default();
        let ctx = NormalizationContext::default();
        let map = NormalizationEngine::normalize_all(&inputs, &ctx);
        let event_driven = ["fibonacci", "support_resistance", "patterns"];
        let divergence_keys = [
            "rsi_divergence",
            "macd_divergence",
            "stochastic_divergence",
            "chandemo_divergence",
            "mfi_divergence",
            "cmf_divergence",
            "obv_divergence",
            "squeeze_divergence",
        ];
        for &key in &event_driven {
            let v = map
                .get(key)
                .unwrap_or_else(|| panic!("{key} must be present"));
            assert_eq!(v.state_label, "INACTIVE", "{key} should be INACTIVE");
        }
        for &key in &divergence_keys {
            assert!(!map.contains_key(key), "{key} mirror should NOT be present");
        }
    }

    #[test]
    fn inactive_fill_skips_non_directional_gates() {
        // Non-directional gates (adx/atr/bbwp/hv/volume/rvol/choppiness) must NOT
        // be INACTIVE-filled (they are read by value by gate logic / context).
        let inputs = IndicatorInputs::default();
        let ctx = NormalizationContext::default();
        let map = NormalizationEngine::normalize_all(&inputs, &ctx);
        for key in ["adx", "atr", "bbwp", "hv", "rvol", "choppiness"] {
            assert!(
                !map.contains_key(key),
                "{key} gate must remain absent, not INACTIVE-filled"
            );
        }
    }
}
