use rust_decimal::prelude::ToPrimitive;
use std::sync::Arc;
use tokio::sync::RwLock;

use core_domain::advisory;
use core_domain::alignment::{self, AlignmentMatrix};
use core_domain::analysis::{
    self, AnalysisMatrix, OpportunityProfile, OpportunityType, SetupQuality,
};
use core_domain::liquidity::{LiquidationClusterMatrix, LiquidityFlow};
use core_domain::models::MarketSnapshot;
use core_domain::models::TimeframeSlot;
use core_domain::opportunity::{ConfluentLevel, LevelSource, OpportunityMatrix};
use core_domain::risk::{self, RiskMatrix};
use core_domain::indicator_dtos::NormalizedIndicatorValue;
use core_domain::market_context::MarketContext;
use std::collections::HashMap;

pub struct CrossTfSynthesisResult {
    pub alignment: AlignmentMatrix,
    pub analysis: AnalysisMatrix,
    pub opportunity: Option<OpportunityMatrix>,
    pub risk: RiskMatrix,
    pub advisory: advisory::AdvisoryMatrix,
}

fn setup_quality_band(score: f64) -> SetupQuality {
    if score >= 85.0 {
        SetupQuality::Prime
    } else if score >= 70.0 {
        SetupQuality::Strong
    } else if score >= 50.0 {
        SetupQuality::Moderate
    } else if score >= 30.0 {
        SetupQuality::Marginal
    } else {
        SetupQuality::None
    }
}

fn default_time_horizon(ot: OpportunityType) -> &'static str {
    match ot {
        OpportunityType::Scalp => "SCALP",
        OpportunityType::Breakout | OpportunityType::MeanReversion
        | OpportunityType::LiquiditySqueeze | OpportunityType::NoClearOpportunity => "INTRADAY",
        OpportunityType::TrendContinuation | OpportunityType::Pullback => "SWING",
        OpportunityType::Reversal => "POSITION",
    }
}

fn compute_candidate_score(
    opportunity_type: OpportunityType,
    analysis: &AnalysisMatrix,
    alignment: &AlignmentMatrix,
    signals: &HashMap<String, NormalizedIndicatorValue>,
    preconditions_met: u32,
    preconditions_total: u32,
) -> (f64, String) {
    let q_ctx = match analysis.market_quality {
        analysis::QualityLevel::Excellent => 95.0,
        analysis::QualityLevel::Good => 80.0,
        analysis::QualityLevel::Average => 55.0,
        analysis::QualityLevel::Weak => 30.0,
        analysis::QualityLevel::Poor => 10.0,
    };

    let s_sig = {
        let mut total_strength = 0.0;
        let mut count = 0;
        for (_, v) in signals {
            for s in &v.signals {
                total_strength += s.strength.min(1.0);
                count += 1;
            }
        }
        if count > 0 {
            (total_strength / count as f64 * 100.0).min(100.0)
        } else {
            40.0
        }
    };

    let a_mtf = alignment.trend_agreement_pct;

    let f_fresh = {
        let min_age = signals
            .values()
            .flat_map(|v| v.signals.iter())
            .map(|s| s.age_bars)
            .min()
            .unwrap_or(10);
        100.0 * (1.0 - (min_age as f64 / 20.0).min(1.0))
    };

    let score = (0.35 * q_ctx + 0.30 * s_sig + 0.20 * a_mtf + 0.15 * f_fresh).clamp(0.0, 100.0);

    let notes = format!(
        "{:?}: preconditions {}/{}, Q_ctx={:.0} S_sig={:.0} A_mtf={:.0} F_fresh={:.0}",
        opportunity_type, preconditions_met, preconditions_total,
        q_ctx, s_sig, a_mtf, f_fresh
    );

    (score, notes)
}

struct LevelCandidate {
    price: f64,
    source: LevelSource,
    weight: f64,
}

fn indicator_sub_value(
    indicators: &HashMap<String, NormalizedIndicatorValue>,
    key: &str,
    sub: &str,
) -> Option<f64> {
    indicators
        .get(key)
        .and_then(|v| v.values.as_ref())
        .and_then(|m| m.get(sub))
        .copied()
}

