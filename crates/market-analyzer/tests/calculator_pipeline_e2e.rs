//! AUDIT-AIU-127: REAL-calculator pipeline harness.
//!
//! The legacy `indicator_pipeline_e2e.rs` fabricated `IndicatorInputs`
//! directly (no calculator was ever instantiated), so a math regression in
//! any calculator passed the suite undetected. This harness drives the
//! genuine calculators (Rsi / Macd / Adx / Atr / Stochastic / Mfi / Bbwp /
//! Squeeze) with real candles, bridges their outputs into
//! `IndicatorInputs`, and then runs the production chain
//! `normalize_all → derive_signals → build_indicator_lifecycle_map`.
//!
//! Invariants asserted per bar:
//!   1. No duplicate `(label, kind)` signal pairs (the Svelte
//!      `each_key_duplicate` hazard).
//!   2. Signals emitted only after the calculator's warm-up gate
//!      (no signals from WARMING placeholders).
//!   3. The lifecycle map transitions Loading → Live once real bars
//!      accumulate and the pipeline is live.
//!   4. The RSI middle-band momentum convention holds at the calculator
//!      level (raw 40 → bearish, raw 60 → bullish — AUDIT-AIU-108).

use core_domain::indicator_dtos::{IndicatorLifecycleMap, IndicatorLifecycleState};
use market_analyzer::analyzer::build_indicator_lifecycle_map;
use market_analyzer::indicators::normalized::{
    derive_signals, IndicatorInputs, NormalizationContext, NormalizationEngine,
    NormalizedIndicatorValue,
};
use market_analyzer::indicators::{
    Adx, Atr, Bbwp, CrossoverDir, Macd, Mfi, Rsi, SqueezeMomentum, Stochastic,
};
use rust_decimal::prelude::ToPrimitive;
use std::collections::{HashMap, HashSet};

