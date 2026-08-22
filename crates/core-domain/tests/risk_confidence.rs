//! Risk-dimension confidence propagation tests.
//!
//! Per docs/matrices `02-00b-confidence-hierarchy.md`, the L3 `state_confidence`
//! is the canonical upstream confidence signal that flows downstream. The L5
//! Risk Matrix dimensions should propagate that signal as
//! `confidence = state_confidence * 100` (per the agreed fix in plan-mode).
//!
//! Exception per docs `03-02-12-mme-configurable-activation.md` §CA-15:
//! `cascade_risk` emits `confidence: 0` when the liquidity feed is off
//! (no `flow` AND no `cluster`).

use core_domain::analysis::AnalysisMatrix;
use core_domain::liquidity::{CascadeState, LiquidationClusterMatrix, LiquidityFlow};
use core_domain::market_context::MarketContext;
use core_domain::risk::{compute_risk, RiskLevel};
use std::collections::HashMap;

fn make_analysis(state_confidence: f64) -> AnalysisMatrix {
    AnalysisMatrix {
        symbol: "BTC-USDT".into(),
        bias: core_domain::analysis::MarketBias::Neutral,
        market_bias_score: 0.0,
        state_confidence,
        confidence: state_confidence,
        market_quality_score: 50.0,
        market_regime: core_domain::analysis::MarketRegime::Range,
        trend_assessment: core_domain::analysis::TrendAssessment::Healthy,
        momentum_assessment: core_domain::analysis::MomentumAssessment::Stable,
        structure_assessment: core_domain::analysis::StructureAssessment::Healthy,
        volatility_assessment: core_domain::analysis::VolatilityAssessment::Normal,
        volume_assessment: core_domain::analysis::VolumeAssessment::Normal,
        market_quality: core_domain::analysis::QualityLevel::Average,
        trend_score: None,
        momentum_score: None,
        structure_score: None,
        volatility_score: None,
        volume_score: None,
        representative_bbwp: None,
        representative_adx: None,
        market_phase: core_domain::analysis::MarketPhase::Unknown,
        market_interpretation: "test".into(),
        rationale: String::new(),
        supporting_signals: Vec::new(),
        contradicting_signals: Vec::new(),
        timeframes_considered: 4,
    }
}

#[test]
fn confidence_propagates_from_state_confidence() {
    // state_confidence = 0.85 → expected confidence = 85.0 for every dimension
    // EXCEPT cascade_risk, which also reads 85.0 because flow is present.
    let analysis = make_analysis(0.85);
    let indicators = HashMap::new();
    let risk = compute_risk(
        "BTC-USDT",
        &analysis,
        &indicators,
        None,
        None,
        0.0,
        &[],
        None,
        &[],
    );

    assert!((risk.market_risk.confidence - 85.0).abs() < 1e-9);
    assert!((risk.volatility_risk.confidence - 85.0).abs() < 1e-9);
    assert!((risk.execution_liquidity_risk.confidence - 85.0).abs() < 1e-9);
    assert!((risk.structure_risk.confidence - 85.0).abs() < 1e-9);
    assert!((risk.momentum_risk.confidence - 85.0).abs() < 1e-9);
    assert!((risk.signal_risk.confidence - 85.0).abs() < 1e-9);
    assert!((risk.execution_risk.confidence - 85.0).abs() < 1e-9);
    assert!((risk.overall_risk.confidence - 85.0).abs() < 1e-9);
}

#[test]
fn confidence_low_with_low_state_confidence() {
    // state_confidence = 0.30 → expected confidence = 30.0
    let analysis = make_analysis(0.30);
    let indicators = HashMap::new();
    let risk = compute_risk(
        "BTC-USDT",
        &analysis,
        &indicators,
        None,
        None,
        0.0,
        &[],
        None,
        &[],
    );

    assert!((risk.market_risk.confidence - 30.0).abs() < 1e-9);
    assert!((risk.volatility_risk.confidence - 30.0).abs() < 1e-9);
    assert!((risk.structure_risk.confidence - 30.0).abs() < 1e-9);
    assert!((risk.momentum_risk.confidence - 30.0).abs() < 1e-9);
    assert!((risk.signal_risk.confidence - 30.0).abs() < 1e-9);
    assert!((risk.execution_risk.confidence - 30.0).abs() < 1e-9);
    assert!((risk.execution_liquidity_risk.confidence - 30.0).abs() < 1e-9);
    assert!((risk.overall_risk.confidence - 30.0).abs() < 1e-9);
}

#[test]
fn cascade_risk_confidence_zero_when_liquidity_off() {
    // When flow=None AND cluster=None → cascade confidence must be 0.0
    // (docs §CA-15: cascade_risk NO_DATA + confidence 0 when liquidity off).
    let analysis = make_analysis(0.85);
    let indicators = HashMap::new();
    let risk = compute_risk(
        "BTC-USDT",
        &analysis,
        &indicators,
        None,
        None,
        0.0,
        &[],
        None,
        &[],
    );

    assert_eq!(risk.cascade_risk.confidence, 0.0);
}

#[test]
fn cascade_risk_confidence_propagates_when_flow_present() {
    // When flow=Some(...) → cascade confidence = state_confidence * 100.
    let analysis = make_analysis(0.72);
    let indicators = HashMap::new();
    let flow = LiquidityFlow::default();
    let risk = compute_risk(
        "BTC-USDT",
        &analysis,
        &indicators,
        Some(&flow),
        None,
        0.0,
        &[],
        None,
        &[],
    );

    assert!((risk.cascade_risk.confidence - 72.0).abs() < 1e-9);
}

#[test]
fn empty_risk_matrix_keeps_50_confidence_floor() {
    // The docs §6 "Empty State" guarantees RiskMatrix::empty has score=50 and
    // confidence=50 (maximal uncertainty in the absence of data).
    let risk = core_domain::risk::RiskMatrix::empty("BTC-USDT");
    assert_eq!(risk.market_risk.confidence, 50.0);
    assert_eq!(risk.overall_risk.confidence, 50.0);
    assert_eq!(risk.cascade_risk.confidence, 50.0);
}

// Keep the imports referenced so the file doesn't warn under no-unwrap.
#[allow(dead_code)]
fn _keep_imports(
    ctx: MarketContext,
    level: RiskLevel,
    st: CascadeState,
    _cl: LiquidationClusterMatrix,
) {
    let _ = (ctx, level, st);
}
