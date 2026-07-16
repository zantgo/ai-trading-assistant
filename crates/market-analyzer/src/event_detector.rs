//! # Event Detector
//!
//! Detects discrete state transitions on the normalized indicator map:
//! squeeze release, S/R role flips, EMA200 crosses, and confirmed
//! divergences. Operates directly on a `&HashMap<String, NormalizedIndicatorValue>`
//! so it stays free of HTTP-shaped DTOs (`IndicatorSnapshot`).
//!
//! This module is currently dormant (no callers outside its own file); it
//! is preserved as a reference implementation for future event-driven
//! trigger pipelines.

use std::collections::HashMap;

use core_domain::indicator_dtos::NormalizedIndicatorValue;

fn raw(map: &HashMap<String, NormalizedIndicatorValue>, k: &str) -> Option<f64> {
    map.get(k).map(|v| v.raw_value)
}

fn sub(map: &HashMap<String, NormalizedIndicatorValue>, k: &str, s: &str) -> Option<f64> {
    map.get(k).and_then(|v| v.values.as_ref()).and_then(|m| m.get(s)).copied()
}

fn lbl<'a>(
    map: &'a HashMap<String, NormalizedIndicatorValue>,
    k: &str,
) -> Option<&'a str> {
    map.get(k).map(|v| v.state_label.as_str())
}

/// Detects squeeze release: squeeze was active (coiling) in prev and is now
/// released (volatility release label). Returns true only on the transition.
pub fn check_squeeze_release(
    prev: Option<&HashMap<String, NormalizedIndicatorValue>>,
    curr: &HashMap<String, NormalizedIndicatorValue>,
) -> bool {
    let prev_map = match prev {
        Some(m) => m,
        None => return false,
    };
    let was_squeezing = lbl(prev_map, "squeeze").map(|s| s.contains("COMPRESSION")).unwrap_or(false);
    let is_released = lbl(curr, "squeeze").map(|s| s.contains("RELEASE")).unwrap_or(false);
    was_squeezing && is_released
}

/// Detects a Support/Resistance role flip via the state label transition.
pub fn check_sr_flip(
    prev: Option<&HashMap<String, NormalizedIndicatorValue>>,
    curr: &HashMap<String, NormalizedIndicatorValue>,
) -> bool {
    let prev_map = match prev {
        Some(m) => m,
        None => return false,
    };
    let prev_has_flip = lbl(prev_map, "support_resistance").map(|s| s.contains("FLIP")).unwrap_or(false);
    let curr_has_flip = lbl(curr, "support_resistance").map(|s| s.contains("FLIP")).unwrap_or(false);
    curr_has_flip && !prev_has_flip
}

/// Detects an EMA 200 cross: price crosses from one side of the 200 EMA to
/// the other between prev and curr.
pub fn check_ema200_cross(
    prev: Option<&HashMap<String, NormalizedIndicatorValue>>,
    curr: &HashMap<String, NormalizedIndicatorValue>,
    prev_price: Option<f64>,
    curr_price: Option<f64>,
) -> bool {
    let prev_map = match prev {
        Some(m) => m,
        None => return false,
    };
    let prev_ema = sub(prev_map, "ema_stack", "long");
    let curr_ema = sub(curr, "ema_stack", "long");
    match (prev_price, prev_ema, curr_price, curr_ema) {
        (Some(pp), Some(pe), Some(cp), Some(ce)) => (pp <= pe && cp > ce) || (pp >= pe && cp < ce),
        _ => false,
    }
}

/// Detects confirmed divergence on RSI or MACD: `Potential` -> `Confirmed`.
pub fn check_confirmed_divergence(
    prev: Option<&HashMap<String, NormalizedIndicatorValue>>,
    curr: &HashMap<String, NormalizedIndicatorValue>,
) -> bool {
    let prev_map = match prev {
        Some(m) => m,
        None => return false,
    };
    let prev_rsi_potential = lbl(prev_map, "rsi").map(|s| s.starts_with("potential")).unwrap_or(false);
    let curr_rsi_confirmed = lbl(curr, "rsi").map(|s| s.starts_with("confirmed")).unwrap_or(false);
    let prev_macd_potential = lbl(prev_map, "macd").map(|s| s.starts_with("potential")).unwrap_or(false);
    let curr_macd_confirmed = lbl(curr, "macd").map(|s| s.starts_with("confirmed")).unwrap_or(false);
    (prev_rsi_potential && curr_rsi_confirmed) || (prev_macd_potential && curr_macd_confirmed)
}

/// Evaluate all enabled events against prev and curr indicator maps.
/// `prev_price` / `curr_price` supply the raw price input for `ema200_cross`.
pub fn evaluate_trigger_events(
    prev: Option<&HashMap<String, NormalizedIndicatorValue>>,
    curr: &HashMap<String, NormalizedIndicatorValue>,
    prev_price: Option<f64>,
    curr_price: Option<f64>,
    enabled_events: &[String],
) -> Vec<String> {
    let mut triggered = Vec::new();
    for event in enabled_events {
        let fired = match event.as_str() {
            "squeeze_release" => check_squeeze_release(prev, curr),
            "sr_flip" => check_sr_flip(prev, curr),
            "ema200_cross" => check_ema200_cross(prev, curr, prev_price, curr_price),
            "confirmed_divergence" => check_confirmed_divergence(prev, curr),
            _ => false,
        };
        if fired {
            triggered.push(event.clone());
        }
    }
    triggered
}

#[allow(dead_code)]
fn _unused_raw_helper() {
    let _ = raw;
}
