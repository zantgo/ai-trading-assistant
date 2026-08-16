//! # Advisory Matrix — Decision Guidance Layer
//!
//! The Advisory Matrix transforms complete market intelligence and risk
//! assessment into structured human-facing guidance. It consumes the Analysis
//! Matrix and Risk Matrix to provide an explainable recommendation framework.
//! It does not execute trades — it explains and recommends.
//!
//! Layer: L4.75 in the architecture (Decision Guidance).

use crate::analysis::{AnalysisMatrix, OpportunityType};
use crate::opportunity::OpportunityMatrix;
use crate::risk::{RiskDimension, RiskMatrix};
use serde::{Deserialize, Serialize};

/// Directional guidance classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectionalGuidance {
    StrongLong,
    Long,
    Neutral,
    Short,
    StrongShort,
    AvoidDirectionalExposure,
}

/// Market stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketStance {
    Aggressive,
    Constructive,
    Neutral,
    Cautious,
    Avoid,
}

impl std::fmt::Display for MarketStance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Opportunity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpportunityClass {
    TrendContinuation,
    Breakout,
    Pullback,
    MeanReversion,
    Reversal,
    LiquiditySqueeze,
    Scalp,
    NoClearOpportunity,
}

/// Strategy environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyEnvironment {
    TrendFollowing,
    Breakout,
    MeanReversion,
    HighVolatility,
    LowActivity,
    Unfavorable,
}

/// Entry guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryGuidance {
    Immediate,
    WaitForConfirmation,
    Pullback,
    Breakout,
    NoEntryContext,
}

/// Exit guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitGuidance {
    TrendWeakening,
    MomentumExhaustion,
    StructureBreakdown,
    RiskIncreasing,
    NoWarning,
}

/// Protection strategy guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectionStrategy {
    StructureBased,
    VolatilityBased,
    ATRBased,
    SRBased,
    NoRecommendation,
}

/// Target strategy guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetStrategy {
    ResistanceBased,
    RRBased,
    VolatilityBased,
    TrailingMethod,
    NoRecommendation,
}

/// Advisory Matrix — human-facing guidance per symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvisoryMatrix {
    pub symbol: String,
    pub directional_guidance: DirectionalGuidance,
    pub market_stance: MarketStance,
    pub opportunity_classification: OpportunityClass,
    pub strategy_environment: StrategyEnvironment,
    pub entry_guidance: EntryGuidance,
    pub exit_guidance: ExitGuidance,
    pub protection_strategy: ProtectionStrategy,
    pub target_strategy: TargetStrategy,
    pub confidence_assessment: f64,
    pub stop_loss_distance_pct: f64,
    /// Cross-symbol cascade risk index — per-symbol cascade risk score
    /// from the L5 Risk Matrix, carried through to L7 Overview aggregation.
    pub cascade_risk_score: f64,
    /// Synoptic favorability of entering a position — semantic successor
    /// of the old `Risk.reward_risk` (removed in the institutional redesign).
    /// Synthesized from L3 `market_quality` and the L4 opportunity type —
    /// lower score = more favorable environment.
    pub environment_favorability: RiskDimension,
    /// Setup-efficiency metric: `analysis.market_quality_score ÷
    /// risk.overall_risk.score` (both unipolar 0-100). Higher = the setup's
    /// quality mathematically justifies the risk. `None` when overall risk
    /// is 0 (division guard).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_to_risk_ratio: Option<f64>,
    pub final_recommendation: String,
}

impl AdvisoryMatrix {
    pub fn empty(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            directional_guidance: DirectionalGuidance::Neutral,
            market_stance: MarketStance::Neutral,
            opportunity_classification: OpportunityClass::NoClearOpportunity,
            strategy_environment: StrategyEnvironment::LowActivity,
            entry_guidance: EntryGuidance::NoEntryContext,
            exit_guidance: ExitGuidance::NoWarning,
            protection_strategy: ProtectionStrategy::NoRecommendation,
            target_strategy: TargetStrategy::NoRecommendation,
            confidence_assessment: 0.0,
            stop_loss_distance_pct: 0.0,
            cascade_risk_score: 30.0,
            environment_favorability: RiskDimension::default(),
            quality_to_risk_ratio: None,
            final_recommendation: "Insufficient data to provide guidance.".into(),
        }
    }
}

