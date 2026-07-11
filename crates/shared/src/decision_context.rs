//! # Decision Context — Quantitative Decision Metadata
//!
//! Computes quantitative decision-support metadata from the normalized
//! indicator map and confluence score.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    pub score: f64,
    pub bias: String,
    pub confidence: f64,
    pub contributing_indicators: Vec<String>,
}

impl DecisionContext {
    /// Compute decision context from an indicator map and confluence score.
    pub fn compute(
        _indicators: &HashMap<String, super::indicators::normalized::NormalizedIndicatorValue>,
        _close: f64,
        _atr: f64,
        _confluence_score: f64,
    ) -> Self {
        // Placeholder — full computation to be implemented.
        let bias = if _confluence_score > 20.0 {
            "BULLISH"
        } else if _confluence_score < -20.0 {
            "BEARISH"
        } else {
            "NEUTRAL"
        };

        Self {
            score: _confluence_score,
            bias: bias.to_string(),
            confidence: (_confluence_score.abs() / 100.0).min(1.0),
            contributing_indicators: Vec::new(),
        }
    }
}
