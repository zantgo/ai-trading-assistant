//! # Decision Context — Quantitative Decision Metadata
//!
//! Computes quantitative decision-support metadata from the Analysis matrix (L3),
//! the Opportunity matrix (L4), and the Risk matrix (L5). The three matrices are
//! always consumed together — they form the canonical L6 synthesis triad.
//!
//! ## Canonical reference
//!
//! - [Decision Matrix §2.1](../matrices/02-04-decision-matrix.md) for the
//!   field-by-field contract.
//! - [Risk Matrix §2.1](../matrices/02-11-risk-matrix.md) for the
//!   `overall_risk.score ∈ [0, 100]` unit convention (and the `/ 100.0`
//!   divisor in `expected_reward_risk_ratio`).

use crate::analysis::AnalysisMatrix;
use crate::risk::RiskMatrix;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::indicators::normalized::NormalizedIndicatorValue;

/// Quantitative decision-metadata matrix published on `MarketSnapshot.decision_context`.
///
/// Synthesizes the canonical L3 (state) + L4 (forecast) + L5 (danger) triad. Distinct
/// from the human-facing `AdvisoryMatrix` (which carries the `trade_readiness`,
/// `directional_guidance`, `entry_guidance`, etc. guidance fields).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Quantitative confluence score in `[-100, +100]`.
    pub score: f64,
    /// `BULLISH` / `BEARISH` / `NEUTRAL` — mirrors `Analysis.bias` family.
    pub bias: String,
    /// `[0.0, 1.0]` derived as `|score| / 100.0`.
    pub confidence: f64,
    /// `score_confidence` in the wire schema; see
    /// [02-00b-confidence-hierarchy.md](../matrices/02-00b-confidence-hierarchy.md).
    pub score_confidence: f64,
    /// Synoptic entry-danger in `[0, 100]` (high = dangerous). Synthesized from
    /// L3 `market_quality` and L4 `opportunity_score` via the formula in Decision Matrix §3.8.
    pub entry_danger: f64,
    /// Risk-discounted R:R — `L4.expected_rr_internal × (1 − L5.overall_risk / 100.0)`.
    pub expected_reward_risk_ratio: f64,
    /// Gate label — one of `READY` / `FORMING` / `WATCH` / `STAND_ASIDE`.
    pub trade_readiness: String,
    /// Indicator keys whose `confidence ≥ 0.6` contribute to the score.
    pub contributing_indicators: Vec<String>,
}

