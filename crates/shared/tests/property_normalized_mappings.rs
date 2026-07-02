//! Property + boundary tests for the Fractional Normalized Indicator Model.
//!
//! Verifies that raw telemetry maps into the continuous `[-1.0, 1.0]` scale
//! with the correct context-aware state labels at each mathematical boundary.

use proptest::prelude::*;
use shared::indicators::normalized::{DivergenceState, NormalizationEngine};

// ─────────────────────────── RSI ───────────────────────────

proptest! {
    #[test]
    fn rsi_always_within_unit_interval(rsi in 0.0f64..=100.0) {
        let v = NormalizationEngine::normalize_rsi(rsi, DivergenceState::None);
        prop_assert!(v.normalized >= -1.0 && v.normalized <= 1.0);
    }

    #[test]
    fn rsi_oversold_maps_positive_accumulation(rsi in 0.0f64..30.0) {
        let v = NormalizationEngine::normalize_rsi(rsi, DivergenceState::None);
        prop_assert!(v.normalized >= 0.70 && v.normalized <= 1.0,
            "rsi={} normalized={}", rsi, v.normalized);
        prop_assert_eq!(v.state_label, "OVERSOLD_ACCUMULATION");
    }

    #[test]
    fn rsi_overbought_maps_negative_distribution(rsi in 70.0001f64..=100.0) {
        let v = NormalizationEngine::normalize_rsi(rsi, DivergenceState::None);
        prop_assert!(v.normalized <= -0.70 && v.normalized >= -1.0,
            "rsi={} normalized={}", rsi, v.normalized);
        prop_assert_eq!(v.state_label, "OVERBOUGHT_DISTRIBUTION");
    }
}

#[test]
fn rsi_equilibrium_at_fifty() {
    let v = NormalizationEngine::normalize_rsi(50.0, DivergenceState::None);
    assert!((v.normalized - 0.0).abs() < 1e-9);
    assert_eq!(v.state_label, "EQUILIBRIUM");
}

#[test]
fn rsi_confirmed_divergence_hard_overrides() {
    let bull = NormalizationEngine::normalize_rsi(80.0, DivergenceState::ConfirmedBullish);
    assert_eq!(bull.normalized, 1.0);
    let bear = NormalizationEngine::normalize_rsi(20.0, DivergenceState::ConfirmedBearish);
    assert_eq!(bear.normalized, -1.0);
}

#[test]
fn rsi_potential_divergence_capped() {
    let v = NormalizationEngine::normalize_rsi(10.0, DivergenceState::PotentialBullish);
    assert!(v.normalized <= 0.90);
}

// ─────────────────────────── RVOL ───────────────────────────

proptest! {
    #[test]
    fn rvol_below_one_is_negative(rvol in 0.0f64..1.0) {
        let v = NormalizationEngine::normalize_rvol(rvol);
        prop_assert!(v.normalized < 0.0);
        prop_assert_eq!(v.state_label, "CONSOLIDATION_VOLUME");
    }

    #[test]
    fn rvol_climax_is_blocked(rvol in 3.0f64..100.0) {
        let v = NormalizationEngine::normalize_rvol(rvol);
        prop_assert_eq!(v.normalized, -1.0);
        prop_assert_eq!(v.state_label, "EXHAUSTION_CLIMAX_VOLUME");
    }
}

#[test]
fn rvol_institutional_band() {
    let v = NormalizationEngine::normalize_rvol(2.0);
    assert_eq!(v.normalized, 0.8);
    assert_eq!(v.state_label, "INSTITUTIONAL_BREAKOUT_VOLUME");
}

// ─────────────────────────── ADX ───────────────────────────

proptest! {
    #[test]
    fn adx_always_within_unit_interval(
        adx in 0.0f64..=100.0,
        plus in 0.0f64..=100.0,
        minus in 0.0f64..=100.0,
        slope in -10.0f64..=10.0,
    ) {
        let v = NormalizationEngine::normalize_adx(adx, plus, minus, slope, false);
        prop_assert!(v.normalized >= -1.0 && v.normalized <= 1.0);
    }

    #[test]
    fn adx_congestion_is_neutral(adx in 0.0f64..20.0) {
        let v = NormalizationEngine::normalize_adx(adx, 30.0, 10.0, 1.0, false);
        prop_assert_eq!(v.normalized, 0.0);
        prop_assert_eq!(v.state_label, "TRENDLESS_CONGESTION");
    }
}

#[test]
fn adx_sign_follows_di_bias() {
    let bull = NormalizationEngine::normalize_adx(30.0, 30.0, 10.0, 1.0, false);
    let bear = NormalizationEngine::normalize_adx(30.0, 10.0, 30.0, 1.0, false);
    assert!(bull.normalized > 0.0);
    assert!(bear.normalized < 0.0);
}

// ─────────────────────────── BBWP ───────────────────────────

proptest! {
    #[test]
    fn bbwp_within_unit_interval(bbwp in 0.0f64..=100.0, bias in -1i8..=1) {
        let v = NormalizationEngine::normalize_bbwp(bbwp, bias);
        prop_assert!(v.normalized >= -1.0 && v.normalized <= 1.0);
    }
}

#[test]
fn bbwp_compression_is_neutral() {
    let v = NormalizationEngine::normalize_bbwp(5.0, 1);
    assert_eq!(v.normalized, 0.0);
    assert_eq!(v.state_label, "MAX_VOLATILITY_COMPRESSION");
}

// ─────────────────────────── MACD / Squeeze / VWAP ───────────────────────────

#[test]
fn macd_bullish_crossover_below_zero_accelerates() {
    let v = NormalizationEngine::normalize_macd(-12.4, -17.6, 5.2, 8.0, Some(1));
    assert!(v.normalized >= 0.8);
    assert_eq!(v.state_label, "BULLISH_CROSSOVER_ACCELERATING");
}

#[test]
fn macd_fomo_crossover_above_zero_rejected() {
    let v = NormalizationEngine::normalize_macd(12.4, 8.0, 4.0, 8.0, Some(1));
    assert_eq!(v.normalized, 0.2);
    assert_eq!(v.state_label, "FOMO_BULLISH_CROSSOVER_REJECTED");
}

#[test]
fn squeeze_on_is_coiling() {
    use shared::indicators::squeeze::MomentumDirection;
    let v = NormalizationEngine::normalize_squeeze(true, false, 0.0, MomentumDirection::Flat);
    assert_eq!(v.normalized, 0.0);
    assert_eq!(v.state_label, "COMPRESSION_COILING");
}

#[test]
fn vwap_extreme_premium_is_reversion() {
    let v = NormalizationEngine::normalize_vwap(101.5, 100.0);
    assert_eq!(v.normalized, -0.8);
    assert_eq!(v.state_label, "EXTREME_PREMIUM_REVERSION_ZONE");
}
