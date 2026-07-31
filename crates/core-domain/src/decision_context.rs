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
use crate::risk::{RiskDimension, RiskMatrix};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::indicator_dtos::NormalizedIndicatorValue;

/// Quantitative decision-metadata matrix published on `MarketSnapshot.decision_context`.
///
/// Synthesizes the canonical L3 (state) + L4 (forecast) + L5 (danger) triad. Distinct
/// from the human-facing `AdvisoryMatrix` (which carries the `trade_readiness`,
/// `directional_guidance`, `entry_guidance`, etc. guidance fields).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Quantitative confluence score in `[-100, +100]` — signed product of
    /// `analysis.market_bias_score` (sign) and the 3-factor quality blend
    /// (magnitude). Mirrors `Analysis.bias` family on the sign axis.
    pub score: f64,
    /// 5-state `MarketBias` family: `STRONG_BULLISH` / `BULLISH` / `NEUTRAL`
    /// / `BEARISH` / `STRONG_BEARISH`. Mirrors `Analysis.bias` exactly.
    pub bias: String,
    /// `[0.0, 1.0]` derived as `|score| / 100.0`.
    pub confidence: f64,
    /// `score_confidence` in the wire schema; see
    /// [02-00b-confidence-hierarchy.md](../matrices/02-00b-confidence-hierarchy.md).
    pub score_confidence: f64,
    /// Synoptic entry-danger in `[0, 100]` (high = dangerous). `RiskDimension`
    /// shape matching the wire contract documented at
    /// [02-00-matrix-field-ownership.md](../matrices/02-00-matrix-field-ownership.md).
    /// Synthesized from L3 `market_quality` and L4 `opportunity_score` via
    /// the formula in Decision Matrix §3.8.
    pub entry_danger: RiskDimension,
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
    /// produces a valid `DecisionContext` in this case.
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

        // -- 5-state Bias mapping from the signed confluence score
        // Thresholds: ±80 = strong, ±20 = neutral band. Mirrors
        // `analysis.rs::derive_analysis` 5-state bucketing of the signed
        // mtf_overall_score, so a `BULLISH` decision bias corresponds to a
        // `Bullish`/`StrongBullish` analysis bias.
        let bias = if confluence_score > 80.0 {
            "STRONG_BULLISH"
        } else if confluence_score > 20.0 {
            "BULLISH"
        } else if confluence_score >= -20.0 {
            "NEUTRAL"
        } else if confluence_score >= -80.0 {
            "BEARISH"
        } else {
            "STRONG_BEARISH"
        }
        .to_string();

        // -- expected_reward_risk_ratio = L4.expected_rr_internal × (1 − overall_risk/100)
        // `expected_rr_internal == 0` is the explicit "no setup" sentinel from
        // `compute_opportunity`; propagate as 0 here too rather than masking
        // a NoClearOpportunity with the legacy 2.5 default.
        let expected_rr_internal = opportunity.map(|o| o.expected_rr_internal).unwrap_or(0.0);
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
        let opportunity_score = opportunity.map(|o| o.opportunity_score).unwrap_or(50.0);
        let entry_danger_score =
            ((quality_penalty + (100.0 - opportunity_score)) / 2.0).clamp(0.0, 100.0);
        let entry_danger = RiskDimension::from_score_with_confidence(
            entry_danger_score,
            analysis.state_confidence,
        );

        // -- trade_readiness gate (documented priority rules from
        //    `02-04-decision-matrix.md §4`)
        let trade_readiness = match (entry_danger_score, expected_reward_risk_ratio) {
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
        AnalysisMatrix, MarketBias, MarketPhase, MarketRegime, MomentumAssessment, OpportunityType,
        QualityLevel, StructureAssessment, TrendAssessment, VolatilityAssessment, VolumeAssessment,
    };
    use crate::risk::{RiskDimension, RiskLevel, RiskMatrix, RiskState};

    fn make_analysis_with_quality(q: QualityLevel) -> AnalysisMatrix {
        AnalysisMatrix {
            symbol: "BTC-USD".to_string(),
            bias: MarketBias::Neutral,
            market_bias_score: 0.0,
            state_confidence: 0.82,
            confidence: 0.82,
            market_regime: MarketRegime::Range,
            trend_assessment: TrendAssessment::Healthy,
            momentum_assessment: MomentumAssessment::Stable,
            structure_assessment: StructureAssessment::Healthy,
            volatility_assessment: VolatilityAssessment::Normal,
            volume_assessment: VolumeAssessment::Normal,
            opportunity_analysis: OpportunityType::NoClearOpportunity,
            market_quality: q,
            market_quality_score: 0.0,
            market_phase: MarketPhase::Unknown,
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
            primary_opportunity: OpportunityType::Breakout,
            opportunity_score: 85.0,
            setup_quality: crate::analysis::SetupQuality::Prime,
            profiles: vec![],
            forecast_confidence: 0.85,
            expected_rr_internal: 2.5,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
        assert!((ctx.expected_reward_risk_ratio - 2.0).abs() < 1e-9);
        assert!((ctx.entry_danger.score - 20.0).abs() < 1e-9);
        assert_eq!(ctx.trade_readiness, "READY");
    }

    #[test]
    fn high_risk_blocks_at_70() {
        let analysis = make_analysis_with_quality(QualityLevel::Average);
        let risk = make_risk_with_overall(80.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::Breakout,
            opportunity_score: 70.0,
            setup_quality: crate::analysis::SetupQuality::Strong,
            profiles: vec![],
            forecast_confidence: 0.7,
            expected_rr_internal: 2.5,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
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
            primary_opportunity: OpportunityType::Breakout,
            opportunity_score: 30.0,
            setup_quality: crate::analysis::SetupQuality::Marginal,
            profiles: vec![],
            forecast_confidence: 0.3,
            expected_rr_internal: 2.5,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
        assert!((ctx.entry_danger.score - 70.0).abs() < 1e-9);
        assert_eq!(ctx.trade_readiness, "STAND_ASIDE");
    }

    #[test]
    fn confluence_score_can_be_negative_with_signed_pipeline() {
        // `analysis.market_bias_score = -0.5` ⇒ BEARISH. With a quality
        // magnitude of 1.0, the confluence score should be ~-50.
        let mut analysis = make_analysis_with_quality(QualityLevel::Good);
        analysis.market_bias_score = -0.5;
        let risk = make_risk_with_overall(20.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::Breakout,
            opportunity_score: 100.0,
            setup_quality: crate::analysis::SetupQuality::Prime,
            profiles: vec![],
            forecast_confidence: 0.85,
            expected_rr_internal: 2.5,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let signed = -50.0;
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, signed, &analysis, Some(&opp), &risk);
        assert_eq!(ctx.bias, "BEARISH");
        assert!((ctx.score - signed).abs() < 1e-9);
    }

    #[test]
    fn confluence_score_strong_negative_yields_strong_bearish() {
        let mut analysis = make_analysis_with_quality(QualityLevel::Good);
        analysis.market_bias_score = -0.9;
        let risk = make_risk_with_overall(20.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::TrendContinuation,
            opportunity_score: 100.0,
            setup_quality: crate::analysis::SetupQuality::Prime,
            profiles: vec![],
            forecast_confidence: 0.85,
            expected_rr_internal: 2.5,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, -90.0, &analysis, Some(&opp), &risk);
        assert_eq!(ctx.bias, "STRONG_BEARISH");
    }

    #[test]
    fn confluence_score_neutral_band_yields_neutral_bias() {
        let analysis = make_analysis_with_quality(QualityLevel::Good);
        let risk = make_risk_with_overall(20.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::NoClearOpportunity,
            opportunity_score: 0.0,
            setup_quality: crate::analysis::SetupQuality::None,
            profiles: vec![],
            forecast_confidence: 0.0,
            expected_rr_internal: 0.0,
            time_horizon: "INTRADAY".to_string(),
            ..Default::default()
        };
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, 0.0, &analysis, Some(&opp), &risk);
        assert_eq!(ctx.bias, "NEUTRAL");
        assert!((ctx.score - 0.0).abs() < 1e-9);
    }

    #[test]
    fn confluence_score_strong_positive_yields_strong_bullish() {
        let analysis = make_analysis_with_quality(QualityLevel::Excellent);
        let risk = make_risk_with_overall(10.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::TrendContinuation,
            opportunity_score: 100.0,
            setup_quality: crate::analysis::SetupQuality::Prime,
            profiles: vec![],
            forecast_confidence: 0.85,
            expected_rr_internal: 3.0,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, 90.0, &analysis, Some(&opp), &risk);
        assert_eq!(ctx.bias, "STRONG_BULLISH");
    }

    #[test]
    fn no_clear_opportunity_expected_rr_internal_is_zero_propagated() {
        let analysis = make_analysis_with_quality(QualityLevel::Good);
        let risk = make_risk_with_overall(20.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::NoClearOpportunity,
            opportunity_score: 0.0,
            setup_quality: crate::analysis::SetupQuality::None,
            profiles: vec![],
            forecast_confidence: 0.0,
            expected_rr_internal: 0.0, // explicit Neutral sentinel
            time_horizon: "INTRADAY".to_string(),
            ..Default::default()
        };
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, 0.0, &analysis, Some(&opp), &risk);
        assert!((ctx.expected_reward_risk_ratio - 0.0).abs() < 1e-9);
    }
}
