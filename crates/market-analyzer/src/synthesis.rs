use rust_decimal::prelude::ToPrimitive;
use std::sync::Arc;
use tokio::sync::RwLock;

use core_domain::advisory;
use core_domain::alignment::{self, AlignmentMatrix};
use core_domain::analysis::{
    self, AnalysisMatrix, OpportunityProfile, OpportunityType, SetupQuality,
};
use core_domain::indicator_dtos::NormalizedIndicatorValue;
use core_domain::liquidity::{LiquidationClusterMatrix, LiquidityFlow};
use core_domain::market_context::MarketContext;
use core_domain::models::MarketSnapshot;
use core_domain::models::TimeframeSlot;
use core_domain::opportunity::{ConfluentLevel, LevelSource, OpportunityMatrix};
use core_domain::risk::{self, RiskMatrix};
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
        OpportunityType::Breakout
        | OpportunityType::MeanReversion
        | OpportunityType::LiquiditySqueeze
        | OpportunityType::NoClearOpportunity => "INTRADAY",
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
) -> (f64, String, f64, f64) {
    // v6.10 (Phase 2 / B2): align L4's QualityLevel → f64 mapping with the
    // canonical L6 fallback table at
    // `docs/matrices/02-04-decision-matrix.md §2.3` (POOR=20, WEAK=40,
    // AVERAGE=55, GOOD=70, EXCELLENT=100). The previous L4 mapping
    // (10/30/55/80/95) drifted from L6 (20/40/55/70/100) and caused the
    // same QualityLevel value to contribute a different f64 score
    // depending on which layer read it. With this change, the same enum
    // yields the same contribution whether consumed by L4 or L6.
    let q_ctx = match analysis.market_quality {
        analysis::QualityLevel::Excellent => 100.0,
        analysis::QualityLevel::Good => 70.0,
        analysis::QualityLevel::Average => 55.0,
        analysis::QualityLevel::Weak => 40.0,
        analysis::QualityLevel::Poor => 20.0,
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

    let raw = (0.35 * q_ctx + 0.30 * s_sig + 0.20 * a_mtf + 0.15 * f_fresh).clamp(0.0, 100.0);
    // v6.10.1 (bug-fix): `score` is the raw viability blend, NOT gated by
    // the precondition completion ratio. The previous expression
    // `raw * ratio` collapsed every active-setup-but-inactive-condition
    // candidate to score = 0, hiding the operator's view of how close
    // each setup was to firing (every inactive profile card showed
    // `preconditions 0/N met` AND `score 0`). The activation signal is
    // already published separately on every `OpportunityProfile` as
    // `preconditions_met` / `preconditions_total`, and is also surfaced
    // in `scoring_factors.precondition_ratio` (Rust-only, serde-skipped)
    // for telemetry. The dashboard renders this as a dedicated progress
    // bar (`ui/src/components/OpportunitiesPanel.svelte:430-437`).
    let ratio = if preconditions_total > 0 {
        preconditions_met as f64 / preconditions_total as f64
    } else {
        0.0
    };
    // NoClearOpportunity is the unconditional-zero sentinel: it is the
    // explicit "no setup detected" placeholder and can never surface as an
    // actionable trade. Every other opportunity emits the raw viability
    // score so the operator can compare setups head-to-head even when
    // their preconditions are currently unmet.
    let score = if matches!(opportunity_type, OpportunityType::NoClearOpportunity) {
        0.0
    } else {
        raw.clamp(0.0, 100.0)
    };

    // User-facing rationale. Precondition count is displayed separately
    // via the structured `preconditions_met` / `preconditions_total`
    // fields on every profile card — keep the `notes` lean.
    let notes = format!("{:?}", opportunity_type);

    (score, notes, raw, ratio)
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
    let vp = indicators
        .get("volume_profile")
        .and_then(|v| v.values.as_ref());
    let pp = indicators
        .get("pivot_points")
        .and_then(|v| v.values.as_ref());

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
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::Fibonacci,
                                weight: source_weight(LevelSource::Fibonacci),
                            });
                        }
                    }
                }
            }
            if let Some(m) = vp {
                for key in &["vah"] {
                    if let Some(&v) = m.get(*key) {
                        if v > close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::VolumeProfile,
                                weight: source_weight(LevelSource::VolumeProfile),
                            });
                        }
                    }
                }
                for key in &["lvn_0", "lvn_1", "lvn_2"] {
                    if let Some(&v) = m.get(*key) {
                        if v > close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::VolumeProfile,
                                weight: source_weight(LevelSource::VolumeProfile) * 0.8,
                            });
                        }
                    }
                }
            }
            if let Some(m) = pp {
                for key in &["r1", "r2", "r3"] {
                    if let Some(&v) = m.get(*key) {
                        if v > close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::PivotPoints,
                                weight: source_weight(LevelSource::PivotPoints),
                            });
                        }
                    }
                }
            }
            if let Some(c) = cluster {
                for lc in &c.long_clusters {
                    let p = (lc.price_low + lc.price_high) / 2.0;
                    if p > close {
                        candidates.push(LevelCandidate {
                            price: p,
                            source: LevelSource::LiquidityCluster,
                            weight: source_weight(LevelSource::LiquidityCluster)
                                * (lc.notional_usd / c.total_short_oi_usd.max(1.0)).min(1.0),
                        });
                    }
                }
            }
        } else {
            if let Some(m) = fib {
                for key in &["ext_1272", "ext_1618", "ext_2000", "ext_2618"] {
                    if let Some(&v) = m.get(*key) {
                        if v < close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::Fibonacci,
                                weight: source_weight(LevelSource::Fibonacci),
                            });
                        }
                    }
                }
            }
            if let Some(m) = vp {
                for key in &["val"] {
                    if let Some(&v) = m.get(*key) {
                        if v < close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::VolumeProfile,
                                weight: source_weight(LevelSource::VolumeProfile),
                            });
                        }
                    }
                }
                for key in &["lvn_0", "lvn_1", "lvn_2"] {
                    if let Some(&v) = m.get(*key) {
                        if v < close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::VolumeProfile,
                                weight: source_weight(LevelSource::VolumeProfile) * 0.8,
                            });
                        }
                    }
                }
            }
            if let Some(m) = pp {
                for key in &["s1", "s2", "s3"] {
                    if let Some(&v) = m.get(*key) {
                        if v < close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::PivotPoints,
                                weight: source_weight(LevelSource::PivotPoints),
                            });
                        }
                    }
                }
            }
            if let Some(c) = cluster {
                for lc in &c.short_clusters {
                    let p = (lc.price_low + lc.price_high) / 2.0;
                    if p < close {
                        candidates.push(LevelCandidate {
                            price: p,
                            source: LevelSource::LiquidityCluster,
                            weight: source_weight(LevelSource::LiquidityCluster)
                                * (lc.notional_usd / c.total_long_oi_usd.max(1.0)).min(1.0),
                        });
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
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::Fibonacci,
                                weight: source_weight(LevelSource::Fibonacci),
                            });
                        }
                    }
                }
            }
            if let Some(m) = vp {
                for key in &["poc", "val"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v < close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::VolumeProfile,
                                weight: source_weight(LevelSource::VolumeProfile),
                            });
                        }
                    }
                }
            }
            if let Some(m) = pp {
                for key in &["s1", "s2", "s3"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v < close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::PivotPoints,
                                weight: source_weight(LevelSource::PivotPoints),
                            });
                        }
                    }
                }
            }
            if let Some(c) = cluster {
                for lc in &c.short_clusters {
                    let p = (lc.price_low + lc.price_high) / 2.0;
                    if p > 0.0 && p < close {
                        candidates.push(LevelCandidate {
                            price: p,
                            source: LevelSource::LiquidityCluster,
                            weight: source_weight(LevelSource::LiquidityCluster)
                                * (lc.notional_usd / c.total_long_oi_usd.max(1.0)).min(1.0),
                        });
                    }
                }
            }
        } else {
            if let Some(m) = fib {
                for key in &["fib_0382", "fib_0500", "fib_0618", "fib_0660", "fib_0786"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v > close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::Fibonacci,
                                weight: source_weight(LevelSource::Fibonacci),
                            });
                        }
                    }
                }
            }
            if let Some(m) = vp {
                for key in &["poc", "vah"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v > close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::VolumeProfile,
                                weight: source_weight(LevelSource::VolumeProfile),
                            });
                        }
                    }
                }
            }
            if let Some(m) = pp {
                for key in &["r1", "r2", "r3"] {
                    if let Some(&v) = m.get(*key) {
                        if v > 0.0 && v > close {
                            candidates.push(LevelCandidate {
                                price: v,
                                source: LevelSource::PivotPoints,
                                weight: source_weight(LevelSource::PivotPoints),
                            });
                        }
                    }
                }
            }
            if let Some(c) = cluster {
                for lc in &c.long_clusters {
                    let p = (lc.price_low + lc.price_high) / 2.0;
                    if p > 0.0 && p > close {
                        candidates.push(LevelCandidate {
                            price: p,
                            source: LevelSource::LiquidityCluster,
                            weight: source_weight(LevelSource::LiquidityCluster)
                                * (lc.notional_usd / c.total_short_oi_usd.max(1.0)).min(1.0),
                        });
                    }
                }
            }
        }
    }

    candidates
}