impl DecisionContext {
    /// Compute `DecisionContext` from the L3/L4/L5 triad plus the indicator map.
    ///
    /// `opportunity` may be `None` if the L4 Opportunity Matrix is not yet
    /// populated for this symbol (early-warmup state). The function still
    /// produces a valid `DecisionContext` in this case (defaults: `2.5` for the
    /// internal R:R, `50.0` for the opportunity score).
    pub fn compute(
        indicators: &HashMap<String, NormalizedIndicatorValue>,
        close: f64,
        atr: f64,
        confluence_score: f64,
        analysis: &AnalysisMatrix,
        opportunity: Option<&super::opportunity::OpportunityMatrix>,
        risk: &RiskMatrix,
    ) -> Self {
        let _ = (close, atr); // kept for API symmetry / future volatility-aware extensions

        // -- Bias mapping from the confluence score
        let bias = if confluence_score > 20.0 {
            "BULLISH"
        } else if confluence_score < -20.0 {
            "BEARISH"
        } else {
            "NEUTRAL"
        }
        .to_string();

        // -- expected_reward_risk_ratio = L4.expected_rr_internal × (1 − overall_risk/100)
        let expected_rr_internal = opportunity
            .map(|o| o.expected_rr_internal)
            .unwrap_or(2.5);
        let risk_disc = 1.0 - risk.overall_risk.score / 100.0;
        let expected_reward_risk_ratio = expected_rr_internal * risk_disc;

        // -- entry_danger = mean(quality_penalty, 100 − opportunity_score)
        let quality_penalty: f64 = match analysis.market_quality {
            crate::analysis::QualityLevel::Excellent => 10.0,
            crate::analysis::QualityLevel::Good => 25.0,
            crate::analysis::QualityLevel::Average => 50.0,
            crate::analysis::QualityLevel::Weak => 70.0,
            crate::analysis::QualityLevel::Poor => 80.0,
        };
        let opportunity_score = opportunity
            .map(|o| o.opportunity_score)
            .unwrap_or(50.0);
        let entry_danger =
            ((quality_penalty + (100.0 - opportunity_score)) / 2.0).clamp(0.0, 100.0);

        // -- trade_readiness gate
        let trade_readiness = match (entry_danger, expected_reward_risk_ratio) {
            (d, _) if d >= 70.0 => "STAND_ASIDE",
            (_, r) if r < 1.0 => "FORMING",
            (d, _) if d >= 50.0 => "WATCH",
            _ => "READY",
        }
        .to_string();

        let confidence = (confluence_score.abs() / 100.0).min(1.0);
        let score_confidence = confidence;

        // Contributing indicators: any indicator with confidence >= 0.6
        let contributing_indicators: Vec<String> = indicators
            .iter()
            .filter(|(_, v)| v.confidence >= 0.6)
            .map(|(k, _)| k.clone())
            .collect();

        Self {
            score: confluence_score,
            bias,
            confidence,
            score_confidence,
            entry_danger,
            expected_reward_risk_ratio,
            trade_readiness,
            contributing_indicators,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{
        AnalysisMatrix, MarketBias, MarketRegime, MomentumAssessment, OpportunityType,
        QualityLevel, StructureAssessment, TrendAssessment, VolatilityAssessment,
        VolumeAssessment,
    };
    use crate::risk::{RiskDimension, RiskMatrix, RiskLevel, RiskState};

    fn make_analysis_with_quality(q: QualityLevel) -> AnalysisMatrix {
        AnalysisMatrix {
            symbol: "BTC-USD".to_string(),
            bias: MarketBias::Neutral,
            confidence: 0.82,
            market_regime: MarketRegime::Range,
            trend_assessment: TrendAssessment::Healthy,
            momentum_assessment: MomentumAssessment::Stable,
            structure_assessment: StructureAssessment::Healthy,
            volatility_assessment: VolatilityAssessment::Normal,
            volume_assessment: VolumeAssessment::Normal,
            opportunity_analysis: OpportunityType::NoClearOpportunity,
            market_quality: q,
            market_interpretation: "Test".into(),
            rationale: String::new(),
            supporting_signals: Vec::new(),
            contradicting_signals: Vec::new(),
            timeframes_considered: 4,
        }
    }

    fn make_risk_with_overall(score: f64) -> RiskMatrix {
        let dim = RiskDimension {
            score,
            level: RiskLevel::Low,
            state: RiskState::Stable,
            confidence: 50.0,
            evidence: Vec::new(),
        };
        RiskMatrix {
            symbol: "BTC-USD".to_string(),
            market_risk: dim.clone(),
            volatility_risk: dim.clone(),
            execution_liquidity_risk: dim.clone(),
            structure_risk: dim.clone(),
            momentum_risk: dim.clone(),
            signal_risk: dim.clone(),
            execution_risk: dim.clone(),
            cascade_risk: dim.clone(),
            overall_risk: dim,
        }
    }

    #[test]
    fn bearish_low_risk_yields_high_rr() {
        let analysis = make_analysis_with_quality(QualityLevel::Good);
        let risk = make_risk_with_overall(20.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            opportunity_type: "BREAKOUT".to_string(),
            opportunity_score: 85.0,
            expected_rr_internal: 2.5,
            invalidation_level: 0.0,
            entry_zone: (0.0, 0.0),
            target_zone: (0.0, 0.0),
            time_horizon: "SWING".to_string(),
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
        // expected_rr_internal × (1 - 0.20) = 2.5 × 0.80 = 2.0
        assert!((ctx.expected_reward_risk_ratio - 2.0).abs() < 1e-9);
        // entry_danger = mean(25, 100 - 85) = mean(25, 15) = 20
        assert!((ctx.entry_danger - 20.0).abs() < 1e-9);
        // trade_readiness: entry_danger=20 < 50 and rr=2.0 >= 1 → READY
        assert_eq!(ctx.trade_readiness, "READY");
    }

    #[test]
    fn high_risk_blocks_at_70() {
        let analysis = make_analysis_with_quality(QualityLevel::Average);
        let risk = make_risk_with_overall(80.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            opportunity_type: "BREAKOUT".to_string(),
            opportunity_score: 70.0,
            expected_rr_internal: 2.5,
            invalidation_level: 0.0,
            entry_zone: (0.0, 0.0),
            target_zone: (0.0, 0.0),
            time_horizon: "SWING".to_string(),
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
        // entry_danger = mean(50, 30) = 40 < 70 → not blocked on entry_danger
        // But risk_disc = 1 - 0.80 = 0.20 → rr = 2.5 * 0.20 = 0.5 < 1 → FORMING
        assert!((ctx.expected_reward_risk_ratio - 0.5).abs() < 1e-9);
        assert_eq!(ctx.trade_readiness, "FORMING");
    }

    #[test]
    fn dangerous_setup_blocks_at_70() {
        let analysis = make_analysis_with_quality(QualityLevel::Weak);
        let risk = make_risk_with_overall(85.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            opportunity_type: "BREAKOUT".to_string(),
            opportunity_score: 30.0,
            expected_rr_internal: 2.5,
            invalidation_level: 0.0,
            entry_zone: (0.0, 0.0),
            target_zone: (0.0, 0.0),
            time_horizon: "SWING".to_string(),
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
        // entry_danger = mean(70, 70) = 70 → STAND_ASIDE
        assert!((ctx.entry_danger - 70.0).abs() < 1e-9);
        assert_eq!(ctx.trade_readiness, "STAND_ASIDE");
    }
}
