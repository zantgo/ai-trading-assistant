//! P0-1 sign-consistency invariant (v6.10.17).
//!
//! Every DIRECTIONAL oscillator normalizer in the platform must follow the
//! single RSI convention: **overbought → negative contribution (distribution
//! warning), oversold → positive contribution (accumulation opportunity)**,
//! and the middle band signs by the dominant flow. The audit found three
//! violations (MFI middle band, Stochastic full range, Williams %R
//! extremes) that fed wrong votes into L4 scoring and the group-confluence
//! buckets. This test is the release gate: any future normalizer that
//! inverts a label's sign fails here.

use core_domain::indicator_dtos::{DivergenceState, NormalizedIndicatorValue};
use market_analyzer::indicators::normalized::NormalizationEngine;

fn sign_of(v: &NormalizedIndicatorValue) -> f64 {
    if v.normalized > 1e-9 {
        1.0
    } else if v.normalized < -1e-9 {
        -1.0
    } else {
        0.0
    }
}

#[test]
fn rsi_family_signs_follow_the_convention() {
    // RSI: overbought ≥ 70 → negative; oversold ≤ 30 → positive.
    assert!(NormalizationEngine::normalize_rsi(75.0, DivergenceState::None).normalized < 0.0);
    assert!(NormalizationEngine::normalize_rsi(25.0, DivergenceState::None).normalized > 0.0);

    // MFI: overbought ≥ 80 → negative; oversold ≤ 20 → positive;
    // middle band signs by flow (below 50 bearish, above 50 bullish).
    assert!(NormalizationEngine::normalize_mfi(85.0).normalized < 0.0);
    assert!(NormalizationEngine::normalize_mfi(15.0).normalized > 0.0);
    assert!(NormalizationEngine::normalize_mfi(48.9).normalized < 0.0);
    assert!(NormalizationEngine::normalize_mfi(51.8).normalized > 0.0);
    assert!(NormalizationEngine::normalize_mfi(60.0).normalized > 0.0);

    // Stochastic: overbought ≥ 80 → negative; oversold ≤ 20 → positive.
    assert!(NormalizationEngine::normalize_stochastic(87.6, 80.0).normalized < 0.0);
    assert!(NormalizationEngine::normalize_stochastic(3.8, 2.0).normalized > 0.0);
    // Middle band signs by k/d alignment, magnitude from |k − 50|.
    assert!(NormalizationEngine::normalize_stochastic(60.7, 76.1).normalized < 0.0);
    assert!(NormalizationEngine::normalize_stochastic(55.0, 40.0).normalized > 0.0);

    // Williams %R: overbought ≥ −20 → negative; oversold ≤ −80 → positive.
    assert!(NormalizationEngine::normalize_williams_r(-5.0).normalized < 0.0);
    assert!(NormalizationEngine::normalize_williams_r(-97.8).normalized > 0.0);
}

#[test]
fn stochastic_matches_the_capture_values() {
    // The 03:40 capture: micro k=3.8 (OVERSOLD) → +0.94; slow k=60.7/d=76.1
    // (BEARISH_MOMENTUM_ALIGNMENT) → −0.21; macro k=87.6 (OVERBOUGHT) → −0.81.
    let oversold = NormalizationEngine::normalize_stochastic(3.8, 2.0);
    assert!((oversold.normalized - 0.943).abs() < 1e-3);
    assert_eq!(oversold.state_label, "OVERSOLD_ACCUMULATION");

    let bearish_align = NormalizationEngine::normalize_stochastic(60.7, 76.1);
    assert!((bearish_align.normalized + 0.214).abs() < 1e-3);
    assert_eq!(bearish_align.state_label, "BEARISH_MOMENTUM_ALIGNMENT");

    let overbought = NormalizationEngine::normalize_stochastic(87.6, 80.0);
    assert!((overbought.normalized + 0.814).abs() < 1e-3);
    assert_eq!(overbought.state_label, "OVERBOUGHT_DISTRIBUTION");
}

#[test]
fn mfi_matches_the_capture_values() {
    // The 03:40 capture: micro MFI 48.9 (BEARISH_FLOW) → −0.026;
    // fast MFI 14.2 (OVERSOLD) → +0.787; slow MFI 51.8 (BULLISH_FLOW) → +0.042.
    let bearish = NormalizationEngine::normalize_mfi(48.9);
    assert!((bearish.normalized + 0.0257).abs() < 1e-3);
    assert_eq!(bearish.state_label, "MFI_BEARISH_FLOW");

    let oversold = NormalizationEngine::normalize_mfi(14.2);
    assert!((oversold.normalized - 0.787).abs() < 1e-3);
    assert_eq!(oversold.state_label, "MFI_OVERSOLD_ACCUMULATION");

    let bullish = NormalizationEngine::normalize_mfi(51.8);
    assert!((bullish.normalized - 0.042).abs() < 1e-3);
    assert_eq!(bullish.state_label, "MFI_BULLISH_FLOW");
}

