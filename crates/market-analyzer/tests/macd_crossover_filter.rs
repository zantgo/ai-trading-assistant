//! AUDIT-AIU-112 regression pins: the MACD crossover signal must be emitted
//! EXACTLY ONCE per legitimate bar by the structured block in `normalize_all`
//! (`all.rs`), and zero-line-rejected crossovers (FOMO bullish above the zero
//! line / PANIC bearish below it) must emit NOTHING.
//!
//! The legacy label-string branch in `derive_signals` matched any label
//! containing "CROSSOVER" — including `FOMO_BULLISH_CROSSOVER_REJECTED` /
//! `PANIC_BEARISH_CROSSOVER_REJECTED` (re-emitting the very event the
//! zero-line filter suppresses) and `BULLISH_CROSSOVER_ACCELERATING`
//! (double-emitting a second, differently-labelled Crossover on legitimate
//! bars). The branch is removed; these pins protect the contract.

use market_analyzer::indicators::normalized::{
    derive_signals, DivergenceState, IndicatorInputs, IndicatorSignal, NormalizationContext,
    NormalizationEngine, NormalizedIndicatorValue, SignalDirection, SignalKind,
};
use std::collections::HashMap;

fn crossover_signals(
    macd_line: f64,
    macd_signal: f64,
    histogram: f64,
    crossover: Option<i8>,
) -> Vec<IndicatorSignal> {
    let inputs = IndicatorInputs {
        macd_line: Some(macd_line),
        macd_signal: Some(macd_signal),
        macd_histogram: Some(histogram),
        macd_histogram_peak: Some(10.0),
        macd_crossover: crossover,
        macd_divergence: DivergenceState::None,
        ..Default::default()
    };
    let ctx = NormalizationContext::default();
    let mut map: HashMap<String, NormalizedIndicatorValue> =
        NormalizationEngine::normalize_all(&inputs, &ctx, false);
    derive_signals(&mut map);
    map.get("macd")
        .map(|v| v.signals.clone())
        .unwrap_or_default()
}

#[test]
fn legitimate_bullish_crossover_emits_exactly_one_signal() {
    // Bullish cross BELOW the zero line → exactly one Crossover(Bullish),
    // labelled `BULLISH_CROSSOVER` (the structured canonical label).
    let sigs = crossover_signals(-12.4, -17.6, 5.2, Some(1));
    let crossovers: Vec<_> = sigs
        .iter()
        .filter(|s| s.kind == SignalKind::Crossover)
        .collect();
    assert_eq!(
        crossovers.len(),
        1,
        "legit cross must emit exactly one Crossover, got {:?}",
        sigs
    );
    assert_eq!(crossovers[0].direction, SignalDirection::Bullish);
    assert_eq!(crossovers[0].label, "BULLISH_CROSSOVER");
}

#[test]
fn legitimate_bearish_crossover_emits_exactly_one_signal() {
    // Bearish cross ABOVE the zero line → exactly one Crossover(Bearish).
    let sigs = crossover_signals(12.4, 17.6, -5.2, Some(-1));
    let crossovers: Vec<_> = sigs
        .iter()
        .filter(|s| s.kind == SignalKind::Crossover)
        .collect();
    assert_eq!(crossovers.len(), 1);
    assert_eq!(crossovers[0].direction, SignalDirection::Bearish);
    assert_eq!(crossovers[0].label, "BEARISH_CROSSOVER");
}

#[test]
fn fomo_rejected_crossover_emits_no_signal() {
    // Bullish cross ABOVE the zero line (FOMO) is zero-line filtered — the
    // normalizer reports `FOMO_BULLISH_CROSSOVER_REJECTED` with zero
    // contribution, and NO Crossover signal may be derived from it.
    let sigs = crossover_signals(12.4, 8.0, 4.0, Some(1));
    assert!(
        sigs.iter().all(|s| s.kind != SignalKind::Crossover),
        "rejected crossover must not emit a Crossover: {:?}",
        sigs
    );
}

#[test]
fn panic_rejected_crossover_emits_no_signal() {
    // Bearish cross BELOW the zero line (PANIC) — same suppression.
    let sigs = crossover_signals(-12.4, -8.0, -4.0, Some(-1));
    assert!(
        sigs.iter().all(|s| s.kind != SignalKind::Crossover),
        "rejected crossover must not emit a Crossover: {:?}",
        sigs
    );
}
