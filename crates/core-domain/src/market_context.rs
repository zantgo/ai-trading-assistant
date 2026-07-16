//! # Market Context Synthesis
//!
//! A higher-level, human- and AI-readable summary of the current market state,
//! derived from the normalized indicator map by aggregating registry functional
//! groups (Trend / Momentum / Volume / Volatility) plus regime and liquidity.
//! This is meta-intelligence built on top of the indicators — not another
//! indicator itself.
//!
//! The struct types live in `core-domain`; the `synthesize` constructor lives
//! in `market_analyzer::market_context_synth` because it depends on the
//! indicator registry.

use serde::{Deserialize, Serialize};

/// One synthesized dimension of market context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextDimension {
    /// Signed score in `[-1.0, 1.0]` (bull/bear) or magnitude for non-directional.
    pub score: f64,
    /// Confidence in `[0.0, 1.0]` (mean confidence of contributing indicators).
    pub confidence: f64,
    /// Human-readable classification.
    pub label: String,
}

impl ContextDimension {
    /// Neutral/empty dimension used as a default.
    pub fn neutral() -> Self {
        Self {
            score: 0.0,
            confidence: 0.0,
            label: "NEUTRAL".into(),
        }
    }
}

/// Top-level synthesized market intelligence for a snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketContext {
    pub trend: ContextDimension,
    pub momentum: ContextDimension,
    pub volatility: ContextDimension,
    pub volume: ContextDimension,
    pub liquidity: ContextDimension,
    /// Regime classification: TRENDING | RANGE | EXPANSION | COMPRESSION.
    pub regime: String,
    /// Overall directional conviction in `[-100, 100]`.
    pub overall_score: i32,
    /// Overall label, e.g. STRONG_BULL / WEAK_BEAR / NEUTRAL.
    pub overall_label: String,
}
