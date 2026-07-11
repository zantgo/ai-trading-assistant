//! # Confluence Matrix — Multi-Timeframe Alignment
//!
//! The Confluence Matrix aggregates the four Metrics Matrices (micro, fast,
//! slow, macro) for a single symbol to measure cross-timeframe agreement.
//! This is distinct from the per-instance confluence score inside MarketContext
//! (which is a single-timeframe weighted mean).
//!
//! Layer: L2.5 in the 9-layer architecture (between Normalize+Sig and Registry).

use crate::market_context::MarketContext;
use crate::indicators::normalized::NormalizedIndicatorValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Alignment breakdown for a single timeframe within Confluence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TfAlignmentInfo {
    pub timeframe: String,
    pub timeframe_secs: u64,
    pub trend_score: f64,
    pub momentum_score: f64,
    pub overall_score: i32,
    pub regime: String,
    pub active_signals: u32,
    pub price: f64,
}

/// Cross-timeframe Confluence Matrix for one symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentMatrix {
    pub symbol: String,
    pub timeframes_present: u8,
    pub mtf_trend_alignment: f64,
    pub mtf_momentum_alignment: f64,
    pub mtf_volume_alignment: f64,
    pub mtf_volatility_alignment: f64,
    pub mtf_overall_score: f64,
    pub mtf_overall_label: String,
    pub timeframe_alignments: Vec<TfAlignmentInfo>,
    pub signal_cross_tf_count: u32,
    pub trend_agreement_pct: f64,
}

impl AlignmentMatrix {
    /// Create an empty AlignmentMatrix for a symbol with no data.
    pub fn empty(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            timeframes_present: 0,
            mtf_trend_alignment: 0.0,
            mtf_momentum_alignment: 0.0,
            mtf_volume_alignment: 0.0,
            mtf_volatility_alignment: 0.0,
            mtf_overall_score: 0.0,
            mtf_overall_label: "NO_DATA".to_string(),
            timeframe_alignments: Vec::new(),
            signal_cross_tf_count: 0,
            trend_agreement_pct: 0.0,
        }
    }

    fn overall_label(score: f64) -> String {
        if score >= 60.0 {
            "STRONG_BULL_MTF".into()
        } else if score >= 20.0 {
            "WEAK_BULL_MTF".into()
        } else if score <= -60.0 {
            "STRONG_BEAR_MTF".into()
        } else if score <= -20.0 {
            "WEAK_BEAR_MTF".into()
        } else {
            "NEUTRAL_MTF".into()
        }
    }
}

