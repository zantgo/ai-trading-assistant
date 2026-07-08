//! Structural Integrity Score — composite metric quantifying how clean and
//! well-defined the market structure is. Pure math, no I/O, testable under TEST-CORE.

use crate::indicators::normalized::NormalizedIndicatorValue;
use std::collections::HashMap;

pub fn compute_structural_integrity(
    indicators: &HashMap<String, NormalizedIndicatorValue>,
) -> f64 {
    let level_clarity = compute_level_clarity(indicators);
    let pattern_confidence = get_pattern_confidence(indicators);
    let smc_activity = get_active_zone_count(indicators);
    let trend_cleanliness = compute_trend_cleanliness(indicators);
    let level_proximity = get_nearest_level_proximity(indicators);

    0.25 * level_clarity
        + 0.25 * pattern_confidence
        + 0.20 * smc_activity
        + 0.15 * trend_cleanliness
        + 0.15 * level_proximity
}

fn compute_level_clarity(indicators: &HashMap<String, NormalizedIndicatorValue>) -> f64 {
    let sr = indicators.get("support_resistance");
    let label = sr.map(|v| v.state_label.as_str()).unwrap_or("STRUCTURE_NEUTRAL");
    if label == "STRUCTURE_NEUTRAL" {
        return 0.3;
    }
    let norm = sr.map(|v| v.normalized.abs()).unwrap_or(0.0);
    (0.5 + norm * 0.5).clamp(0.0, 1.0)
}

fn get_pattern_confidence(indicators: &HashMap<String, NormalizedIndicatorValue>) -> f64 {
    let patterns = indicators.get("patterns");
    let pattern_conf = patterns.map(|v| v.confidence).unwrap_or(0.0);
    let candlestick_conf = indicators
        .get("candlestick")
        .map(|v| v.confidence)
        .unwrap_or(0.0);
    let has_pattern = patterns
        .map(|v| !v.state_label.contains("NO_PATTERN"))
        .unwrap_or(false);
    let has_candle = indicators
        .get("candlestick")
        .map(|v| v.confidence > 0.3)
        .unwrap_or(false);

    if !has_pattern && !has_candle {
        return 0.0;
    }
    let pattern_weight = if has_pattern { 1.0 } else { 0.0 };
    let candle_weight = if has_candle { 1.0 } else { 0.0 };
    let total_weight = pattern_weight + candle_weight;
    if total_weight == 0.0 {
        return 0.0;
    }
    (pattern_conf * pattern_weight + candlestick_conf * candle_weight) / total_weight
}

fn get_active_zone_count(indicators: &HashMap<String, NormalizedIndicatorValue>) -> f64 {
    let ob_active = indicators
        .get("smc_order_blocks")
        .map(|v| v.state_label.contains("ACTIVE") || v.state_label.contains("TEST"))
        .unwrap_or(false);
    let fvg_open = indicators
        .get("smc_fvg")
        .map(|v| v.state_label.contains("OPEN"))
        .unwrap_or(false);
    let bos_present = indicators
        .get("smc_structure")
        .map(|v| v.state_label.contains("BOS"))
        .unwrap_or(false);
    let sweep_present = indicators
        .get("smc_liquidity")
        .map(|v| v.state_label.contains("SWEEP"))
        .unwrap_or(false);

    let zone_count =
        (ob_active as u8 + fvg_open as u8 + bos_present as u8 + sweep_present as u8) as f64;
    (zone_count / 4.0).clamp(0.0, 1.0)
}

fn compute_trend_cleanliness(indicators: &HashMap<String, NormalizedIndicatorValue>) -> f64 {
    let chop_norm = indicators
        .get("choppiness")
        .map(|v| v.raw_value / 100.0)
        .unwrap_or(0.5);
    1.0 - chop_norm
}

fn get_nearest_level_proximity(indicators: &HashMap<String, NormalizedIndicatorValue>) -> f64 {
    let sr_norm = indicators
        .get("support_resistance")
        .map(|v| v.normalized.abs())
        .unwrap_or(0.0);
    let fib_norm = indicators
        .get("fibonacci")
        .map(|v| v.normalized.abs())
        .unwrap_or(0.0);
    let best = sr_norm.max(fib_norm);
    (best * 0.8 + 0.2).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::normalized::NormalizedIndicatorValue;

    #[test]
    fn test_integrity_score_bounded() {
        let map: HashMap<String, NormalizedIndicatorValue> = HashMap::new();
        let score = compute_structural_integrity(&map);
        assert!(score >= 0.0 && score <= 1.0, "score {} out of bounds", score);
    }

    #[test]
    fn test_integrity_with_active_zones() {
        let mut map = HashMap::new();
        map.insert(
            "support_resistance".into(),
            NormalizedIndicatorValue::scalar(0.5, 0.8, "SUPPORT_DEMAND_ZONE"),
        );
        map.insert(
            "smc_order_blocks".into(),
            NormalizedIndicatorValue::scalar(0.5, 0.7, "SMC_OB_BULLISH_ACTIVE"),
        );
        map.insert(
            "smc_fvg".into(),
            NormalizedIndicatorValue::scalar(0.3, 0.5, "SMC_FVG_BULLISH_OPEN"),
        );
        let score = compute_structural_integrity(&map);
        assert!(score > 0.3, "active zones should raise integrity; got {}", score);
    }
}
