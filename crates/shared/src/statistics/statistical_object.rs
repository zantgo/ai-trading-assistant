//! `StatisticValue` — the standard envelope for every statistical quantity
//! produced by the SIL.
//!
//! Instead of exposing a bare scalar (e.g. "ATR: 1.82"), every statistic
//! carries its current value alongside its historical distribution context:
//! rolling mean, standard deviation, percentile rank, z-score, confidence,
//! and directional trend.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticValue {
    /// The current (most recent) raw value.
    pub current: f64,

    /// Rolling arithmetic mean over the historical window.
    pub mean: f64,

    /// Rolling (sample) standard deviation over the historical window.
    pub stddev: f64,

    /// Percentile rank of the current value within the historical
    /// distribution. Range: `[0, 100]`.
    pub percentile: f64,

    /// Z-score: `(current - mean) / stddev`.  How many standard deviations
    /// the current value sits from the rolling mean.
    pub z_score: f64,

    /// A [0, 1] confidence estimate based on the relative variability
    /// (coefficient of variation) of the statistic.  Higher values mean
    /// the mean is well-defined relative to the noise.
    pub confidence: f64,

    /// Directional trend of the statistic: `"increasing"`, `"decreasing"`,
    /// or `"stable"`.
    pub trend: String,
}

impl Default for StatisticValue {
    fn default() -> Self {
        Self {
            current: 0.0,
            mean: 0.0,
            stddev: 0.0,
            percentile: 50.0,
            z_score: 0.0,
            confidence: 0.0,
            trend: "unknown".into(),
        }
    }
}
