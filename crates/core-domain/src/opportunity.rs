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
//!
//! This module is the **canonical source** of the L4 → L6 contract. See
//! [02-00-matrix-field-ownership.md §2.4](../matrices/02-00-matrix-field-ownership.md).

use crate::analysis::{OpportunityProfile, OpportunityType, SetupQuality};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceRange {
    pub low: f64,
    pub high: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpportunityMatrix {
    pub symbol: String,
    /// Canonical `OpportunityType` (institutional redesign: renamed from `opportunity_type`).
    pub primary_opportunity: OpportunityType,
    /// `opportunity_score ∈ [0, 100]` — setup viability from the L4 weighted blend.
    pub opportunity_score: f64,
    /// Categorical quality band.
    pub setup_quality: SetupQuality,
    /// Per-setup-type profiles with precondition fractions.
    pub profiles: Vec<OpportunityProfile>,
    /// Confidence in the profiling [0, 1].
    pub forecast_confidence: f64,
    /// Signal labels supporting the primary opportunity.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributing_signals: Vec<String>,
    /// Condition that would nullify the opportunity.
    pub invalidation_note: String,
    /// Recommended entry band.
    pub entry_zone: PriceRange,
    /// Expected target band.
    pub target_zone: PriceRange,
    /// `invalidation_level` — structural price whose breach nullifies the thesis.
    pub invalidation_level: f64,
    /// `expected_rr_internal` — internal L4 reward/risk ratio for this setup.
    /// The L6 Decision Matrix multiplies this by `1 − L5.overall_risk/100`
    /// to obtain the consumer-facing `expected_reward_risk_ratio`.
    pub expected_rr_internal: f64,
    /// `SCALP` / `INTRADAY` / `SWING` / `POSITION`.
    pub time_horizon: String,
}
