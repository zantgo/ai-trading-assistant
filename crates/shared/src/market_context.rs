//! # Market Context Synthesis
//!
//! A higher-level, human- and AI-readable summary of the current market state,
//! derived from the normalized indicator map by aggregating registry functional
//! groups (Trend / Momentum / Volume / Volatility) plus regime and liquidity.
//! This is meta-intelligence built on top of the indicators — not another
//! indicator itself.

use crate::indicators::normalized::NormalizedIndicatorValue;
use crate::indicators::registry::{IndicatorGroup, INDICATORS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One synthesized dimension of market context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextDimension {
    /// Signed score in `[-1.0, 1.0]` (bull/bear) or magnitude for non-directional.
    pub score: f64,
    /// Confidence in `[0.0, 1.0]` (mean confidence of contributing indicators).
    pub confidence: f64,
    /// Human-readable classification.
    pub label: String,
}

impl ContextDimension {
    fn neutral() -> Self {
        Self { score: 0.0, confidence: 0.0, label: "NEUTRAL".into() }
    }
}

/// Full market-context synthesis for a single timeframe snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketContext {
    pub trend: ContextDimension,
    pub momentum: ContextDimension,
    pub volatility: ContextDimension,
    pub volume: ContextDimension,
    pub liquidity: ContextDimension,
    /// Regime classification: TRENDING | RANGE | EXPANSION | COMPRESSION.
    pub regime: String,
    /// Overall directional conviction in `[-100, 100]`.
    pub overall_score: i32,
    /// Overall label, e.g. STRONG_BULL / WEAK_BEAR / NEUTRAL.
    pub overall_label: String,
}

fn dir_label(score: f64, strong: &str, weak: &str, bear_strong: &str, bear_weak: &str, neutral: &str) -> String {
    if score >= 0.6 { strong } else if score >= 0.15 { weak }
    else if score <= -0.6 { bear_strong } else if score <= -0.15 { bear_weak }
    else { neutral }.to_string()
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
            sum += v.normalized;
            conf += v.confidence;
            n += 1.0;
        }
    }
    if n == 0.0 {
        return ContextDimension::neutral();
    }
    let score = (sum / n).clamp(-1.0, 1.0);
    let confidence = (conf / n).clamp(0.0, 1.0);
    ContextDimension {
        score,
        confidence,
        label: dir_label(score, "STRONG_BULL", "BULL", "STRONG_BEAR", "BEAR", "NEUTRAL"),
    }
}

impl MarketContext {
    /// Synthesize the context from a normalized indicator map.
    pub fn synthesize(map: &HashMap<String, NormalizedIndicatorValue>) -> Self {
        let trend = group_dimension(map, IndicatorGroup::Trend, true);
        let momentum = group_dimension(map, IndicatorGroup::Momentum, true);

        // Volatility: magnitude from BBWP/HV (expansion vs compression), non-directional.
        let bbwp = map.get("bbwp").map(|v| v.raw_value).unwrap_or(50.0);
        let vol_score = ((bbwp - 50.0) / 50.0).clamp(-1.0, 1.0);
        let volatility = ContextDimension {
            score: vol_score,
            confidence: (bbwp / 100.0).clamp(0.0, 1.0),
            label: if bbwp >= 90.0 { "EXPANSION_CLIMAX".into() }
                else if bbwp >= 60.0 { "EXPANDING".into() }
                else if bbwp <= 10.0 { "MAX_COMPRESSION".into() }
                else if bbwp <= 30.0 { "CONTRACTING".into() }
                else { "NORMAL".into() },
        };

        // Volume/participation: RVOL magnitude gate.
        let rvol = map.get("rvol").map(|v| v.raw_value).unwrap_or(1.0);
        let volume = ContextDimension {
            score: ((rvol - 1.0)).clamp(-1.0, 1.0),
            confidence: (rvol / 3.0).clamp(0.0, 1.0),
            label: if rvol >= 3.0 { "CLIMACTIC".into() }
                else if rvol >= 1.5 { "HIGH".into() }
                else if rvol < 0.7 { "THIN".into() }
                else { "NORMAL".into() },
        };

        // Liquidity proxy: VWAP proximity + volume participation.
        let vwap_conf = map.get("vwap").map(|v| v.confidence).unwrap_or(0.0);
        let liquidity = ContextDimension {
            score: 0.0,
            confidence: ((vwap_conf + volume.confidence) / 2.0).clamp(0.0, 1.0),
            label: if rvol >= 1.2 { "GOOD".into() } else if rvol < 0.6 { "LOW".into() } else { "ADEQUATE".into() },
        };

        // Regime from ADX strength + BBWP compression + trend agreement.
        let adx = map.get("adx").map(|v| v.raw_value).unwrap_or(0.0);
        let chop = map.get("choppiness").map(|v| v.raw_value).unwrap_or(50.0);
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
        let overall_label = dir_label(blended, "STRONG_BULL", "WEAK_BULL", "STRONG_BEAR", "WEAK_BEAR", "NEUTRAL");

        Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn niv(norm: f64) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(norm, norm, "X")
    }
    fn niv_raw(raw: f64) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(raw, 0.0, "X")
    }

    #[test]
    fn bullish_trend_and_momentum_positive_overall() {
        let mut m = HashMap::new();
        m.insert("ema_stack".into(), niv(1.0));
        m.insert("supertrend".into(), niv(0.9));
        m.insert("rsi".into(), niv(0.7));
        m.insert("adx".into(), niv_raw(30.0));
        let ctx = MarketContext::synthesize(&m);
        assert!(ctx.trend.score > 0.5);
        assert!(ctx.overall_score > 0);
        assert_eq!(ctx.regime, "TRENDING");
    }

    #[test]
    fn empty_map_is_neutral() {
        let ctx = MarketContext::synthesize(&HashMap::new());
        assert_eq!(ctx.overall_score, 0);
        assert_eq!(ctx.trend.label, "NEUTRAL");
    }

    #[test]
    fn high_bbwp_is_expansion() {
        let mut m = HashMap::new();
        m.insert("bbwp".into(), niv_raw(95.0));
        let ctx = MarketContext::synthesize(&m);
        assert_eq!(ctx.regime, "EXPANSION");
        assert_eq!(ctx.volatility.label, "EXPANSION_CLIMAX");
    }
}
