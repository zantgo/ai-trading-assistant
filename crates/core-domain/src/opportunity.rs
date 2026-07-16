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
//! - `opportunity_type` and `opportunity_score` — produced by L4 (Opportunity Layer).
//! - `expected_rr_internal` — produced by L4 from the candidate setup profiles.
//! - `invalidation_level`, `entry_zone`, `target_zone`, `time_horizon` — produced by L4 from
//!   structural context (S/R, fib, pivot, VWAP).
//!
//! This module is the **canonical source** of the L4 → L6 contract. See
//! [02-00-matrix-field-ownership.md §2.4](../matrices/02-00-matrix-field-ownership.md).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpportunityMatrix {
    pub symbol: String,
    /// Canonical `OpportunityType` enum value as a string (one of `TrendContinuation`,
    /// `Breakout`, `Pullback`, `MeanReversion`, `Reversal`, `LiquiditySqueeze`,
    /// `NoClearOpportunity`). Enum lives in
    /// `crates/shared/src/analysis.rs::OpportunityType`; this field carries the
    /// serialized name.
    pub opportunity_type: String,
    /// `opportunity_score ∈ [0, 100]` — setup viability from the L4 weighted blend.
    pub opportunity_score: f64,
    /// `expected_rr_internal` — internal L4 reward/risk ratio for this setup.
    /// The L6 Decision Matrix multiplies this by `1 − L5.overall_risk/100`
    /// to obtain the consumer-facing `expected_reward_risk_ratio`.
    pub expected_rr_internal: f64,
    /// `invalidation_level` — structural price whose breach nullifies the thesis.
    /// Matches the Decision Matrix and Position Matrix canonical name (renamed
    /// from `invalid_level` in v2.1).
    pub invalidation_level: f64,
    /// `(low, high)` entry band.
    pub entry_zone: (f64, f64),
    /// `(low, high)` expected target band.
    pub target_zone: (f64, f64),
    /// `INTRADAY` / `SWING` / `POSITION`.
    pub time_horizon: String,
}