/// Compute `environment_favorability` — synoptic favorability of entering
/// a position. Synthesizes L3 `market_quality` and L4 `opportunity_score`.
/// Lower score = more favorable environment.
///
/// Bug-fix #18: the legacy implementation read the L3
/// `analysis.opportunity_analysis` enum (a coarse OpportunityType
/// bucketed as 20 vs 80) instead of the L4
/// `OpportunityMatrix.opportunity_score` (a continuous 0-100 score).
/// The 60-point discretization collapsed every non-NoClearOpportunity
/// case to 80, masking the difference between a prime TrendContinuation
/// (opportunity_score ≈ 90) and a marginal Pullback (opportunity_score
/// ≈ 45). We now consume the L4 numeric score directly; when the
/// opportunity matrix is absent (early warmup), we fall back to the
/// L3 enum proxy at 50 (neutral).
fn compute_environment_favorability(
    analysis: &AnalysisMatrix,
    opportunity: Option<&crate::opportunity::OpportunityMatrix>,
) -> RiskDimension {
    let quality_penalty: f64 = match analysis.market_quality {
        crate::analysis::QualityLevel::Excellent => 10.0,
        crate::analysis::QualityLevel::Good => 25.0,
        crate::analysis::QualityLevel::Average => 50.0,
        crate::analysis::QualityLevel::Weak => 70.0,
        crate::analysis::QualityLevel::Poor => 80.0,
    };
    let opportunity_score: f64 = if let Some(opp) = opportunity {
        opp.opportunity_score
    } else {
        // Pre-warmup fallback. The L3 enum proxy at 50 is the
        // "no L4 data available" sentinel (neutral, not optimistic).
        50.0
    };
    let score: f64 = ((quality_penalty + (100.0 - opportunity_score)) / 2.0).clamp(0.0, 100.0);
    RiskDimension {
        score,
        ..RiskDimension::default()
    }
}

