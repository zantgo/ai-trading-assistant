//! # Advisory Matrix — Decision Guidance Layer
//!
//! The Advisory Matrix transforms complete market intelligence and risk
//! assessment into structured human-facing guidance. It consumes the Analysis
//! Matrix and Risk Matrix to provide an explainable recommendation framework.
//! It does not execute trades — it explains and recommends.
//!
//! Layer: L4.75 in the architecture (Decision Guidance).

use crate::analysis::AnalysisMatrix;
use crate::risk::RiskMatrix;
use serde::{Deserialize, Serialize};

/// Directional guidance classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketStance {
    Aggressive,
    Constructive,
    Neutral,
    Cautious,
    Avoid,
}

/// Opportunity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpportunityClass {
    TrendContinuation,
    Breakout,
    Pullback,
    MeanReversion,
    Reversal,
    LiquiditySqueeze,
    NoClearOpportunity,
}

/// Strategy environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntryGuidance {
    Immediate,
    WaitForConfirmation,
    Pullback,
    Breakout,
    NoEntryContext,
}

/// Exit guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExitGuidance {
    TrendWeakening,
    MomentumExhaustion,
    StructureBreakdown,
    RiskIncreasing,
    NoWarning,
}

/// Protection strategy guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtectionStrategy {
    StructureBased,
    VolatilityBased,
    ATRBased,
    SRBased,
    NoRecommendation,
}

/// Target strategy guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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
            final_recommendation: "Insufficient data to provide guidance.".into(),
        }
    }
}

/// Compute Advisory Matrix from Analysis + Risk.
pub fn compute_advisory(
    analysis: &AnalysisMatrix,
    risk: &RiskMatrix,
) -> AdvisoryMatrix {
    if analysis.timeframes_considered == 0 {
        return AdvisoryMatrix::empty(&analysis.symbol);
    }

    // Directional guidance from bias × risk
    let directional = match analysis.bias {
        crate::analysis::MarketBias::StrongBullish => {
            if risk.overall_risk.score < 50.0 { DirectionalGuidance::StrongLong }
            else { DirectionalGuidance::Long }
        }
        crate::analysis::MarketBias::Bullish => {
            if risk.overall_risk.score < 40.0 { DirectionalGuidance::Long }
            else { DirectionalGuidance::Neutral }
        }
        crate::analysis::MarketBias::StrongBearish => {
            if risk.overall_risk.score < 50.0 { DirectionalGuidance::StrongShort }
            else { DirectionalGuidance::Short }
        }
        crate::analysis::MarketBias::Bearish => {
            if risk.overall_risk.score < 40.0 { DirectionalGuidance::Short }
            else { DirectionalGuidance::Neutral }
        }
        crate::analysis::MarketBias::Neutral => DirectionalGuidance::Neutral,
    };

    // Market stance from quality × risk
    let stance = match analysis.market_quality {
        crate::analysis::QualityLevel::Excellent => MarketStance::Aggressive,
        crate::analysis::QualityLevel::Good => {
            if risk.overall_risk.score < 50.0 { MarketStance::Constructive }
            else { MarketStance::Neutral }
        }
        crate::analysis::QualityLevel::Average => {
            if risk.overall_risk.score > 60.0 { MarketStance::Cautious }
            else { MarketStance::Neutral }
        }
        crate::analysis::QualityLevel::Weak => {
            if risk.overall_risk.score > 40.0 { MarketStance::Avoid }
            else { MarketStance::Cautious }
        }
        crate::analysis::QualityLevel::Poor => MarketStance::Avoid,
    };

    // Opportunity from analysis
    let opportunity = match analysis.opportunity_analysis {
        crate::analysis::OpportunityType::TrendContinuation => OpportunityClass::TrendContinuation,
        crate::analysis::OpportunityType::Breakout => OpportunityClass::Breakout,
        crate::analysis::OpportunityType::Pullback => OpportunityClass::Pullback,
        crate::analysis::OpportunityType::MeanReversion => OpportunityClass::MeanReversion,
        crate::analysis::OpportunityType::Reversal => OpportunityClass::Reversal,
        crate::analysis::OpportunityType::LiquiditySqueeze => OpportunityClass::LiquiditySqueeze,
        crate::analysis::OpportunityType::NoClearOpportunity => OpportunityClass::NoClearOpportunity,
    };

    // Strategy environment from regime + volatility
    let strategy_env = match analysis.market_regime {
        crate::analysis::MarketRegime::TrendingBull | crate::analysis::MarketRegime::TrendingBear => {
            StrategyEnvironment::TrendFollowing
        }
        crate::analysis::MarketRegime::Expansion => StrategyEnvironment::Breakout,
        crate::analysis::MarketRegime::Range | crate::analysis::MarketRegime::Contraction => {
            StrategyEnvironment::MeanReversion
        }
        _ => StrategyEnvironment::Unfavorable,
    };

    // Entry guidance from trend quality + risk
    let entry = match analysis.trend_assessment {
        crate::analysis::TrendAssessment::Strong | crate::analysis::TrendAssessment::Healthy => {
            if risk.volatility_risk.score < 50.0 { EntryGuidance::Immediate }
            else { EntryGuidance::WaitForConfirmation }
        }
        crate::analysis::TrendAssessment::Developing => {
            if risk.overall_risk.score < 50.0 { EntryGuidance::Pullback }
            else { EntryGuidance::WaitForConfirmation }
        }
        _ => EntryGuidance::NoEntryContext,
    };

    // Exit guidance from momentum + risk
    let exit = match analysis.momentum_assessment {
        crate::analysis::MomentumAssessment::Exhausted => ExitGuidance::MomentumExhaustion,
        crate::analysis::MomentumAssessment::Reversing => ExitGuidance::MomentumExhaustion,
        crate::analysis::MomentumAssessment::Weakening => {
            if risk.overall_risk.score > 50.0 { ExitGuidance::RiskIncreasing }
            else { ExitGuidance::TrendWeakening }
        }
        _ => ExitGuidance::NoWarning,
    };

    // Stop loss from volatility + structure
    let protection = if analysis.volatility_assessment == crate::analysis::VolatilityAssessment::Compressed {
        ProtectionStrategy::StructureBased
    } else if risk.volatility_risk.score > 60.0 {
        ProtectionStrategy::VolatilityBased
    } else {
        ProtectionStrategy::ATRBased
    };

    // Take profit from structure + reward risk
    let target = if analysis.structure_assessment == crate::analysis::StructureAssessment::Strong
        || analysis.structure_assessment == crate::analysis::StructureAssessment::Healthy {
        TargetStrategy::ResistanceBased
    } else if risk.reward_risk.score < 40.0 {
        TargetStrategy::RRBased
    } else {
        TargetStrategy::VolatilityBased
    };

    // Confidence: analysis.confidence × (1 - risk.overall/100)
    let confidence = (analysis.confidence * (1.0 - risk.overall_risk.score / 100.0) * 100.0).clamp(0.0, 100.0);

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
        let adv = compute_advisory(&analysis, &risk);
        assert!(matches!(adv.directional_guidance, DirectionalGuidance::Neutral));
        assert_eq!(adv.confidence_assessment, 0.0);
    }
}
