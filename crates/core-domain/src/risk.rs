//! # Risk Matrix — Risk Assessment Layer
//!
//! The Risk Matrix evaluates the level of uncertainty surrounding the current
//! market interpretation. Risk is a property of an interpretation, not of raw
//! observations. It consumes the Analysis Matrix — you cannot evaluate how
//! risky a bullish trend is without first determining that there IS a bullish
//! trend.
//!
//! Risk is independent from market direction. A bullish market can be high risk.
//! A bearish market can be low risk.
//!
//! Layer: L4.25 in the architecture (Risk Assessment).

use crate::analysis::AnalysisMatrix;
use crate::indicator_dtos::NormalizedIndicatorValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Risk level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    VeryLow,
    Low,
    #[default]
    Moderate,
    High,
    Extreme,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::VeryLow => write!(f, "VERY_LOW"),
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Moderate => write!(f, "MODERATE"),
            RiskLevel::High => write!(f, "HIGH"),
            RiskLevel::Extreme => write!(f, "EXTREME"),
        }
    }
}

/// Risk dimension state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskState {
    #[default]
    Stable,
    Increasing,
    Elevated,
    Critical,
    Improving,
}

impl std::fmt::Display for RiskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskState::Stable => write!(f, "STABLE"),
            RiskState::Increasing => write!(f, "INCREASING"),
            RiskState::Elevated => write!(f, "ELEVATED"),
            RiskState::Critical => write!(f, "CRITICAL"),
            RiskState::Improving => write!(f, "IMPROVING"),
        }
    }
}

/// One risk dimension with score, level, state, confidence, and evidence.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RiskDimension {
    /// Risk score 0-100 (higher = riskier).
    pub score: f64,
    /// Risk level.
    pub level: RiskLevel,
    /// Risk state (trend).
    pub state: RiskState,
    /// Confidence in this measurement 0-100%.
    pub confidence: f64,
    /// Supporting evidence strings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

impl RiskDimension {
    fn from_score(score: f64) -> Self {
        let level = if score >= 80.0 {
            RiskLevel::Extreme
        } else if score >= 60.0 {
            RiskLevel::High
        } else if score >= 40.0 {
            RiskLevel::Moderate
        } else if score >= 20.0 {
            RiskLevel::Low
        } else {
            RiskLevel::VeryLow
        };
        Self {
            score: score.max(0.0).min(100.0),
            level,
            state: RiskState::Stable,
            confidence: 50.0,
            evidence: Vec::new(),
        }
    }

    fn with_evidence(mut self, evidence: Vec<String>) -> Self {
        self.evidence = evidence;
        self
    }
}

/// Market risk assessment for a single symbol — 9 dimensions.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RiskMatrix {
    pub symbol: String,
    pub market_risk: RiskDimension,
    pub volatility_risk: RiskDimension,
    /// Phase 3 rename: was `liquidity_risk` (execution-liquidity / slippage).
    /// Now: execution liquidity / market depth.
    #[serde(rename = "execution_liquidity_risk")]
    pub execution_liquidity_risk: RiskDimension,
    pub structure_risk: RiskDimension,
    pub momentum_risk: RiskDimension,
    pub signal_risk: RiskDimension,
    pub execution_risk: RiskDimension,
    /// Phase 3: cascade risk — danger from forced liquidation cascades.
    #[serde(default)]
    pub cascade_risk: RiskDimension,
    pub overall_risk: RiskDimension,
}

impl RiskMatrix {
    pub fn empty(symbol: &str) -> Self {
        let def = RiskDimension::from_score(50.0);
        Self {
            symbol: symbol.to_string(),
            market_risk: def.clone(),
            volatility_risk: def.clone(),
            execution_liquidity_risk: def.clone(),
            structure_risk: def.clone(),
            momentum_risk: def.clone(),
            signal_risk: def.clone(),
            execution_risk: def.clone(),
            cascade_risk: def.clone(),
            overall_risk: def.clone(),
        }
    }
}

/// Level from score 0-100.
fn level_from(score: f64) -> RiskLevel {
    if score >= 80.0 {
        RiskLevel::Extreme
    } else if score >= 60.0 {
        RiskLevel::High
    } else if score >= 40.0 {
        RiskLevel::Moderate
    } else if score >= 20.0 {
        RiskLevel::Low
    } else {
        RiskLevel::VeryLow
    }
}

/// Score from magnitude: maps raw values to 0-100 risk score.
fn score_mag(value: f64, max: f64) -> f64 {
    (value / max * 100.0).max(0.0).min(100.0)
}

