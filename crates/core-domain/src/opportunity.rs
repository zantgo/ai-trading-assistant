//! # Opportunity Matrix — Strategic Forecast
//!
//! Per the [Decision Matrix §2.1](../matrices/02-04-decision-matrix.md) and the
//! [Opportunity Matrix spec](../matrices/02-08-opportunity-matrix.md), the L4 layer
//! publishes a strategy-agnostic forecast. The `OpportunityMatrix` struct is the
//! shape consumed by L6 (Decision Support) for the `expected_reward_risk_ratio`
//! synthesis.
//!
//! ## Field provenance
//!
//! - `primary_opportunity`, `opportunity_score`, `setup_quality` — produced by L4.
//! - `profiles[]` — per-setup-type scored profiles with precondition fractions.
//! - `forecast_confidence` — confidence in the profiling [0, 1].
//! - `expected_rr_internal` — produced by L4 from the candidate setup profiles.
//! - `invalidation_level`, `entry_zone`, `target_zone`, `time_horizon` — produced by L4 from
//!   structural context (S/R, fib, pivot, VWAP).
//! - `confluent_entry_levels`, `confluent_target_levels`, `confluent_invalidation_levels` —
//!   enriched multi-source zone derivation from structural indicators.
//!
//! This module is the **canonical source** of the L4 → L6 contract. See
//! [02-00-matrix-field-ownership.md §2.4](../matrices/02-00-matrix-field-ownership.md).

use crate::analysis::{OpportunityProfile, OpportunityType, SetupQuality};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PriceRange {
    pub low: f64,
    pub high: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LevelSource {
    Fibonacci,
    VolumeProfile,
    PivotPoints,
    SupportResistance,
    LiquidityCluster,
    AtrFallback,
}

impl std::fmt::Display for LevelSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LevelSource::Fibonacci => write!(f, "FIB"),
            LevelSource::VolumeProfile => write!(f, "VP"),
            LevelSource::PivotPoints => write!(f, "PP"),
            LevelSource::SupportResistance => write!(f, "SR"),
            LevelSource::LiquidityCluster => write!(f, "LIQ"),
            LevelSource::AtrFallback => write!(f, "ATR"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfluentLevel {
    pub price: f64,
    pub confluence_count: u32,
    pub sources: Vec<LevelSource>,
    pub strength: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OpportunityMatrix {
    pub symbol: String,
    pub primary_opportunity: OpportunityType,
    pub opportunity_score: f64,
    pub setup_quality: SetupQuality,
    pub profiles: Vec<OpportunityProfile>,
    pub forecast_confidence: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributing_signals: Vec<String>,
    pub invalidation_note: String,
    pub entry_zone: PriceRange,
    pub target_zone: PriceRange,
    pub invalidation_level: f64,
    pub expected_rr_internal: f64,
    pub time_horizon: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub confluent_entry_levels: Vec<ConfluentLevel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub confluent_target_levels: Vec<ConfluentLevel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub confluent_invalidation_levels: Vec<ConfluentLevel>,
}
