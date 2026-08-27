// v9 (F-06): the score-tiered allocation-curve machinery is ERASED with
// the scaled-entry/pyramiding legacy — `evaluate_allocation_curve` and
// its threshold constants had no remaining consumers (sizing is the
// v8.2 `allocation_pct` model + the strategy's `tae.sizing.quality_curve`).
// This module retains only the registry-confluence computation consumed
// by the `/api/monitor` surface.

/// Equal-weighted registry confluence over ALL directional indicators.
///
/// Every directional indicator present in the snapshot contributes equally
/// (±1 weight). Non-directional gates (choppiness, ADX congestion) act as
/// multiplicative dampening on the overall score. No per-indicator weighting,
/// no regime-aware multipliers, no configurable weights.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistryConfluence {
    /// Bias-projected confluence in `[-1.0, 1.0]`.
    pub normalized: f64,
    /// Scaled integer score in `[-100, 100]` for display/thresholds.
    pub score: i32,
    /// Number of active directional indicators contributing.
    pub active_count: u32,
    /// Non-directional gate multiplier applied (choppiness/adx).
    pub regime_gate: f64,
}

/// Compute the equal-weighted registry confluence: simple arithmetic mean of
/// every enabled, present, directional registry indicator, dampened by
/// non-directional regime gates (Choppiness / ADX congestion) and projected
/// onto the trade bias.
pub fn calculate_registry_confluence(
    bias: &str,
    snap: &crate::profile_evaluation::SnapshotValues,
) -> RegistryConfluence {
    use market_analyzer::indicators::registry::INDICATORS;

    // ── Non-directional regime gates (multiplicative confidence) ──
    let adx_congested =
        snap.raw("adx").is_some_and(|a| a < 20.0) || snap.label("adx") == "TRENDLESS_CONGESTION";
    let chop_gate = match snap.raw("choppiness") {
        Some(c) if c >= 61.8 => 0.5,
        Some(c) if c <= 38.2 => 1.0,
        _ => 0.85,
    };
    let adx_gate = if adx_congested { 0.6 } else { 1.0 };
    let regime_gate = chop_gate * adx_gate;

    let mut sum = 0.0f64;
    let mut count: u32 = 0;

    for meta in INDICATORS {
        if !meta.directional {
            continue;
        }
        if !snap.indicators.contains_key(meta.key) {
            continue;
        }
        sum += snap.norm(meta.key);
        count += 1;
    }

    let mean = if count > 0 { sum / count as f64 } else { 0.0 };
    let gated = (mean * regime_gate).clamp(-1.0, 1.0);
    let projected = if bias == "BULLISH" { gated } else { -gated };

    RegistryConfluence {
        normalized: projected,
        score: (projected * 100.0).round() as i32,
        active_count: count,
        regime_gate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use market_analyzer::indicators::normalized::NormalizedIndicatorValue;
    use std::collections::HashMap;

    fn niv(norm: f64, label: &str) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(norm, norm, label)
    }

    fn snap_with(entries: &[(&str, f64, &str)]) -> crate::profile_evaluation::SnapshotValues {
        let mut map = HashMap::new();
        for (k, n, l) in entries {
            map.insert((*k).to_string(), niv(*n, l));
        }
        crate::profile_evaluation::SnapshotValues::from_map(map, 100.0)
    }

    #[test]
    fn registry_confluence_bullish_alignment_positive() {
        let snap = snap_with(&[
            ("rsi", 0.8, "OVERSOLD_ACCUMULATION"),
            ("supertrend", 0.9, "SUPERTREND_BULLISH"),
            ("mfi", 0.6, "MFI_BULLISH_FLOW"),
        ]);
        let c = calculate_registry_confluence("BULLISH", &snap);
        assert!(
            c.normalized > 0.0,
            "aligned bulls → positive, got {}",
            c.normalized
        );
        assert!(c.normalized <= 1.0 && c.normalized >= -1.0);
        assert_eq!(c.active_count, 3);
    }

    #[test]
    fn registry_confluence_equal_weighted_no_bias() {
        let mut snap = snap_with(&[
            ("rsi", 1.0, "OVERSOLD_ACCUMULATION"),
            ("supertrend", 1.0, "SUPERTREND_BULLISH"),
        ]);
        // Inject low choppiness to remove the dampening gate.
        snap.indicators.insert(
            "choppiness".into(),
            NormalizedIndicatorValue::scalar(30.0, 0.0, "CHOP_STRONG_TREND"),
        );
        let c = calculate_registry_confluence("BULLISH", &snap);
        assert!(
            (c.normalized - 1.0).abs() < 1e-9,
            "two +1 should average to +1, got {}",
            c.normalized
        );
    }

    #[test]
    fn registry_confluence_choppy_regime_dampens() {
        let mut trend = snap_with(&[
            ("supertrend", 1.0, "SUPERTREND_BULLISH"),
            ("rsi", 0.8, "OVERSOLD_ACCUMULATION"),
        ]);
        trend.indicators.insert(
            "choppiness".into(),
            NormalizedIndicatorValue::scalar(20.0, 0.0, "CHOP_STRONG_TREND"),
        );

        let mut choppy = snap_with(&[
            ("supertrend", 1.0, "SUPERTREND_BULLISH"),
            ("rsi", 0.8, "OVERSOLD_ACCUMULATION"),
        ]);
        choppy.indicators.insert(
            "choppiness".into(),
            NormalizedIndicatorValue::scalar(80.0, 0.0, "CHOP_CONSOLIDATION_RANGE"),
        );

        let ct = calculate_registry_confluence("BULLISH", &trend);
        let cc = calculate_registry_confluence("BULLISH", &choppy);
        assert!(
            cc.normalized < ct.normalized,
            "choppy regime should dampen conviction"
        );
    }
}
