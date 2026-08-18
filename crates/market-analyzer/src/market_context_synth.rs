//! # Market Context Synthesis Implementation
//!
//! The `MarketContext` DTO lives in `core-domain`; this module holds the
//! `synthesize` constructor and its helpers because they need access to the
//! indicator registry (`INDICATORS`) to group indicators by functional category.

use crate::indicators::registry::{IndicatorGroup, INDICATORS};
use core_domain::indicator_dtos::NormalizedIndicatorValue;
use core_domain::market_context::{ContextDimension, MarketContext};
use std::collections::HashMap;

fn dir_label(
    score: f64,
    strong: &str,
    weak: &str,
    bear_strong: &str,
    bear_weak: &str,
    neutral: &str,
) -> String {
    if score >= 0.6 {
        strong
    } else if score >= 0.15 {
        weak
    } else if score <= -0.6 {
        bear_strong
    } else if score <= -0.15 {
        bear_weak
    } else {
        neutral
    }
    .to_string()
}

/// Aggregate the enabled directional indicators of a functional group into a
/// weighted-mean signed score + mean confidence.
fn group_dimension(
    map: &HashMap<String, NormalizedIndicatorValue>,
    group: IndicatorGroup,
    directional_only: bool,
) -> ContextDimension {
    let mut sum = 0.0;
    let mut conf = 0.0;
    let mut n = 0.0;
    for meta in INDICATORS {
        if meta.group != group {
            continue;
        }
        if directional_only && !meta.directional {
            continue;
        }
        if let Some(v) = map.get(meta.key) {
            // Guarded: `normalized` is clamp_unit-sanitized, but a NaN
            // `confidence` from any future calculator would poison the
            // weighted mean — collapse non-finite entries to neutral.
            let norm_i = finite(v.normalized, 0.0);
            let conf_i = finite(v.confidence, 0.0);
            sum += norm_i * conf_i;
            conf += conf_i;
            n += 1.0;
        }
    }
    if n < f64::EPSILON {
        return ContextDimension::neutral();
    }
    // AUDIT-AIU-060: the docs (02-07 §5.1) specify a confidence-WEIGHTED
    // mean `Σ(score×confidence)/Σ(confidence)`. The previous code computed
    // `Σ(score×confidence)/n` — the mean of the products — which shrank
    // every score toward zero as confidence dropped. The weighted mean is
    // invariant to confidence scale: an indicator with confidence 0.9
    // contributes 9× the weight of one with confidence 0.1.
    let score = if conf > 1e-9 { sum / conf } else { 0.0 };
    let conf = conf / n;
    let label = dir_label(
        score,
        "STRONG_BULL",
        "WEAK_BULL",
        "STRONG_BEAR",
        "WEAK_BEAR",
        "NEUTRAL",
    );
    ContextDimension {
        score,
        confidence: conf,
        label,
    }
}

/// Collapse non-finite indicator inputs to a safe fallback. `f64::clamp`
/// passes NaN through (and NaN comparisons are all false), so every raw
/// value entering the synthesis is sanitized first — one NaN anywhere
/// would otherwise poison the dimension scores, confidence fields and
/// `overall_score` on the wire (serde_json emits NaN as `null`).
fn finite(v: f64, fallback: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        fallback
    }
}