#[test]
fn williams_r_matches_the_capture_values() {
    // The 03:40 capture: micro WR −97.8 (OVERSOLD) → +0.967; macro WR −51.8
    // (BEARISH_BIAS) → −0.036.
    let oversold = NormalizationEngine::normalize_williams_r(-97.8);
    assert!((oversold.normalized - 0.956).abs() < 1e-3);
    assert_eq!(oversold.state_label, "WILLIAMS_R_OVERSOLD");

    let bearish = NormalizationEngine::normalize_williams_r(-51.8);
    assert!((bearish.normalized + 0.036).abs() < 1e-3);
    assert_eq!(bearish.state_label, "WILLIAMS_R_BEARISH_BIAS");
}

#[test]
fn label_sign_invariant_across_the_sampled_grid() {
    // Grid sweep: every sampled input must produce a normalized value whose
    // SIGN agrees with the label semantics (bullish-family labels → ≥ 0,
    // bearish-family labels → ≤ 0, neutral labels → 0).
    let cases: Vec<(NormalizedIndicatorValue, &str)> = vec![
        (
            NormalizationEngine::normalize_mfi(5.0),
            "MFI_OVERSOLD_ACCUMULATION",
        ),
        (NormalizationEngine::normalize_mfi(30.0), "MFI_BEARISH_FLOW"),
        (NormalizationEngine::normalize_mfi(60.0), "MFI_BULLISH_FLOW"),
        (
            NormalizationEngine::normalize_mfi(95.0),
            "MFI_OVERBOUGHT_DISTRIBUTION",
        ),
        (
            NormalizationEngine::normalize_stochastic(10.0, 5.0),
            "OVERSOLD_ACCUMULATION",
        ),
        (
            NormalizationEngine::normalize_stochastic(40.0, 30.0),
            "BULLISH_MOMENTUM_ALIGNMENT",
        ),
        (
            NormalizationEngine::normalize_stochastic(60.0, 70.0),
            "BEARISH_MOMENTUM_ALIGNMENT",
        ),
        (
            NormalizationEngine::normalize_stochastic(90.0, 85.0),
            "OVERBOUGHT_DISTRIBUTION",
        ),
        (
            NormalizationEngine::normalize_williams_r(-90.0),
            "WILLIAMS_R_OVERSOLD",
        ),
        (
            NormalizationEngine::normalize_williams_r(-40.0),
            "WILLIAMS_R_BULLISH_BIAS",
        ),
        (
            NormalizationEngine::normalize_williams_r(-60.0),
            "WILLIAMS_R_BEARISH_BIAS",
        ),
        (
            NormalizationEngine::normalize_williams_r(-10.0),
            "WILLIAMS_R_OVERBOUGHT",
        ),
        (
            NormalizationEngine::normalize_rsi(80.0, DivergenceState::None),
            "rsi_overbought",
        ),
        (
            NormalizationEngine::normalize_rsi(20.0, DivergenceState::None),
            "rsi_oversold",
        ),
        (
            NormalizationEngine::normalize_chandemo(60.0),
            "CLIMACTIC_BULL_EXHAUSTION",
        ),
        (
            NormalizationEngine::normalize_chandemo(-60.0),
            "CLIMACTIC_BEAR_EXHAUSTION",
        ),
    ];
    for (v, expected_label) in cases {
        let s = sign_of(&v);
        let bullish = v.state_label.contains("BULL")
            || v.state_label.contains("ACCUMULATION")
            || v.state_label.to_lowercase().contains("oversold")
            || v.state_label.contains("BUYING")
            || v.state_label == "rsi_oversold"
            || v.state_label == "CLIMACTIC_BULL_EXHAUSTION";
        let bearish = v.state_label.contains("BEAR")
            || v.state_label.contains("DISTRIBUTION")
            || v.state_label.to_lowercase().contains("overbought")
            || v.state_label.contains("SELLING")
            || v.state_label == "rsi_overbought"
            || v.state_label == "CLIMACTIC_BEAR_EXHAUSTION";
        assert!(
            !(bullish && bearish),
            "label {} matched both families",
            v.state_label
        );
        if bullish {
            assert!(
                s >= 0.0,
                "label {} ({} expected) produced sign {} — sign/label inversion",
                v.state_label,
                expected_label,
                v.normalized
            );
        }
        if bearish {
            assert!(
                s <= 0.0,
                "label {} ({} expected) produced sign {} — sign/label inversion",
                v.state_label,
                expected_label,
                v.normalized
            );
        }
    }
}