fn collect_candidate_levels(
    indicators: &HashMap<String, NormalizedIndicatorValue>,
    cluster: Option<&LiquidationClusterMatrix>,
    close: f64,
    bias_bullish: bool,
    for_target: bool,
) -> Vec<LevelCandidate> {
    let mut candidates: Vec<LevelCandidate> = Vec::new();

    let fib = indicators.get("fibonacci").and_then(|v| v.values.as_ref());
    let vp = indicators.get("volume_profile").and_then(|v| v.values.as_ref());
    let pp = indicators.get("pivot_points").and_then(|v| v.values.as_ref());

    let source_weight = |s: LevelSource| -> f64 {
        match s {
            LevelSource::Fibonacci => 0.25,
            LevelSource::VolumeProfile => 0.30,
            LevelSource::PivotPoints => 0.15,
            LevelSource::SupportResistance => 0.20,
            LevelSource::LiquidityCluster => 0.10,
            LevelSource::AtrFallback => 0.05,
        }
    };

    if for_target {
        if bias_bullish {
            if let Some(m) = fib {
                for key in &["ext_1272", "ext_1618", "ext_2000", "ext_2618"] {
                    if let Some(&v) = m.get(*key) {
                        if v > close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::Fibonacci, weight: source_weight(LevelSource::Fibonacci) });
                        }
                    }
                }
            }
            if let Some(m) = vp {
                for key in &["vah"] {
                    if let Some(&v) = m.get(*key) {
                        if v > close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::VolumeProfile, weight: source_weight(LevelSource::VolumeProfile) });
                        }
                    }
                }
                for key in &["lvn_0", "lvn_1", "lvn_2"] {
                    if let Some(&v) = m.get(*key) {
                        if v > close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::VolumeProfile, weight: source_weight(LevelSource::VolumeProfile) * 0.8 });
                        }
                    }
                }
            }
            if let Some(m) = pp {
                for key in &["r1", "r2", "r3"] {
                    if let Some(&v) = m.get(*key) {
                        if v > close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::PivotPoints, weight: source_weight(LevelSource::PivotPoints) });
                        }
                    }
                }
            }
            if let Some(c) = cluster {
                for lc in &c.long_clusters {
                    let p = (lc.price_low + lc.price_high) / 2.0;
                    if p > close {
                        candidates.push(LevelCandidate { price: p, source: LevelSource::LiquidityCluster, weight: source_weight(LevelSource::LiquidityCluster) * (lc.notional_usd / c.total_short_oi_usd.max(1.0)).min(1.0) });
                    }
                }
            }
        } else {
            if let Some(m) = fib {
                for key in &["ext_1272", "ext_1618", "ext_2000", "ext_2618"] {
                    if let Some(&v) = m.get(*key) {
                        if v < close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::Fibonacci, weight: source_weight(LevelSource::Fibonacci) });
                        }
                    }
                }
            }
            if let Some(m) = vp {
                for key in &["val"] {
                    if let Some(&v) = m.get(*key) {
                        if v < close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::VolumeProfile, weight: source_weight(LevelSource::VolumeProfile) });
                        }
                    }
                }
                for key in &["lvn_0", "lvn_1", "lvn_2"] {
                    if let Some(&v) = m.get(*key) {
                        if v < close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::VolumeProfile, weight: source_weight(LevelSource::VolumeProfile) * 0.8 });
                        }
                    }
                }
            }
            if let Some(m) = pp {
                for key in &["s1", "s2", "s3"] {
                    if let Some(&v) = m.get(*key) {
                        if v < close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::PivotPoints, weight: source_weight(LevelSource::PivotPoints) });
                        }
                    }
                }
            }
            if let Some(c) = cluster {
                for lc in &c.short_clusters {
                    let p = (lc.price_low + lc.price_high) / 2.0;
                    if p < close {
                        candidates.push(LevelCandidate { price: p, source: LevelSource::LiquidityCluster, weight: source_weight(LevelSource::LiquidityCluster) * (lc.notional_usd / c.total_long_oi_usd.max(1.0)).min(1.0) });
                    }
                }
            }
        }
    } else {
        if bias_bullish {
            if let Some(m) = fib {
                for key in &["fib_0382", "fib_0500", "fib_0618", "fib_0660", "fib_0786"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v < close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::Fibonacci, weight: source_weight(LevelSource::Fibonacci) });
                        }
                    }
                }
            }
            if let Some(m) = vp {
                for key in &["poc", "val"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v < close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::VolumeProfile, weight: source_weight(LevelSource::VolumeProfile) });
                        }
                    }
                }
            }
            if let Some(m) = pp {
                for key in &["s1", "s2", "s3"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v < close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::PivotPoints, weight: source_weight(LevelSource::PivotPoints) });
                        }
                    }
                }
            }
            if let Some(c) = cluster {
                for lc in &c.short_clusters {
                    let p = (lc.price_low + lc.price_high) / 2.0;
                    if p > 0.0 && p < close {
                        candidates.push(LevelCandidate { price: p, source: LevelSource::LiquidityCluster, weight: source_weight(LevelSource::LiquidityCluster) * (lc.notional_usd / c.total_long_oi_usd.max(1.0)).min(1.0) });
                    }
                }
            }
        } else {
            if let Some(m) = fib {
                for key in &["fib_0382", "fib_0500", "fib_0618", "fib_0660", "fib_0786"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v > close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::Fibonacci, weight: source_weight(LevelSource::Fibonacci) });
                        }
                    }
                }
            }
            if let Some(m) = vp {
                for key in &["poc", "vah"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v > close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::VolumeProfile, weight: source_weight(LevelSource::VolumeProfile) });
                        }
                    }
                }
            }
            if let Some(m) = pp {
                for key in &["r1", "r2", "r3"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v > close {
                            candidates.push(LevelCandidate { price: v, source: LevelSource::PivotPoints, weight: source_weight(LevelSource::PivotPoints) });
                        }
                    }
                }
            }
            if let Some(c) = cluster {
                for lc in &c.long_clusters {
                    let p = (lc.price_low + lc.price_high) / 2.0;
                    if p > 0.0 && p > close {
                        candidates.push(LevelCandidate { price: p, source: LevelSource::LiquidityCluster, weight: source_weight(LevelSource::LiquidityCluster) * (lc.notional_usd / c.total_short_oi_usd.max(1.0)).min(1.0) });
                    }
                }
            }
        }
    }

    candidates
}