/// Synthesize the context from a normalized indicator map.
///
/// Lives in `market-analyzer` because it reads `INDICATORS` to group
/// contributions by functional category.
pub fn synthesize_market_context(map: &HashMap<String, NormalizedIndicatorValue>) -> MarketContext {
    let trend = group_dimension(map, IndicatorGroup::Trend, true);
    let momentum = group_dimension(map, IndicatorGroup::Momentum, true);

    // Volatility: magnitude from BBWP/HV (expansion vs compression), non-directional.
    let bbwp = finite(map.get("bbwp").map(|v| v.raw_value).unwrap_or(50.0), 50.0);
    let vol_score = ((bbwp - 50.0) / 50.0).clamp(-1.0, 1.0);
    let volatility = ContextDimension {
        score: vol_score,
        confidence: (bbwp / 100.0).clamp(0.0, 1.0),
        label: if bbwp >= 90.0 {
            "EXPANSION_CLIMAX".into()
        } else if bbwp >= 60.0 {
            "EXPANDING".into()
        } else if bbwp <= 10.0 {
            "MAX_COMPRESSION".into()
        } else if bbwp <= 30.0 {
            "CONTRACTING".into()
        } else {
            "NORMAL".into()
        },
    };

    // Volume/participation: RVOL magnitude gate.
    let rvol = finite(map.get("rvol").map(|v| v.raw_value).unwrap_or(1.0), 1.0);
    let volume = ContextDimension {
        score: (rvol - 1.0).clamp(-1.0, 1.0),
        confidence: (rvol / 3.0).clamp(0.0, 1.0),
        label: if rvol >= 3.0 {
            "CLIMACTIC".into()
        } else if rvol >= 1.5 {
            "HIGH".into()
        } else if rvol < 0.7 {
            "THIN".into()
        } else {
            "NORMAL".into()
        },
    };

    // Liquidity proxy: VWAP proximity + volume participation.
    //
    // AUDIT-AIU-061: the previous code scored the VWAP contribution as
    // `vwap_score × 50` where `vwap_score = vwap.normalized` — a SIGNED
    // premium/discount reading. The comment claimed "near fair value →
    // higher liquidity", but `confidence = |normalized|` means high
    // confidence = FAR from fair value, so the mapping contradicted its own
    // intent (extreme premium −0.8 → −40 liquidity; equilibrium → 0).
    // The corrected contribution is proximity-based: `(1 − |normalized|)`
    // so price AT fair value contributes +50 and a stretched price
    // contributes ~0.
    let vwap_conf = finite(map.get("vwap").map(|v| v.confidence).unwrap_or(0.0), 0.0);
    let vwap_score = finite(map.get("vwap").map(|v| v.normalized).unwrap_or(0.0), 0.0);
    let rvol_contrib = ((rvol - 1.0) * 50.0).clamp(-50.0, 50.0);
    let vwap_contrib = ((1.0 - vwap_score.abs()).clamp(0.0, 1.0)) * 50.0;
    let liquidity_score = (rvol_contrib + vwap_contrib).clamp(-100.0, 100.0);
    let liquidity = ContextDimension {
        score: liquidity_score,
        confidence: ((vwap_conf + volume.confidence) / 2.0).clamp(0.0, 1.0),
        label: if rvol >= 1.2 {
            "GOOD".into()
        } else if rvol < 0.6 {
            "LOW".into()
        } else {
            "ADEQUATE".into()
        },
    };

    // Regime from ADX strength + BBWP compression + trend agreement.
    let adx = finite(map.get("adx").map(|v| v.raw_value).unwrap_or(0.0), 0.0);
    let chop = finite(
        map.get("choppiness").map(|v| v.raw_value).unwrap_or(50.0),
        50.0,
    );
    let regime = if bbwp <= 15.0 || chop >= 61.8 {
        "COMPRESSION"
    } else if bbwp >= 85.0 {
        "EXPANSION"
    } else if adx >= 25.0 || chop <= 38.2 {
        "TRENDING"
    } else {
        "RANGE"
    }
    .to_string();

    // Overall = confidence-weighted blend of trend + momentum (directional),
    // dampened when the regime is range/compression.
    let regime_gate = match regime.as_str() {
        "TRENDING" | "EXPANSION" => 1.0,
        "RANGE" => 0.6,
        _ => 0.5,
    };
    let blended = (trend.score * 0.6 + momentum.score * 0.4) * regime_gate;
    let overall_score = (blended * 100.0).round() as i32;
    let overall_label = dir_label(
        blended,
        "STRONG_BULL",
        "WEAK_BULL",
        "STRONG_BEAR",
        "WEAK_BEAR",
        "NEUTRAL",
    );

    MarketContext {
        trend,
        momentum,
        volatility,
        volume,
        liquidity,
        regime,
        overall_score,
        overall_label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_snapshot_inputs() -> HashMap<String, NormalizedIndicatorValue> {
        HashMap::new()
    }

    fn scalar(key: &str, raw: f64, norm: f64) -> HashMap<String, NormalizedIndicatorValue> {
        let mut m = empty_snapshot_inputs();
        m.insert(
            key.to_string(),
            NormalizedIndicatorValue::scalar(raw, norm, "TEST"),
        );
        m
    }

    #[test]
    fn empty_map_is_neutral() {
        let ctx = synthesize_market_context(&empty_snapshot_inputs());
        assert_eq!(ctx.regime, "RANGE");
        assert_eq!(ctx.overall_label, "NEUTRAL");
        assert_eq!(ctx.overall_score, 0);
    }

    #[test]
    fn high_bbwp_is_expansion() {
        let map = scalar("bbwp", 95.0, 0.9);
        let ctx = synthesize_market_context(&map);
        assert_eq!(ctx.regime, "EXPANSION");
        assert_eq!(ctx.volatility.label, "EXPANSION_CLIMAX");
    }

    #[test]
    fn bullish_trend_and_momentum_positive_overall() {
        let mut map = empty_snapshot_inputs();
        // Inject a few indicators so trend+momentum groups have something.
        map.insert(
            "rsi".into(),
            NormalizedIndicatorValue::scalar(75.0, 0.8, "OVERBOUGHT"),
        );
        map.insert(
            "macd".into(),
            NormalizedIndicatorValue::scalar(1.5, 0.7, "BULLISH_EXPANDING"),
        );
        let ctx = synthesize_market_context(&map);
        assert!(ctx.overall_score > 0, "expected positive overall score");
    }
}
