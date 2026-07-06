//! Signal derivation: converts the current normalized indicator map into
//! discrete `IndicatorSignal`s (badges/markers) plus divergence scored entries.
//!
//! Signals detectable from the current snapshot's `state_label` are emitted with
//! `Active` status (threshold/breakout/compression/pattern). Divergence signals
//! come from the generalized detectors passed via `IndicatorInputs`.

use super::{
    DivergenceState, IndicatorSignal, NormalizedIndicatorValue, SignalDirection, SignalKind,
    SignalStatus,
};
use std::collections::HashMap;

type Map = HashMap<String, NormalizedIndicatorValue>;

fn push_signal(map: &mut Map, key: &str, sig: IndicatorSignal) {
    if let Some(entry) = map.get_mut(key) {
        entry.signals.push(sig);
    }
}

fn threshold(dir: SignalDirection, label: &str) -> IndicatorSignal {
    IndicatorSignal::new(SignalKind::Threshold, dir, SignalStatus::Active, label)
}

/// Emit a `Divergence` signal on the parent oscillator and return the paired
/// scored-key `NormalizedIndicatorValue` (±1 confirmed / ±0.5 potential).
pub(super) fn divergence_entry(
    map: &mut Map,
    parent: &str,
    state: DivergenceState,
) -> Option<NormalizedIndicatorValue> {
    let (norm, dir, status, label) = match state {
        DivergenceState::ConfirmedBullish => (1.0_f64, SignalDirection::Bullish, SignalStatus::Confirmed, "CONFIRMED_BULLISH_DIVERGENCE"),
        DivergenceState::PotentialBullish => (0.5_f64, SignalDirection::Bullish, SignalStatus::Potential, "POTENTIAL_BULLISH_DIVERGENCE"),
        DivergenceState::ConfirmedBearish => (-1.0_f64, SignalDirection::Bearish, SignalStatus::Confirmed, "CONFIRMED_BEARISH_DIVERGENCE"),
        DivergenceState::PotentialBearish => (-0.5_f64, SignalDirection::Bearish, SignalStatus::Potential, "POTENTIAL_BEARISH_DIVERGENCE"),
        DivergenceState::None => return None,
    };
    push_signal(
        map,
        parent,
        IndicatorSignal::new(SignalKind::Divergence, dir, status, label).with_strength(norm.abs()),
    );
    Some(NormalizedIndicatorValue::scalar(norm, norm, label))
}

/// Derive state-based signals (threshold/breakout/compression/pattern/level)
/// from each indicator's current `state_label`.
pub(super) fn derive_signals(map: &mut Map) {
    // Collect labels first to avoid borrow conflicts.
    let labels: Vec<(String, String)> = map
        .iter()
        .map(|(k, v)| (k.clone(), v.state_label.clone()))
        .collect();

    for (key, label) in labels {
        let l = label.as_str();
        let mut sigs: Vec<IndicatorSignal> = Vec::new();

        // Overbought / oversold thresholds (oscillators).
        if l.contains("OVERBOUGHT") {
            sigs.push(threshold(SignalDirection::Bearish, "OVERBOUGHT"));
        } else if l.contains("OVERSOLD") {
            sigs.push(threshold(SignalDirection::Bullish, "OVERSOLD"));
        }
        // Exhaustion extremes (CMO / Z-Score).
        if l.contains("CLIMACTIC_BULL") || l.contains("OVEREXTENDED_HIGH") {
            sigs.push(threshold(SignalDirection::Bearish, l));
        } else if l.contains("CLIMACTIC_BEAR") || l.contains("OVEREXTENDED_LOW") {
            sigs.push(threshold(SignalDirection::Bullish, l));
        }
        // Breakouts (Donchian / Keltner / Bollinger).
        if l.contains("UPPER_BREAKOUT") {
            sigs.push(IndicatorSignal::new(SignalKind::Breakout, SignalDirection::Bullish, SignalStatus::Active, l));
        } else if l.contains("LOWER_BREAKOUT") {
            sigs.push(IndicatorSignal::new(SignalKind::Breakout, SignalDirection::Bearish, SignalStatus::Active, l));
        }
        // Volatility compression / release (Squeeze / BBWP).
        if l.contains("VOLATILITY_RELEASE") {
            let d = if l.contains("BULLISH") { SignalDirection::Bullish } else { SignalDirection::Bearish };
            sigs.push(IndicatorSignal::new(SignalKind::CompressionRelease, d, SignalStatus::Active, l));
        } else if l == "COMPRESSION_COILING" || l.contains("MAX_VOLATILITY_COMPRESSION") {
            sigs.push(IndicatorSignal::new(SignalKind::CompressionRelease, SignalDirection::Neutral, SignalStatus::Active, l));
        }
        // Chart pattern forming.
        if l.contains("PATTERN") {
            let d = if l.contains("BULLISH") { SignalDirection::Bullish } else if l.contains("BEARISH") { SignalDirection::Bearish } else { SignalDirection::Neutral };
            sigs.push(IndicatorSignal::new(SignalKind::PatternForming, d, SignalStatus::Active, l));
        }
        // Structural level tests (Fibonacci GP / S-R zones / VWAP reversion).
        if l.contains("GOLDEN_POCKET") || l.contains("DEMAND_ZONE") || l.contains("SUPPLY_ZONE") || l.contains("REVERSION_ZONE") {
            let d = if l.contains("BULLISH") || l.contains("DEMAND") || l.contains("DISCOUNT") { SignalDirection::Bullish }
                else if l.contains("BEARISH") || l.contains("SUPPLY") || l.contains("PREMIUM") { SignalDirection::Bearish }
                else { SignalDirection::Neutral };
            sigs.push(IndicatorSignal::new(SignalKind::LevelTest, d, SignalStatus::Active, l));
        }
        // Supertrend / SR flip trend changes.
        if l.contains("FLIP_CONFIRMED") {
            let d = if l.contains("RESISTANCE_FLIP") { SignalDirection::Bullish } else { SignalDirection::Bearish };
            sigs.push(IndicatorSignal::new(SignalKind::TrendFlip, d, SignalStatus::Active, l));
        }
        // Volume climax.
        if l.contains("CLIMAX") && (key == "rvol" || key == "volume") {
            sigs.push(IndicatorSignal::new(SignalKind::VolumeClimax, SignalDirection::Neutral, SignalStatus::Active, l));
        }

        if !sigs.is_empty() {
            if let Some(entry) = map.get_mut(&key) {
                entry.signals.extend(sigs);
            }
        }
    }

    // Confidence boost: entries carrying confirmed/active discrete signals gain
    // conviction beyond their base |normalized|.
    for entry in map.values_mut() {
        if entry.signals.is_empty() {
            continue;
        }
        let mut boost = 0.0f64;
        for s in &entry.signals {
            let base = match s.status {
                SignalStatus::Confirmed => 0.25,
                SignalStatus::Active => 0.15,
                SignalStatus::Potential => 0.08,
            };
            boost = boost.max(base + s.strength * 0.2);
        }
        entry.confidence = (entry.confidence + boost).clamp(0.0, 1.0);
    }
}
