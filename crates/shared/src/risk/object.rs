//! Risk levels, trend, and the atomic Risk Object.
//!
//! Part of the Institutional Risk Management Layer (IRML).
//! See `docs/institutional-risk-management-layer.md` Sections 6 and 14.

use serde::{Deserialize, Serialize};

#[inline]
pub(crate) fn clamp01(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}

/// Seven-level gradual risk band (Section 14). Ordered from safest to most
/// dangerous. Used both for individual Risk Objects and the overall profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    VerySafe,
    Safe,
    Normal,
    Elevated,
    High,
    Critical,
    Emergency,
}

impl RiskLevel {
    /// Ordinal index (0 = VerySafe .. 6 = Emergency).
    pub fn index(self) -> usize {
        match self {
            RiskLevel::VerySafe => 0,
            RiskLevel::Safe => 1,
            RiskLevel::Normal => 2,
            RiskLevel::Elevated => 3,
            RiskLevel::High => 4,
            RiskLevel::Critical => 5,
            RiskLevel::Emergency => 6,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => RiskLevel::VerySafe,
            1 => RiskLevel::Safe,
            2 => RiskLevel::Normal,
            3 => RiskLevel::Elevated,
            4 => RiskLevel::High,
            5 => RiskLevel::Critical,
            _ => RiskLevel::Emergency,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RiskLevel::VerySafe => "Very Safe",
            RiskLevel::Safe => "Safe",
            RiskLevel::Normal => "Normal",
            RiskLevel::Elevated => "Elevated",
            RiskLevel::High => "High",
            RiskLevel::Critical => "Critical",
            RiskLevel::Emergency => "Emergency",
        }
    }

    /// The score at or above which a level is entered (Section 14 default edges).
    fn enter_threshold(i: usize) -> f64 {
        match i {
            0 => 0.00,
            1 => 0.15,
            2 => 0.30,
            3 => 0.45,
            4 => 0.60,
            5 => 0.75,
            _ => 0.90,
        }
    }

    /// Instantaneous level for a `[0,1]` score, ignoring history.
    pub fn from_score(score: f64) -> Self {
        let s = clamp01(score);
        let mut idx = 0usize;
        for i in 0..=6 {
            if s >= Self::enter_threshold(i) {
                idx = i;
            }
        }
        RiskLevel::from_index(idx)
    }

    /// Level for a score applying hysteresis relative to a previous level:
    /// escalation is immediate, de-escalation requires the score to fall below
    /// `enter_threshold - margin` before stepping down one band at a time.
    pub fn with_hysteresis(score: f64, previous: RiskLevel, margin: f64) -> Self {
        let s = clamp01(score);
        let naive = Self::from_score(s).index();
        let mut idx = previous.index();
        if naive >= idx {
            idx = naive;
        } else {
            while idx > naive && s < (Self::enter_threshold(idx) - margin) {
                idx -= 1;
            }
        }
        RiskLevel::from_index(idx)
    }
}

/// Direction of change of a risk score versus its recent history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskTrend {
    Increasing,
    Stable,
    Decreasing,
}

impl RiskTrend {
    pub fn as_str(self) -> &'static str {
        match self {
            RiskTrend::Increasing => "Increasing",
            RiskTrend::Stable => "Stable",
            RiskTrend::Decreasing => "Decreasing",
        }
    }

    /// Classify a change relative to a previous value (0.05 dead-band).
    pub fn from_delta(current: f64, previous: f64) -> Self {
        let d = current - previous;
        if d > 0.05 {
            RiskTrend::Increasing
        } else if d < -0.05 {
            RiskTrend::Decreasing
        } else {
            RiskTrend::Stable
        }
    }
}

/// The atomic unit of the IRML — one category's risk assessment (Section 6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskObject {
    /// Current risk magnitude in `[0,1]` (0 = negligible, 1 = extreme).
    pub score: f64,
    /// How much evidence supports the score, `[0,1]`.
    pub confidence: f64,
    /// Where the score sits versus its own history, `[0,100]`.
    pub historical_percentile: f64,
    /// Direction of change versus recent history.
    pub trend: RiskTrend,
    /// Seven-level band derived from `score`.
    pub level: RiskLevel,
    /// Human-readable justification.
    pub explanation: String,
}

impl RiskObject {
    /// Build a Risk Object from a raw score + confidence + explanation.
    /// `historical_percentile` and `trend` default and are enriched later by
    /// the stateful engine from persisted history.
    pub fn new(score: f64, confidence: f64, explanation: impl Into<String>) -> Self {
        let score = clamp01(score);
        Self {
            score,
            confidence: clamp01(confidence),
            historical_percentile: 50.0,
            trend: RiskTrend::Stable,
            level: RiskLevel::from_score(score),
            explanation: explanation.into(),
        }
    }
}