/// Assess market risk: general uncertainty from conflicting signals, weak structure.
fn assess_market_risk(
    analysis: &AnalysisMatrix,
    indicators: &HashMap<String, NormalizedIndicatorValue>,
) -> RiskDimension {
    let mut score: f64 = 50.0;
    let mut evidence = Vec::new();
    if analysis.trend_assessment == crate::analysis::TrendAssessment::Weak {
        score += 15.0;
        evidence.push("Weak trend".into());
    }
    if analysis.structure_assessment == crate::analysis::StructureAssessment::Broken {
        score += 15.0;
        evidence.push("Broken structure".into());
    }
    if analysis.market_quality == crate::analysis::QualityLevel::Poor {
        score += 10.0;
        evidence.push("Poor market quality".into());
    }
    if analysis.state_confidence < 0.4 {
        score += 10.0;
        evidence.push("Low confidence".into());
    }
    if !analysis.contradicting_signals.is_empty() {
        score += 10.0;
        evidence.push("Conflicting signals".into());
    }
    // Reduce risk if conditions are good
    if analysis.trend_assessment == crate::analysis::TrendAssessment::Strong {
        score -= 10.0;
    }
    if analysis.state_confidence > 0.7 {
        score -= 10.0;
    }
    RiskDimension::from_score(score.max(0.0).min(100.0)).with_evidence(evidence)
}

/// Assess volatility risk: danger from abnormal price movement.
fn assess_volatility_risk(indicators: &HashMap<String, NormalizedIndicatorValue>) -> RiskDimension {
    let bbwp = indicators.get("bbwp").map(|v| v.raw_value).unwrap_or(50.0);
    let atr = indicators.get("atr").map(|v| v.raw_value).unwrap_or(0.0);
    let squeeze_on = indicators
        .get("squeeze")
        .map(|v| v.state_label.contains("COMPRESSION"))
        .unwrap_or(false);
    let mut evidence = Vec::new();
    let mut score: f64 = 30.0;
    if bbwp >= 90.0 {
        score += 30.0;
        evidence.push("BBWP extreme expansion".into());
    } else if bbwp >= 70.0 {
        score += 15.0;
        evidence.push("BBWP elevated".into());
    } else if squeeze_on {
        score += 10.0;
        evidence.push("Squeeze compression active".into());
    }
    if atr > 0.0 {
        let rel_atr = score_mag(atr, 500.0);
        score = (score + rel_atr) / 2.0;
    }
    RiskDimension::from_score(score.max(0.0).min(100.0)).with_evidence(evidence)
}

/// Assess liquidity risk: quality of market participation.
fn assess_execution_liquidity_risk(
    indicators: &HashMap<String, NormalizedIndicatorValue>,
) -> RiskDimension {
    let rvol = indicators.get("rvol").map(|v| v.raw_value).unwrap_or(1.0);
    let spread = indicators.get("spread").map(|v| v.raw_value).unwrap_or(0.0);
    let mut evidence = Vec::new();
    let mut score: f64 = 30.0;
    if rvol < 0.5 {
        score += 30.0;
        evidence.push("Very low relative volume".into());
    } else if rvol < 0.8 {
        score += 15.0;
        evidence.push("Low relative volume".into());
    } else if rvol > 2.0 {
        score -= 15.0;
        evidence.push("Strong participation".into());
    }
    if spread > 0.2 {
        score += 20.0;
        evidence.push("Wide spread".into());
    } else if spread < 0.05 {
        score -= 10.0;
        evidence.push("Tight spread".into());
    }
    RiskDimension::from_score(score.max(0.0).min(100.0)).with_evidence(evidence)
}

/// Assess structure risk: uncertainty from weak/damaged price structure.
fn assess_structure_risk(
    analysis: &AnalysisMatrix,
    indicators: &HashMap<String, NormalizedIndicatorValue>,
) -> RiskDimension {
    let mut score: f64 = 40.0;
    let mut evidence = Vec::new();
    match analysis.structure_assessment {
        crate::analysis::StructureAssessment::Broken => {
            score += 30.0;
            evidence.push("Broken structure".into());
        }
        crate::analysis::StructureAssessment::Weak => {
            score += 15.0;
            evidence.push("Weak structure".into());
        }
        crate::analysis::StructureAssessment::Strong
        | crate::analysis::StructureAssessment::Healthy => {
            score -= 15.0;
        }
        _ => {}
    }
    let sr_label = indicators
        .get("support_resistance")
        .map(|v| v.state_label.as_str())
        .unwrap_or("");
    if sr_label.contains("FLIP") {
        score += 15.0;
        evidence.push("S/R level flip".into());
    }
    RiskDimension::from_score(score.max(0.0).min(100.0)).with_evidence(evidence)
}

