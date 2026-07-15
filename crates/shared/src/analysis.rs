//! # Analysis Matrix — Market Intelligence Layer
//!
//! The Analysis Matrix transforms structured observations and multi-timeframe
//! relationships into a complete interpretation of current market conditions.
//! It represents the transition from market observation to market understanding.
//!
//! Layer: L4.5 in the architecture (Market Intelligence).

use crate::alignment::AlignmentMatrix;
use serde::{Deserialize, Serialize};

/// Directional market bias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketBias {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

impl std::fmt::Display for MarketBias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketBias::StrongBullish => write!(f, "STRONG_BULLISH"),
            MarketBias::Bullish => write!(f, "BULLISH"),
            MarketBias::Neutral => write!(f, "NEUTRAL"),
            MarketBias::Bearish => write!(f, "BEARISH"),
            MarketBias::StrongBearish => write!(f, "STRONG_BEARISH"),
        }
    }
}

/// Market regime classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketRegime {
    TrendingBull,
    TrendingBear,
    Range,
    Accumulation,
    Distribution,
    Expansion,
    Contraction,
    Transition,
}

impl std::fmt::Display for MarketRegime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketRegime::TrendingBull => write!(f, "TRENDING_BULL"),
            MarketRegime::TrendingBear => write!(f, "TRENDING_BEAR"),
            MarketRegime::Range => write!(f, "RANGE"),
            MarketRegime::Accumulation => write!(f, "ACCUMULATION"),
            MarketRegime::Distribution => write!(f, "DISTRIBUTION"),
            MarketRegime::Expansion => write!(f, "EXPANSION"),
            MarketRegime::Contraction => write!(f, "CONTRACTION"),
            MarketRegime::Transition => write!(f, "TRANSITION"),
        }
    }
}

/// Trend quality assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrendAssessment {
    Weak,
    Developing,
    Healthy,
    Strong,
    Exhausted,
}

/// Momentum state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MomentumAssessment {
    Increasing,
    Stable,
    Weakening,
    Exhausted,
    Reversing,
}

/// Structure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StructureAssessment {
    Strong,
    Healthy,
    Weak,
    Broken,
    Unclear,
}

/// Volatility state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VolatilityAssessment {
    Compressed,
    Normal,
    Expanding,
    Extreme,
    Unstable,
}

/// Volume participation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VolumeAssessment {
    Weak,
    Normal,
    Strong,
    Exceptional,
}

/// Opportunity type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpportunityType {
    TrendContinuation,
    Breakout,
    Pullback,
    MeanReversion,
    Reversal,
    /// Phase 3: a cascade or cluster setup signals a likely squeeze.
    LiquiditySqueeze,
    NoClearOpportunity,
}

/// Market quality level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QualityLevel {
    Poor,
    Weak,
    Average,
    Good,
    Excellent,
}

/// Analysis Matrix — complete market interpretation per symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisMatrix {
    pub symbol: String,
    pub bias: MarketBias,
    pub confidence: f64,
    pub market_regime: MarketRegime,
    pub trend_assessment: TrendAssessment,
    pub momentum_assessment: MomentumAssessment,
    pub structure_assessment: StructureAssessment,
    pub volatility_assessment: VolatilityAssessment,
    pub volume_assessment: VolumeAssessment,
    pub opportunity_analysis: OpportunityType,
    pub market_quality: QualityLevel,
    pub market_interpretation: String,
    pub rationale: String,
    pub supporting_signals: Vec<String>,
    pub contradicting_signals: Vec<String>,
    pub timeframes_considered: u8,
}

impl AnalysisMatrix {
    pub fn empty(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            bias: MarketBias::Neutral,
            confidence: 0.0,
            market_regime: MarketRegime::Transition,
            trend_assessment: TrendAssessment::Weak,
            momentum_assessment: MomentumAssessment::Stable,
            structure_assessment: StructureAssessment::Unclear,
            volatility_assessment: VolatilityAssessment::Normal,
            volume_assessment: VolumeAssessment::Normal,
            opportunity_analysis: OpportunityType::NoClearOpportunity,
            market_quality: QualityLevel::Poor,
            market_interpretation: "No data available — no candles have been completed.".into(),
            rationale: String::new(),
            supporting_signals: Vec::new(),
            contradicting_signals: Vec::new(),
            timeframes_considered: 0,
        }
    }
}

