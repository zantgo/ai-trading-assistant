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
    /// `[0.0, 1.0]` derived as `|score| / 100.0`. Canonical name in the wire
    /// schema (renamed from `confidence` in the institutional redesign; see
    /// [02-00b-confidence-hierarchy.md](../matrices/02-00b-confidence-hierarchy.md)).
    /// The legacy `confidence` field has been removed; the audit identified
    /// the duplicate as breaking the canonical field-ownership contract.
    pub score_confidence: f64,
    /// Synoptic entry-danger in `[0, 100]` (high = dangerous). `RiskDimension`
    /// shape matching the wire contract documented at
    /// [02-00-matrix-field-ownership.md](../matrices/02-00-matrix-field-ownership.md).
    /// Synthesized from L3 `market_quality` and L4 `opportunity_score` via
    /// the formula in Decision Matrix §3.8.
    pub entry_danger: RiskDimension,
    /// Risk-discounted R:R — `(active-side R:R) × (1 − L5.overall_risk / 100.0)`.
    /// The active-side R:R is `L4.long_expected_rr_internal` for bullish
    /// bias, `L4.short_expected_rr_internal` for bearish, 0 for Neutral.
    /// The legacy matrix-level `L4.expected_rr_internal` was removed in v6.9.
    pub expected_reward_risk_ratio: f64,
    /// Gate label — one of `READY` / `FORMING` / `WATCH` / `STAND_ASIDE`.
    pub trade_readiness: String,
    /// Indicator keys whose `confidence ≥ 0.6` contribute to the score.
    pub contributing_indicators: Vec<String>,
    /// Long-side normalized probability (0–100, integer). Sums to 100 with
    /// `short_probability` + `hold_probability`. Canonical source of truth
    /// for the "X% long" display; replaces the frontend-local computation
    /// that previously lived in `ui/src/lib/decisionRank.ts`.
    #[serde(default)]
    pub long_probability: f64,
    /// Short-side normalized probability (0–100, integer).
    #[serde(default)]
    pub short_probability: f64,
    /// Hold (no-position) normalized probability (0–100, integer).
    #[serde(default)]
    pub hold_probability: f64,
    /// Net directional bias in percentage points (`long − short`),
    /// range `[−100, +100]`. Positive = net long-leaning, negative = net
    /// short-leaning.
    #[serde(default)]
    pub net_bias_pct: f64,
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

        // Mirror `analysis.bias` exactly (5-state MarketBias family).
        // The unsigned confluence_score cannot encode direction; the
        // canonical source of directional bias is Analysis.bias.
        // Uses the same PascalCase as `AnalysisMatrix.bias` serialisation
        // (MarketBias derives `Serialize` without `rename_all`), NOT the
        // SCREAMING_SNAKE_CASE from the Display impl. The frontend type
        // `MarketBias` only covers PascalCase.
        let bias = match analysis.bias {
            crate::analysis::MarketBias::StrongBullish => "StrongBullish".to_string(),
            crate::analysis::MarketBias::Bullish => "Bullish".to_string(),
            crate::analysis::MarketBias::Neutral => "Neutral".to_string(),
            crate::analysis::MarketBias::Bearish => "Bearish".to_string(),
            crate::analysis::MarketBias::StrongBearish => "StrongBearish".to_string(),
        };

        // Score carries the directional sign from analysis.bias so the
        // frontend `computeDecisionRank` can split probability between
        // LONG (positive score) and SHORT (negative score) arms.
        let signed_confluence = match analysis.bias {
            crate::analysis::MarketBias::StrongBearish
            | crate::analysis::MarketBias::Bearish => -confluence_score,
            crate::analysis::MarketBias::Neutral => 0.0,
            crate::analysis::MarketBias::Bullish
            | crate::analysis::MarketBias::StrongBullish => confluence_score,
        };

        // -- expected_reward_risk_ratio = active-side R:R × (1 − overall_risk/100)
        let active_rr = opportunity
            .map(|o| match analysis.bias {
                crate::analysis::MarketBias::StrongBullish
                | crate::analysis::MarketBias::Bullish => o.long_expected_rr_internal,
                crate::analysis::MarketBias::StrongBearish
                | crate::analysis::MarketBias::Bearish => o.short_expected_rr_internal,
                crate::analysis::MarketBias::Neutral => 0.0,
            })
            .unwrap_or(0.0);
        let risk_disc = 1.0 - risk.overall_risk.score / 100.0;
        let expected_reward_risk_ratio = active_rr * risk_disc;

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

        // -- confidence_assessment (matches AdvisoryMatrix `compute_advisory`)
        let confidence_assessment =
            (analysis.state_confidence * (1.0 - risk.overall_risk.score / 100.0) * 100.0)
                .clamp(0.0, 100.0);

        let score_confidence = (confluence_score.abs() / 100.0).min(1.0);

        // -- Market stance (exact replication of AdvisoryMatrix A5 / §3.2)
        //    Returns (is_avoid, is_aggressive_or_constructive) so the
        //    percentage modulation matches `compute_advisory()` exactly.
        let (stance_is_avoid, stance_is_aggressive_or_constructive) =
            match analysis.market_quality {
                crate::analysis::QualityLevel::Poor => {
                    if risk.overall_risk.score >= 80.0 {
                        (true, false)
                    } else {
                        (false, false) // Cautious
                    }
                }
                crate::analysis::QualityLevel::Weak => {
                    (false, false) // always Cautious
                }
                crate::analysis::QualityLevel::Average => {
                    (false, false) // Neutral or Cautious — neither avoid nor aggressive
                }
                crate::analysis::QualityLevel::Good => {
                    if risk.overall_risk.score < 30.0 {
                        (false, true) // Constructive
                    } else {
                        (false, false) // Cautious
                    }
                }
                crate::analysis::QualityLevel::Excellent => {
                    if risk.overall_risk.score >= 80.0 {
                        (true, false) // Avoid
                    } else if risk.overall_risk.score < 30.0 {
                        (false, true) // Aggressive or Constructive
                    } else {
                        (false, false) // Cautious
                    }
                }
            };

        // -- Directional guidance (exact replication of AdvisoryMatrix A3 / §3.1)
        //    Returns (is_long, is_short) booleans for the percentage modulation.
        let (direction_is_long, direction_is_short) = if stance_is_avoid {
            (false, false) // AvoidDirectionalExposure — no directional lean
        } else {
            match analysis.bias {
                crate::analysis::MarketBias::StrongBullish => {
                    if risk.overall_risk.score < 50.0 {
                        (true, false) // StrongLong
                    } else {
                        (true, false) // Long
                    }
                }
                crate::analysis::MarketBias::Bullish => {
                    if risk.overall_risk.score < 40.0 {
                        (true, false) // Long
                    } else {
                        (false, false) // Neutral
                    }
                }
                crate::analysis::MarketBias::StrongBearish => {
                    if risk.overall_risk.score < 50.0 {
                        (false, true) // StrongShort
                    } else {
                        (false, true) // Short
                    }
                }
                crate::analysis::MarketBias::Bearish => {
                    if risk.overall_risk.score < 40.0 {
                        (false, true) // Short
                    } else {
                        (false, false) // Neutral
                    }
                }
                crate::analysis::MarketBias::Neutral => (false, false),
            }
        };

        let directional_is_non_neutral = direction_is_long || direction_is_short;

        // -- entry_guidance (matches AdvisoryMatrix `compute_advisory`)
        let entry_guidance_is_wait = risk.volatility_risk.score >= 60.0;

        let trade_readiness = if stance_is_avoid || confidence_assessment < 20.0 {
            "STAND_ASIDE"
        } else if directional_is_non_neutral
            && confidence_assessment >= 60.0
            && stance_is_aggressive_or_constructive
            && !entry_guidance_is_wait
        {
            "READY"
        } else if directional_is_non_neutral {
            "FORMING"
        } else {
            "WATCH"
        }
        .to_string();

        // Contributing indicators: any indicator with confidence >= 0.6
        let contributing_indicators: Vec<String> = indicators
            .iter()
            .filter(|(_, v)| v.confidence >= 0.6)
            .map(|(k, _)| k.clone())
            .collect();

        // ── Probability fields ─────────────────────────────────────────
        // Replicate the frontend `computeDecisionRank()` algorithm
        // (ui/src/lib/decisionRank.ts:66–202) so the canonical percentage
        // numbers (e.g. "+7% long") live server-side and are the source
        // of truth for all consumers.

        // Geometric offset: when bias is Neutral (score=0), inspect the
        // top-scored profile for a latent directional lean.
        let effective_score = if signed_confluence.abs() < 1e-9 {
            let mut offset = 0.0;
            if let Some(opp) = opportunity {
                let has_long = opp.long_entry_zone.low > 0.0;
                let has_short = opp.short_entry_zone.low > 0.0;
                let qualifying: Vec<_> = opp
                    .profiles
                    .iter()
                    .filter(|p| {
                        p.preconditions_met > 0
                            && p.opportunity_type
                                != crate::analysis::OpportunityType::NoClearOpportunity
                    })
                    .collect();
                if let Some(top) = qualifying.iter().max_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    if has_long && !has_short {
                        offset = top.score * 0.15;
                    } else if has_short && !has_long {
                        offset = -top.score * 0.15;
                    } else if has_long && has_short {
                        if opp.long_expected_rr_internal >= opp.short_expected_rr_internal {
                            offset = top.score * 0.15;
                        } else {
                            offset = -top.score * 0.15;
                        }
                    }
                }
            }
            offset
        } else {
            signed_confluence
        };

        let effective_confidence = if score_confidence >= 0.5 {
            score_confidence
        } else {
            0.5
        };

        let base_long = (effective_score.max(0.0) * effective_confidence).clamp(0.0, 100.0);
        let base_short = ((-effective_score).max(0.0) * effective_confidence).clamp(0.0, 100.0);
        let base_hold = ((entry_danger_score / 100.0) * 50.0).clamp(0.0, 100.0);

        let mut long = base_long;
        let mut short = base_short;
        let mut hold = base_hold;

        // Modulation by directional guidance
        if direction_is_long {
            long *= 1.2;
            short *= 0.5;
        } else if direction_is_short {
            short *= 1.2;
            long *= 0.5;
        }

        // Modulation by market stance
        if stance_is_aggressive_or_constructive {
            if direction_is_long {
                long *= 1.15;
            } else if direction_is_short {
                short *= 1.15;
            }
        }
        if stance_is_avoid {
            long *= 0.5;
            short *= 0.5;
            hold *= 1.5;
        }

        // R:R modulation
        if expected_reward_risk_ratio < 1.0 {
            if direction_is_long {
                long *= 0.6;
            } else if direction_is_short {
                short *= 0.6;
            }
        }

        long = long.clamp(0.0, 100.0);
        short = short.clamp(0.0, 100.0);
        hold = hold.clamp(0.0, 100.0);

        // Renormalize to sum to 100 (largest absorbs rounding residual)
        let sum = long + short + hold;
        let (mut l, mut s, mut h) = if sum <= 0.0 {
            (34.0, 33.0, 33.0)
        } else {
            let l_rounded = ((long / sum) * 100.0).round();
            let s_rounded = ((short / sum) * 100.0).round();
            let h_rounded = 100.0 - l_rounded - s_rounded;
            (l_rounded, s_rounded, h_rounded)
        };

        // MIN_PCT floor = 2% (prevents degenerate 100/0/0 distributions)
        const MIN_PCT: f64 = 2.0;
        l = l.max(MIN_PCT);
        s = s.max(MIN_PCT);
        h = h.max(MIN_PCT);
        let re_sum = l + s + h;
        let long_probability = ((l / re_sum) * 100.0).round();
        let short_probability = ((s / re_sum) * 100.0).round();
        let hold_probability = 100.0 - long_probability - short_probability;
        let net_bias_pct = long_probability - short_probability;

        Self {
            score: signed_confluence,
            bias,
            score_confidence,
            entry_danger,
            expected_reward_risk_ratio,
            trade_readiness,
            contributing_indicators,
            long_probability,
            short_probability,
            hold_probability,
            net_bias_pct,
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
    fn bullish_bias_produces_positive_score() {
        let mut analysis = make_analysis_with_quality(QualityLevel::Good);
        analysis.bias = MarketBias::Bullish;
        let risk = make_risk_with_overall(20.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::Breakout,
            opportunity_score: 85.0,
            setup_quality: crate::analysis::SetupQuality::Prime,
            profiles: vec![],
            forecast_confidence: 0.85,
            long_expected_rr_internal: 2.5,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
        assert_eq!(ctx.bias, "Bullish");
        assert!((ctx.score - 30.0).abs() < 1e-9);
        assert!((ctx.expected_reward_risk_ratio - 2.0).abs() < 1e-9);
        assert!((ctx.entry_danger.score - 20.0).abs() < 1e-9);
        assert_eq!(ctx.trade_readiness, "READY");
        assert!(ctx.long_probability > ctx.short_probability);
        assert!((ctx.long_probability + ctx.short_probability + ctx.hold_probability - 100.0).abs() < 1e-9);
        assert!((ctx.net_bias_pct - (ctx.long_probability - ctx.short_probability)).abs() < 1e-9);
    }

    #[test]
    fn bearish_bias_produces_negative_score() {
        let mut analysis = make_analysis_with_quality(QualityLevel::Good);
        analysis.bias = MarketBias::Bearish;
        let risk = make_risk_with_overall(20.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::TrendContinuation,
            opportunity_score: 70.0,
            setup_quality: crate::analysis::SetupQuality::Prime,
            profiles: vec![],
            forecast_confidence: 0.7,
            long_expected_rr_internal: 0.0,
            short_expected_rr_internal: 2.0,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 50.0, &analysis, Some(&opp), &risk);
        assert_eq!(ctx.bias, "Bearish");
        assert!((ctx.score - -50.0).abs() < 1e-9);
        // active side is Bearish → reads short_expected_rr_internal
        assert!((ctx.expected_reward_risk_ratio - (2.0 * 0.8)).abs() < 1e-9);
        assert!(ctx.short_probability > ctx.long_probability);
        assert!((ctx.long_probability + ctx.short_probability + ctx.hold_probability - 100.0).abs() < 1e-9);
    }

    #[test]
    fn neutral_bias_produces_zero_score() {
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
            long_expected_rr_internal: 0.0,
            time_horizon: "INTRADAY".to_string(),
            ..Default::default()
        };
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
        assert_eq!(ctx.bias, "Neutral");
        assert!((ctx.score - 0.0).abs() < 1e-9);
        // Neutral bias with no directional offset should produce a balanced distribution
        assert!((ctx.long_probability + ctx.short_probability + ctx.hold_probability - 100.0).abs() < 1e-9);
        assert!(ctx.net_bias_pct >= -5.0 && ctx.net_bias_pct <= 5.0);
    }

    #[test]
    fn strong_bullish_mirrors_analysis_bias() {
        let mut analysis = make_analysis_with_quality(QualityLevel::Excellent);
        analysis.bias = MarketBias::StrongBullish;
        let risk = make_risk_with_overall(10.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::TrendContinuation,
            opportunity_score: 100.0,
            setup_quality: crate::analysis::SetupQuality::Prime,
            profiles: vec![],
            forecast_confidence: 0.85,
            long_expected_rr_internal: 3.0,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, 90.0, &analysis, Some(&opp), &risk);
        assert_eq!(ctx.bias, "StrongBullish");
        assert!((ctx.score - 90.0).abs() < 1e-9);
        assert!(ctx.long_probability > ctx.short_probability);
    }

    #[test]
    fn strong_bearish_mirrors_analysis_bias() {
        let mut analysis = make_analysis_with_quality(QualityLevel::Weak);
        analysis.bias = MarketBias::StrongBearish;
        let risk = make_risk_with_overall(10.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::Breakout,
            opportunity_score: 80.0,
            setup_quality: crate::analysis::SetupQuality::Strong,
            profiles: vec![],
            forecast_confidence: 0.8,
            long_expected_rr_internal: 0.0,
            short_expected_rr_internal: 1.8,
            time_horizon: "INTRADAY".to_string(),
            ..Default::default()
        };
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, 65.0, &analysis, Some(&opp), &risk);
        assert_eq!(ctx.bias, "StrongBearish");
        assert!((ctx.score - -65.0).abs() < 1e-9);
        assert!(ctx.short_probability > ctx.long_probability);
    }

    #[test]
    fn high_risk_blocks_at_70() {
        let mut analysis = make_analysis_with_quality(QualityLevel::Average);
        analysis.bias = MarketBias::Bullish;
        let risk = make_risk_with_overall(80.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::Breakout,
            opportunity_score: 70.0,
            setup_quality: crate::analysis::SetupQuality::Strong,
            profiles: vec![],
            forecast_confidence: 0.7,
            long_expected_rr_internal: 2.5,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
        assert!((ctx.expected_reward_risk_ratio - 0.5).abs() < 1e-9);
        assert_eq!(ctx.trade_readiness, "STAND_ASIDE");
        assert!(ctx.long_probability > 0.0);
        assert!(ctx.short_probability > 0.0);
        assert!(ctx.hold_probability > 0.0);
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
            long_expected_rr_internal: 2.5,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx =
            DecisionContext::compute(&indicators, 100.0, 1.0, 30.0, &analysis, Some(&opp), &risk);
        assert!((ctx.entry_danger.score - 70.0).abs() < 1e-9);
        assert_eq!(ctx.trade_readiness, "STAND_ASIDE");
        assert!(ctx.long_probability > 0.0);
    }

    #[test]
    fn no_clear_opportunity_active_side_rr_is_zero_propagated() {
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
            long_expected_rr_internal: 0.0, // explicit Neutral sentinel
            time_horizon: "INTRADAY".to_string(),
            ..Default::default()
        };
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, 0.0, &analysis, Some(&opp), &risk);
        assert!((ctx.expected_reward_risk_ratio - 0.0).abs() < 1e-9);
        assert!(ctx.long_probability + ctx.short_probability + ctx.hold_probability > 0.0);
    }

    #[test]
    fn probabilities_sum_to_100_for_strong_bullish_ready() {
        let mut analysis = make_analysis_with_quality(QualityLevel::Excellent);
        analysis.bias = MarketBias::StrongBullish;
        let risk = make_risk_with_overall(10.0);
        let indicators = HashMap::new();
        let opp = crate::opportunity::OpportunityMatrix {
            symbol: "BTC-USD".to_string(),
            primary_opportunity: OpportunityType::TrendContinuation,
            opportunity_score: 100.0,
            setup_quality: crate::analysis::SetupQuality::Prime,
            profiles: vec![],
            forecast_confidence: 0.85,
            long_expected_rr_internal: 3.0,
            time_horizon: "SWING".to_string(),
            ..Default::default()
        };
        let ctx = DecisionContext::compute(&indicators, 100.0, 1.0, 90.0, &analysis, Some(&opp), &risk);
        let total = ctx.long_probability + ctx.short_probability + ctx.hold_probability;
        assert!((total - 100.0).abs() < 1e-9, "probabilities must sum to 100, got {}", total);
        assert_eq!(ctx.net_bias_pct, ctx.long_probability - ctx.short_probability);
        assert!(ctx.long_probability >= 2.0);
        assert!(ctx.short_probability >= 2.0);
        assert!(ctx.hold_probability >= 2.0);
        // High-conviction bullish → long should dominate
        assert!(ctx.long_probability > ctx.short_probability);
        assert!(ctx.net_bias_pct > 20.0);
    }
}