/// Assess momentum risk: vulnerability from exhausted/diverging momentum.
fn assess_momentum_risk(analysis: &AnalysisMatrix) -> RiskDimension {
    let mut score: f64 = 30.0;
    let mut evidence = Vec::new();
    match analysis.momentum_assessment {
        crate::analysis::MomentumAssessment::Exhausted => {
            score += 40.0;
            evidence.push("Momentum exhausted".into());
        }
        crate::analysis::MomentumAssessment::Reversing => {
            score += 30.0;
            evidence.push("Momentum reversing".into());
        }
        crate::analysis::MomentumAssessment::Weakening => {
            score += 15.0;
            evidence.push("Momentum weakening".into());
        }
        crate::analysis::MomentumAssessment::Increasing => {
            score -= 10.0;
            evidence.push("Momentum increasing".into());
        }
        _ => {}
    }
    RiskDimension::from_score(score.max(0.0).min(100.0)).with_evidence(evidence)
}

/// Assess signal risk: uncertainty from conflicting/unreliable signals.
fn assess_signal_risk(analysis: &AnalysisMatrix) -> RiskDimension {
    let mut score: f64 = 30.0;
    let mut evidence = Vec::new();
    if !analysis.contradicting_signals.is_empty() {
        let n = analysis.contradicting_signals.len() as f64;
        score += (n * 10.0).min(40.0);
        evidence.push(format!(
            "{} contradicting signals",
            analysis.contradicting_signals.len()
        ));
    }
    if analysis.supporting_signals.is_empty() && analysis.contradicting_signals.is_empty() {
        score += 10.0;
        evidence.push("No signals active".into());
    }
    if analysis.state_confidence < 0.5 {
        score += 15.0;
        evidence.push("Low analysis confidence".into());
    }
    RiskDimension::from_score(score.max(0.0).min(100.0)).with_evidence(evidence)
}

/// Assess execution risk: practical difficulties from spread/movement.
fn assess_execution_risk(indicators: &HashMap<String, NormalizedIndicatorValue>) -> RiskDimension {
    let spread = indicators.get("spread").map(|v| v.raw_value).unwrap_or(0.0);
    let rvol = indicators.get("rvol").map(|v| v.raw_value).unwrap_or(1.0);
    let mut evidence = Vec::new();
    let mut score: f64 = 25.0;
    if spread > 0.15 {
        score += 25.0;
        evidence.push("Wide spread".into());
    } else if spread > 0.08 {
        score += 10.0;
        evidence.push("Moderate spread".into());
    }
    if rvol < 0.7 {
        score += 15.0;
        evidence.push("Low participation".into());
    }
    RiskDimension::from_score(score.max(0.0).min(100.0)).with_evidence(evidence)
}

/// Assess cascade risk (Phase 3): danger from forced liquidation
/// cascades. Reads `cascade_intensity` from the per-candle
/// `LiquidityFlow` if present, plus `cascade_asymmetry` from the
/// `LiquidationClusterMatrix` for forward-looking pressure. Higher
/// intensity or larger absolute asymmetry → higher risk.
fn assess_cascade_risk(
    flow: Option<&crate::liquidity::LiquidityFlow>,
    cluster: Option<&crate::liquidity::LiquidationClusterMatrix>,
) -> RiskDimension {
    let mut score: f64 = 30.0; // baseline
    let mut evidence = Vec::new();
    if let Some(f) = flow {
        // The intensity is already 0..100; pull it in proportionally.
        score = score.max(f.cascade_intensity);
        match f.cascade_state {
            crate::liquidity::CascadeState::Sustained => {
                score = (score + 30.0).min(100.0);
                evidence.push("Cascade sustained in rolling window".into());
            }
            crate::liquidity::CascadeState::Detected => {
                score = (score + 15.0).min(100.0);
                evidence.push("Cascade detected this bar".into());
            }
            crate::liquidity::CascadeState::Exhausted => {
                evidence.push("Cascade exhausted (decaying)".into());
            }
            _ => {}
        }
    }
    if let Some(c) = cluster {
        // Forward-looking pressure: |asymmetry| contributes up to 20.
        let asym = c.cascade_asymmetry.abs();
        if asym > 0.3 {
            score = (score + asym * 30.0).min(100.0);
            evidence.push(format!(
                "Cluster asymmetry {:.2} (significant one-sided pressure)",
                c.cascade_asymmetry
            ));
        }
    }
    RiskDimension::from_score(score).with_evidence(evidence)
}

