//! # Alignment Matrix — Multi-Timeframe Signal Correlation
//!
//! The Alignment Matrix compares, correlates, and evaluates relationships
//! between multiple independent Metrics Matrices for the same asset across
//! different timeframes. It measures timeframe agreement — not indicator
//! confluence (which lives in each Metrics Matrix's local bias score).
//!
//! Layer: L2.5 in the architecture (Signal Correlation).

use crate::indicator_dtos::NormalizedIndicatorValue;
use crate::market_context::MarketContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Alignment state classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignState {
    StrongBullish,
    Bullish,
    Bearish,
    StrongBearish,
    Neutral,
    Mixed,
    NoData,
}

impl std::fmt::Display for AlignState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlignState::StrongBullish => write!(f, "STRONG_BULLISH"),
            AlignState::Bullish => write!(f, "BULLISH"),
            AlignState::Bearish => write!(f, "BEARISH"),
            AlignState::StrongBearish => write!(f, "STRONG_BEARISH"),
            AlignState::Neutral => write!(f, "NEUTRAL"),
            AlignState::Mixed => write!(f, "MIXED"),
            AlignState::NoData => write!(f, "NO_DATA"),
        }
    }
}

/// One alignment dimension with score, state, and confidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentDimension {
    /// Alignment score 0-100%.
    pub score: f64,
    /// Directional alignment state.
    pub state: AlignState,
    /// Confidence in this alignment measurement 0-100%.
    pub confidence: f64,
}

impl AlignmentDimension {
    fn new(score: f64) -> Self {
        let state = if score == 0.0 {
            AlignState::NoData
        } else if score >= 80.0 {
            AlignState::StrongBullish
        } else if score >= 60.0 {
            AlignState::Bullish
        } else if score <= 20.0 {
            AlignState::StrongBearish
        } else if score <= 40.0 {
            AlignState::Bearish
        } else if score < 60.0 {
            AlignState::Neutral
        } else {
            AlignState::Mixed
        };
        let confidence = (score / 100.0).max(0.0).min(1.0) * 100.0;
        Self {
            score: score.max(0.0).min(100.0),
            state,
            confidence,
        }
    }

    fn from_signed(mean: f64) -> Self {
        let score = ((mean + 1.0) / 2.0 * 100.0).max(0.0).min(100.0);
        let state = if mean > 0.6 {
            AlignState::StrongBullish
        } else if mean > 0.3 {
            AlignState::Bullish
        } else if mean < -0.6 {
            AlignState::StrongBearish
        } else if mean < -0.3 {
            AlignState::Bearish
        } else {
            AlignState::Neutral
        };
        Self {
            score,
            state,
            confidence: mean.abs() * 100.0,
        }
    }
}

/// Alignment breakdown for a single timeframe.
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

/// Cross-timeframe Alignment Matrix for one symbol — 10 dimensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentMatrix {
    pub symbol: String,
    pub timeframes_present: u8,
    /// 10 alignment dimensions (spec order).
    pub dimensions: Vec<AlignmentDimension>,
    // ── Legacy top-level summaries (preserved for backward compat) ──
    pub mtf_trend_alignment: f64,
    pub mtf_momentum_alignment: f64,
    pub mtf_volume_alignment: f64,
    pub mtf_volatility_alignment: f64,
    pub mtf_overall_score: f64,
    pub mtf_overall_label: String,
    /// Effective blend weights applied to `mtf_overall_score`
    /// (`[(key, weight)]` over Trend / Momentum / Volume / Volatility).
    /// Always populated by `compute_alignment` so consumers (the
    /// Alignment export's score_calculation block, docs) mirror the EXACT
    /// formula used. v6.10.16: under thin participation (volume dimension
    /// < 25) the volume weight drops 0.10 → 0.05 and the freed weight
    /// moves to Trend/Momentum (0.50/0.30 → 0.55/0.35) so four aligned
    /// timeframes can no longer be vetoed below threshold by a
    /// low-participation volume read. v6.10.18: keys are the full
    /// dimension names — the legacy "Vt"/"Vm" abbreviations labeled
    /// Volume/Volatility swapped vs. the spec (`V_t` = volatility,
    /// `V_m` = volume, 02-01 §4.2); full-word keys make each weight bind
    /// to exactly one `mtf_*_alignment` field with no ambiguity.
    #[serde(default)]
    pub blend_weights: Vec<(String, f64)>,
    pub timeframe_alignments: Vec<TfAlignmentInfo>,
    pub signal_cross_tf_count: u32,
    pub trend_agreement_pct: f64,
}