/// Build the Confluence Matrix by aggregating indicator maps from multiple
/// timeframes for a single symbol.
///
/// `tf_data` is a vec of `(timeframe_label, timeframe_secs, price, indicator_map)` tuples.
/// Higher timeframes (slower, longer candles) get proportionally more weight.
pub fn compute_alignment(
    symbol: &str,
    tf_data: &[(&str, u64, f64, &HashMap<String, NormalizedIndicatorValue>)],
) -> AlignmentMatrix {
    if tf_data.is_empty() {
        return AlignmentMatrix::empty(symbol);
    }

    let mut alignments = Vec::with_capacity(tf_data.len());
    let mut trend_sum = 0.0;
    let mut momentum_sum = 0.0;
    let mut volume_sum = 0.0;
    let mut volatility_sum = 0.0;
    let mut total_weight = 0.0;
    let mut positive_tf_count = 0u32;
    let mut negative_tf_count = 0u32;
    let mut total_signals = 0u32;

    // Compute a MarketContext per timeframe from its indicator map.
    for &(label, secs, price, map) in tf_data {
        let ctx = MarketContext::synthesize(map);

        let tf_signals: u32 = map.values()
            .map(|v| v.signals.len() as u32)
            .sum();
        total_signals += tf_signals;

        // Weight: higher timeframes (slower) get more weight. 900s (15m) =
        // weight 1.0, 60s (1m) = weight ~0.25. 180=0.35, 300=0.55.
        let weight = (secs as f64 / 900.0).clamp(0.2, 1.0);
        total_weight += weight;

        trend_sum += ctx.trend.score * weight;
        momentum_sum += ctx.momentum.score * weight;
        volume_sum += ctx.volume.score * weight;
        volatility_sum += ctx.volatility.score * weight;

        if ctx.overall_score > 0 {
            positive_tf_count += 1;
        } else if ctx.overall_score < 0 {
            negative_tf_count += 1;
        }

        alignments.push(TfAlignmentInfo {
            timeframe: label.to_string(),
            timeframe_secs: secs,
            trend_score: ctx.trend.score,
            momentum_score: ctx.momentum.score,
            overall_score: ctx.overall_score,
            regime: ctx.regime,
            active_signals: tf_signals,
            price,
        });
    }

    let mtf_trend_alignment = if total_weight > 0.0 {
        (trend_sum / total_weight).clamp(-1.0, 1.0)
    } else { 0.0 };

    let mtf_momentum_alignment = if total_weight > 0.0 {
        (momentum_sum / total_weight).clamp(-1.0, 1.0)
    } else { 0.0 };

    let mtf_volume_alignment = if total_weight > 0.0 {
        (volume_sum / total_weight).clamp(-1.0, 1.0)
    } else { 0.0 };

    let mtf_volatility_alignment = if total_weight > 0.0 {
        (volatility_sum / total_weight).clamp(-1.0, 1.0)
    } else { 0.0 };

    // MTF overall = weighted blend of trend (0.5) + momentum (0.3) + volume (0.1) + volatility (0.1)
    let mtf_blend = mtf_trend_alignment * 0.5
        + mtf_momentum_alignment * 0.3
        + mtf_volume_alignment * 0.1
        + mtf_volatility_alignment * 0.1;
    let mtf_overall_score = (mtf_blend * 100.0).clamp(-100.0, 100.0);
    let mtf_overall_label = AlignmentMatrix::overall_label(mtf_overall_score);

    // Trend agreement: % of TFs with the same directional sign.
    let total_tf = tf_data.len() as f64;
    let agreement = if total_tf > 0.0 {
        let dominant = positive_tf_count.max(negative_tf_count) as f64;
        (dominant / total_tf * 100.0).clamp(0.0, 100.0)
    } else { 0.0 };

    // Cross-TF signal count: signals where same kind+label appears in ≥2 TFs.
    // This is approximated by counting total signals and applying a decay factor
    // based on TF count (proxies the likelihood of multi-TF occurrence).
    let cross_tf_count = if tf_data.len() >= 2 {
        (total_signals as f64 * 0.3).round() as u32
    } else {
        0
    };

    AlignmentMatrix {
        symbol: symbol.to_string(),
        timeframes_present: tf_data.len() as u8,
        mtf_trend_alignment,
        mtf_momentum_alignment,
        mtf_volume_alignment,
        mtf_volatility_alignment,
        mtf_overall_score,
        mtf_overall_label,
        timeframe_alignments: alignments,
        signal_cross_tf_count: cross_tf_count,
        trend_agreement_pct: agreement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_map(rsi: f64, ema: f64, adx: f64, bbwp: f64, rvol: f64) -> HashMap<String, NormalizedIndicatorValue> {
        let mut m = HashMap::new();
        m.insert("rsi".into(), NormalizedIndicatorValue::scalar(50.0 + rsi, rsi / 100.0, "X"));
        m.insert("ema_stack".into(), NormalizedIndicatorValue::scalar(0.0, ema, "X"));
        m.insert("adx".into(), NormalizedIndicatorValue::scalar(adx, 0.0, "X"));
        m.insert("bbwp".into(), NormalizedIndicatorValue::scalar(bbwp, 0.0, "X"));
        m.insert("rvol".into(), NormalizedIndicatorValue::scalar(rvol, 0.0, "X"));
        m
    }

    #[test]
    fn empty_tf_data_returns_no_data() {
        let c = compute_alignment("BTC-USD", &[]);
        assert_eq!(c.timeframes_present, 0);
        assert_eq!(c.mtf_overall_label, "NO_DATA");
    }

    #[test]
    fn single_tf_produces_basic_confluence() {
        let map = build_map(40.0, 0.6, 30.0, 55.0, 1.2);
        let c = compute_alignment("BTC-USD", &[("fast180", 180, 64000.0, &map)]);
        assert_eq!(c.timeframes_present, 1);
        assert_eq!(c.timeframe_alignments.len(), 1);
        assert!(c.trend_agreement_pct == 100.0 || c.trend_agreement_pct == 0.0);
    }

    #[test]
    fn aligned_bullish_mtf_produces_strong_signal() {
        let bull = build_map(70.0, 0.8, 30.0, 60.0, 1.5);
        let c = compute_alignment("BTC-USD", &[
            ("micro60", 60, 64000.0, &bull),
            ("fast180", 180, 64000.0, &bull),
            ("slow300", 300, 64000.0, &bull),
            ("macro900", 900, 64000.0, &bull),
        ]);
        assert_eq!(c.timeframes_present, 4);
        assert!(c.mtf_overall_score > 0.0);
        assert!(c.trend_agreement_pct >= 75.0);
    }

    #[test]
    fn mixed_tf_directions_lowers_agreement() {
        let bull = build_map(70.0, 0.8, 30.0, 60.0, 1.5);
        let bear = build_map(-70.0, -0.8, 30.0, 60.0, 1.5);
        let c = compute_alignment("BTC-USD", &[
            ("micro60", 60, 64000.0, &bull),
            ("fast180", 180, 64000.0, &bull),
            ("slow300", 300, 64000.0, &bear),
            ("macro900", 900, 64000.0, &bear),
        ]);
        assert!(c.mtf_overall_score.abs() < 30.0);
        assert!(c.trend_agreement_pct <= 75.0);
    }
}