fn cluster_levels(candidates: &[LevelCandidate], tolerance: f64) -> Vec<Vec<&LevelCandidate>> {
    if candidates.is_empty() {
        return vec![];
    }
    let mut sorted: Vec<usize> = (0..candidates.len()).collect();
    sorted.sort_by(|&a, &b| {
        candidates[a]
            .price
            .partial_cmp(&candidates[b].price)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

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
    out.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn derive_confluent_zones(
    indicators: &HashMap<String, NormalizedIndicatorValue>,
    cluster: Option<&LiquidationClusterMatrix>,
    close: f64,
    bias_bullish: bool,
) -> (
    Vec<ConfluentLevel>,
    Vec<ConfluentLevel>,
    Vec<ConfluentLevel>,
) {
    let atr = indicators
        .get("atr")
        .and_then(|v| v.values.as_ref())
        .and_then(|vals| vals.get("atr_14").copied())
        .unwrap_or(close * 0.01);
    let tolerance = (atr * 0.2).max(close * 0.001);

    let entry_candidates =
        collect_candidate_levels(indicators, cluster, close, bias_bullish, false);
    let target_candidates =
        collect_candidate_levels(indicators, cluster, close, bias_bullish, true);

    let entry_clusters = cluster_levels(&entry_candidates, tolerance);
    let target_clusters = cluster_levels(&target_candidates, tolerance);

    let mut entry_levels = clusters_to_confluent(entry_clusters);
    let mut target_levels = clusters_to_confluent(target_clusters);

    // ── ATR-based fallback for entry/target ──
    // When every structural source (Fibonacci / Volume Profile / Pivot
    // Points / Liquidation Clusters) fails to produce a candidate for
    // entry or target, the surface goes empty. The Opportunities panel
    // then shows "No confluent levels" — which is technically correct
    // but unhelpful in practice: a healthy market with a clear bias
    // should always surface at least one actionable bracket. We
    // therefore fall back to a single ATR-derived level:
    //
    //   bullish: entry = close − k_entry·ATR, target = close + k_target·ATR
    //   bearish: entry = close + k_entry·ATR, target = close − k_target·ATR
    //
    // Defaults match `OpportunityMatrixConfig::default()` so the
    // config field is the canonical knob (when plumbed through). For
    // now the defaults are hard-coded inline; the workspace-config
    // threading will be added in a follow-up so the panel can be
    // tuned per-workspace without a recompile.
    const FALLBACK_ENABLED: bool = true;
    const K_ENTRY: f64 = 1.5;
    const K_TARGET: f64 = 2.5;

    if FALLBACK_ENABLED {
        if entry_levels.is_empty() && atr > 0.0 {
            let entry_price = if bias_bullish {
                close - K_ENTRY * atr
            } else {
                close + K_ENTRY * atr
            };
            entry_levels.push(ConfluentLevel {
                price: entry_price,
                confluence_count: 1,
                sources: vec![LevelSource::AtrFallback],
                strength: 35.0, // synthetic strength below typical real levels
            });
        }
        if target_levels.is_empty() && atr > 0.0 {
            let target_price = if bias_bullish {
                close + K_TARGET * atr
            } else {
                close - K_TARGET * atr
            };
            target_levels.push(ConfluentLevel {
                price: target_price,
                confluence_count: 1,
                sources: vec![LevelSource::AtrFallback],
                strength: 35.0,
            });
        }
    }

    let invalidation_candidates: Vec<LevelCandidate> = if bias_bullish {
        let mut inval = Vec::new();
        if let Some(v) = indicator_sub_value(indicators, "fibonacci", "fib_0786") {
            if v > 0.0 && v < close {
                inval.push(LevelCandidate {
                    price: v,
                    source: LevelSource::Fibonacci,
                    weight: 0.5,
                });
            }
        }
        if let Some(v) = indicator_sub_value(indicators, "volume_profile", "val") {
            if v > 0.0 && v < close {
                inval.push(LevelCandidate {
                    price: v,
                    source: LevelSource::VolumeProfile,
                    weight: 0.4,
                });
            }
        }
        inval
    } else {
        let mut inval = Vec::new();
        if let Some(v) = indicator_sub_value(indicators, "fibonacci", "fib_0786") {
            if v > 0.0 && v > close {
                inval.push(LevelCandidate {
                    price: v,
                    source: LevelSource::Fibonacci,
                    weight: 0.5,
                });
            }
        }
        if let Some(v) = indicator_sub_value(indicators, "volume_profile", "vah") {
            if v > 0.0 && v > close {
                inval.push(LevelCandidate {
                    price: v,
                    source: LevelSource::VolumeProfile,
                    weight: 0.4,
                });
            }
        }
        inval
    };

    let inval_clusters = cluster_levels(&invalidation_candidates, tolerance);
    let invalidation_levels = clusters_to_confluent(inval_clusters);

    (entry_levels, target_levels, invalidation_levels)
}

/// Build entry/target/invalidation zones for one directional side.
///
/// `bias_long = true` produces a long-oriented bracket (entry below close,
/// target above, invalidation below). `bias_long = false` mirrors that
/// (entry above close, target below, invalidation above). Returns the three
/// zone values together with the per-side confluent level vectors so the
/// matrix-level fields can be sourced from the active side without an extra
/// `derive_confluent_zones` call.
fn derive_side_zones(
    indicators: &HashMap<String, NormalizedIndicatorValue>,
    cluster: Option<&LiquidationClusterMatrix>,
    close: f64,
    atr: f64,
    primary_score: f64,
    bias_long: bool,
) -> (
    core_domain::opportunity::PriceRange,
    core_domain::opportunity::PriceRange,
    f64,
    Vec<ConfluentLevel>,
    Vec<ConfluentLevel>,
    Vec<ConfluentLevel>,
) {
    let (confluent_entry, confluent_target, confluent_inval) =
        derive_confluent_zones(indicators, cluster, close, bias_long);

    let has_confluent_entry = confluent_entry.len() >= 2;
    let has_confluent_target = confluent_target.len() >= 2;
    let has_confluent_inval = !confluent_inval.is_empty();

    // ── Entry zone — side-specific clamp ───────────────────────────────
    // LONG:  zone must sit BELOW close (`high ≤ close`).
    // SHORT: zone must sit ABOVE close (`low ≥ close`).
    // The legacy implementation clamped both bounds to `close` in the
    // same direction (`low = low.min(close); high = high.max(close)`)
    // which produced zones straddling close instead of sitting cleanly
    // on one side. Fix: clamp the bound that touches `close`, then
    // widen the other bound away from `close` by ATR.
    let entry_zone = if has_confluent_entry {
        let prices: Vec<f64> = confluent_entry.iter().map(|c| c.price).collect();
        let raw_low = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let raw_high = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let (low, high) = if bias_long {
            // LONG: high must NOT exceed close; widen low further below.
            let high = raw_high.min(close);
            let low = raw_low.min(high).min(close - atr * 0.1).max(0.0);
            (low, high)
        } else {
            // SHORT: low must NOT go below close; widen high further above.
            let low = raw_low.max(close);
            let high = raw_high.max(low).max(close + atr * 0.1);
            (low, high)
        };
        core_domain::opportunity::PriceRange { low, high }
    } else {
        // ATR fallback — symmetric, side-correct.
        if bias_long {
            core_domain::opportunity::PriceRange {
                low: (close - atr * 0.5).max(0.0),
                high: close,
            }
        } else {
            core_domain::opportunity::PriceRange {
                low: close,
                high: close + atr * 0.5,
            }
        }
    };

    // ── Target zone — side-correct, with min distance from close ────────
    // LONG:  zone must sit ABOVE close (`low ≥ close + δ`).
    // SHORT: zone must sit BELOW close (`high ≤ close − δ`).
    let target_zone = if has_confluent_target {
        let prices: Vec<f64> = confluent_target.iter().map(|c| c.price).collect();
        let raw_low = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let raw_high = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let (low, high) = if bias_long {
            // LONG: low must be above close; widen high further above.
            let low = raw_low.max(close + atr * 0.1);
            let high = raw_high.max(low);
            (low, high)
        } else {
            // SHORT: high must be below close; widen low further below.
            let high = raw_high.min(close - atr * 0.1);
            let low = raw_low.min(high);
            (low, high)
        };
        core_domain::opportunity::PriceRange { low, high }
    } else if bias_long {
        let k = if primary_score >= 70.0 { 2.0 } else { 1.5 };
        core_domain::opportunity::PriceRange {
            low: close + atr * k,
            high: close + atr * (k + 1.0),
        }
    } else {
        let k = if primary_score >= 70.0 { 2.0 } else { 1.5 };
        core_domain::opportunity::PriceRange {
            low: close - atr * (k + 1.0),
            high: close - atr * k,
        }
    };

    // ── Invalidation — MUST sit OUTSIDE the entry zone ────────────────
    // LONG:  inv < entry.low  (a stop above entry.high would be a no-op).
    // SHORT: inv > entry.high.
    // The legacy implementation picked `confluent_inval[0].price`
    // regardless of side, which surfaced the screenshot bug where
    // SL = $63937 sat at entry.low (= $63937).
    let invalidation_level = if has_confluent_inval {
        // Side-prune the candidates: keep only those on the correct
        // side of the entry zone. If none survive, fall through to the
        // ATR fallback below.
        let survivors: Vec<&ConfluentLevel> = confluent_inval
            .iter()
            .filter(|c| {
                if bias_long {
                    c.price < entry_zone.low
                } else {
                    c.price > entry_zone.high
                }
            })
            .collect();
        if let Some(best) = survivors.first() {
            best.price.max(0.0)
        } else if bias_long {
            (entry_zone.low - atr * 0.5).max(0.0)
        } else {
            entry_zone.high + atr * 0.5
        }
    } else if bias_long {
        (entry_zone.low - atr * 0.5).max(0.0)
    } else {
        entry_zone.high + atr * 0.5
    };

    // ── Geometry invariant assertions ────────────────────────────────
    // These debug-only checks prevent silent geometry violations from
    // reaching the frontend. In release builds the values are still
    // clamped by the logic above; these are the safety net.
    #[cfg(debug_assertions)]
    {
        if bias_long {
            // LONG: entry below close, target above close, inval below entry.
            debug_assert!(
                entry_zone.high <= close + atr * 0.01,
                "derive_side_zones (LONG): entry_zone.high {:.2} > close {:.2} + epsilon — entry straddles or sits above close",
                entry_zone.high, close
            );
            debug_assert!(
                target_zone.low >= entry_zone.high,
                "derive_side_zones (LONG): target_zone.low {:.2} < entry_zone.high {:.2} — target below entry",
                target_zone.low, entry_zone.high
            );
            debug_assert!(
                invalidation_level < entry_zone.low,
                "derive_side_zones (LONG): invalidation_level {:.2} >= entry_zone.low {:.2} — SL at or above entry",
                invalidation_level, entry_zone.low
            );
        } else {
            // SHORT: entry above close, target below close, inval above entry.
            debug_assert!(
                entry_zone.low >= close,
                "derive_side_zones (SHORT): entry_zone.low {:.2} < close {:.2} — entry sits below close",
                entry_zone.low, close
            );
            debug_assert!(
                target_zone.high <= entry_zone.low,
                "derive_side_zones (SHORT): target_zone.high {:.2} > entry_zone.low {:.2} — target above entry",
                target_zone.high, entry_zone.low
            );
            debug_assert!(
                invalidation_level > entry_zone.high,
                "derive_side_zones (SHORT): invalidation_level {:.2} <= entry_zone.high {:.2} — SL at or below entry",
                invalidation_level, entry_zone.high
            );
        }
    }

    (
        entry_zone,
        target_zone,
        invalidation_level,
        confluent_entry,
        confluent_target,
        confluent_inval,
    )
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

    // Bug-fix #20: read the canonical signed `mtf_*_alignment` fields
    // and convert to 0-100 here, instead of reading the 0-100 mapped
    // `dimensions[i].score` (which is `from_signed` output and is
    // indistinguishable from the other 0-100 dimensions in the
    // `AlignmentMatrix.dimensions` vector). The L4 opportunity
    // preconditions and the per-candidate score blend now operate on
    // the same scale the L2 emitted, eliminating the historical
    // "trend_dim is in 0-100 but mtf_trend_alignment is signed"
    // asymmetry that caused `OpportunityType::TrendContinuation` to
    // never fire on a perfectly balanced trend (signed = 0, mapped
    // = 50, but legacy 50 is "Neutral" not "Weak Bull").
    let trend_dim = ((alignment.mtf_trend_alignment + 1.0) / 2.0 * 100.0)
        .max(0.0)
        .min(100.0);
    let momentum_dim = ((alignment.mtf_momentum_alignment + 1.0) / 2.0 * 100.0)
        .max(0.0)
        .min(100.0);
    let vol_dim = ((alignment.mtf_volatility_alignment + 1.0) / 2.0 * 100.0)
        .max(0.0)
        .min(100.0);
    let struct_dim = alignment.dimensions.get(4).map(|d| d.score).unwrap_or(50.0);
    let tradability_dim = alignment.dimensions.get(9).map(|d| d.score).unwrap_or(50.0);

    let bbwp = indicators.get("bbwp").map(|v| v.raw_value).unwrap_or(50.0);

    // Divergence detection: the L1.5 signal flow has three label families.
    //   1. RSI/MACD/Stochastic/ChandeMO/MFI/CMF/OBV/Squeeze: the per-indicator
    //      `SeriesDivergence::state` produces `CONFIRMED_BULLISH_DIVERGENCE` /
    //      `CONFIRMED_BEARISH_DIVERGENCE`. The legacy "CONFIRMED + DIVERGENCE"
    //      substring check still matches these.
    //   2. OI-Price divergence: the derivatives-WS path emits
    //      `OI_PRICE_DIVERGENCE` (no "CONFIRMED" prefix). The legacy substring
    //      check would miss this entirely, breaking the L1.5→L4→L6
    //      Reversal flow. We now match any label containing the substring
    //      `DIVERGENCE`, which subsumes both label families.
    let has_confirmed_divergence = indicators.values().any(|v| {
        v.signals
            .iter()
            .any(|s| s.label.contains("DIVERGENCE"))
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
        .map(|lf| {
            matches!(
                lf.cascade_state,
                core_domain::liquidity::CascadeState::Detected
                    | core_domain::liquidity::CascadeState::Sustained
            )
        })
        .unwrap_or(false);
    let cascade_asymmetry = cluster.map(|c| c.cascade_asymmetry).unwrap_or(0.0);
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

    let primary_opportunity = if cascade_active
        && cascade_asymmetry.abs() > 0.3
        && regime_is_expansion_or_transition
    {
        OpportunityType::LiquiditySqueeze
    } else if bbwp >= 70.0 && bbwp < 95.0 && struct_dim >= 70.0 && bias_directional && is_trending {
        OpportunityType::Scalp
    } else if trend_dim >= 75.0 && bias_directional && momentum_not_exhausted {
        OpportunityType::TrendContinuation
    } else if vol_dim >= 70.0 && struct_dim >= 60.0 {
        OpportunityType::Breakout
    } else if has_confirmed_divergence && structure_broken && momentum_exhausted {
        OpportunityType::Reversal
    } else if trend_dim >= 60.0 && momentum_weakening {
        OpportunityType::Pullback
    } else if vol_dim <= 30.0 {
        OpportunityType::MeanReversion
    } else {
        OpportunityType::NoClearOpportunity
    };

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

    // First pass: score every candidate so we can resolve `primary_score`
    // BEFORE deriving zones. The zone helper widens its ATR fallback
    // bracket when `primary_score >= 70.0`, so the value must be in hand
    // before `derive_side_zones` is called. We collect everything we need
    // for the second pass into a Vec.
    let mut scored: Vec<(
        OpportunityType,
        f64,
        String,
        f64,
        f64,
        u32,
        u32,
    )> = Vec::with_capacity(candidates.len());
    for ot in &candidates {
        let (met, total) = match ot {
            OpportunityType::LiquiditySqueeze => (
                if cascade_active
                    && cascade_asymmetry.abs() > 0.3
                    && regime_is_expansion_or_transition
                {
                    3
                } else {
                    0
                },
                3,
            ),
            OpportunityType::Scalp => (
                if bbwp >= 70.0 && bbwp < 95.0 && struct_dim >= 70.0 && bias_directional && is_trending {
                    3
                } else {
                    0
                },
                3,
            ),
            OpportunityType::TrendContinuation => (
                if trend_dim >= 75.0 && bias_directional && momentum_not_exhausted {
                    3
                } else {
                    0
                },
                3,
            ),
            OpportunityType::Breakout => {
                (if vol_dim >= 70.0 && struct_dim >= 60.0 { 2 } else { 0 }, 2)
            }
            OpportunityType::Reversal => (
                if has_confirmed_divergence && structure_broken && momentum_exhausted {
                    3
                } else {
                    0
                },
                3,
            ),
            OpportunityType::Pullback => {
                (if trend_dim >= 60.0 && momentum_weakening { 2 } else { 0 }, 2)
            }
            OpportunityType::MeanReversion => {
                (if vol_dim <= 30.0 && is_range { 2 } else { 0 }, 2)
            }
            OpportunityType::NoClearOpportunity => {
                (if tradability_dim < 30.0 { 1 } else { 0 }, 1)
            }
        };

        let (score, notes, raw_score, precondition_ratio) = compute_candidate_score(
            *ot,
            analysis,
            alignment,
            indicators,
            met as u32,
            total as u32,
        );
        scored.push((
            *ot,
            score,
            notes,
            raw_score,
            precondition_ratio,
            met as u32,
            total as u32,
        ));
    }

    let primary_score = scored
        .iter()
        .find(|(ot, _, _, _, _, _, _)| *ot == primary_opportunity)
        .map(|(_, s, _, _, _, _, _)| *s)
        .unwrap_or(0.0);

    let atr = indicators
        .get("atr")
        .and_then(|v| v.values.as_ref())
        .and_then(|vals| vals.get("atr_14").copied())
        .unwrap_or(close * 0.01);

    let (
        long_entry_zone,
        long_target_zone,
        long_invalidation_level,
        long_conf_entry,
        long_conf_target,
        long_conf_inval,
    ) = derive_side_zones(indicators, cluster, close, atr, primary_score, true);
    let (
        short_entry_zone,
        short_target_zone,
        short_invalidation_level,
        short_conf_entry,
        short_conf_target,
        short_conf_inval,
    ) = derive_side_zones(indicators, cluster, close, atr, primary_score, false);

    // Per-side reward/risk computed with the three-state model
    // (`core_domain::risk_reward::compute_side_rr_v2`) which distinguishes:
    //   Value(f64)  — bracket is geometrically valid
    //   NoValue(r)  — bracket exists but geometry is inverted
    //   Error(msg)  — computation failed (NaN, division by zero)
    // The legacy closure conflated NoValue and Error as `None`.
    use core_domain::risk_reward::{compute_side_rr_v2, SideRrStatus};
    let long_rr_status = compute_side_rr_v2(
        long_entry_zone.low,
        long_entry_zone.high,
        long_target_zone.low,
        long_target_zone.high,
        long_invalidation_level,
        close,
        core_domain::risk_reward::Side::Long,
    );
    let short_rr_status = compute_side_rr_v2(
        short_entry_zone.low,
        short_entry_zone.high,
        short_target_zone.low,
        short_target_zone.high,
        short_invalidation_level,
        close,
        core_domain::risk_reward::Side::Short,
    );

    // Extract the f64 from the three-state result (for backward compat
    // with the per-profile `f64` fields). The trade_viability badge
    // reads the three-state status directly; the per-profile R:R
    // fields carry the numeric value.
    fn rr_value(status: &SideRrStatus) -> Option<f64> {
        match status {
            SideRrStatus::Value(v) => Some(*v),
            _ => None,
        }
    }
    fn rr_is_ok(status: &SideRrStatus) -> bool {
        matches!(status, SideRrStatus::Value(_))
    }
    let long_expected_rr_internal = rr_value(&long_rr_status);
    let short_expected_rr_internal = rr_value(&short_rr_status);

    let long_geometry_consistent = rr_is_ok(&long_rr_status);
    let short_geometry_consistent = rr_is_ok(&short_rr_status);

    // Legacy scalar fields mirror the active side so PME/TAE consumers that
    // read `entry_zone` / `target_zone` / `invalidation_level` see unchanged
    // numbers. The Opportunities tab reads the per-direction siblings
    // (`long_*_zone` / `short_*_zone`) instead.
    let (
        entry_zone,
        target_zone,
        invalidation_level,
        confluent_entry,
        confluent_target,
        confluent_inval,
    ) = if bias_bullish {
        (
            long_entry_zone.clone(),
            long_target_zone.clone(),
            long_invalidation_level,
            long_conf_entry,
            long_conf_target,
            long_conf_inval,
        )
    } else {
        (
            short_entry_zone.clone(),
            short_target_zone.clone(),
            short_invalidation_level,
            short_conf_entry,
            short_conf_target,
            short_conf_inval,
        )
    };

    // `direction_family`: maps the active bias to a structured tag so
    // the frontend `selectProfileSide` can produce directional
    // arrows on profile cards. The per-profile `direction_family`
    // (TrendRiding/CounterTrend/Neutral) is set per-profile below
    // (each OpportunityType maps to one family via `direction_family_for`).
    let matrix_direction_family: Option<core_domain::opportunity::DirectionFamily> =
        if bias_directional {
            Some(core_domain::opportunity::DirectionFamily::TrendRiding)
        } else {
            Some(core_domain::opportunity::DirectionFamily::Neutral)
        };

    let time_horizon = default_time_horizon(primary_opportunity).to_string();

    let forecast_confidence =
        (analysis.state_confidence * (primary_score / 100.0)).clamp(0.0, 1.0);

    // Second pass: build each `OpportunityProfile` from precomputed zones,
    // R:R ratios, and the profile's own `direction_family` (which is a
    // function of `OpportunityType`, not the active bias). The per-profile
    // direction family decides which side's zones (long or short) the
    // profile populates:
    //   - TrendRiding  + bullish bias → LONG zones
    //   - TrendRiding  + bearish bias → SHORT zones
    //   - CounterTrend + bullish bias → SHORT zones (counter to bias)
    //   - CounterTrend + bearish bias → LONG zones
    //   - Neutral      + any bias     → no zones (DirectionalNeutral)
    //   - any family   + neutral bias → no zones (DirectionalNeutral)
    // The audit's bug-fix #1 was: the per-profile
    // `long_expected_rr_internal` / `short_expected_rr_internal` were
    // hardcoded to 0.0, breaking every per-profile card. We now
    // propagate the geometric R:R from the same zones that drive
    // `entry_zone` / `target_zone` / `invalidation_level`.
    for (ot, score, notes, raw_score, precondition_ratio, met, total) in &scored {
        let profile_family = analysis::direction_family_for(*ot);

        // Resolves the per-profile side based on the family + macro bias.
        // The tuple is (long_ez, long_tz, long_inv, long_rr, short_ez,
        // short_tz, short_inv, short_rr). Sides that don't apply carry
        // `None` for zones and 0.0 for R:R.
        let (pf_long_ez, pf_long_tz, pf_long_inv, pf_long_rr, pf_short_ez, pf_short_tz, pf_short_inv, pf_short_rr) =
            match (profile_family, bias_bullish, bias_bearish) {
                (analysis::DirectionFamily::TrendRiding, true, _) => (
                    Some(long_entry_zone.clone()),
                    Some(long_target_zone.clone()),
                    Some(long_invalidation_level),
                    long_expected_rr_internal.unwrap_or(0.0),
                    None,
                    None,
                    None,
                    0.0,
                ),
                (analysis::DirectionFamily::TrendRiding, false, true) => (
                    None,
                    None,
                    None,
                    0.0,
                    Some(short_entry_zone.clone()),
                    Some(short_target_zone.clone()),
                    Some(short_invalidation_level),
                    short_expected_rr_internal.unwrap_or(0.0),
                ),
                (analysis::DirectionFamily::CounterTrend, true, _) => (
                    None,
                    None,
                    None,
                    0.0,
                    Some(short_entry_zone.clone()),
                    Some(short_target_zone.clone()),
                    Some(short_invalidation_level),
                    short_expected_rr_internal.unwrap_or(0.0),
                ),
                (analysis::DirectionFamily::CounterTrend, false, true) => (
                    Some(long_entry_zone.clone()),
                    Some(long_target_zone.clone()),
                    Some(long_invalidation_level),
                    long_expected_rr_internal.unwrap_or(0.0),
                    None,
                    None,
                    None,
                    0.0,
                ),
                (analysis::DirectionFamily::Neutral, _, _)
                | (analysis::DirectionFamily::TrendRiding, _, _)
                | (analysis::DirectionFamily::CounterTrend, _, _) => (
                    None, None, None, 0.0, None, None, None, 0.0,
                ),
            };

        // Per-profile `trade_viability`: only set when the profile is
        // the PRIMARY opportunity. The frontend uses this to highlight
        // actionable setups versus side profiles.
        let trade_viability_at_profile = if *ot == primary_opportunity {
            match (profile_family, bias_bullish, bias_bearish) {
                (analysis::DirectionFamily::Neutral, _, _) => {
                    Some(core_domain::opportunity::TradeViability::DirectionalNeutral)
                }
                (analysis::DirectionFamily::TrendRiding, true, _)
                | (analysis::DirectionFamily::CounterTrend, false, true) => {
                    if rr_is_ok(&long_rr_status) {
                        Some(core_domain::opportunity::TradeViability::Actionable)
                    } else {
                        Some(core_domain::opportunity::TradeViability::GeometryInverted)
                    }
                }
                (analysis::DirectionFamily::TrendRiding, false, true)
                | (analysis::DirectionFamily::CounterTrend, true, _) => {
                    if rr_is_ok(&short_rr_status) {
                        Some(core_domain::opportunity::TradeViability::Actionable)
                    } else {
                        Some(core_domain::opportunity::TradeViability::GeometryInverted)
                    }
                }
                _ => Some(core_domain::opportunity::TradeViability::DirectionalNeutral),
            }
        } else {
            None
        };

        profiles.push(OpportunityProfile {
            opportunity_type: *ot,
            score: *score,
            preconditions_met: *met,
            preconditions_total: *total,
            notes: notes.clone(),
            direction_family: Some(profile_family),
            long_geometry_consistent: pf_long_ez.is_some() && long_geometry_consistent,
            short_geometry_consistent: pf_short_ez.is_some() && short_geometry_consistent,
            long_entry_zone: pf_long_ez,
            long_target_zone: pf_long_tz,
            long_invalidation_level: pf_long_inv,
            long_expected_rr_internal: pf_long_rr,
            short_entry_zone: pf_short_ez,
            short_target_zone: pf_short_tz,
            short_invalidation_level: pf_short_inv,
            short_expected_rr_internal: pf_short_rr,
            trade_viability: trade_viability_at_profile,
            scoring_factors: Some(core_domain::analysis::ScoringFactors {
                raw_score: *raw_score,
                precondition_ratio: *precondition_ratio,
            }),
        });
    }

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
        long_entry_zone,
        long_target_zone,
        long_invalidation_level,
        short_entry_zone,
        short_target_zone,
        short_invalidation_level,
        long_expected_rr_internal: long_expected_rr_internal.unwrap_or(0.0),
        short_expected_rr_internal: short_expected_rr_internal.unwrap_or(0.0),
        time_horizon,
        confluent_entry_levels: confluent_entry,
        confluent_target_levels: confluent_target,
        confluent_invalidation_levels: confluent_inval,
        direction_family: matrix_direction_family,
        long_geometry_consistent,
        short_geometry_consistent,
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
            let price = snap.close.as_ref().and_then(|d| d.to_f64()).unwrap_or(0.0);
            let label = slot_label(snap);
            Some((label_box_to_static(&label), *secs, price, &snap.indicators, ctx))
        })
        .collect();

    let alignment = alignment::compute_alignment(symbol, &tf_data);

    // Build per-key union of indicators across all 4 TFs. The previous
    // implementation took the FIRST non-empty TF's indicator map as the
    // "representative" set, which meant: if TF1 had no Fibonacci / Volume
    // Profile / Pivot Points (e.g. macro still warming up) the confluent
    // level surface stayed empty even though TF3 / TF4 had the data.
    //
    // This per-key merge matches the cross-TF pattern already used by
    // `alignment::compute_alignment` (line 1286 above). Each indicator
    // key is filled from the FIRST TF that has it; subsequent TFs don't
    // overwrite. The "first wins" rule is deterministic and matches the
    // iteration order of `tf_snapshots` (micro, fast, slow, macro —
    // fastest candle first, so a populated faster TF shadows a stale
    // slower TF).
    let mut representative_indicators: HashMap<String, NormalizedIndicatorValue> =
        HashMap::new();
    for (_, snap) in tf_snapshots {
        for (k, v) in &snap.indicators {
            representative_indicators
                .entry(k.clone())
                .or_insert_with(|| v.clone());
        }
    }

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

    let close = tf_snapshots
        .first()
        .and_then(|(_, s)| s.close.as_ref())
        .and_then(|d| d.to_f64())
        .unwrap_or(0.0);

    let risk = risk::compute_risk(
        &analysis.symbol,
        &analysis,
        &representative_indicators,
        liquidity_flow,
        cluster,
        close,
    );

    let opportunity = compute_opportunity(
        &analysis,
        &alignment,
        &representative_indicators,
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

fn slot_label(snap: &MarketSnapshot) -> String {
    match snap.timeframe_slot.unwrap_or(TimeframeSlot::Micro) {
        TimeframeSlot::Micro => "MICRO".to_string(),
        TimeframeSlot::Fast => "FAST".to_string(),
        TimeframeSlot::Slow => "SLOW".to_string(),
        TimeframeSlot::Macro => "MACRO".to_string(),
        TimeframeSlot::Custom { id } => format!("CUSTOM-{}", id),
    }
}

/// Leak a per-call `slot_label` String into a `&'static str`. Acceptable in
/// the small-N synthesis path (≤ 16 timeframes per call) — the leaked
/// memory is bounded by the per-call allocation cost and is reclaimed when
/// the host process exits.
fn label_box_to_static(s: &String) -> &'static str {
    Box::leak(s.clone().into_boxed_str())
}

#[allow(dead_code)]
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
    pub async fn gather_snapshots(&self) -> Vec<(u64, MarketSnapshot)> {
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
    use core_domain::indicator_dtos::NormalizedIndicatorValue;
    use core_domain::market_context::{ContextDimension, MarketContext};
    use core_domain::models::MarketSnapshot;
    use rust_decimal::Decimal;

    fn make_context(
        regime: &str,
        trend_score: f64,
        momentum_score: f64,
        vol_score: f64,
        volm_score: f64,
        overall: i32,
    ) -> MarketContext {
        MarketContext {
            trend: ContextDimension {
                score: trend_score,
                confidence: 0.7,
                label: "WEAK_BULL".into(),
            },
            momentum: ContextDimension {
                score: momentum_score,
                confidence: 0.6,
                label: "WEAK_BULL".into(),
            },
            volatility: ContextDimension {
                score: vol_score,
                confidence: 0.5,
                label: "NORMAL".into(),
            },
            volume: ContextDimension {
                score: volm_score,
                confidence: 0.5,
                label: "NORMAL".into(),
            },
            liquidity: ContextDimension::neutral(),
            regime: regime.to_string(),
            overall_score: overall,
            overall_label: if overall > 20 {
                "BULLISH".into()
            } else if overall < -20 {
                "BEARISH".into()
            } else {
                "NEUTRAL".into()
            },
        }
    }

    fn make_snapshot(secs: u64, price: f64, ctx: MarketContext) -> MarketSnapshot {
        let mut indicators: HashMap<String, NormalizedIndicatorValue> = HashMap::new();
        indicators.insert(
            "rsi".into(),
            NormalizedIndicatorValue::scalar(55.0, 0.5, "NEUTRAL"),
        );
        indicators.insert(
            "adx".into(),
            NormalizedIndicatorValue::scalar(28.0, 0.6, "TRENDING"),
        );
        indicators.insert(
            "rvol".into(),
            NormalizedIndicatorValue::scalar(1.2, 0.3, "NORMAL"),
        );
        indicators.insert(
            "bbwp".into(),
            NormalizedIndicatorValue::scalar(45.0, 0.5, "NORMAL"),
        );
        indicators.insert(
            "zscore".into(),
            NormalizedIndicatorValue::scalar(0.5, 0.2, "NEUTRAL"),
        );
        indicators.insert(
            "support_resistance".into(),
            NormalizedIndicatorValue::scalar(0.0, 0.0, "SUPPORT"),
        );

        let mut atr_values = HashMap::new();
        atr_values.insert("atr_14".into(), price * 0.01);
        indicators.insert(
            "atr".into(),
            NormalizedIndicatorValue {
                raw_value: price * 0.01,
                normalized: 0.0,
                state_label: "NORMAL".into(),
                values: Some(atr_values),
                signals: vec![],
                confidence: 0.5,
            },
        );

        let mut macd_values = HashMap::new();
        macd_values.insert("line".into(), 10.0);
        macd_values.insert("signal".into(), 8.0);
        macd_values.insert("histogram".into(), 2.0);
        indicators.insert(
            "macd".into(),
            NormalizedIndicatorValue {
                raw_value: 2.0,
                normalized: 0.4,
                state_label: "BULLISH".into(),
                values: Some(macd_values),
                signals: vec![],
                confidence: 0.6,
            },
        );

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
        assert_eq!(
            result.advisory.directional_guidance,
            advisory::DirectionalGuidance::Neutral
        );
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
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[
                (60, &snap60),
                (180, &snap180),
                (300, &snap300),
                (900, &snap900),
            ],
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(result.alignment.timeframes_present, 4);
        assert!(result.alignment.mtf_overall_score > 0.0);
        assert!(result.analysis.state_confidence > 0.5);
        assert!(matches!(
            result.analysis.bias,
            analysis::MarketBias::Bullish | analysis::MarketBias::StrongBullish
        ));
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
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[
                (60, &snap60),
                (180, &snap180),
                (300, &snap300),
                (900, &snap900),
            ],
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.alignment.mtf_overall_score.abs() < 40.0);
    }

    #[test]
    fn opportunity_emits_both_directional_zones() {
        let ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);
        let snap60 = make_snapshot(60, 64000.0, ctx.clone());
        let snap180 = make_snapshot(180, 64100.0, ctx.clone());
        let snap300 = make_snapshot(300, 64200.0, ctx.clone());
        let snap900 = make_snapshot(900, 64300.0, ctx.clone());
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[
                (60, &snap60),
                (180, &snap180),
                (300, &snap300),
                (900, &snap900),
            ],
            None,
            None,
            None,
            None,
            None,
        );
        let opp = result.opportunity.expect("opportunity must be emitted");
        assert!(opp.long_entry_zone.high >= opp.long_entry_zone.low);
        assert!(opp.long_target_zone.high >= opp.long_target_zone.low);
        assert!(opp.short_entry_zone.high >= opp.short_entry_zone.low);
        assert!(opp.short_target_zone.high >= opp.short_target_zone.low);
        assert!(opp.long_invalidation_level > 0.0);
        assert!(opp.short_invalidation_level > 0.0);
    }

    #[test]
    fn directional_target_zones_are_geometrically_separated() {
        let ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);
        let snap60 = make_snapshot(60, 64000.0, ctx.clone());
        let snap180 = make_snapshot(180, 64100.0, ctx.clone());
        let snap300 = make_snapshot(300, 64200.0, ctx.clone());
        let snap900 = make_snapshot(900, 64300.0, ctx.clone());
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[
                (60, &snap60),
                (180, &snap180),
                (300, &snap300),
                (900, &snap900),
            ],
            None,
            None,
            None,
            None,
            None,
        );
        let opp = result.opportunity.expect("opportunity must be emitted");
        let long_target_mid =
            (opp.long_target_zone.low + opp.long_target_zone.high) / 2.0;
        let short_target_mid =
            (opp.short_target_zone.low + opp.short_target_zone.high) / 2.0;
        let close = 64000.0;
        assert!(
            long_target_mid >= close,
            "long target mid {long_target_mid} must be >= close {close}"
        );
        assert!(
            short_target_mid <= close,
            "short target mid {short_target_mid} must be <= close {close}"
        );
    }

    #[test]
    fn directional_invalidation_levels_are_geometrically_separated() {
        let ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);
        let snap60 = make_snapshot(60, 64000.0, ctx.clone());
        let snap180 = make_snapshot(180, 64100.0, ctx.clone());
        let snap300 = make_snapshot(300, 64200.0, ctx.clone());
        let snap900 = make_snapshot(900, 64300.0, ctx.clone());
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[
                (60, &snap60),
                (180, &snap180),
                (300, &snap300),
                (900, &snap900),
            ],
            None,
            None,
            None,
            None,
            None,
        );
        let opp = result.opportunity.expect("opportunity must be emitted");
        let close = 64000.0;
        assert!(
            opp.long_invalidation_level < close,
            "long invalidation {} must be < close {close}",
            opp.long_invalidation_level
        );
        assert!(
            opp.short_invalidation_level > close,
            "short invalidation {} must be > close {close}",
            opp.short_invalidation_level
        );
    }

    #[test]
    fn legacy_scalar_fields_mirror_long_side_when_bullish() {
        let ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);
        let snap60 = make_snapshot(60, 64000.0, ctx.clone());
        let snap180 = make_snapshot(180, 64100.0, ctx.clone());
        let snap300 = make_snapshot(300, 64200.0, ctx.clone());
        let snap900 = make_snapshot(900, 64300.0, ctx.clone());
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[
                (60, &snap60),
                (180, &snap180),
                (300, &snap300),
                (900, &snap900),
            ],
            None,
            None,
            None,
            None,
            None,
        );
        let opp = result.opportunity.expect("opportunity must be emitted");
        assert!(matches!(
            result.analysis.bias,
            analysis::MarketBias::Bullish | analysis::MarketBias::StrongBullish
        ));
        assert_eq!(opp.entry_zone.low, opp.long_entry_zone.low);
        assert_eq!(opp.entry_zone.high, opp.long_entry_zone.high);
        assert_eq!(opp.target_zone.low, opp.long_target_zone.low);
        assert_eq!(opp.target_zone.high, opp.long_target_zone.high);
        assert_eq!(opp.invalidation_level, opp.long_invalidation_level);
    }

    #[test]
    fn legacy_scalar_fields_mirror_short_side_when_bearish() {
        let bear_ctx = make_context("TRENDING", -0.7, -0.6, 0.2, 0.1, -60);
        let snap60 = make_snapshot(60, 64000.0, bear_ctx.clone());
        let snap180 = make_snapshot(180, 63900.0, bear_ctx.clone());
        let snap300 = make_snapshot(300, 63800.0, bear_ctx.clone());
        let snap900 = make_snapshot(900, 63700.0, bear_ctx.clone());
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[
                (60, &snap60),
                (180, &snap180),
                (300, &snap300),
                (900, &snap900),
            ],
            None,
            None,
            None,
            None,
            None,
        );
        let opp = result.opportunity.expect("opportunity must be emitted");
        assert!(matches!(
            result.analysis.bias,
            analysis::MarketBias::Bearish | analysis::MarketBias::StrongBearish
        ));
        assert_eq!(opp.entry_zone.low, opp.short_entry_zone.low);
        assert_eq!(opp.entry_zone.high, opp.short_entry_zone.high);
        assert_eq!(opp.target_zone.low, opp.short_target_zone.low);
        assert_eq!(opp.target_zone.high, opp.short_target_zone.high);
            assert_eq!(opp.invalidation_level, opp.short_invalidation_level);
    }

    // ─── v6.10.1 (bug-fix): the four regression-locking tests for the
    // `opportunity_score = raw * ratio` bug — the user observed 5 of 7
    // profiles silently scored 0 whenever preconditions were unmet. These
    // tests lock in:
    //   (1) inactive setups still surface raw viability;
    //   (2) NoClearOpportunity stays the unconditional zero;
    //   (3) `scoring_factors.precondition_ratio` is preserved on the
    //       Rust struct (telemetry consumers can still read the ratio);
    //   (4) `primary_opportunity` selection is unaffected by the fix
    //       (it was already driven by raw preconditions, not by score).

    #[test]
    fn inactive_candidates_survive_precondition_discount() {
        // Mirrors the user's screenshot: BTC +0.78% with a moderate-vol
        // mid-range regime. The four big conditional setups (Trend,
        // Breakout, Reversal, MeanReversion) almost never have all
        // preconditions met on a quiet-volatility regime, but their raw
        // viability must still show through to the dashboard.
        let ctx = make_context("COMPRESSION", 0.55, 0.50, 0.40, 0.45, 25);
        let snap = make_snapshot(60, 64000.0, ctx);
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[(60, &snap), (180, &snap), (300, &snap), (900, &snap)],
            None,
            None,
            None,
            None,
            None,
        );

        let opp = result.opportunity.as_ref().expect("opportunity must be emitted");
        assert!(!opp.profiles.is_empty());

        // Every non-NoClear profile must have a non-zero score now
        // (the previous v6.10 implementation forced every score with
        // 0/N preconditions to 0).
        let opp = result.opportunity.as_ref().expect("opportunity must be emitted");
        for p in &opp.profiles {
            if p.opportunity_type != analysis::OpportunityType::NoClearOpportunity {
                assert!(
                    p.score > 0.0,
                    "inactive profile {:?} has score 0 (raw viability dropped): score={}, raw={:?}",
                    p.opportunity_type,
                    p.score,
                    p.scoring_factors.as_ref().map(|sf| sf.raw_score),
                );
            }
        }
    }

    #[test]
    fn no_clear_opportunity_score_is_unconditional_zero() {
        // NoClearOpportunity is the explicit "no setup detected"
        // placeholder and must stay at score 0 regardless of the fix.
        // It has a single precondition (`tradability_dim < 30.0`); when
        // met, the previous code still emitted score 0. The fix
        // preserves that semantic via the explicit branch above.
        let ctx = make_context("RANGE", 0.30, 0.30, 0.20, 0.20, -5);
        let snap = make_snapshot(60, 64000.0, ctx);
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[(60, &snap), (180, &snap), (300, &snap), (900, &snap)],
            None,
            None,
            None,
            None,
            None,
        );

        let opp = result.opportunity.as_ref().expect("opportunity must be emitted");
        let no_clear = opp
            .profiles
            .iter()
            .find(|p| p.opportunity_type == analysis::OpportunityType::NoClearOpportunity)
            .expect("NoClearOpportunity profile must be present in every OpportunityMatrix");
        assert_eq!(no_clear.score, 0.0, "NoClearOpportunity must stay at score 0");
    }

    #[test]
    fn precondition_ratio_is_preserved_in_scoring_factors() {
        // The fix dropped `raw * ratio` from `score`, but the ratio is
        // still published on the wire via the per-profile
        // `scoring_factors.precondition_ratio` field (serde-skipped per
        // the Rust struct definition, but kept for telemetry consumers
        // that read profiles directly).
        let ctx = make_context("TRENDING", 0.50, 0.50, 0.50, 0.50, 10);
        let snap = make_snapshot(60, 64000.0, ctx);
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[(60, &snap), (180, &snap), (300, &snap), (900, &snap)],
            None,
            None,
            None,
            None,
            None,
        );

        let opp = result.opportunity.as_ref().expect("opportunity must be emitted");
        for p in &opp.profiles {
            let sf = p
                .scoring_factors
                .as_ref()
                .expect("scoring_factors must be present on every profile");
            let expected_ratio = if p.preconditions_total > 0 {
                p.preconditions_met as f64 / p.preconditions_total as f64
            } else {
                0.0
            };
            assert!(
                (sf.precondition_ratio - expected_ratio).abs() < 1e-9,
                "precondition_ratio drifted: {} (expected {})",
                sf.precondition_ratio,
                expected_ratio,
            );
            // raw_score must also still be in [0, 100]
            assert!(
                sf.raw_score >= 0.0 && sf.raw_score <= 100.0,
                "raw_score out of range: {}",
                sf.raw_score,
            );
            // After the fix, score == raw_score for non-NoClear profiles
            if p.opportunity_type != analysis::OpportunityType::NoClearOpportunity {
                assert!(
                    (p.score - sf.raw_score).abs() < 1e-9,
                    "non-NoClear score ({}) must equal raw_score ({}) after fix",
                    p.score,
                    sf.raw_score,
                );
            } else {
                // NoClearOpportunity stays at 0 regardless of raw_score
                assert_eq!(p.score, 0.0);
            }
        }
    }

    #[test]
    fn primary_opportunity_unaffected_by_score_fix() {
        // The fix changed `score` to drop the precondition ratio, but
        // `primary_opportunity` is selected from a separate chain at
        // synthesis.rs:800-819 (raw preconditions, not the score). The
        // primary's reported `opportunity_score` should also be the raw
        // viability, not a discounted value.
        let ctx = make_context("TRENDING", 0.65, 0.60, 0.55, 0.55, 45);
        let snap = make_snapshot(60, 64000.0, ctx);
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[(60, &snap), (180, &snap), (300, &snap), (900, &snap)],
            None,
            None,
            None,
            None,
            None,
        );

        // The headline `opportunity_score` equals the selected primary
        // profile's `score` (synthesis.rs:916-920). After the fix both
        // are equal to the primary profile's raw viability.
        let opp = result.opportunity.as_ref().expect("opportunity must be emitted");
        let primary_type = opp.primary_opportunity;
        let primary_profile = opp
            .profiles
            .iter()
            .find(|p| p.opportunity_type == primary_type)
            .expect("primary_opportunity must be present in profiles");
        assert!(
            (opp.opportunity_score - primary_profile.score).abs() < 1e-9,
            "matrix-level score ({}) must equal primary profile score ({})",
            opp.opportunity_score,
            primary_profile.score,
        );
        // Setup quality derives from opportunity_score via the same
        // private `setup_quality_band` helper, so the matrix-level and
        // primary-profile scores must classify identically.
        assert_eq!(
            opp.setup_quality,
            setup_quality_band(primary_profile.score),
            "matrix-level setup_quality must match primary profile score",
        );
    }

    /// Phase B regression: `representative_indicators` is now a per-key
    /// union across all 4 TFs rather than the first non-empty TF's
    /// snapshot. Build a scenario where TF1 (the first iteration slot)
    /// has no `fibonacci` indicator, but TF4 (last iteration slot) does.
    /// The confluent levels must still populate from TF4's Fibonacci.
    ///
    /// We can't easily null out an indicator on the existing
    /// `make_snapshot` helper, so we hand-build the TF1 snapshot and
    /// reuse `make_snapshot` for the others.
    #[test]
    fn representative_indicators_merges_across_tfs_when_first_tf_lacks_fib() {
        let ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);

        // TF1 snapshot: drop the `fibonacci` indicator entirely.
        let snap1 = {
            let mut s = make_snapshot(60, 64000.0, ctx.clone());
            s.indicators.remove("fibonacci");
            s
        };
        let snap2 = make_snapshot(180, 64100.0, ctx.clone());
        let snap3 = make_snapshot(300, 64200.0, ctx.clone());
        // TF4 keeps Fibonacci (default in make_snapshot).
        let snap4 = make_snapshot(900, 64300.0, ctx);

        let result = synthesize_cross_tf(
            "BTC-USD",
            &[(60, &snap1), (180, &snap2), (300, &snap3), (900, &snap4)],
            None,
            None,
            None,
            None,
            None,
        );
        let opp = result.opportunity.expect("opportunity must be emitted");

        // Before the fix, the per-key merge took the first non-empty
        // TF's indicator map; since TF1 had no fibonacci at all,
        // confluent levels stayed empty. After the fix, the union pulls
        // fibonacci from TF4 (or whichever later TF has it) and the
        // entry/target pools populate.
        assert!(
            !opp.confluent_entry_levels.is_empty()
                || !opp.confluent_target_levels.is_empty(),
            "confluent levels must populate from a later TF even when TF1 \
             lacks fibonacci (got entry={:?}, target={:?})",
            opp.confluent_entry_levels.len(),
            opp.confluent_target_levels.len(),
        );
    }

    /// Phase C regression: when every structural source (Fibonacci /
    /// Volume Profile / Pivot Points / Liquidation Clusters) is empty,
    /// the ATR fallback fires and emits at least one entry / target
    /// level derived from `close ± k·ATR`. The fallback is hard-coded
    /// ON by default (matches `OpportunityMatrixConfig::default()`) and
    /// exists so the Opportunities panel never shows the empty state
    /// for a healthy market.
    #[test]
    fn atr_fallback_fires_when_candidate_pool_is_empty() {
        let ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);

        // Build a snapshot whose indicators have NO fibonacci / Volume
        // Profile / Pivot Points / Liquidation Cluster values that
        // match the entry/target proximity conditions. `make_snapshot`
        // already sets Fibonacci/VP to empty (no values are emitted by
        // `make_snapshot`), and we explicitly clear the support_resistance
        // indicator and pass `None` for the cluster.
        let snap = make_snapshot(60, 64000.0, ctx);
        let result = synthesize_cross_tf(
            "BTC-USD",
            &[(60, &snap)],
            None,
            None,
            None,
            None,
            None,
        );
        let opp = result.opportunity.expect("opportunity must be emitted");

        // ATR fallback must populate at least one entry level.
        assert!(
            !opp.confluent_entry_levels.is_empty(),
            "ATR fallback must emit at least one entry level when candidate pool is empty (got {})",
            opp.confluent_entry_levels.len(),
        );
        // And at least one target level.
        assert!(
            !opp.confluent_target_levels.is_empty(),
            "ATR fallback must emit at least one target level when candidate pool is empty (got {})",
            opp.confluent_target_levels.len(),
        );
        // The fallback levels must be flagged with the AtrFallback
        // source so the dashboard can render them with a distinct
        // visual style (the panel's `sourceColor` already maps
        // `LevelSource::AtrFallback` to its own colour).
        assert!(
            opp.confluent_entry_levels
                .iter()
                .any(|l| l.sources.contains(&LevelSource::AtrFallback)),
            "entry fallback level must carry the AtrFallback source marker"
        );
    }

    /// Phase C pin: the ATR fallback's directionality is correct.
    /// For a bullish bias the fallback entry sits BELOW close and the
    /// fallback target sits ABOVE close. For a bearish bias it's the
    /// mirror.
    #[test]
    fn atr_fallback_levels_respect_bias_directionality() {
        // Bullish context → bias Bullish → entry below close.
        let bull_ctx = make_context("TRENDING", 0.7, 0.6, 0.2, 0.1, 60);
        let bull_snap = make_snapshot(60, 64000.0, bull_ctx);
        let bull_result = synthesize_cross_tf(
            "BTC-USD",
            &[(60, &bull_snap)],
            None,
            None,
            None,
            None,
            None,
        );
        let bull_opp = bull_result.opportunity.expect("bullish opp");
        let close = 64000.0_f64;
        let bull_entry = bull_opp
            .confluent_entry_levels
            .first()
            .expect("bullish fallback entry must be present");
        let bull_target = bull_opp
            .confluent_target_levels
            .first()
            .expect("bullish fallback target must be present");
        assert!(
            bull_entry.price < close,
            "bullish fallback entry {} must be < close {close}",
            bull_entry.price
        );
        assert!(
            bull_target.price > close,
            "bullish fallback target {} must be > close {close}",
            bull_target.price
        );

        // Bearish context → bias Bearish → entry above close.
        let bear_ctx = make_context("TRENDING", -0.7, -0.6, 0.2, 0.1, -60);
        let bear_snap = make_snapshot(60, 64000.0, bear_ctx);
        let bear_result = synthesize_cross_tf(
            "BTC-USD",
            &[(60, &bear_snap)],
            None,
            None,
            None,
            None,
            None,
        );
        let bear_opp = bear_result.opportunity.expect("bearish opp");
        let bear_entry = bear_opp
            .confluent_entry_levels
            .first()
            .expect("bearish fallback entry must be present");
        let bear_target = bear_opp
            .confluent_target_levels
            .first()
            .expect("bearish fallback target must be present");
        assert!(
            bear_entry.price > close,
            "bearish fallback entry {} must be > close {close}",
            bear_entry.price
        );
        assert!(
            bear_target.price < close,
            "bearish fallback target {} must be < close {close}",
            bear_target.price
        );
    }
}
