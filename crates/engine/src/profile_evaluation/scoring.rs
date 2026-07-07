use super::SnapshotValues;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistryConfluence {
    /// Bias-projected confluence in `[-1.0, 1.0]` (weighted mean of directional
    /// normalized values × gates).
    pub normalized: f64,
    /// Scaled integer score in `[-100, 100]` for display/thresholds.
    pub score: i32,
    /// Total active weight of enabled/present directional indicators.
    pub active_weight: f64,
    /// Non-directional gate multiplier applied this run (choppiness/adx).
    pub regime_gate: f64,
    /// Per-indicator signed contributions (`weight × normalized`).
    pub contributions: Vec<(String, f64)>,
}

/// Compute the registry-driven confluence: `Σ(weight × normalized)` over every
/// enabled, present, **directional** registry indicator, divided by the active
/// weight, then dampened by non-directional regime gates (Choppiness / ADX
/// congestion) and projected onto the trade bias.
///
/// `weights` / `enabled` override the registry defaults (weight 1.0, enabled).
/// `regime_multipliers` (optional) scales each indicator's weight by the active
/// regime's multiplier (regime derived from the snapshot's market context).
pub fn calculate_registry_confluence(
    bias: &str,
    snap: &SnapshotValues,
    weights: &HashMap<String, f64>,
    enabled: &HashMap<String, bool>,
    regime_multipliers: Option<&HashMap<String, HashMap<String, f64>>>,
) -> RegistryConfluence {
    use shared::indicators::registry::INDICATORS;

    // Active regime for regime-aware weighting (from BBWP/ADX/choppiness).
    let regime = classify_regime_label(snap);
    let regime_tbl = regime_multipliers.and_then(|m| m.get(&regime));

    // ── Non-directional regime gates (multiplicative confidence) ──
    let adx_congested = snap.raw("adx").is_some_and(|a| a < 20.0)
        || snap.label("adx") == "TRENDLESS_CONGESTION";
    let chop_gate = match snap.raw("choppiness") {
        Some(c) if c >= 61.8 => 0.5, // choppy/range → halve conviction
        Some(c) if c <= 38.2 => 1.0, // strong trend → full
        _ => 0.85,
    };
    let adx_gate = if adx_congested { 0.6 } else { 1.0 };
    let regime_gate = chop_gate * adx_gate;

    let mut sum = 0.0f64;
    let mut active_weight = 0.0f64;
    let mut contributions: Vec<(String, f64)> = Vec::new();

    for meta in INDICATORS {
        if !meta.directional {
            continue; // gates handled above
        }
        if !enabled.get(meta.key).copied().unwrap_or(meta.default_enabled) {
            continue;
        }
        // Only count indicators actually present in this snapshot.
        if !snap.indicators.contains_key(meta.key) {
            continue;
        }
        // Skip explicit INACTIVE placeholders (untriggered event-driven
        // indicators). Including them would inflate the denominator and dilute
        // the weighted-average score (attenuation bias). Genuine equilibrium
        // readings carry their own labels and still count.
        if snap.label(meta.key) == "INACTIVE" {
            continue;
        }
        let base_w = weights.get(meta.key).copied().unwrap_or(meta.default_weight);
        let regime_mult = regime_tbl.and_then(|t| t.get(meta.key)).copied().unwrap_or(1.0);
        let w = base_w * regime_mult;
        if w == 0.0 {
            continue;
        }
        let contrib = snap.norm(meta.key) * w;
        sum += contrib;
        active_weight += w;
        contributions.push((meta.key.to_string(), contrib));
    }

    let mean = if active_weight > 0.0 { sum / active_weight } else { 0.0 };
    let gated = (mean * regime_gate).clamp(-1.0, 1.0);
    let projected = if bias == "BULLISH" { gated } else { -gated };

    RegistryConfluence {
        normalized: projected,
        score: (projected * 100.0).round() as i32,
        active_weight,
        regime_gate,
        contributions,
    }
}

/// Coarse regime classifier used for regime-aware weighting (mirrors the market
/// context synthesis thresholds).
fn classify_regime_label(snap: &SnapshotValues) -> String {
    let bbwp = snap.raw("bbwp").unwrap_or(50.0);
    let chop = snap.raw("choppiness").unwrap_or(50.0);
    let adx = snap.raw("adx").unwrap_or(0.0);
    if bbwp <= 15.0 || chop >= 61.8 {
        "COMPRESSION"
    } else if bbwp >= 85.0 {
        "EXPANSION"
    } else if adx >= 25.0 || chop <= 38.2 {
        "TRENDING"
    } else {
        "RANGE"
    }
    .to_string()
}

/// Opposite-signal exit threshold under the registry confluence ±100 scale.
/// Proportional equivalent of the legacy 54/90 ≈ 60/100 (60% conviction bar).
pub const REGISTRY_OPPOSITE_EXIT_THRESHOLD: f64 = 60.0;