fn cluster_levels(
    candidates: &[LevelCandidate],
    tolerance: f64,
) -> Vec<Vec<&LevelCandidate>> {
    if candidates.is_empty() {
        return vec![];
    }
    let mut sorted: Vec<usize> = (0..candidates.len()).collect();
    sorted.sort_by(|&a, &b| candidates[a].price.partial_cmp(&candidates[b].price).unwrap_or(std::cmp::Ordering::Equal));

    let mut clusters: Vec<Vec<&LevelCandidate>> = Vec::new();
    for &idx in &sorted {
        let cand = &candidates[idx];
        let mut found = false;
        for cluster in &mut clusters {
            let cluster_avg = cluster.iter().map(|c| c.price).sum::<f64>() / cluster.len() as f64;
            if (cand.price - cluster_avg).abs() <= tolerance {
                cluster.push(cand);
                found = true;
                break;
            }
        }
        if !found {
            clusters.push(vec![cand]);
        }
    }
    clusters
}

fn clusters_to_confluent(clusters: Vec<Vec<&LevelCandidate>>) -> Vec<ConfluentLevel> {
    let mut out: Vec<ConfluentLevel> = Vec::new();
    for cluster in &clusters {
        let avg_price = cluster.iter().map(|c| c.price).sum::<f64>() / cluster.len() as f64;
        let mut sources: Vec<LevelSource> = cluster.iter().map(|c| c.source).collect();
        sources.sort_by_key(|s| *s as u8);
        sources.dedup_by_key(|s| *s as u8);
        let confluence_count = sources.len() as u32;
        let total_weight: f64 = cluster.iter().map(|c| c.weight).sum();
        let strength = (total_weight * 100.0).min(100.0);
        out.push(ConfluentLevel {
            price: avg_price,
            confluence_count,
            sources,
            strength,
        });
    }
    out.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn derive_confluent_zones(
    indicators: &HashMap<String, NormalizedIndicatorValue>,
    cluster: Option<&LiquidationClusterMatrix>,
    close: f64,
    bias_bullish: bool,
) -> (Vec<ConfluentLevel>, Vec<ConfluentLevel>, Vec<ConfluentLevel>) {
    let atr = indicators
        .get("atr")
        .and_then(|v| v.values.as_ref())
        .and_then(|vals| vals.get("atr_14").copied())
        .unwrap_or(close * 0.01);
    let tolerance = (atr * 0.2).max(close * 0.001);

    let entry_candidates = collect_candidate_levels(indicators, cluster, close, bias_bullish, false);
    let target_candidates = collect_candidate_levels(indicators, cluster, close, bias_bullish, true);

    let entry_clusters = cluster_levels(&entry_candidates, tolerance);
    let target_clusters = cluster_levels(&target_candidates, tolerance);

    let entry_levels = clusters_to_confluent(entry_clusters);
    let target_levels = clusters_to_confluent(target_clusters);

    let invalidation_candidates: Vec<LevelCandidate> = if bias_bullish {
        let mut inval = Vec::new();
        if let Some(v) = indicator_sub_value(indicators, "fibonacci", "fib_0786") {
            if v > 0.0 && v < close {
                inval.push(LevelCandidate { price: v, source: LevelSource::Fibonacci, weight: 0.5 });
            }
        }
        if let Some(v) = indicator_sub_value(indicators, "volume_profile", "val") {
            if v > 0.0 && v < close {
                inval.push(LevelCandidate { price: v, source: LevelSource::VolumeProfile, weight: 0.4 });
            }
        }
        inval
    } else {
        let mut inval = Vec::new();
        if let Some(v) = indicator_sub_value(indicators, "fibonacci", "fib_0786") {
            if v > 0.0 && v > close {
                inval.push(LevelCandidate { price: v, source: LevelSource::Fibonacci, weight: 0.5 });
            }
        }
        if let Some(v) = indicator_sub_value(indicators, "volume_profile", "vah") {
            if v > 0.0 && v > close {
                inval.push(LevelCandidate { price: v, source: LevelSource::VolumeProfile, weight: 0.4 });
            }
        }
        inval
    };

    let inval_clusters = cluster_levels(&invalidation_candidates, tolerance);
    let invalidation_levels = clusters_to_confluent(inval_clusters);

    (entry_levels, target_levels, invalidation_levels)
}

fn compute_opportunity(
    analysis: &AnalysisMatrix,
    alignment: &AlignmentMatrix,
    indicators: &HashMap<String, NormalizedIndicatorValue>,
    liquidity_flow: Option<&LiquidityFlow>,
    cluster: Option<&LiquidationClusterMatrix>,
    close: f64,
) -> Option<OpportunityMatrix> {
    if analysis.timeframes_considered == 0 {
        return None;
    }

    let trend_dim = alignment.dimensions.get(0).map(|d| d.score).unwrap_or(50.0);
    let momentum_dim = alignment.dimensions.get(1).map(|d| d.score).unwrap_or(50.0);
    let vol_dim = alignment.dimensions.get(3).map(|d| d.score).unwrap_or(50.0);
    let struct_dim = alignment.dimensions.get(4).map(|d| d.score).unwrap_or(50.0);
    let tradability_dim = alignment.dimensions.get(9).map(|d| d.score).unwrap_or(50.0);

    let bbwp = indicators
        .get("bbwp")
        .map(|v| v.raw_value)
        .unwrap_or(50.0);

    let has_confirmed_divergence = indicators.values().any(|v| {
        v.signals
            .iter()
            .any(|s| s.label.contains("CONFIRMED") && s.label.contains("DIVERGENCE"))
    });

    let momentum_exhausted = momentum_dim < 25.0;
    let structure_broken = struct_dim < 40.0;
    let momentum_weakening = matches!(
        analysis.momentum_assessment,
        analysis::MomentumAssessment::Weakening
    );

    let bias_bullish = matches!(
        analysis.bias,
        analysis::MarketBias::Bullish | analysis::MarketBias::StrongBullish
    );
    let bias_bearish = matches!(
        analysis.bias,
        analysis::MarketBias::Bearish | analysis::MarketBias::StrongBearish
    );
    let bias_directional = bias_bullish || bias_bearish;

    let cascade_active = liquidity_flow
        .map(|lf| matches!(lf.cascade_state, core_domain::liquidity::CascadeState::Detected | core_domain::liquidity::CascadeState::Sustained))
        .unwrap_or(false);
    let cascade_asymmetry = cluster
        .map(|c| c.cascade_asymmetry)
        .unwrap_or(0.0);
    let regime_is_expansion_or_transition = matches!(
        analysis.market_regime,
        analysis::MarketRegime::Expansion | analysis::MarketRegime::Transition
    );

    let is_trending = matches!(
        analysis.market_regime,
        analysis::MarketRegime::TrendingBull | analysis::MarketRegime::TrendingBear
    );

    let is_range = matches!(
        analysis.market_regime,
        analysis::MarketRegime::Range | analysis::MarketRegime::Contraction
    );

    let momentum_not_exhausted = !matches!(
        analysis.momentum_assessment,
        analysis::MomentumAssessment::Exhausted | analysis::MomentumAssessment::Reversing
    );

    let mut profiles: Vec<OpportunityProfile> = Vec::new();

    let primary_opportunity;
    let primary_score;

    if cascade_active && cascade_asymmetry.abs() > 0.3 && regime_is_expansion_or_transition {
        primary_opportunity = OpportunityType::LiquiditySqueeze;
    } else if bbwp >= 70.0 && bbwp < 95.0 && struct_dim >= 70.0 && bias_directional && is_trending {
        primary_opportunity = OpportunityType::Scalp;
    } else if trend_dim >= 75.0 && bias_directional && momentum_not_exhausted {
        primary_opportunity = OpportunityType::TrendContinuation;
    } else if vol_dim >= 70.0 && struct_dim >= 60.0 {
        primary_opportunity = OpportunityType::Breakout;
    } else if has_confirmed_divergence && structure_broken && momentum_exhausted {
        primary_opportunity = OpportunityType::Reversal;
    } else if trend_dim >= 60.0 && momentum_weakening {
        primary_opportunity = OpportunityType::Pullback;
    } else if vol_dim <= 30.0 {
        primary_opportunity = OpportunityType::MeanReversion;
    } else if tradability_dim < 30.0 {
        primary_opportunity = OpportunityType::NoClearOpportunity;
    } else {
        primary_opportunity = OpportunityType::NoClearOpportunity;
    }

    let candidates: [OpportunityType; 8] = [
        OpportunityType::LiquiditySqueeze,
        OpportunityType::Scalp,
        OpportunityType::TrendContinuation,
        OpportunityType::Breakout,
        OpportunityType::Reversal,
        OpportunityType::Pullback,
        OpportunityType::MeanReversion,
        OpportunityType::NoClearOpportunity,
    ];

    let mut best_score = 0.0f64;
    for ot in &candidates {
        let (met, total) = match ot {
            OpportunityType::LiquiditySqueeze => (
                if cascade_active && cascade_asymmetry.abs() > 0.3 && regime_is_expansion_or_transition { 3 } else { 0 },
                3,
            ),
            OpportunityType::Scalp => (
                if bbwp >= 70.0 && bbwp < 95.0 && struct_dim >= 70.0 && bias_directional && is_trending { 3 } else { 0 },
                3,
            ),
            OpportunityType::TrendContinuation => (
                if trend_dim >= 75.0 && bias_directional && momentum_not_exhausted { 3 } else { 0 },
                3,
            ),
            OpportunityType::Breakout => (
                if vol_dim >= 70.0 && struct_dim >= 60.0 { 2 } else { 0 },
                2,
            ),
            OpportunityType::Reversal => (
                if has_confirmed_divergence && structure_broken && momentum_exhausted { 3 } else { 0 },
                3,
            ),
            OpportunityType::Pullback => (
                if trend_dim >= 60.0 && momentum_weakening { 2 } else { 0 },
                2,
            ),
            OpportunityType::MeanReversion => (
                if vol_dim <= 30.0 && is_range { 2 } else { 0 },
                2,
            ),
            OpportunityType::NoClearOpportunity => (
                if tradability_dim < 30.0 { 1 } else { 0 },
                1,
            ),
        };

        let (score, notes) = compute_candidate_score(
            *ot, analysis, alignment, indicators,
            met as u32, total as u32,
        );
        if *ot == primary_opportunity {
            best_score = score;
        }
        profiles.push(OpportunityProfile {
            opportunity_type: *ot,
            score,
            preconditions_met: met as u32,
            preconditions_total: total as u32,
            notes,
        });
    }

    primary_score = best_score;

    let atr = indicators
        .get("atr")
        .and_then(|v| v.values.as_ref())
        .and_then(|vals| vals.get("atr_14").copied())
        .unwrap_or(close * 0.01);

    let (confluent_entry, confluent_target, confluent_inval) =
        derive_confluent_zones(indicators, cluster, close, bias_bullish);

    let has_confluent_entry = confluent_entry.len() >= 2;
    let has_confluent_target = confluent_target.len() >= 2;
    let has_confluent_inval = !confluent_inval.is_empty();

    let entry_zone = if has_confluent_entry {
        let prices: Vec<f64> = confluent_entry.iter().map(|c| c.price).collect();
        let low = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let high = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let low = low.min(close).max(0.0);
        let high = high.max(close);
        core_domain::opportunity::PriceRange { low, high }
    } else {
        let entry_low = (close - atr * 0.5).max(0.0);
        let entry_high = close + atr * 0.5;
        core_domain::opportunity::PriceRange { low: entry_low, high: entry_high }
    };

    let target_zone = if has_confluent_target {
        let prices: Vec<f64> = confluent_target.iter().map(|c| c.price).collect();
        let low = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let high = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        core_domain::opportunity::PriceRange { low, high }
    } else {
        let target_low = close + atr * (if primary_score >= 70.0 { 2.0 } else { 1.5 });
        let target_high = close + atr * (if primary_score >= 70.0 { 3.0 } else { 2.0 });
        core_domain::opportunity::PriceRange { low: target_low, high: target_high }
    };

    let invalidation_level = if has_confluent_inval {
        confluent_inval[0].price.max(0.0)
    } else if primary_score >= 70.0 {
        (close - atr * 2.0).max(0.0)
    } else {
        (close - atr * 1.5).max(0.0)
    };

    let expected_rr_internal = if atr > 0.0 {
        let avg_target = (target_zone.low + target_zone.high) / 2.0;
        let entry_mid = (entry_zone.low + entry_zone.high) / 2.0;
        let reward = (avg_target - entry_mid).abs();
        let risk_val = (entry_mid - invalidation_level).abs();
        if risk_val > 0.0 {
            (reward / risk_val).max(0.5).min(5.0)
        } else {
            2.5
        }
    } else {
        2.5
    };

    let time_horizon = default_time_horizon(primary_opportunity).to_string();

    let forecast_confidence = (analysis.state_confidence * (primary_score / 100.0)).clamp(0.0, 1.0);

    let contributing_signals: Vec<String> = indicators
        .values()
        .flat_map(|v| v.signals.iter())
        .filter(|s| s.strength > 0.3)
        .map(|s| s.label.clone())
        .collect();

    let invalidation_note = format!(
        "Close below {:.1} invalidates the {:?} thesis.",
        invalidation_level, primary_opportunity
    );

    Some(OpportunityMatrix {
        symbol: analysis.symbol.clone(),
        primary_opportunity,
        opportunity_score: primary_score,
        setup_quality: setup_quality_band(primary_score),
        profiles,
        forecast_confidence,
        contributing_signals,
        invalidation_note,
        entry_zone,
        target_zone,
        invalidation_level,
        expected_rr_internal,
        time_horizon,
        confluent_entry_levels: confluent_entry,
        confluent_target_levels: confluent_target,
        confluent_invalidation_levels: confluent_inval,
    })
}

pub fn synthesize_cross_tf(
    symbol: &str,
    tf_snapshots: &[(u64, &MarketSnapshot)],
    liquidity_flow: Option<&LiquidityFlow>,
    cluster: Option<&LiquidationClusterMatrix>,
    previous_score: Option<f64>,
    previous_regime: Option<core_domain::analysis::MarketRegime>,
    previous_volume_dim: Option<f64>,
) -> CrossTfSynthesisResult {
    let tf_data: Vec<(
        &str,
        u64,
        f64,
        &HashMap<String, NormalizedIndicatorValue>,
        &MarketContext,
    )> = tf_snapshots
        .iter()
        .filter_map(|(secs, snap)| {
            let ctx = snap.context.as_ref()?;
            let price = snap
                .close
                .as_ref()
                .and_then(|d| d.to_f64())
                .unwrap_or(0.0);
            Some((
                slot_label(snap),
                *secs,
                price,
                &snap.indicators,
                ctx,
            ))
        })
        .collect();

    let alignment = alignment::compute_alignment(symbol, &tf_data);

    let representative_indicators = tf_snapshots
        .iter()
        .find_map(|(_, s)| {
            if !s.indicators.is_empty() {
                Some(&s.indicators)
            } else {
                None
            }
        })
        .unwrap_or(empty_map());

    let bbwp = representative_indicators.get("bbwp").map(|v| v.raw_value);
    let adx = representative_indicators.get("adx").map(|v| v.raw_value);

    let analysis = analysis::derive_analysis(
        &alignment,
        bbwp,
        adx,
        previous_score,
        previous_regime,
        previous_volume_dim,
    );

    let risk = risk::compute_risk(
        &analysis.symbol,
        &analysis,
        representative_indicators,
        liquidity_flow,
        cluster,
    );

    let close = tf_snapshots
        .first()
        .and_then(|(_, s)| s.close.as_ref())
        .and_then(|d| d.to_f64())
        .unwrap_or(0.0);

    let opportunity = compute_opportunity(
        &analysis,
        &alignment,
        representative_indicators,
        liquidity_flow,
        cluster,
        close,
    );

    let advisory = advisory::compute_advisory(&analysis, &risk, opportunity.as_ref(), cluster);

    CrossTfSynthesisResult {
        alignment,
        analysis,
        opportunity,
        risk,
        advisory,
    }
}

fn slot_label(snap: &MarketSnapshot) -> &'static str {
    match snap.timeframe_slot.unwrap_or(TimeframeSlot::Micro) {
        TimeframeSlot::Micro => "MICRO",
        TimeframeSlot::Fast => "FAST",
        TimeframeSlot::Slow => "SLOW",
        TimeframeSlot::Macro => "MACRO",
    }
}