/// Derive an Analysis Matrix from the Alignment Matrix.
pub fn derive_analysis(alignment: &AlignmentMatrix) -> AnalysisMatrix {
    if alignment.timeframes_present == 0 {
        return AnalysisMatrix::empty(&alignment.symbol);
    }

    let score = alignment.mtf_overall_score;
    let bias = if score > 40.0 { MarketBias::StrongBullish }
        else if score > 20.0 { MarketBias::Bullish }
        else if score < -40.0 { MarketBias::StrongBearish }
        else if score < -20.0 { MarketBias::Bearish }
        else { MarketBias::Neutral };

    let base_confidence = (score.abs() / 100.0).max(0.0).min(1.0);
    let mut confidence = base_confidence;
    if alignment.trend_agreement_pct >= 75.0 { confidence = (confidence + 0.15).min(1.0); }
    else if alignment.trend_agreement_pct < 50.0 { confidence = confidence.min(0.5); }
    if alignment.signal_cross_tf_count >= 3 { confidence = (confidence + 0.1).min(1.0); }
    if alignment.timeframes_present <= 1 { confidence = confidence.min(0.5); }

    // Market regime from alignment context
    let regime = if score > 20.0 { MarketRegime::TrendingBull }
        else if score < -20.0 { MarketRegime::TrendingBear }
        else { MarketRegime::Range };

    // Trend assessment from alignment trend dimension
    let trend_dim = alignment.dimensions.get(0).map(|d| d.score).unwrap_or(50.0);
    let trend_assessment = if trend_dim >= 90.0 { TrendAssessment::Strong }
        else if trend_dim >= 75.0 { TrendAssessment::Healthy }
        else if trend_dim >= 50.0 { TrendAssessment::Developing }
        else if trend_dim >= 25.0 { TrendAssessment::Weak }
        else { TrendAssessment::Exhausted };

    // Momentum from alignment momentum dimension
    let mom_dim = alignment.dimensions.get(1).map(|d| d.score).unwrap_or(50.0);
    let momentum_assessment = if mom_dim >= 80.0 { MomentumAssessment::Increasing }
        else if mom_dim >= 60.0 { MomentumAssessment::Stable }
        else if mom_dim >= 40.0 { MomentumAssessment::Weakening }
        else { MomentumAssessment::Reversing };

    // Structure from alignment structure dimension
    let struct_dim = alignment.dimensions.get(4).map(|d| d.score).unwrap_or(50.0);
    let structure_assessment = if struct_dim >= 80.0 { StructureAssessment::Strong }
        else if struct_dim >= 60.0 { StructureAssessment::Healthy }
        else if struct_dim >= 40.0 { StructureAssessment::Weak }
        else if struct_dim >= 20.0 { StructureAssessment::Broken }
        else { StructureAssessment::Unclear };

    // Volatility from alignment volatility dimension
    let vol_dim = alignment.dimensions.get(3).map(|d| d.score).unwrap_or(50.0);
    let volatility_assessment = if vol_dim >= 90.0 { VolatilityAssessment::Extreme }
        else if vol_dim >= 70.0 { VolatilityAssessment::Expanding }
        else if vol_dim >= 40.0 { VolatilityAssessment::Normal }
        else if vol_dim >= 20.0 { VolatilityAssessment::Compressed }
        else { VolatilityAssessment::Unstable };

    // Volume from alignment volume dimension
    let volu_dim = alignment.dimensions.get(2).map(|d| d.score).unwrap_or(50.0);
    let volume_assessment = if volu_dim >= 90.0 { VolumeAssessment::Exceptional }
        else if volu_dim >= 70.0 { VolumeAssessment::Strong }
        else if volu_dim >= 40.0 { VolumeAssessment::Normal }
        else { VolumeAssessment::Weak };

    // Opportunity from alignment opportunity dimension
    let opp_dim = alignment.dimensions.get(9).map(|d| d.score).unwrap_or(50.0);
    let opportunity = if trend_dim >= 75.0 && bias as i32 > 0 {
        OpportunityType::TrendContinuation
    } else if vol_dim >= 70.0 && struct_dim >= 60.0 {
        OpportunityType::Breakout
    } else if trend_dim >= 60.0 && momentum_assessment == MomentumAssessment::Weakening {
        OpportunityType::Pullback
    } else if vol_dim <= 30.0 {
        OpportunityType::MeanReversion
    } else if opp_dim < 30.0 {
        OpportunityType::NoClearOpportunity
    } else if opp_dim >= 90.0 && vol_dim >= 60.0 {
        // Phase 3: very high opportunity score with elevated volatility
        // can indicate a squeeze setup. Detailed cascade/squeeze logic
        // lives in the Decision layer (which can override this).
        OpportunityType::LiquiditySqueeze
    } else {
        OpportunityType::TrendContinuation
    };

    // Market quality aggregate
    let quality_score = (trend_dim + mom_dim + struct_dim + volu_dim) / 4.0;
    let market_quality = if quality_score >= 80.0 { QualityLevel::Excellent }
        else if quality_score >= 65.0 { QualityLevel::Good }
        else if quality_score >= 50.0 { QualityLevel::Average }
        else if quality_score >= 35.0 { QualityLevel::Weak }
        else { QualityLevel::Poor };

    let mut rationale_parts: Vec<String> = Vec::new();
    rationale_parts.push(format!(
        "MTF overall score {:.0}/100 → {}. {} of {} timeframes agree ({:.0}%).",
        score, bias, if alignment.trend_agreement_pct >= 50.0 { "Majority" } else { "Minority" },
        alignment.timeframes_present, alignment.trend_agreement_pct
    ));
    if alignment.signal_cross_tf_count > 0 {
        rationale_parts.push(format!(
            "{} signals across multiple timeframes.", alignment.signal_cross_tf_count
        ));
    }

    let mut supporting: Vec<String> = Vec::new();
    let mut contradicting: Vec<String> = Vec::new();
    for tf in &alignment.timeframe_alignments {
        let dir = if tf.overall_score > 0 { "bullish" } else if tf.overall_score < 0 { "bearish" } else { "neutral" };
        let label = format!("{} ({}): score {:+}, {} regime, {} signals",
            tf.timeframe, dir, tf.overall_score, tf.regime, tf.active_signals);
        if (bias == MarketBias::Bullish || bias == MarketBias::StrongBullish) && tf.overall_score > 0 { supporting.push(label); }
        else if (bias == MarketBias::Bearish || bias == MarketBias::StrongBearish) && tf.overall_score < 0 { supporting.push(label); }
        else if bias == MarketBias::Neutral && tf.overall_score.abs() < 10 { supporting.push(label); }
        else { contradicting.push(label); }
    }

    let interpretation = format!(
        "{} market with {} trend, {} momentum, {} structure, {} volatility, and {} volume participation. {}",
        match regime {
            MarketRegime::TrendingBull => "Bullish trending",
            MarketRegime::TrendingBear => "Bearish trending",
            MarketRegime::Range => "Ranging",
            _ => "Transitional",
        },
        format!("{:?}", trend_assessment).to_lowercase(),
        format!("{:?}", momentum_assessment).to_lowercase(),
        format!("{:?}", structure_assessment).to_lowercase(),
        format!("{:?}", volatility_assessment).to_lowercase(),
        format!("{:?}", volume_assessment).to_lowercase(),
        match opportunity {
            OpportunityType::TrendContinuation => "Favors trend continuation.",
            OpportunityType::Breakout => "Breakout conditions present.",
            OpportunityType::Pullback => "Pullback opportunity forming.",
            OpportunityType::MeanReversion => "Mean reversion conditions detected.",
            OpportunityType::Reversal => "Reversal signals emerging.",
            OpportunityType::LiquiditySqueeze => "Liquidity squeeze setup (Phase 3).",
            OpportunityType::NoClearOpportunity => "No clear opportunity identified.",
        }
    );

    AnalysisMatrix {
        symbol: alignment.symbol.clone(),
        bias,
        confidence,
        market_regime: regime,
        trend_assessment,
        momentum_assessment,
        structure_assessment,
        volatility_assessment,
        volume_assessment,
        opportunity_analysis: opportunity,
        market_quality,
        market_interpretation: interpretation,
        rationale: rationale_parts.join(" "),
        supporting_signals: supporting,
        contradicting_signals: contradicting,
        timeframes_considered: alignment.timeframes_present,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::{AlignmentMatrix, TfAlignmentInfo};

    fn simple_alignment(tfs: u8, score: f64, agreement: f64, cross_tf: u32) -> AlignmentMatrix {
        let mut alignments = Vec::new();
        let labels = ["micro60", "fast180", "slow300", "macro900"];
        let secs = [60, 180, 300, 900];
        for i in 0..tfs as usize {
            alignments.push(TfAlignmentInfo {
                timeframe: labels[i].to_string(), timeframe_secs: secs[i],
                trend_score: score / 100.0, momentum_score: score / 120.0,
                overall_score: score as i32,
                regime: if score.abs() > 30.0 { "TRENDING".into() } else { "RANGE".into() },
                active_signals: if cross_tf > 0 { 2 } else { 0 }, price: 64000.0,
            });
        }
        AlignmentMatrix {
            symbol: "BTC-USD".into(), timeframes_present: tfs,
            dimensions: vec![AlignmentMatrix::empty("").dimensions[0].clone(); 10],
            mtf_trend_alignment: score / 100.0, mtf_momentum_alignment: score / 120.0,
            mtf_volume_alignment: 0.0, mtf_volatility_alignment: 0.0,
            mtf_overall_score: score, mtf_overall_label: if score > 20.0 { "WEAK_BULL_MTF".into() } else { "NEUTRAL_MTF".into() },
            timeframe_alignments: alignments, signal_cross_tf_count: cross_tf, trend_agreement_pct: agreement,
        }
    }

    #[test]
    fn strong_bullish_mtf_produces_bullish() {
        let c = simple_alignment(4, 75.0, 100.0, 4);
        let d = derive_analysis(&c);
        assert!(matches!(d.bias, MarketBias::Bullish | MarketBias::StrongBullish));
        assert!(d.confidence > 0.7);
    }

    #[test]
    fn neutral_score_neutral() {
        let c = simple_alignment(4, 10.0, 40.0, 0);
        let d = derive_analysis(&c);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn empty_returns_empty() {
        let c = AlignmentMatrix::empty("BTC-USD");
        let d = derive_analysis(&c);
        assert_eq!(d.bias, MarketBias::Neutral);
        assert_eq!(d.timeframes_considered, 0);
    }
}