impl AlignmentMatrix {
    pub fn empty(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            timeframes_present: 0,
            dimensions: vec![AlignmentDimension::new(0.0); 10],
            mtf_trend_alignment: 0.0,
            mtf_momentum_alignment: 0.0,
            mtf_volume_alignment: 0.0,
            mtf_volatility_alignment: 0.0,
            mtf_overall_score: 0.0,
            mtf_overall_label: "NO_DATA".to_string(),
            blend_weights: Vec::new(),
            timeframe_alignments: Vec::new(),
            signal_cross_tf_count: 0,
            trend_agreement_pct: 0.0,
        }
    }

    fn overall_label(score: f64) -> String {
        // M-1 (v6.10.13): the strong band aligns with the canonical L3
        // `MarketBias` thresholds (±40 strong, ±20 weak — 02-02 §3.1 /
        // `derive_analysis`). The legacy ±60 strong band meant the SAME
        // mtf score (e.g. 45) rendered "WEAK BULL" on the Alignment
        // header while the Analysis header said "STRONG BULLISH".
        if score >= 40.0 {
            "STRONG_BULL_MTF".into()
        } else if score >= 20.0 {
            "WEAK_BULL_MTF".into()
        } else if score <= -40.0 {
            "STRONG_BEAR_MTF".into()
        } else if score <= -20.0 {
            "WEAK_BEAR_MTF".into()
        } else {
            "NEUTRAL_MTF".into()
        }
    }

    /// Compute structure alignment: % of TFs where S/R label agrees.
    fn compute_structure_alignment(
        tf_data: &[&HashMap<String, NormalizedIndicatorValue>],
    ) -> AlignmentDimension {
        let mut bullish = 0u32;
        let mut bearish = 0u32;
        let mut total = 0u32;
        for map in tf_data {
            if let Some(sr) = map.get("support_resistance") {
                total += 1;
                if sr.state_label.contains("DEMAND") || sr.state_label.contains("SUPPORT") {
                    bullish += 1;
                } else if sr.state_label.contains("SUPPLY") || sr.state_label.contains("RESISTANCE")
                {
                    bearish += 1;
                }
            }
        }
        if total == 0 {
            return AlignmentDimension::new(0.0);
        }
        let dominant = bullish.max(bearish) as f64;
        let score = (dominant / total as f64) * 100.0;
        AlignmentDimension::new(score)
    }

    /// Compute signal alignment: % of signals appearing in ≥2 TFs.
    ///
    /// Bug-fix #19: the legacy formula
    /// `signal_cross_tf / tf_count * 33.3` was inconsistent with the
    /// upstream `cross_tf_count = total_signals * 0.3` computation
    /// (line 369). The two formulas lived in different units (one
    /// counts cross-TF signals, the other scales total signals by
    /// 0.3) and the 33.3 multiplier in the score formula meant
    /// "every TF has a cross-TF signal" still produced a 33% score
    /// rather than 100%. We now use the canonical %-of-TFs formula
    /// `signal_cross_tf / tf_count * 100` which matches the trend /
    /// regime / confidence alignment scoring convention and reads
    /// "0% = no cross-TF signals, 100% = every TF has at least one
    /// cross-TF signal".
    fn compute_signal_alignment(signal_cross_tf: u32, tf_count: u32) -> AlignmentDimension {
        if tf_count < 2 {
            return AlignmentDimension::new(0.0);
        }
        let score = (signal_cross_tf as f64 / tf_count as f64 * 100.0).min(100.0);
        AlignmentDimension::new(score)
    }

    /// Compute regime alignment: % of TFs in the same regime.
    fn compute_regime_alignment(regimes: &[String]) -> AlignmentDimension {
        if regimes.is_empty() {
            return AlignmentDimension::new(0.0);
        }
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for r in regimes {
            *counts.entry(r.as_str()).or_insert(0) += 1;
        }
        let dominant = counts.values().max().copied().unwrap_or(0) as f64;
        let score = (dominant / regimes.len() as f64) * 100.0;
        AlignmentDimension::new(score)
    }

    /// Compute confidence alignment: how consistent are per-TF confidence scores.
    fn compute_confidence_alignment(confidences: &[f64]) -> AlignmentDimension {
        if confidences.len() < 2 {
            return AlignmentDimension::new(50.0);
        }
        let mean = confidences.iter().sum::<f64>() / confidences.len() as f64;
        let variance =
            confidences.iter().map(|c| (c - mean).powi(2)).sum::<f64>() / confidences.len() as f64;
        let score = (100.0 - variance.sqrt().min(100.0)).max(0.0);
        AlignmentDimension::new(score)
    }

    /// Compute liquidity alignment: consistency of RVOL across TFs.
    fn compute_liquidity_alignment(
        tf_data: &[&HashMap<String, NormalizedIndicatorValue>],
    ) -> AlignmentDimension {
        let mut rvols: Vec<f64> = Vec::new();
        for map in tf_data {
            if let Some(v) = map.get("rvol").map(|v| v.raw_value) {
                rvols.push(v);
            }
        }
        if rvols.len() < 2 {
            return AlignmentDimension::new(50.0);
        }
        let mean = rvols.iter().sum::<f64>() / rvols.len() as f64;
        let cv = if mean > 0.0 {
            ((rvols.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rvols.len() as f64).sqrt()
                / mean)
                .min(1.0)
        } else {
            1.0
        };
        AlignmentDimension::new((1.0 - cv) * 100.0)
    }

    /// Compute opportunity alignment: % of TFs that see a tradable opportunity
    /// (non-neutral local bias with aligned regime).
    fn compute_opportunity_alignment(ctxs: &[MarketContext]) -> AlignmentDimension {
        if ctxs.is_empty() {
            return AlignmentDimension::new(0.0);
        }
        let opportunities = ctxs
            .iter()
            .filter(|ctx| ctx.overall_score.abs() > 10 && ctx.regime != "COMPRESSION")
            .count() as f64;
        let score = (opportunities / ctxs.len() as f64) * 100.0;
        AlignmentDimension::new(score)
    }
}