/// Compute the Risk Matrix from the Analysis Matrix and per-timeframe indicators.
pub fn compute_risk(
    symbol: &str,
    analysis: &AnalysisMatrix,
    indicators: &HashMap<String, NormalizedIndicatorValue>,
    flow: Option<&crate::liquidity::LiquidityFlow>,
    cluster: Option<&crate::liquidity::LiquidationClusterMatrix>,
) -> RiskMatrix {
    if analysis.timeframes_considered == 0 {
        return RiskMatrix::empty(symbol);
    }

    let market = assess_market_risk(analysis, indicators);
    let volatility = assess_volatility_risk(indicators);
    let liquidity = assess_execution_liquidity_risk(indicators);
    let structure = assess_structure_risk(analysis, indicators);
    let momentum = assess_momentum_risk(analysis);
    let signal = assess_signal_risk(analysis);
    let execution = assess_execution_risk(indicators);
    let cascade = assess_cascade_risk(flow, cluster);

    // Overall: weighted average of all 8 dimensions. Cascade gets 0.14
    // weight — comparable to the other dimensions because cascade events
    // dominate short-term realized volatility.
    let overall_score = (market.score * 0.14
        + volatility.score * 0.14
        + liquidity.score * 0.14
        + structure.score * 0.10
        + momentum.score * 0.14
        + signal.score * 0.10
        + execution.score * 0.10
        + cascade.score * 0.14)
        .max(0.0)
        .min(100.0);
    let overall = RiskDimension::from_score(overall_score);

    RiskMatrix {
        symbol: symbol.to_string(),
        market_risk: market,
        volatility_risk: volatility,
        execution_liquidity_risk: liquidity,
        structure_risk: structure,
        momentum_risk: momentum,
        signal_risk: signal,
        execution_risk: execution,
        cascade_risk: cascade,
        overall_risk: overall,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{
        AnalysisMatrix, MarketBias, MarketRegime, MomentumAssessment, OpportunityType,
        QualityLevel, StructureAssessment, TrendAssessment, VolatilityAssessment, VolumeAssessment,
    };

    fn make_analysis_with_timeframes() -> AnalysisMatrix {
        AnalysisMatrix {
            symbol: "BTC-USD".to_string(),
            bias: MarketBias::Neutral,
            state_confidence: 0.5,
            market_quality_score: 50.0,
            market_regime: MarketRegime::Range,
            trend_assessment: TrendAssessment::Healthy,
            momentum_assessment: MomentumAssessment::Stable,
            structure_assessment: StructureAssessment::Healthy,
            volatility_assessment: VolatilityAssessment::Normal,
            volume_assessment: VolumeAssessment::Normal,
            opportunity_analysis: OpportunityType::NoClearOpportunity,
            market_quality: QualityLevel::Average,
            market_interpretation: "Test".into(),
            rationale: String::new(),
            supporting_signals: Vec::new(),
            contradicting_signals: Vec::new(),
            timeframes_considered: 4,
        }
    }

    #[test]
    fn empty_returns_default() {
        let r = RiskMatrix::empty("BTC-USD");
        assert!(matches!(r.overall_risk.level, RiskLevel::Moderate));
    }

    #[test]
    fn compute_with_analysis_produces_valid_dimensions() {
        let analysis = make_analysis_with_timeframes();
        let indicators = HashMap::new();
        let r = compute_risk("BTC-USD", &analysis, &indicators, None, None);
        // Even with empty analysis, should produce valid scores
        assert!(r.volatility_risk.score >= 0.0 && r.volatility_risk.score <= 100.0);
        assert!(
            r.execution_liquidity_risk.score >= 0.0 && r.execution_liquidity_risk.score <= 100.0
        );
        assert!(r.cascade_risk.score >= 0.0 && r.cascade_risk.score <= 100.0);
    }

    #[test]
    fn cascade_risk_does_not_crash_with_zero_inputs() {
        let analysis = make_analysis_with_timeframes();
        let indicators = HashMap::new();
        let r = compute_risk("BTC-USD", &analysis, &indicators, None, None);
        // Baseline (no flow, no cluster) → score = 30.0
        assert!(
            (r.cascade_risk.score - 30.0).abs() < 1e-9,
            "expected 30.0, got {}",
            r.cascade_risk.score
        );
    }

    #[test]
    fn cascade_risk_with_sustained_state_is_high() {
        use crate::liquidity::{CascadeState, LiquidityFlow};
        let flow = LiquidityFlow {
            cascade_state: CascadeState::Sustained,
            cascade_intensity: 90.0,
            ..Default::default()
        };
        let analysis = make_analysis_with_timeframes();
        let indicators = HashMap::new();
        let r = compute_risk("BTC-USD", &analysis, &indicators, Some(&flow), None);
        // Sustained + high intensity → cascade_risk >= 90 (capped at 100).
        assert!(
            r.cascade_risk.score >= 90.0,
            "sustained cascade should drive risk >= 90, got {}",
            r.cascade_risk.score
        );
    }
}
fn clamp01(x: f64) -> f64 {
    x.max(0.0).min(100.0)
}