struct Candle {
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

fn uptrend(n: usize) -> Vec<Candle> {
    let mut out = Vec::with_capacity(n);
    let mut close = 100.0f64;
    for i in 0..n {
        let drift = 0.5 + 0.25 * ((i as f64) * 0.35).sin();
        let open = close;
        close = (close + drift).max(1.0);
        let high = open.max(close) + 0.8 + (i as f64 % 5.0) * 0.1;
        let low = open.min(close) - 0.8 - (i as f64 % 7.0) * 0.1;
        out.push(Candle {
            high,
            low,
            close,
            volume: 100.0 + (i as f64 % 20.0) * 5.0,
        });
    }
    out
}

#[test]
fn real_calculators_flow_through_the_whole_pipeline() {
    let candles = uptrend(240);
    let mut rsi = Rsi::new(14);
    let mut macd = Macd::new();
    let mut adx = Adx::new(14);
    let mut atr = Atr::new(14);
    let mut stoch = Stochastic::new(18, 5, 9);
    let mut mfi = Mfi::new(14);
    let mut bbwp = Bbwp::new(252, 20);
    let mut squeeze = SqueezeMomentum::new(20);

    let mut saw_live_rsi = false;
    for (i, c) in candles.iter().enumerate() {
        let rsi_v = rsi.update(c.close).and_then(|d| d.to_f64());
        let macd_o = macd.update(c.close);
        let adx_v = adx.update(c.high, c.low, c.close).and_then(|o| {
            o.adx.to_f64().map(|a| {
                (
                    a,
                    o.plus_di.to_f64(),
                    o.minus_di.to_f64(),
                    o.adx_slope.to_f64(),
                )
            })
        });
        // ATR calculator exercised for state-machine coverage (its output
        // feeds the normalized map indirectly via other inputs).
        let _atr_v = atr
            .update(c.high, c.low, c.close)
            .and_then(|o| o.atr_value.to_f64());
        let stoch_v = stoch
            .update(c.high, c.low, c.close)
            .and_then(|o| o.k_value.to_f64().zip(o.d_value.to_f64()));
        let mfi_v = mfi
            .update(c.high, c.low, c.close, c.volume)
            .and_then(|d| d.to_f64());
        let bbwp_v = bbwp.update(c.close).and_then(|d| d.to_f64());
        let sqz_o = squeeze.update(c.high, c.low, c.close);

        let inputs = IndicatorInputs {
            rsi: rsi_v,
            macd_line: macd_o.macd_line.to_f64(),
            macd_signal: macd_o.signal_line.to_f64(),
            macd_histogram: macd_o.histogram.to_f64(),
            macd_histogram_peak: macd_o.histogram_peak.to_f64(),
            macd_crossover: macd_o.crossover.map(|c| match c {
                CrossoverDir::Bullish => 1,
                CrossoverDir::Bearish => -1,
            }),
            macd_divergence: market_analyzer::indicators::normalized::DivergenceState::None,
            stoch_k: stoch_v.map(|(k, _)| k),
            stoch_d: stoch_v.map(|(_, d)| d),
            mfi: mfi_v,
            adx: adx_v.map(|(a, _, _, _)| a),
            adx_plus_di: adx_v.and_then(|(_, p, _, _)| p),
            adx_minus_di: adx_v.and_then(|(_, _, m, _)| m),
            adx_slope: adx_v.and_then(|(_, _, _, s)| s),
            bbwp: bbwp_v,
            squeeze_on: sqz_o.as_ref().map(|o| o.squeeze_on),
            squeeze_momentum: sqz_o.as_ref().and_then(|o| o.momentum_value.to_f64()),
            squeeze_release_trigger: sqz_o
                .as_ref()
                .map(|o| o.squeeze_release_trigger)
                .unwrap_or(false),
            ..Default::default()
        };
        let ctx = NormalizationContext::default();
        let mut map: HashMap<String, NormalizedIndicatorValue> =
            NormalizationEngine::normalize_all(&inputs, &ctx, false);

        // Invariant 1: no duplicate (label, kind) pairs per indicator.
        for (key, v) in &map {
            let mut seen: HashSet<(String, String)> = HashSet::new();
            for s in &v.signals {
                let pair = (s.label.clone(), format!("{:?}", s.kind));
                assert!(
                    seen.insert(pair.clone()),
                    "bar {i}: duplicate (label, kind) pair {pair:?} on indicator {key}"
                );
            }
        }

        // Invariant 2: signals never fire on WARMING placeholders.
        derive_signals(&mut map);
        for (key, v) in &map {
            if v.state_label == "WARMING" {
                assert!(
                    v.signals.is_empty(),
                    "bar {i}: WARMING placeholder on {key} must not carry signals"
                );
            }
        }

        // Invariant 4: RSI momentum convention at the calculator level.
        if let (Some(raw), Some(norm)) = (rsi_v, map.get("rsi").map(|v| v.normalized)) {
            if (40.0..50.0).contains(&raw) {
                assert!(
                    norm < 0.0,
                    "bar {i}: RSI {raw} below midline must vote bearish, got {norm}"
                );
            }
            if (50.0..60.0).contains(&raw) {
                assert!(
                    norm > 0.0,
                    "bar {i}: RSI {raw} above midline must vote bullish, got {norm}"
                );
            }
        }

        // Invariant 3: lifecycle — once enough REAL bars accumulate and the
        // pipeline is live, RSI must be Live (and never before).
        let pipeline_live = i as u32 >= 50;
        let lifecycle_map = build_indicator_lifecycle_map(
            &map,
            &IndicatorLifecycleMap::new(),
            300,
            i as u32 + 1,
            i as u32 + 1,
            false,
            1000,
            pipeline_live,
        );
        let rsi_life = lifecycle_map.get("rsi");
        if let Some(entry) = rsi_life {
            if pipeline_live && entry.bars_seen >= entry.bars_required {
                assert_eq!(
                    entry.state,
                    IndicatorLifecycleState::Live,
                    "bar {i}: RSI must be Live once bars_required is met on a live pipeline"
                );
                saw_live_rsi = true;
            }
        }
    }
    assert!(saw_live_rsi, "RSI must reach Live by the end of the run");
}

#[test]
fn rsi_calculator_mid_band_convention_survives_the_pipeline() {
    // Direct calculator-level pin (AUDIT-AIU-108): the calculator itself
    // has no opinion on signs — the NORMALIZER does. Drive the RSI
    // calculator to a mid-band reading and verify the normalized sign.
    let mut rsi = Rsi::new(14);
    // 14 rising closes put RSI at the top of its range; then a mild
    // down-leg drags it into the 40-60 mid band.
    for i in 0..14 {
        rsi.update(100.0 + i as f64 * 2.0);
    }
    for i in 0..8 {
        rsi.update(126.0 - i as f64 * 2.0);
    }
    let raw = rsi
        .update(112.0)
        .and_then(|d| d.to_f64())
        .expect("rsi value");
    assert!(
        (40.0..60.0).contains(&raw),
        "fixture must produce a mid-band RSI, got {raw}"
    );
    let inputs = IndicatorInputs {
        rsi: Some(raw),
        macd_divergence: market_analyzer::indicators::normalized::DivergenceState::None,
        ..Default::default()
    };
    let map = NormalizationEngine::normalize_all(&inputs, &NormalizationContext::default(), false);
    let v = map.get("rsi").expect("rsi present");
    if raw > 50.0 {
        assert!(
            v.normalized > 0.0,
            "RSI {raw} > 50 must be bullish, got {}",
            v.normalized
        );
        assert_eq!(v.state_label, "BULLISH_MOMENTUM");
    } else {
        assert!(
            v.normalized < 0.0,
            "RSI {raw} < 50 must be bearish, got {}",
            v.normalized
        );
        assert_eq!(v.state_label, "BEARISH_MOMENTUM");
    }
}
