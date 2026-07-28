//! # Indicator DTOs
//!
//! Pure-data shapes shared between `core-domain` (which holds them as
//! snapshot fields) and `market-analyzer` (which produces them via the
//! `NormalizationEngine`).
//!
//! These types have no dependency on raw indicator calculators. They are
//! safe to use from any crate that links against `core-domain`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Discrete signal kind an indicator can emit. Capabilities are declared in
/// the registry (`signal_types`); occurrences are recorded per snapshot in
/// `NormalizedIndicatorValue::signals`.
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

/// Operational lifecycle of a single indicator on a single timeframe pipeline.
///
/// This enum is **not** about market semantics (those live in
/// `NormalizedIndicatorValue::state_label`); it describes whether the current
/// reading is trustworthy, warming up, or unusable. See
/// `docs/engines/market-monitoring-engine/03-02-15-mme-indicator-lifecycle-states.md`
/// for the full state machine (ILS-01 … ILS-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndicatorLifecycleState {
    /// Calculator has fewer than `bars_required` candles of input; output is
    /// not yet trustworthy. Pipeline-level: `bars_seen < bars_required`.
    Loading,
    /// `bars_seen ≥ bars_required` AND parent pipeline LIVE AND last calculator
    /// update succeeded. The reading can be displayed without caveat.
    Live,
    /// Last successful update is older than `stale_threshold_secs`. The
    /// reading is still present but its freshness is degraded.
    Stale,
    /// Calculator panic / `Err`, OR `now - last_updated_at > 2 × stale_threshold_secs`.
    /// The reading should not be trusted.
    Failed,
}

/// Per-indicator operational lifecycle metadata published on every
/// `MarketSnapshot` alongside the `indicators` map. The two maps share keys;
/// `indicator_lifecycle` describes the **status** of each calculator, while
/// `indicators` carries the latest computed **value**.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorLifecycleStatus {
    pub state: IndicatorLifecycleState,
    pub bars_seen: u32,
    pub bars_required: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub stale_threshold_secs: u32,
}

impl IndicatorLifecycleStatus {
    /// Build a fresh `Loading` entry for a just-constructed pipeline.
    pub fn loading(bars_required: u32, stale_threshold_secs: u32) -> Self {
        Self {
            state: IndicatorLifecycleState::Loading,
            bars_seen: 0,
            bars_required,
            last_updated_at: None,
            last_error: None,
            stale_threshold_secs,
        }
    }

    /// Promote to `Live` after the first successful calculator update.
    pub fn live(
        bars_seen: u32,
        bars_required: u32,
        last_updated_at: u64,
        stale_threshold_secs: u32,
    ) -> Self {
        Self {
            state: IndicatorLifecycleState::Live,
            bars_seen,
            bars_required,
            last_updated_at: Some(last_updated_at),
            last_error: None,
            stale_threshold_secs,
        }
    }

    /// Mark as `Failed` with a free-text reason (calculator panic / double-stale).
    pub fn failed(bars_seen: u32, bars_required: u32, last_error: impl Into<String>) -> Self {
        Self {
            state: IndicatorLifecycleState::Failed,
            bars_seen,
            bars_required,
            last_updated_at: None,
            last_error: Some(last_error.into()),
            stale_threshold_secs: 0,
        }
    }
}

/// Type alias for the per-snapshot indicator lifecycle map. Keys are the
/// same registry keys as `MarketSnapshot.indicators` (e.g. `rsi`, `macd`,
/// `vwap`). Disabled indicators are absent from both maps.
pub type IndicatorLifecycleMap = HashMap<String, IndicatorLifecycleStatus>;

/// Clamp a value into the `[-1.0, 1.0]` unit interval.
#[inline]
pub fn clamp_unit(x: f64) -> f64 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(-1.0, 1.0)
    }
}