/// Build the Alignment Matrix by aggregating indicator maps from multiple
/// timeframes for a single symbol.
///
/// The caller is expected to synthesize a [`MarketContext`] per timeframe
/// (via `market_analyzer::market_context_synth::synthesize_market_context`)
/// and supply it alongside the indicator map. This split keeps
/// `core-domain` free of any registry / indicator dependency.
pub fn compute_alignment(
    symbol: &str,
    tf_data: &[(
        &str,
        u64,
        f64,
        &HashMap<String, NormalizedIndicatorValue>,
        &MarketContext,
    )],
) -> AlignmentMatrix {
    if tf_data.is_empty() {
        return AlignmentMatrix::empty(symbol);
    }

    let mut alignments_vec = Vec::with_capacity(tf_data.len());
    let mut trend_sum = 0.0;
    let mut momentum_sum = 0.0;
    let mut volume_sum = 0.0;
    let mut volatility_sum = 0.0;
    let mut total_weight = 0.0;
    let mut positive_tf_count = 0u32;
    let mut negative_tf_count = 0u32;
    let mut total_signals = 0u32;
    let mut regimes: Vec<String> = Vec::new();
    let mut confidences: Vec<f64> = Vec::new();
    let mut ctxs: Vec<MarketContext> = Vec::new();

    let divisor = tf_data.iter().map(|d| d.1).max().unwrap_or(900) as f64;

    for &(label, secs, price, map, ctx) in tf_data {
        let tf_signals: u32 = map.values().map(|v| v.signals.len() as u32).sum();
        total_signals += tf_signals;

        let weight = (secs as f64 / divisor).max(0.2).min(1.0);
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

        regimes.push(ctx.regime.clone());
        confidences.push(ctx.trend.confidence);
        ctxs.push(ctx.clone());

        alignments_vec.push(TfAlignmentInfo {
            timeframe: label.to_string(),
            timeframe_secs: secs,
            trend_score: ctx.trend.score,
            momentum_score: ctx.momentum.score,
            overall_score: ctx.overall_score,
            regime: ctx.regime.clone(),
            active_signals: tf_signals,
            price,
        });
    }

    let mtf_trend_alignment = if total_weight > 0.0 {
        (trend_sum / total_weight).max(-1.0).min(1.0)
    } else {
        0.0
    };
    let mtf_momentum_alignment = if total_weight > 0.0 {
        (momentum_sum / total_weight).max(-1.0).min(1.0)
    } else {
        0.0
    };
    let mtf_volume_alignment = if total_weight > 0.0 {
        (volume_sum / total_weight).max(-1.0).min(1.0)
    } else {
        0.0
    };
    let mtf_volatility_alignment = if total_weight > 0.0 {
        (volatility_sum / total_weight).max(-1.0).min(1.0)
    } else {
        0.0
    };

    // v6.10.16 (FIX-H2, thin-participation reweight): when the volume
    // dimension reads THIN/VERY_THIN (score < 25) the volume vote is a
    // participation qualifier, not a directional signal — a 10%-weight
    // dimension must not be able to veto four aligned timeframes into
    // NEUTRAL. The effective weights shift the volume weight 0.10 → 0.05
    // with the freed weight re-distributed to Trend/Momentum (0.50/0.30 →
    // 0.55/0.35). The applied weights ride on `blend_weights` so the
    // export's score_calculation mirrors the exact formula (02-01 §blend).
    // v6.10.18: keys are full dimension names ("Volume"/"Volatility") —
    // the legacy "Vt"/"Vm" keys were bound to Volume/Volatility swapped
    // vs. the spec, mislabeling the alignment panel's weight chips.
    let volume_dim = AlignmentDimension::from_signed(mtf_volume_alignment);
    let thin_participation = volume_dim.score < 25.0;
    let (wt, wm, wvol, wvola): (f64, f64, f64, f64) = if thin_participation {
        (0.55, 0.35, 0.05, 0.05)
    } else {
        (0.5, 0.3, 0.1, 0.1)
    };
    let blend_weights: Vec<(String, f64)> = vec![
        ("Trend".into(), wt),
        ("Momentum".into(), wm),
        ("Volume".into(), wvol),
        ("Volatility".into(), wvola),
    ];

    let mtf_blend = mtf_trend_alignment * wt
        + mtf_momentum_alignment * wm
        + mtf_volume_alignment * wvol
        + mtf_volatility_alignment * wvola;
    let mtf_overall_score = (mtf_blend * 100.0).max(-100.0).min(100.0);

    let total_tf = tf_data.len() as f64;
    let agreement = if total_tf > 0.0 {
        (positive_tf_count.max(negative_tf_count) as f64 / total_tf * 100.0)
            .max(0.0)
            .min(100.0)
    } else {
        0.0
    };

    let cross_tf_count = if tf_data.len() >= 2 {
        (total_signals as f64 * 0.3).round() as u32
    } else {
        0
    };

    // ── Compute the 10 alignment dimensions ──
    let tf_maps: Vec<&HashMap<String, NormalizedIndicatorValue>> =
        tf_data.iter().map(|d| d.3).collect();
    let dim_1_trend = AlignmentDimension::from_signed(mtf_trend_alignment);
    let dim_2_momentum = AlignmentDimension::from_signed(mtf_momentum_alignment);
    let dim_4_volatility = AlignmentDimension::from_signed(mtf_volatility_alignment);
    let dim_5_structure = AlignmentMatrix::compute_structure_alignment(&tf_maps);
    let dim_6_signal =
        AlignmentMatrix::compute_signal_alignment(cross_tf_count, tf_data.len() as u32);
    let dim_7_regime = AlignmentMatrix::compute_regime_alignment(&regimes);
    let dim_8_confidence = AlignmentMatrix::compute_confidence_alignment(&confidences);
    let dim_9_liquidity = AlignmentMatrix::compute_liquidity_alignment(&tf_maps);
    let dim_10_opportunity = AlignmentMatrix::compute_opportunity_alignment(&ctxs);

    AlignmentMatrix {
        symbol: symbol.to_string(),
        timeframes_present: tf_data.len() as u8,
        dimensions: vec![
            dim_1_trend,
            dim_2_momentum,
            volume_dim,
            dim_4_volatility,
            dim_5_structure,
            dim_6_signal,
            dim_7_regime,
            dim_8_confidence,
            dim_9_liquidity,
            dim_10_opportunity,
        ],
        mtf_trend_alignment,
        mtf_momentum_alignment,
        mtf_volume_alignment,
        mtf_volatility_alignment,
        mtf_overall_score,
        mtf_overall_label: AlignmentMatrix::overall_label(mtf_overall_score),
        blend_weights,
        timeframe_alignments: alignments_vec,
        signal_cross_tf_count: cross_tf_count,
        trend_agreement_pct: agreement,
    }
}