/// Registry-driven opposite-signal score: sum of `|contribution|` for every
/// directional indicator whose sign opposes the held position. Computed from
/// the same full registry contributions as the primary confluence.
pub fn calculate_registry_opposite_score(
    position_direction: &str,
    snap: &SnapshotValues,
    weights: &HashMap<String, f64>,
    enabled: &HashMap<String, bool>,
    regime_multipliers: Option<&HashMap<String, HashMap<String, f64>>>,
) -> u32 {
    let c = calculate_registry_confluence("BULLISH", snap, weights, enabled, regime_multipliers);
    let holding_long = position_direction == "LONG";
    c.contributions
        .iter()
        .filter(|(_, v)| if holding_long { *v < 0.0 } else { *v > 0.0 })
        .map(|(_, v)| v.abs() as u32)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::indicators::normalized::NormalizedIndicatorValue;
    use std::collections::HashMap;

    fn niv(norm: f64, label: &str) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(norm, norm, label)
    }

    fn snap_with(entries: &[(&str, f64, &str)], price: f64) -> SnapshotValues {
        let mut map = HashMap::new();
        for (k, n, l) in entries {
            map.insert((*k).to_string(), niv(*n, l));
        }
        SnapshotValues::from_map(map, price)
    }

    fn niv_raw(raw: f64, norm: f64, label: &str) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(raw, norm, label)
    }

    #[test]
    fn registry_confluence_bullish_alignment_positive() {
        let snap = snap_with(
            &[
                ("rsi", 0.8, "OVERSOLD_ACCUMULATION"),
                ("supertrend", 0.9, "SUPERTREND_BULLISH"),
                ("mfi", 0.6, "MFI_BULLISH_FLOW"),
                ("choppiness", 0.0, "CHOP_STRONG_TREND"),
            ],
            100.0,
        );
        let c = calculate_registry_confluence("BULLISH", &snap, &HashMap::new(), &HashMap::new(), None);
        assert!(c.normalized > 0.0, "aligned bulls → positive, got {}", c.normalized);
        assert!(c.normalized <= 1.0 && c.normalized >= -1.0);
    }

    #[test]
    fn registry_confluence_disabled_indicator_excluded() {
        let snap = snap_with(&[("rsi", 1.0, "OVERSOLD_ACCUMULATION")], 100.0);
        let mut enabled = HashMap::new();
        enabled.insert("rsi".to_string(), false);
        let c = calculate_registry_confluence("BULLISH", &snap, &HashMap::new(), &enabled, None);
        assert_eq!(c.active_weight, 0.0, "disabled rsi contributes no weight");
        assert_eq!(c.score, 0);
    }

    #[test]
    fn registry_confluence_inactive_placeholders_do_not_dilute() {
        let readings: &[(&str, f64, &str)] = &[
            ("rsi", 0.8, "OVERSOLD_ACCUMULATION"),
            ("ema_stack", 1.0, "ESTABLISHED_BULLISH_STACK"),
            ("supertrend", 0.6, "SUPERTREND_BULLISH"),
        ];
        let base = snap_with(readings, 100.0);
        let base_c = calculate_registry_confluence("BULLISH", &base, &HashMap::new(), &HashMap::new(), None);

        let mut withfill = snap_with(readings, 100.0);
        for key in ["rsi_divergence", "macd_divergence", "patterns", "fibonacci", "support_resistance", "zscore"] {
            withfill.indicators.insert(key.to_string(), NormalizedIndicatorValue::scalar(0.0, 0.0, "INACTIVE"));
        }
        let filled_c = calculate_registry_confluence("BULLISH", &withfill, &HashMap::new(), &HashMap::new(), None);

        assert_eq!(base_c.active_weight, filled_c.active_weight, "INACTIVE must not add to active weight");
        assert_eq!(base_c.score, filled_c.score, "INACTIVE placeholders must not dilute the score");
    }

    #[test]
    fn registry_confluence_choppy_regime_dampens() {
        let trend = snap_with(
            &[("supertrend", 1.0, "SUPERTREND_BULLISH"), ("choppiness", 0.0, "CHOP_STRONG_TREND")],
            100.0,
        );
        let choppy = snap_with(
            &[("supertrend", 1.0, "SUPERTREND_BULLISH"), ("choppiness", 0.0, "CHOP_CONSOLIDATION_RANGE")],
            100.0,
        );
        // Inject raw choppiness value so the gate reads it.
        let mut t = trend;
        t.indicators.insert("choppiness".into(), NormalizedIndicatorValue::scalar(20.0, 0.0, "CHOP_STRONG_TREND"));
        let mut ch = choppy;
        ch.indicators.insert("choppiness".into(), NormalizedIndicatorValue::scalar(80.0, 0.0, "CHOP_CONSOLIDATION_RANGE"));
        let ct = calculate_registry_confluence("BULLISH", &t, &HashMap::new(), &HashMap::new(), None);
        let cc = calculate_registry_confluence("BULLISH", &ch, &HashMap::new(), &HashMap::new(), None);
        assert!(cc.normalized < ct.normalized, "choppy regime should dampen conviction");
    }
}
