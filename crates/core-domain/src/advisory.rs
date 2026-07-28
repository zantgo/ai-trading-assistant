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
            final_recommendation: "Insufficient data to provide guidance.".into(),
        }
    }
}

/// Compute `environment_favorability` — synoptic favorability of entering
/// a position. Synthesizes L3 `market_quality` and an opportunity proxy
/// derived from `OpportunityType` (since the Opportunity Matrix's
/// numeric `opportunity_score` is not yet available at L6 synthesis
/// time). Lower score = more favorable environment.
fn compute_environment_favorability(analysis: &AnalysisMatrix) -> RiskDimension {
    let quality_penalty: f64 = match analysis.market_quality {
        crate::analysis::QualityLevel::Excellent => 10.0,
        crate::analysis::QualityLevel::Good => 25.0,
        crate::analysis::QualityLevel::Average => 50.0,
        crate::analysis::QualityLevel::Weak => 70.0,
        crate::analysis::QualityLevel::Poor => 80.0,
    };
    let opportunity_score: f64 = match analysis.opportunity_analysis {
        crate::analysis::OpportunityType::NoClearOpportunity => 20.0,
        _ => 80.0,
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

    // Directional guidance from bias × risk
    let directional = match analysis.bias {
        crate::analysis::MarketBias::StrongBullish => {
            if risk.overall_risk.score < 50.0 {
                DirectionalGuidance::StrongLong
            } else {
                DirectionalGuidance::Long
            }
        }
        crate::analysis::MarketBias::Bullish => {
            if risk.overall_risk.score < 40.0 {
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
            if risk.overall_risk.score < 40.0 {
                DirectionalGuidance::Short
            } else {
                DirectionalGuidance::Neutral
            }
        }
        crate::analysis::MarketBias::Neutral => DirectionalGuidance::Neutral,
    };

    // Market stance from quality × risk
    let stance = match analysis.market_quality {
        crate::analysis::QualityLevel::Excellent => MarketStance::Aggressive,
        crate::analysis::QualityLevel::Good => {
            if risk.overall_risk.score < 50.0 {
                MarketStance::Constructive
            } else {
                MarketStance::Neutral
            }
        }
        crate::analysis::QualityLevel::Average => {
            if risk.overall_risk.score > 60.0 {
                MarketStance::Cautious
            } else {
                MarketStance::Neutral
            }
        }
        crate::analysis::QualityLevel::Weak => {
            if risk.overall_risk.score > 40.0 {
                MarketStance::Avoid
            } else {
                MarketStance::Cautious
            }
        }
        crate::analysis::QualityLevel::Poor => MarketStance::Avoid,
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

    // Environment favorability (synoptic L3+L4 favorability for entering)
    let environment_favorability = compute_environment_favorability(analysis);

    // Confidence: analysis.state_confidence × (1 - risk.overall/100)
    let confidence = (analysis.state_confidence * (1.0 - risk.overall_risk.score / 100.0) * 100.0)
        .clamp(0.0, 100.0);

    // Stop-loss distance: ATR-based structural boundary for the TAE type-boundary handoff.
    // Uses 1.5× ATR as default, tightened to 1.0× when structure is Strong.
    let stop_loss_distance_pct = {
        let base_multiplier = if analysis.structure_assessment
            == crate::analysis::StructureAssessment::Strong
            || analysis.structure_assessment == crate::analysis::StructureAssessment::Healthy
        {
            1.0
        } else {
            1.5
        };
        (base_multiplier * risk.volatility_risk.score.max(1.0) / 100.0).clamp(0.005, 0.15)
    };

    let recommendation = format!(
        "{}: {} bias with {} confidence, {} stance in a {} environment. {} opportunity. Entry: {}. Stop: {}.",
        match directional {
            DirectionalGuidance::StrongLong => "Strong long bias",
            DirectionalGuidance::Long => "Long bias",
            DirectionalGuidance::StrongShort => "Strong short bias",
            DirectionalGuidance::Short => "Short bias",
            DirectionalGuidance::Neutral => "Neutral — no directional edge",
            DirectionalGuidance::AvoidDirectionalExposure => "Avoid directional exposure",
        },
        analysis.bias,
        format!("{:.0}%", confidence),
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
}