fn empty_map() -> &'static HashMap<String, NormalizedIndicatorValue> {
    static MAP: std::sync::LazyLock<HashMap<String, NormalizedIndicatorValue>> =
        std::sync::LazyLock::new(HashMap::new);
    &MAP
}

pub struct SynthesisContext {
    pub micro_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
    pub fast_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
    pub slow_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
    pub macro_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
}

impl SynthesisContext {
    pub async fn gather_snapshots(
        &self,
    ) -> Vec<(u64, MarketSnapshot)> {
        let mut out = Vec::with_capacity(4);
        if let Some(s) = self.micro_snapshot.read().await.clone() {
            out.push((s.timeframe_secs, s));
        }
        if let Some(s) = self.fast_snapshot.read().await.clone() {
            out.push((s.timeframe_secs, s));
        }
        if let Some(s) = self.slow_snapshot.read().await.clone() {
            out.push((s.timeframe_secs, s));
        }
        if let Some(s) = self.macro_snapshot.read().await.clone() {
            out.push((s.timeframe_secs, s));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::models::MarketSnapshot;
    use core_domain::market_context::{ContextDimension, MarketContext};
    use core_domain::indicator_dtos::NormalizedIndicatorValue;
    use rust_decimal::Decimal;

    fn make_context(regime: &str, trend_score: f64, momentum_score: f64, vol_score: f64, volm_score: f64, overall: i32) -> MarketContext {
        MarketContext {
            trend: ContextDimension { score: trend_score, confidence: 0.7, label: "WEAK_BULL".into() },
            momentum: ContextDimension { score: momentum_score, confidence: 0.6, label: "WEAK_BULL".into() },
            volatility: ContextDimension { score: vol_score, confidence: 0.5, label: "NORMAL".into() },
            volume: ContextDimension { score: volm_score, confidence: 0.5, label: "NORMAL".into() },
            liquidity: ContextDimension::neutral(),
            regime: regime.to_string(),
            overall_score: overall,
            overall_label: if overall > 20 { "BULLISH".into() } else if overall < -20 { "BEARISH".into() } else { "NEUTRAL".into() },
        }
    }

    fn make_snapshot(secs: u64, price: f64, ctx: MarketContext) -> MarketSnapshot {
        let mut indicators: HashMap<String, NormalizedIndicatorValue> = HashMap::new();
        indicators.insert("rsi".into(), NormalizedIndicatorValue::scalar(55.0, 0.5, "NEUTRAL"));
        indicators.insert("adx".into(), NormalizedIndicatorValue::scalar(28.0, 0.6, "TRENDING"));
        indicators.insert("rvol".into(), NormalizedIndicatorValue::scalar(1.2, 0.3, "NORMAL"));
        indicators.insert("bbwp".into(), NormalizedIndicatorValue::scalar(45.0, 0.5, "NORMAL"));
        indicators.insert("zscore".into(), NormalizedIndicatorValue::scalar(0.5, 0.2, "NEUTRAL"));
        indicators.insert("support_resistance".into(), NormalizedIndicatorValue::scalar(0.0, 0.0, "SUPPORT"));

        let mut atr_values = HashMap::new();
        atr_values.insert("atr_14".into(), price * 0.01);
        indicators.insert("atr".into(), NormalizedIndicatorValue {
            raw_value: price * 0.01,
            normalized: 0.0,
            state_label: "NORMAL".into(),
            values: Some(atr_values),
            signals: vec![],
            confidence: 0.5,
        });

        let mut macd_values = HashMap::new();
        macd_values.insert("line".into(), 10.0);
        macd_values.insert("signal".into(), 8.0);
        macd_values.insert("histogram".into(), 2.0);
        indicators.insert("macd".into(), NormalizedIndicatorValue {
            raw_value: 2.0,
            normalized: 0.4,
            state_label: "BULLISH".into(),
            values: Some(macd_values),
            signals: vec![],
            confidence: 0.6,
        });

        let close = Decimal::from_f64_retain(price).unwrap();
        MarketSnapshot {
            timeframe_slot: None,
            exchange: None,
            timeframe_secs: secs,
            timestamp: 0,
            symbol: "BTC-USD".into(),
            is_completed: Some(true),
            mid_price: close,
            bid_price: close,
            ask_price: close,
            bid_size: Some(Decimal::ONE),
            ask_size: Some(Decimal::ONE),
            funding_rate: None,
            open_interest: None,
            oi_delta_1h: None,
            mark_price: None,
            index_price: None,
            mark_index_spread_pct: None,
            prev_day_px: None,
            open: Some(close),
            high: Some(close),
            low: Some(close),
            close: Some(close),
            volume: Some(Decimal::ONE_HUNDRED),
            average_volume: Some(Decimal::ONE_HUNDRED),
            context: Some(ctx),
            decision_context: None,
            statistical_context: None,
            indicators,
            alignment: None,
            risk: None,
            analysis: None,
            advisory: None,
            opportunity: None,
            liquidity_signals: vec![],
            metrics_config: None,
            risk_profile: None,
            liquidity: None,
            cluster: None,
            volume_profile: None,
            quality_envelope: None,
        pipeline_state: core_domain::models::CandlePipelineState::default(),
        indicator_lifecycle: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn synthesize_empty_returns_neutral() {
        let result = synthesize_cross_tf("BTC-USD", &[], None, None, None, None, None);
        assert_eq!(result.alignment.timeframes_present, 0);
        assert_eq!(result.analysis.timeframes_considered, 0);
        assert_eq!(result.advisory.directional_guidance, advisory::DirectionalGuidance::Neutral);
    }

    #[test]
    fn synthesize_single_tf_works() {
        let ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);
        let snap = make_snapshot(60, 64000.0, ctx);
        let result = synthesize_cross_tf("BTC-USD", &[(60, &snap)], None, None, None, None, None);
        assert_eq!(result.alignment.timeframes_present, 1);
        assert_eq!(result.alignment.dimensions.len(), 10);
        assert!(result.analysis.state_confidence <= 0.5);
    }

    #[test]
    fn synthesize_four_tf_aligned_bullish() {
        let ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);
        let snap60 = make_snapshot(60, 64000.0, ctx.clone());
        let snap180 = make_snapshot(180, 64100.0, ctx.clone());
        let snap300 = make_snapshot(300, 64200.0, ctx.clone());
        let snap900 = make_snapshot(900, 64300.0, ctx.clone());
        let result = synthesize_cross_tf("BTC-USD", &[
            (60, &snap60), (180, &snap180), (300, &snap300), (900, &snap900),
        ], None, None, None, None, None);
        assert_eq!(result.alignment.timeframes_present, 4);
        assert!(result.alignment.mtf_overall_score > 0.0);
        assert!(result.analysis.state_confidence > 0.5);
        assert!(matches!(result.analysis.bias, analysis::MarketBias::Bullish | analysis::MarketBias::StrongBullish));
        assert!(result.opportunity.is_some());
    }

    #[test]
    fn synthesize_mixed_tf_is_neutral() {
        let bull_ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);
        let bear_ctx = make_context("TRENDING", -0.7, -0.6, 0.2, 0.1, -60);
        let snap60 = make_snapshot(60, 64000.0, bull_ctx.clone());
        let snap180 = make_snapshot(180, 64100.0, bear_ctx.clone());
        let snap300 = make_snapshot(300, 64200.0, bull_ctx.clone());
        let snap900 = make_snapshot(900, 64300.0, bear_ctx.clone());
        let result = synthesize_cross_tf("BTC-USD", &[
            (60, &snap60), (180, &snap180), (300, &snap300), (900, &snap900),
        ], None, None, None, None, None);
        assert!(result.alignment.mtf_overall_score.abs() < 40.0);
    }
}