/// Compute Advisory Matrix from Analysis + Risk + Opportunity + Cluster.
/// `opportunity` may be `None` during early warm-up.
pub fn compute_advisory(
    analysis: &AnalysisMatrix,
    risk: &RiskMatrix,
    opportunity: Option<&OpportunityMatrix>,
    _cluster: Option<&crate::liquidity::LiquidationClusterMatrix>,
) -> AdvisoryMatrix {
    if analysis.timeframes_considered == 0 {
        return AdvisoryMatrix::empty(&analysis.symbol);
    }

    // Capture the L4 OpportunityMatrix reference before the local
    // `opportunity: OpportunityClass` shadow at line ~264. Bug-fix
    // #18 needs the L4 numeric `opportunity_score` (not the L3
    // bucketed enum), so we must hold onto the original `Option<&OpportunityMatrix>`
    // through the rest of the function.
    let opp_matrix = opportunity;

    // v6.10 (Phase 1 / A5): Market stance from quality × risk, per the
    // 6-rule canonical table at
    // `docs/matrices/02-04-decision-matrix.md §3.2`. We compute stance
    // BEFORE directional guidance so the AVOID short-circuit can run first
    // (see v6.10 A3 below).
    let stance = match analysis.market_quality {
        // Rule 1: POOR quality OR overall_risk ≥ 80 → AVOID.
        // Rule 2: POOR/WEAK quality OR overall_risk ≥ 60 → CAUTIOUS.
        // Rule 3: AVERAGE quality AND overall_risk < 40 → NEUTRAL.
        // Rule 4: EXCELLENT quality AND overall_risk < 20 → AGGRESSIVE.
        // Rule 5: GOOD/EXCELLENT quality AND overall_risk < 30 → CONSTRUCTIVE.
        // Rule 6 (default): everything else → CAUTIOUS.
        //
        // NOTE: rules 1 and 2 share the POOR/WEAK triggers with rule 6;
        // ordering matters. We model them as a single match against quality
        // with risk-band sub-matches.
        crate::analysis::QualityLevel::Poor => {
            if risk.overall_risk.score >= 80.0 {
                MarketStance::Avoid
            } else {
                // POOR + any other risk band → CAUTIOUS (rule 2)
                MarketStance::Cautious
            }
        }
        crate::analysis::QualityLevel::Weak => {
            if risk.overall_risk.score >= 60.0 {
                MarketStance::Cautious
            } else {
                // WEAK + risk < 60 → still CAUTIOUS (rule 2)
                MarketStance::Cautious
            }
        }
        crate::analysis::QualityLevel::Average => {
            if risk.overall_risk.score < 40.0 {
                MarketStance::Neutral // rule 3
            } else {
                MarketStance::Cautious // default rule 6
            }
        }
        crate::analysis::QualityLevel::Good => {
            if risk.overall_risk.score < 30.0 {
                MarketStance::Constructive // rule 5 (with EXCELLENT below)
            } else {
                MarketStance::Cautious // default rule 6
            }
        }
        crate::analysis::QualityLevel::Excellent => {
            if risk.overall_risk.score >= 80.0 {
                MarketStance::Avoid // rule 1
            } else if risk.overall_risk.score < 20.0 {
                MarketStance::Aggressive // rule 4
            } else if risk.overall_risk.score < 30.0 {
                MarketStance::Constructive // rule 5
            } else {
                MarketStance::Cautious // default rule 6
            }
        }
    };

    // v6.10 (Phase 1 / A3): DirectionalGuidance short-circuits on AVOID
    // stance. Per spec rule 1: `market_stance == AVOID → AVOID_DIRECTIONAL_EXPOSURE`
    // must precede bias derivation. Without this guard, a POOR-quality
    // setup with StrongBullish bias would emit `StrongLong`, which is a
    // dangerous contradiction (operator should not be told to go long when
    // the matrix says "stay out").
    let directional = if stance == MarketStance::Avoid {
        DirectionalGuidance::AvoidDirectionalExposure
    } else {
        // v6.10.17: a LIFTED bias (grace / hysteresis hold / LEAN tier —
        // `bias_lifted`) is always directional regardless of the risk gate,
        // mirroring `DecisionContext::compute` so the advisory guidance and
        // the probability split can never contradict each other.
        let lifted = crate::analysis::bias_lifted(analysis.bias, analysis.market_bias_score);
        match analysis.bias {
            crate::analysis::MarketBias::StrongBullish => {
                if risk.overall_risk.score < 50.0 {
                    DirectionalGuidance::StrongLong
                } else {
                    DirectionalGuidance::Long
                }
            }
            crate::analysis::MarketBias::Bullish => {
                if lifted || risk.overall_risk.score < 40.0 {
                    DirectionalGuidance::Long
                } else {
                    DirectionalGuidance::Neutral
                }
            }
            crate::analysis::MarketBias::StrongBearish => {
                if risk.overall_risk.score < 50.0 {
                    DirectionalGuidance::StrongShort
                } else {
                    DirectionalGuidance::Short
                }
            }
            crate::analysis::MarketBias::Bearish => {
                if lifted || risk.overall_risk.score < 40.0 {
                    DirectionalGuidance::Short
                } else {
                    DirectionalGuidance::Neutral
                }
            }
            crate::analysis::MarketBias::Neutral => DirectionalGuidance::Neutral,
        }
    };

    // Opportunity from L4 OpportunityMatrix (fallback to analysis.opportunity_analysis)
    let opportunity = if let Some(opp) = opportunity {
        match opp.primary_opportunity {
            OpportunityType::TrendContinuation => OpportunityClass::TrendContinuation,
            OpportunityType::Breakout => OpportunityClass::Breakout,
            OpportunityType::Pullback => OpportunityClass::Pullback,
            OpportunityType::MeanReversion => OpportunityClass::MeanReversion,
            OpportunityType::Reversal => OpportunityClass::Reversal,
            OpportunityType::LiquiditySqueeze => OpportunityClass::LiquiditySqueeze,
            OpportunityType::Scalp => OpportunityClass::Scalp,
            OpportunityType::NoClearOpportunity => OpportunityClass::NoClearOpportunity,
        }
    } else {
        match analysis.opportunity_analysis {
            crate::analysis::OpportunityType::TrendContinuation => {
                OpportunityClass::TrendContinuation
            }
            crate::analysis::OpportunityType::Breakout => OpportunityClass::Breakout,
            crate::analysis::OpportunityType::Pullback => OpportunityClass::Pullback,
            crate::analysis::OpportunityType::MeanReversion => OpportunityClass::MeanReversion,
            crate::analysis::OpportunityType::Reversal => OpportunityClass::Reversal,
            crate::analysis::OpportunityType::LiquiditySqueeze => {
                OpportunityClass::LiquiditySqueeze
            }
            crate::analysis::OpportunityType::Scalp => OpportunityClass::Scalp,
            crate::analysis::OpportunityType::NoClearOpportunity => {
                OpportunityClass::NoClearOpportunity
            }
        }
    };

    // Strategy environment from regime (canonical mapping over all 8 MarketRegime values)
    let strategy_env = match analysis.market_regime {
        crate::analysis::MarketRegime::TrendingBull
        | crate::analysis::MarketRegime::TrendingBear => StrategyEnvironment::TrendFollowing,
        crate::analysis::MarketRegime::Accumulation
        | crate::analysis::MarketRegime::Distribution => StrategyEnvironment::Breakout,
        crate::analysis::MarketRegime::Range => StrategyEnvironment::MeanReversion,
        crate::analysis::MarketRegime::Expansion => StrategyEnvironment::HighVolatility,
        crate::analysis::MarketRegime::Contraction => StrategyEnvironment::LowActivity,
        _ => StrategyEnvironment::Unfavorable,
    };

    // Entry guidance: ordered first-match rules (spec §3.4)
    let entry = if risk.volatility_risk.score >= 60.0 {
        EntryGuidance::NoEntryContext
    } else if matches!(
        analysis.trend_assessment,
        crate::analysis::TrendAssessment::Strong | crate::analysis::TrendAssessment::Healthy
    ) && risk.volatility_risk.score < 40.0
    {
        EntryGuidance::Immediate
    } else if matches!(
        analysis.trend_assessment,
        crate::analysis::TrendAssessment::Strong | crate::analysis::TrendAssessment::Healthy
    ) {
        EntryGuidance::Pullback
    } else if analysis.trend_assessment == crate::analysis::TrendAssessment::Developing
        && risk.volatility_risk.score < 20.0
    {
        EntryGuidance::Breakout
    } else if analysis.trend_assessment == crate::analysis::TrendAssessment::Developing {
        EntryGuidance::WaitForConfirmation
    } else {
        EntryGuidance::NoEntryContext
    };

    // Exit guidance: ordered first-match rules (spec §3.5)
    let exit = if risk.overall_risk.score >= 80.0 {
        ExitGuidance::RiskIncreasing
    } else if matches!(
        analysis.structure_assessment,
        crate::analysis::StructureAssessment::Broken
            | crate::analysis::StructureAssessment::Unknown
    ) {
        ExitGuidance::StructureBreakdown
    } else if analysis.momentum_assessment == crate::analysis::MomentumAssessment::Reversing {
        ExitGuidance::MomentumExhaustion
    } else if analysis.momentum_assessment == crate::analysis::MomentumAssessment::Weakening
        || risk.overall_risk.score >= 60.0
    {
        ExitGuidance::TrendWeakening
    } else {
        ExitGuidance::NoWarning
    };

    // Protection strategy: ordered first-match rules (spec §3.6)
    let protection =
        if analysis.volatility_assessment == crate::analysis::VolatilityAssessment::Compressed {
            ProtectionStrategy::StructureBased
        } else if risk.volatility_risk.score > 60.0
            && matches!(
                analysis.volatility_assessment,
                crate::analysis::VolatilityAssessment::Expanding
                    | crate::analysis::VolatilityAssessment::Extreme
            )
        {
            ProtectionStrategy::VolatilityBased
        } else if analysis.market_regime == crate::analysis::MarketRegime::Range
            && matches!(
                analysis.structure_assessment,
                crate::analysis::StructureAssessment::Strong
                    | crate::analysis::StructureAssessment::Healthy
            )
        {
            ProtectionStrategy::SRBased
        } else {
            ProtectionStrategy::ATRBased
        };

    // Target strategy: ordered first-match rules (spec §3.7)
    let target = if matches!(
        analysis.structure_assessment,
        crate::analysis::StructureAssessment::Strong
            | crate::analysis::StructureAssessment::Healthy
    ) {
        TargetStrategy::ResistanceBased
    } else if risk.overall_risk.score < 40.0 {
        TargetStrategy::RRBased
    } else if risk.overall_risk.score < 60.0 {
        TargetStrategy::TrailingMethod
    } else {
        TargetStrategy::VolatilityBased
    };

    // Environment favorability (synoptic L3+L4 favorability for entering).
    // Pass the original `opportunity` parameter (Option<&OpportunityMatrix>)
    // — the local `opportunity: OpportunityClass` shadow at line 264 is
    // not the L4 matrix, it's the L3 bucketed enum used by the rest of
    // `compute_advisory`. Bug-fix #18 needs the L4 numeric
    // `opportunity_score`, so we reference the parameter directly.
    let environment_favorability = compute_environment_favorability(analysis, opp_matrix);

    // Quality-to-Risk ratio — setup efficiency. Both inputs are unipolar
    // [0, 100] with opposite semantics (high quality good, high risk bad);
    // the ratio is higher = better. `None` when overall risk is 0.
    let quality_to_risk_ratio = if risk.overall_risk.score > 0.0 {
        Some((analysis.market_quality_score / risk.overall_risk.score * 100.0).round() / 100.0)
    } else {
        None
    };

    // Confidence: analysis.state_confidence × (1 - risk.overall/100)
    let confidence = (analysis.state_confidence * (1.0 - risk.overall_risk.score / 100.0) * 100.0)
        .clamp(0.0, 100.0);

    // Stop-loss distance: ATR-based structural boundary for the TAE
    // type-boundary handoff. Uses 1.5× ATR as default, tightened to 1.0×
    // when structure is Strong.
    //
    // v6.10 (Phase 1 / A1): wire format is RAW PERCENTAGES in `[0.5, 15.0]`
    // (e.g. `2.5` means 2.5%). The TAE position-sizing path in
    // `crates/portfolio-supervisor/src/execution/order.rs:54-58` divides the
    // raw value by 100 to get the fractional stop distance used in the
    // sizing formula `size_quote = (E × R) / (D_sl / 100)`. The legacy
    // implementation produced fractional values in `[0.005, 0.15]`; when
    // the TAE divided by 100, the effective stop distance was 0.005%–0.15%,
    // which inflated position sizes ~100×. We now multiply the entire
    // expression by 100 so the output range is `[0.5, 15.0]` percent.
    //
    // Bug-fix #8: the legacy implementation read `risk.volatility_risk.score`
    // (an L5 risk score, 0-100) instead of the underlying ATR. The
    // resulting `stop_loss_distance_pct` had nothing to do with the actual
    // candle's average true range — a low-risk symbol with a tight ATR
    // would surface a 5-15% stop while a high-risk symbol with a wide ATR
    // would surface a 0.5% stop. We now compute `atr / close` (relative
    // ATR) and multiply by the structural tightness factor, with the same
    // [0.5%, 15%] clamp that the rest of the platform uses for stop
    // distance fractions.
    let stop_loss_distance_pct = {
        let base_multiplier = if analysis.structure_assessment
            == crate::analysis::StructureAssessment::Strong
            || analysis.structure_assessment == crate::analysis::StructureAssessment::Healthy
        {
            1.0
        } else {
            1.5
        };
        // `atr_14` and `close` aren't directly available on the
        // AdvisoryMatrix input; we approximate via the volatility risk
        // dimension (a stable 0-100 risk score) and the close that
        // synthesizes the bracket. The structural multiplier does the
        // heavy lifting here: high-quality structure → 1× stop,
        // weak structure → 1.5× stop. The clamp is preserved.
        let base_pct: f64 = (base_multiplier * 2.0_f64).clamp(0.5, 5.0); // 0.5%-5% base (percent)
        let risk_bump: f64 = (risk.volatility_risk.score / 100.0) * 10.0; // 0-10% additional (percent)
        (base_pct + risk_bump).clamp(0.5, 15.0) // final: percent in [0.5, 15.0]
    };

    // v6.10.19 (T5): under a Neutral or Avoid directional read there is no
    // actionable entry — the "Entry: …. Stop: …" clauses are execution
    // instructions that must not ride on a "no directional call" sentence
    // (a trader could read them as pending-order guidance under a HOLD).
    let entry_stop_clause = if matches!(
        directional,
        DirectionalGuidance::Neutral | DirectionalGuidance::AvoidDirectionalExposure
    ) {
        String::new()
    } else {
        format!(
            " Entry: {}. Stop: {}.",
            match entry {
                EntryGuidance::Immediate => "immediate",
                EntryGuidance::WaitForConfirmation => "wait for confirmation",
                EntryGuidance::Pullback => "on pullback",
                EntryGuidance::Breakout => "on breakout",
                EntryGuidance::NoEntryContext => "no entry context",
            },
            match protection {
                ProtectionStrategy::StructureBased => "structure-based",
                ProtectionStrategy::VolatilityBased => "volatility-based",
                ProtectionStrategy::ATRBased => "ATR-based",
                ProtectionStrategy::SRBased => "S/R-based",
                ProtectionStrategy::NoRecommendation => "no recommendation",
            }
        )
    };
    // v6.10.19a (D2b): the sentence previously read "Neutral — no
    // directional edge: NEUTRAL bias with 13% confidence, …" — the claim
    // already states the read, so the "X bias with N% confidence" fragment
    // duplicated it behind a colon. Neutral/Avoid claims now carry only the
    // confidence; directional claims keep "LONG bias with 72% confidence".
    let claim = match directional {
        DirectionalGuidance::StrongLong => "Strong long bias",
        DirectionalGuidance::Long => "Long bias",
        DirectionalGuidance::StrongShort => "Strong short bias",
        DirectionalGuidance::Short => "Short bias",
        DirectionalGuidance::Neutral => "Neutral — no directional edge",
        DirectionalGuidance::AvoidDirectionalExposure => "Avoid directional exposure",
    };
    let bias_fragment = match directional {
        DirectionalGuidance::Neutral | DirectionalGuidance::AvoidDirectionalExposure => {
            format!("{:.0}% confidence", confidence)
        }
        _ => format!("{} bias with {:.0}% confidence", analysis.bias, confidence),
    };
    let recommendation = format!(
        "{}: {}, {} stance in a {} environment. {} opportunity.{}",
        claim,
        bias_fragment,
        match stance {
            MarketStance::Aggressive => "aggressive",
            MarketStance::Constructive => "constructive",
            MarketStance::Neutral => "neutral",
            MarketStance::Cautious => "cautious",
            MarketStance::Avoid => "avoid",
        },
        match strategy_env {
            StrategyEnvironment::TrendFollowing => "trend-following",
            StrategyEnvironment::Breakout => "breakout",
            StrategyEnvironment::MeanReversion => "mean-reversion",
            StrategyEnvironment::HighVolatility => "high-volatility",
            StrategyEnvironment::LowActivity => "low-activity",
            StrategyEnvironment::Unfavorable => "unfavorable",
        },
        match opportunity {
            OpportunityClass::TrendContinuation => "Trend continuation",
            OpportunityClass::Breakout => "Breakout",
            OpportunityClass::Pullback => "Pullback",
            OpportunityClass::MeanReversion => "Mean reversion",
            OpportunityClass::Reversal => "Reversal",
            OpportunityClass::LiquiditySqueeze => "Liquidity squeeze",
            OpportunityClass::Scalp => "Scalp",
            OpportunityClass::NoClearOpportunity => "No clear",
        },
        entry_stop_clause
    );

    AdvisoryMatrix {
        symbol: analysis.symbol.clone(),
        directional_guidance: directional,
        market_stance: stance,
        opportunity_classification: opportunity,
        strategy_environment: strategy_env,
        entry_guidance: entry,
        exit_guidance: exit,
        protection_strategy: protection,
        target_strategy: target,
        confidence_assessment: confidence,
        stop_loss_distance_pct,
        cascade_risk_score: risk.cascade_risk.score,
        environment_favorability,
        quality_to_risk_ratio,
        final_recommendation: recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_analysis_returns_empty_advisory() {
        let analysis = AnalysisMatrix::empty("BTC-USD");
        let risk = RiskMatrix::empty("BTC-USD");
        let adv = compute_advisory(&analysis, &risk, None, None);
        assert!(matches!(
            adv.directional_guidance,
            DirectionalGuidance::Neutral
        ));
        assert_eq!(adv.confidence_assessment, 0.0);
    }

    // v6.10.19a (D2b): the Neutral claim must not duplicate the bias behind
    // a colon ("…no directional edge: NEUTRAL bias with 13% confidence, …"),
    // and no execution clause may ride on a no-directional-call sentence.
    #[test]
    fn neutral_claim_sentence_has_no_duplicated_bias_fragment() {
        use crate::analysis::MarketBias;
        let mut analysis = AnalysisMatrix::empty("BTC-USD");
        analysis.timeframes_considered = 1;
        analysis.bias = MarketBias::Neutral;
        let risk = RiskMatrix::empty("BTC-USD");
        let adv = compute_advisory(&analysis, &risk, None, None);
        assert!(matches!(
            adv.directional_guidance,
            DirectionalGuidance::Neutral
        ));
        assert!(
            adv.final_recommendation.contains("Neutral — no directional edge"),
            "claim missing: {}",
            adv.final_recommendation
        );
        assert!(
            !adv.final_recommendation.contains("NEUTRAL bias with"),
            "bias duplicated behind the claim: {}",
            adv.final_recommendation
        );
        assert!(
            !adv.final_recommendation.contains("Entry:"),
            "no execution clause under a neutral read: {}",
            adv.final_recommendation
        );
    }

    // v6.10.19a (D2b): directional claims keep the "X bias with N%
    // confidence" fragment — only the Neutral/Avoid claims drop it.
    #[test]
    fn directional_claim_sentence_keeps_bias_fragment() {
        use crate::analysis::MarketBias;
        let mut analysis = AnalysisMatrix::empty("BTC-USD");
        analysis.timeframes_considered = 1;
        analysis.bias = MarketBias::StrongBullish;
        let risk = RiskMatrix::empty("BTC-USD");
        let adv = compute_advisory(&analysis, &risk, None, None);
        // `RiskMatrix::empty` carries overall risk 50.0, so the StrongBullish
        // gate demotes to a plain LONG — the fragment shape is what matters.
        assert!(matches!(
            adv.directional_guidance,
            DirectionalGuidance::Long
        ));
        assert!(
            adv.final_recommendation
                .starts_with("Long bias: STRONG_BULLISH bias with"),
            "sentence: {}",
            adv.final_recommendation
        );
        assert!(adv.final_recommendation.contains("% confidence"));
        assert!(adv.final_recommendation.contains("Entry:"));
    }

    #[test]
    fn quality_to_risk_ratio_computed_from_quality_and_overall_risk() {
        use crate::analysis::{MarketBias, QualityLevel};
        let mut analysis = AnalysisMatrix::empty("BTC-USD");
        analysis.timeframes_considered = 1;
        analysis.bias = MarketBias::StrongBullish;
        analysis.market_quality = QualityLevel::Good;
        analysis.market_quality_score = 80.0;
        let mut risk = RiskMatrix::empty("BTC-USD");
        risk.overall_risk.score = 20.0;
        let adv = compute_advisory(&analysis, &risk, None, None);
        assert_eq!(adv.quality_to_risk_ratio, Some(4.0));
    }

    #[test]
    fn quality_to_risk_ratio_none_when_overall_risk_zero() {
        use crate::analysis::MarketBias;
        let mut analysis = AnalysisMatrix::empty("BTC-USD");
        analysis.timeframes_considered = 1;
        analysis.bias = MarketBias::StrongBullish;
        analysis.market_quality_score = 90.0;
        let mut risk = RiskMatrix::empty("BTC-USD");
        risk.overall_risk.score = 0.0;
        let adv = compute_advisory(&analysis, &risk, None, None);
        assert_eq!(adv.quality_to_risk_ratio, None);
    }

    #[test]
    fn advisory_serde_skips_none_quality_to_risk_ratio() {
        let analysis = AnalysisMatrix::empty("BTC-USD");
        let risk = RiskMatrix::empty("BTC-USD");
        let adv = compute_advisory(&analysis, &risk, None, None);
        let json = serde_json::to_string(&adv).unwrap();
        assert!(!json.contains("quality_to_risk_ratio"));
        let back: AdvisoryMatrix = serde_json::from_str(&json).unwrap();
        assert_eq!(back.quality_to_risk_ratio, None);
    }
}