#[allow(dead_code)]
fn clamp01f(x: f64) -> f64 {
    x.max(0.0).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_context::ContextDimension;

    fn build_map(
        rsi: f64,
        ema: f64,
        adx: f64,
        bbwp: f64,
        rvol: f64,
    ) -> HashMap<String, NormalizedIndicatorValue> {
        let mut m = HashMap::new();
        m.insert(
            "rsi".into(),
            NormalizedIndicatorValue::scalar(50.0 + rsi, rsi / 100.0, "X"),
        );
        m.insert(
            "ema_stack".into(),
            NormalizedIndicatorValue::scalar(0.0, ema, "X"),
        );
        m.insert(
            "adx".into(),
            NormalizedIndicatorValue::scalar(adx, 0.0, "X"),
        );
        m.insert(
            "bbwp".into(),
            NormalizedIndicatorValue::scalar(bbwp, 0.0, "X"),
        );
        m.insert(
            "rvol".into(),
            NormalizedIndicatorValue::scalar(rvol, 0.0, "X"),
        );
        m
    }

    fn empty_ctx() -> MarketContext {
        MarketContext {
            trend: ContextDimension::neutral(),
            momentum: ContextDimension::neutral(),
            volatility: ContextDimension::neutral(),
            volume: ContextDimension::neutral(),
            liquidity: ContextDimension::neutral(),
            regime: "RANGE".to_string(),
            overall_score: 0,
            overall_label: "NEUTRAL".to_string(),
        }
    }

    fn bull_ctx(score: i32) -> MarketContext {
        MarketContext {
            trend: ContextDimension {
                score: 0.7,
                confidence: 0.8,
                label: "STRONG_BULL".into(),
            },
            momentum: ContextDimension {
                score: 0.6,
                confidence: 0.7,
                label: "WEAK_BULL".into(),
            },
            volatility: ContextDimension::neutral(),
            volume: ContextDimension::neutral(),
            liquidity: ContextDimension::neutral(),
            regime: "TRENDING".to_string(),
            overall_score: score,
            overall_label: "STRONG_BULL".to_string(),
        }
    }

    fn bear_ctx(score: i32) -> MarketContext {
        MarketContext {
            trend: ContextDimension {
                score: -0.7,
                confidence: 0.8,
                label: "STRONG_BEAR".into(),
            },
            momentum: ContextDimension {
                score: -0.6,
                confidence: 0.7,
                label: "WEAK_BEAR".into(),
            },
            volatility: ContextDimension::neutral(),
            volume: ContextDimension::neutral(),
            liquidity: ContextDimension::neutral(),
            regime: "TRENDING".to_string(),
            overall_score: score,
            overall_label: "STRONG_BEAR".to_string(),
        }
    }

    #[test]
    fn empty_returns_no_data() {
        let c = compute_alignment("BTC-USD", &[]);
        assert_eq!(c.timeframes_present, 0);
        assert_eq!(c.mtf_overall_label, "NO_DATA");
        assert_eq!(c.dimensions.len(), 10);
    }

    #[test]
    fn single_tf_has_10_dims() {
        let map = build_map(40.0, 0.6, 30.0, 55.0, 1.2);
        let c = compute_alignment("BTC-USD", &[("fast180", 180, 64000.0, &map, &empty_ctx())]);
        assert_eq!(c.timeframes_present, 1);
        assert_eq!(c.dimensions.len(), 10);
    }

    #[test]
    fn aligned_bullish_mtf_positive() {
        let bull = build_map(70.0, 0.8, 30.0, 60.0, 1.5);
        let ctx = bull_ctx(70);
        let c = compute_alignment(
            "BTC-USD",
            &[
                ("micro60", 60, 64000.0, &bull, &ctx),
                ("fast180", 180, 64000.0, &bull, &ctx),
                ("slow300", 300, 64000.0, &bull, &ctx),
                ("macro900", 900, 64000.0, &bull, &ctx),
            ],
        );
        assert_eq!(c.timeframes_present, 4);
        assert!(c.mtf_overall_score > 0.0);
        assert!(c.trend_agreement_pct >= 75.0);
        assert!(
            c.dimensions[0].score > 50.0,
            "trend alignment should be high"
        );
    }

    #[test]
    fn mixed_tf_lowers_agreement() {
        let bull = build_map(70.0, 0.8, 30.0, 60.0, 1.5);
        let bear = build_map(-70.0, -0.8, 30.0, 60.0, 1.5);
        let bull_ctx = bull_ctx(70);
        let bear_ctx = bear_ctx(-70);
        let c = compute_alignment(
            "BTC-USD",
            &[
                ("micro60", 60, 64000.0, &bull, &bull_ctx),
                ("fast180", 180, 64000.0, &bull, &bull_ctx),
                ("slow300", 300, 64000.0, &bear, &bear_ctx),
                ("macro900", 900, 64000.0, &bear, &bear_ctx),
            ],
        );
        assert!(c.mtf_overall_score.abs() < 30.0);
        assert!(c.trend_agreement_pct <= 75.0);
    }

    #[test]
    fn standard_blend_weights_applied_by_default() {
        let bull = build_map(70.0, 0.8, 30.0, 60.0, 1.5);
        let ctx = bull_ctx(70);
        let c = compute_alignment(
            "BTC-USD",
            &[
                ("micro60", 60, 64000.0, &bull, &ctx),
                ("fast180", 180, 64000.0, &bull, &ctx),
                ("slow300", 300, 64000.0, &bull, &ctx),
                ("macro900", 900, 64000.0, &bull, &ctx),
            ],
        );
        assert_eq!(c.blend_weights.len(), 4);
        let w = |k: &str| c.blend_weights.iter().find(|(kk, _)| kk == k).map(|(_, v)| *v).unwrap();
        assert_eq!(w("Trend"), 0.5);
        assert_eq!(w("Momentum"), 0.3);
        assert_eq!(w("Volume"), 0.1);
        assert_eq!(w("Volatility"), 0.1);
        // v6.10.18: keys are full dimension names — no "Vt"/"Vm"
        // abbreviations (legacy keys mislabeled Volume/Volatility).
        assert!(c.blend_weights.iter().all(|(k, _)| {
            k == "Trend" || k == "Momentum" || k == "Volume" || k == "Volatility"
        }));
    }

    #[test]
    fn thin_participation_reweights_volume_down() {
        // v6.10.16 (FIX-H2): THIN volume (dim score < 25) shifts the blend
        // to Trend 0.55 / Momentum 0.35 / Volume 0.05 / Volatility 0.05
        // so a low-participation volume read cannot veto four aligned
        // timeframes into NEUTRAL.
        let bull = build_map(70.0, 0.8, 30.0, 60.0, 1.5);
        let mut ctx = bull_ctx(70);
        ctx.volume = ContextDimension {
            score: -0.95,
            confidence: 0.8,
            label: "THIN".into(),
        };
        let c = compute_alignment(
            "BTC-USD",
            &[
                ("micro60", 60, 64000.0, &bull, &ctx),
                ("fast180", 180, 64000.0, &bull, &ctx),
                ("slow300", 300, 64000.0, &bull, &ctx),
                ("macro900", 900, 64000.0, &bull, &ctx),
            ],
        );
        assert!(c.dimensions[2].score < 25.0, "volume dim must read THIN");
        let w = |k: &str| c.blend_weights.iter().find(|(kk, _)| kk == k).map(|(_, v)| *v).unwrap();
        assert_eq!(w("Trend"), 0.55);
        assert_eq!(w("Momentum"), 0.35);
        assert_eq!(w("Volume"), 0.05);
        assert_eq!(w("Volatility"), 0.05);
        // With the drag reweighted (0.10 → 0.05) the composite reads
        // materially higher than the standard blend would give.
        let standard_blend = c.mtf_trend_alignment * 0.5
            + c.mtf_momentum_alignment * 0.3
            + c.mtf_volume_alignment * 0.1
            + c.mtf_volatility_alignment * 0.1;
        assert!(
            c.mtf_overall_score / 100.0 > standard_blend + 0.04,
            "reweighted composite must beat the standard blend under thin participation"
        );
    }
}


