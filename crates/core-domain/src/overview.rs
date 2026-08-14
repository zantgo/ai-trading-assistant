//! # Overview Matrix — Market Synthesis Layer
//!
//! The Overview Matrix aggregates all Advisory Matrices and instance metadata
//! to provide a unified representation of the observed market environment.
//! It summarizes the collective state of all analyzed assets.
//!
//! Layer: L7 in the architecture (Overview).

use crate::advisory::{AdvisoryMatrix, DirectionalGuidance, StrategyEnvironment};
use crate::alignment::AlignmentMatrix;
use crate::risk::{RiskDimension, RiskLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Global market bias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GlobalBias {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
    Mixed,
}

impl std::fmt::Display for GlobalBias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobalBias::StrongBullish => write!(f, "STRONG_BULLISH"),
            GlobalBias::Bullish => write!(f, "BULLISH"),
            GlobalBias::Neutral => write!(f, "NEUTRAL"),
            GlobalBias::Bearish => write!(f, "BEARISH"),
            GlobalBias::StrongBearish => write!(f, "STRONG_BEARISH"),
            GlobalBias::Mixed => write!(f, "MIXED"),
        }
    }
}

/// Market breadth level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketBreadth {
    VeryWeak,
    Weak,
    Balanced,
    Positive,
    StrongPositive,
    Negative,
    StrongNegative,
}

/// Market synchronization level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SyncLevel {
    HighlySynchronized,
    Synchronized,
    Mixed,
    Fragmented,
    HighlyFragmented,
}

/// Market health level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HealthLevel {
    Poor,
    Weak,
    Neutral,
    Healthy,
    Strong,
}

/// Per-asset ranking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetRank {
    pub symbol: String,
    pub score: f64,
    pub bias: String,
    pub confidence: f64,
    pub regime: String,
    pub risk_level: String,
    /// `AlignmentMatrix.mtf_overall_score` for this symbol ∈ [-100, 100].
    /// `0.0` when no alignment is available for the symbol.
    #[serde(default)]
    pub mtf_score: f64,
    /// `AlignmentMatrix.mtf_overall_label` for this symbol
    /// (`STRONG_BULL_MTF` / `WEAK_BULL_MTF` / `NEUTRAL_MTF` /
    /// `WEAK_BEAR_MTF` / `STRONG_BEAR_MTF` / `NO_DATA`).
    #[serde(default)]
    pub mtf_label: String,
}

/// Risk distribution summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskDistribution {
    pub low_pct: f64,
    pub moderate_pct: f64,
    pub high_pct: f64,
    pub risk_environment: String,
}

/// Overview Matrix — global market synthesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverviewMatrix {
    pub global_market_bias: GlobalBias,
    pub market_breadth: MarketBreadth,
    pub regime_distribution: HashMap<String, f64>,
    pub opportunity_distribution: HashMap<String, u32>,
    pub risk_distribution: RiskDistribution,
    pub asset_ranking: Vec<AssetRank>,
    pub market_synchronization: SyncLevel,
    pub market_health: HealthLevel,
    /// Cross-symbol aggregate cascade risk (mean of all per-symbol
    /// cascade_risk scores, 0..100).
    #[serde(default)]
    pub cascade_risk_index: RiskDimension,
    /// Market-wide danger index consumed by the PME safety veto.
    /// Formula: `0.6 * high_pct + 0.4 * sync_penalty`.
    #[serde(default)]
    pub systemic_risk_score: f64,
    /// Continuous signed breadth percentage ∈ [-100, 100].
    #[serde(default)]
    pub breadth_pct: f64,
    /// True when fewer than 4 of 12 SignalKinds are enabled.
    #[serde(default)]
    pub low_coverage: bool,
    /// Count of assets per `AlignmentMatrix.mtf_overall_label`
    /// (`STRONG_BULL_MTF`, `WEAK_BULL_MTF`, `NEUTRAL_MTF`,
    /// `WEAK_BEAR_MTF`, `STRONG_BEAR_MTF`, `NO_DATA`). `u32` because
    /// an asset can satisfy at most one label. Mirrors the existing
    /// `opportunity_distribution` shape — both are per-type counts,
    /// not partitions, so an asset counts toward exactly one entry.
    #[serde(default)]
    pub alignment_distribution: HashMap<String, u32>,
    /// Mean of all per-symbol `AlignmentMatrix.mtf_overall_score` ∈
    /// `[-100, 100]`. Cross-timeframe counterpart to `breadth_pct`
    /// (which is cross-symbol). `0.0` when no alignments are
    /// available.
    #[serde(default)]
    pub alignment_consensus_index: f64,
    /// Mean of all per-symbol
    /// `AlignmentMatrix.trend_agreement_pct` ∈ `[0, 100]`. Answers
    /// "how well do timeframes within each symbol agree?". Distinct
    /// from `market_synchronization` (which is cross-symbol,
    /// derived from `breadth_pct`). `0.0` when no alignments are
    /// available.
    #[serde(default)]
    pub multi_tf_agreement_pct: f64,
    pub global_summary: String,
    pub instance_count: u32,
    pub active_symbols: Vec<String>,
}

impl OverviewMatrix {
    pub fn empty() -> Self {
        Self {
            global_market_bias: GlobalBias::Neutral,
            market_breadth: MarketBreadth::Balanced,
            regime_distribution: HashMap::new(),
            opportunity_distribution: HashMap::new(),
            risk_distribution: RiskDistribution {
                low_pct: 0.0,
                moderate_pct: 100.0,
                high_pct: 0.0,
                risk_environment: "NO_DATA".into(),
            },
            asset_ranking: Vec::new(),
            market_synchronization: SyncLevel::HighlyFragmented,
            market_health: HealthLevel::Neutral,
            cascade_risk_index: RiskDimension::default(),
            systemic_risk_score: 0.0,
            breadth_pct: 0.0,
            low_coverage: false,
            alignment_distribution: HashMap::new(),
            alignment_consensus_index: 0.0,
            multi_tf_agreement_pct: 0.0,
            global_summary: "No active instances — no market data available.".into(),
            instance_count: 0,
            active_symbols: Vec::new(),
        }
    }
}

/// A minimal instance metadata record.
#[derive(Debug, Clone)]
pub struct InstanceMeta {
    pub symbol: String,
    pub timeframe_secs: u64,
    pub timeframe_label: String,
    pub is_active: bool,
    /// Per-symbol L5 `overall_risk.score` — the canonical aggregate the L7
    /// risk distribution / risk_environment / systemic_risk_score bin on
    /// (L7-A, v6.10.13).
    pub overall_risk: f64,
}

/// Compute the Overview Matrix from all Advisory Matrices, instance
/// metadata, and per-symbol Alignment Matrices. The Alignment
/// Matrices are optional (empty slice is permitted); when omitted,
/// the three new aggregate fields (`alignment_distribution`,
/// `alignment_consensus_index`, `multi_tf_agreement_pct`) default to
/// neutral / empty values while the existing breadth / sync / health
/// aggregates remain populated from the advisories.
pub fn compute_overview(
    advisories: &[AdvisoryMatrix],
    instances: &[InstanceMeta],
    alignments: &[AlignmentMatrix],
) -> OverviewMatrix {
    if advisories.is_empty() && instances.iter().all(|i| !i.is_active) {
        return OverviewMatrix::empty();
    }

    let active_instances: Vec<&InstanceMeta> = instances.iter().filter(|i| i.is_active).collect();
    let instance_count = active_instances.len() as u32;

    // Per-symbol alignment lookup. Built once so the per-asset
    // AssetRank enrichment below is O(n) rather than O(n²).
    let alignments_by_symbol: HashMap<&str, &AlignmentMatrix> =
        alignments.iter().map(|a| (a.symbol.as_str(), a)).collect();

    let mut symbols_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for inst in &active_instances {
        symbols_set.insert(inst.symbol.clone());
    }
    for a in advisories {
        symbols_set.insert(a.symbol.clone());
    }
    let mut active_symbols: Vec<String> = symbols_set.into_iter().collect();
    active_symbols.sort();

    let mut long_count = 0u32;
    let mut short_count = 0u32;
    let mut neutral_count = 0u32;
    for adv in advisories {
        match adv.directional_guidance {
            DirectionalGuidance::StrongLong | DirectionalGuidance::Long => long_count += 1,
            DirectionalGuidance::StrongShort | DirectionalGuidance::Short => short_count += 1,
            _ => neutral_count += 1,
        }
    }
// v6.10 (Phase 6 / F4): `breadth_pct` formula is the canonical spec
        // calculation `(L - S) / (L + S + N) × 100` per
        // `docs/engines/market-monitoring-engine/03-02-08-mme-layer7-overview.md` line 30
        // and `docs/matrices/02-09-overview-matrix.md` line 104. The
        // previous v6.9 implementation used the advance-decline formula
        // `(L - S) / (L + S)` which excluded neutrals from the
        // denominator, producing non-canonical values for mixed-mood
        // markets. The corrected formula includes neutrals in the
        // denominator so that a 50L/0S/50N mix yields +50% (vs the
        // v6.9 result of +100%).
        let total = (long_count + short_count + neutral_count).max(1) as f64;

        // Market breadth
        let breadth_pct = (long_count as f64 - short_count as f64) / total * 100.0;
    let breadth = if breadth_pct > 60.0 {
        MarketBreadth::StrongPositive
    } else if breadth_pct > 20.0 {
        MarketBreadth::Positive
    } else if breadth_pct < -60.0 {
        MarketBreadth::StrongNegative
    } else if breadth_pct < -20.0 {
        MarketBreadth::Negative
    } else if breadth_pct.abs() < 10.0 {
        MarketBreadth::Balanced
    } else if breadth_pct > 0.0 {
        MarketBreadth::Weak
    } else {
        MarketBreadth::VeryWeak
    };

    // Synchronization
    let sync = if breadth_pct.abs() > 75.0 {
        SyncLevel::HighlySynchronized
    } else if breadth_pct.abs() > 50.0 {
        SyncLevel::Synchronized
    } else if breadth_pct.abs() > 25.0 {
        SyncLevel::Mixed
    } else if breadth_pct.abs() > 10.0 {
        SyncLevel::Fragmented
    } else {
        SyncLevel::HighlyFragmented
    };

    // Global bias: priority-ordered per spec §3.1
    let long_pct = long_count as f64 / total;
    let short_pct = short_count as f64 / total;
    let is_synced = matches!(
        sync,
        SyncLevel::HighlySynchronized | SyncLevel::Synchronized
    );

    // Bug-fix #14: removed the `neutral_pct >= 0.6 → GlobalBias::Neutral`
    // branch. It was semantically dead: a 60%+ neutral market is, by
    // construction, also a 0% directional market, and the resulting
    // `Neutral` bias was indistinguishable from a 100% neutral market
    // (which falls through to `Mixed` after the `long_count >
    // short_count` / `short_count > long_count` checks both fail).
    // Removing the branch makes the priority chain consistent: any
    // market where no direction has a clear majority defaults to
    // `Mixed`, and the empty-input case is handled by the early
    // `OverviewMatrix::empty()` return at the top of the function.
    let global_bias = if long_pct >= 0.8 && is_synced {
        GlobalBias::StrongBullish
    } else if short_pct >= 0.8 && is_synced {
        GlobalBias::StrongBearish
    } else if long_pct >= 0.6 {
        GlobalBias::Bullish
    } else if short_pct >= 0.6 {
        GlobalBias::Bearish
    } else if long_count > short_count {
        GlobalBias::Bullish
    } else if short_count > long_count {
        GlobalBias::Bearish
    } else {
        GlobalBias::Mixed
    };

    // Asset ranking per spec §3: score = 0.5 * confidence_assessment + 50.0
    let mut rankings: Vec<AssetRank> = advisories
        .iter()
        .map(|a| {
            let score = 0.5 * a.confidence_assessment + 50.0;
            let risk_level = if a.confidence_assessment >= 70.0 {
                "LOW"
            } else if a.confidence_assessment >= 40.0 {
                "MODERATE"
            } else {
                "HIGH"
            };
            // Mirror the per-symbol AlignmentMatrix onto the
            // AssetRank so REST / export consumers can show
            // multi-timeframe alignment per asset without a second
            // fetch. When no alignment is available for this symbol
            // (cold start, transient snapshot gap, or symbol not in
            // the alignment slice), default to neutral values.
            let (mtf_score, mtf_label) = match alignments_by_symbol.get(a.symbol.as_str()) {
                Some(aln) => (aln.mtf_overall_score, aln.mtf_overall_label.clone()),
                None => (0.0, "NO_DATA".to_string()),
            };
            AssetRank {
                symbol: a.symbol.clone(),
                score,
                bias: format!("{:?}", a.directional_guidance),
                confidence: a.confidence_assessment,
                regime: format!("{:?}", a.strategy_environment),
                risk_level: risk_level.to_string(),
                mtf_score,
                mtf_label,
            }
        })
        .collect();
    rankings.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Regime distribution
    let mut regime_counts: HashMap<String, f64> = HashMap::new();
    for adv in advisories {
        let regime_key = match adv.strategy_environment {
            StrategyEnvironment::TrendFollowing => "TRENDING",
            StrategyEnvironment::Breakout => "EXPANSION",
            StrategyEnvironment::MeanReversion => "RANGE",
            StrategyEnvironment::HighVolatility => "HIGH_VOLATILITY",
            StrategyEnvironment::LowActivity => "LOW_ACTIVITY",
            StrategyEnvironment::Unfavorable => "UNFAVORABLE",
        };
        *regime_counts.entry(regime_key.to_string()).or_insert(0.0) += 1.0;
    }
    let mut regime_distribution: HashMap<String, f64> = HashMap::new();
    for (regime, count) in &regime_counts {
        regime_distribution.insert(regime.clone(), count / total);
    }

    // Opportunity distribution
    let mut opportunity_distribution: HashMap<String, u32> = HashMap::new();
    for adv in advisories {
        let opp_key = format!("{:?}", adv.opportunity_classification);
        *opportunity_distribution.entry(opp_key).or_insert(0) += 1;
    }

    // Risk distribution. L7-A (v6.10.13): bins per-asset L5 OVERALL risk
    // (`overall_risk.score` from the active instances) — the canonical
    // aggregate the dashboard's RiskDistributionCard uses. The previous
    // implementation binned on `advisory.cascade_risk_score` (chosen only
    // because it was the single risk scalar the signature carried), which
    // made the L7 export disagree with the dashboard card for the same
    // labelled split; the 02-09 doc's confidence-based definition was
    // conceptually wrong (confidence ≠ risk). Bands: ≤30 low, ≥70 high,
    // else moderate — missing symbols fall back to 50 (moderate), the
    // same default the dashboard card uses.
    let overall_by_symbol: HashMap<&str, f64> = active_instances
        .iter()
        .map(|i| (i.symbol.as_str(), i.overall_risk))
        .collect();
    let overall_for = |symbol: &str| overall_by_symbol.get(symbol).copied().unwrap_or(50.0);
    let total_adv = advisories.len().max(1) as f64;
    let low_risk = advisories
        .iter()
        .filter(|a| overall_for(&a.symbol) <= 30.0)
        .count() as f64
        / total_adv
        * 100.0;
    let high_pct = advisories
        .iter()
        .filter(|a| overall_for(&a.symbol) >= 70.0)
        .count() as f64
        / total_adv
        * 100.0;

    // risk_environment per spec §2.3: NO_DATA → HIGH_RISK (≥50) → MODERATE (≥25) → LOW_RISK
    let risk_environment = if instance_count == 0 {
        "NO_DATA"
    } else if high_pct >= 50.0 {
        "HIGH_RISK"
    } else if high_pct >= 25.0 {
        "MODERATE"
    } else {
        "LOW_RISK"
    };

    // systemic_risk_score = 0.6 * high_pct + 0.4 * sync_penalty.
    // Bug-fix #10: the legacy `sync_penalty` was only computed when
    // `global_bias` was Bearish or StrongBearish — a synchronized bull
    // market produced `sync_penalty = 0.0`, understating the systemic
    // risk for a perfectly correlated long-everything environment.
    // We now apply the sync penalty for any directional bias
    // (StrongBullish, Bullish, Neutral, Bearish, StrongBearish) with
    // a magnitude that scales with directional strength. A
    // StrongBullish synchronized market is just as risky (correlated
    // longs unwind together on the next catalyst) as a StrongBearish
    // one.
    // v6.10 (Phase 6 / F5): `sync_penalty` is restricted to BEARISH /
    // STRONG_BEARISH global market biases per the spec table at
    // `docs/matrices/02-09-overview-matrix.md` lines 165-170. The previous
    // implementation applied a `directional_intensity` multiplier (0/0.3/0.7/1.0)
    // to ALL directional biases; the canonical spec restricts the penalty
    // to bearish regimes only. For Bullish / StrongBullish / Mixed /
    // Neutral we emit `sync_penalty = 0` regardless of sync level; the
    // `high_pct` term still contributes to `SystemicRisk`.
    let directional_intensity: f64 = match global_bias {
        GlobalBias::StrongBearish | GlobalBias::Bearish => 1.0,
        _ => 0.0,
    };
    let sync_penalty = directional_intensity
        * match sync {
            SyncLevel::HighlySynchronized => 100.0,
            SyncLevel::Synchronized => 60.0,
            SyncLevel::Mixed => 30.0,
            SyncLevel::Fragmented => 10.0,
            SyncLevel::HighlyFragmented => 0.0,
        };
    let systemic_risk_score = 0.6 * high_pct + 0.4 * sync_penalty;

    // market_health per spec §3.4: POOR when HIGH_RISK, then bias-based
    let health = if risk_environment == "HIGH_RISK" {
        HealthLevel::Poor
    } else {
        match global_bias {
            GlobalBias::StrongBullish | GlobalBias::StrongBearish => HealthLevel::Strong,
            GlobalBias::Bullish | GlobalBias::Bearish => HealthLevel::Healthy,
            GlobalBias::Mixed => HealthLevel::Weak,
            _ => HealthLevel::Neutral,
        }
    };

    // cascade_risk_index: mean of per-symbol cascade risk scores from AdvisoryMatrix
    let mut cascade_total = 0.0;
    let mut cascade_count = 0;
    for adv in advisories {
        cascade_total += adv.cascade_risk_score;
        cascade_count += 1;
    }
    let cascade_score = if cascade_count > 0 {
        cascade_total / cascade_count as f64
    } else {
        50.0
    };
    let cascade_risk = RiskDimension {
        score: cascade_score,
        level: if cascade_score >= 80.0 {
            RiskLevel::Extreme
        } else if cascade_score >= 60.0 {
            RiskLevel::High
        } else if cascade_score >= 40.0 {
            RiskLevel::Moderate
        } else if cascade_score >= 20.0 {
            RiskLevel::Low
        } else {
            RiskLevel::VeryLow
        },
        // L7-B (v6.10.13): the fraction must scale to 0-100 (`×100`) —
        // the legacy `count / total.min(1)` yielded ≈1% for every sample.
        confidence: (cascade_count as f64 / total_adv * 100.0).min(100.0),
        ..RiskDimension::default()
    };

    // ── Alignment aggregation ─────────────────────────────────
    // `alignment_distribution` — count of assets per
    // `mtf_overall_label`. Mirrors `opportunity_distribution` shape
    // (HashMap<String, u32>); unlike `regime_distribution` (which is
    // a partition that sums to 1.0), an asset satisfies exactly
    // one label, so this is a count, not a fraction.
    let mut alignment_distribution: HashMap<String, u32> = HashMap::new();
    let mut alignment_score_sum = 0.0_f64;
    let mut trend_agreement_sum = 0.0_f64;
    let alignment_count = alignments.len();
    for aln in alignments {
        *alignment_distribution
            .entry(aln.mtf_overall_label.clone())
            .or_insert(0) += 1;
        alignment_score_sum += aln.mtf_overall_score;
        trend_agreement_sum += aln.trend_agreement_pct;
    }
    let (alignment_consensus_index, multi_tf_agreement_pct) = if alignment_count > 0 {
        let n = alignment_count as f64;
        (alignment_score_sum / n, trend_agreement_sum / n)
    } else {
        (0.0, 0.0)
    };

    let summary = format!(
        "{} active instances across {} symbols. Global bias: {} with {} market breadth. Risk environment: {}.",
        instance_count,
        active_symbols.len(),
        global_bias,
        match breadth {
            MarketBreadth::StrongPositive | MarketBreadth::Positive => "positive",
            MarketBreadth::StrongNegative | MarketBreadth::Negative => "negative",
            _ => "balanced",
        },
        risk_environment
    );

    OverviewMatrix {
        global_market_bias: global_bias,
        market_breadth: breadth,
        regime_distribution,
        opportunity_distribution,
        risk_distribution: RiskDistribution {
            low_pct: low_risk.round(),
            moderate_pct: (100.0 - low_risk - high_pct).max(0.0).round(),
            high_pct: high_pct.round(),
            risk_environment: risk_environment.to_string(),
        },
        asset_ranking: rankings,
        market_synchronization: sync,
        market_health: health,
        cascade_risk_index: cascade_risk,
        systemic_risk_score,
        breadth_pct,
        // Bug-fix #15: `low_coverage` was hardcoded to `false` on every
        // call, so the dashboard never knew when the overview was
        // computed on a thin sample. The frontend used the flag to
        // dim global aggregates that are statistically unreliable
        // below 3 active symbols. The threshold of 3 matches the
        // §3.5 "minimum coverage for breadth / sync / health
        // aggregation" rule in the L7 spec; below this, market_breadth
        // is just a count of 1-2 advisories and market_synchronization
        // is undefined for n < 3.
        low_coverage: active_symbols.len() < 3,
        global_summary: summary,
        instance_count,
        active_symbols,
        alignment_distribution,
        alignment_consensus_index,
        multi_tf_agreement_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_default() {
        let o = compute_overview(&[], &[], &[]);
        // Empty input → OverviewMatrix::empty() early-return path.
        // The empty OverviewMatrix declares GlobalBias::Neutral as its
        // canonical default; the live `compute_overview` path now
        // returns Mixed (after the dead-branch removal) for the
        // empty-advisories case, so we exercise the early-return by
        // passing zero advisories AND zero active instances.
        assert!(matches!(o.global_market_bias, GlobalBias::Neutral));
        assert_eq!(o.instance_count, 0);
        assert_eq!(o.systemic_risk_score, 0.0);
        // New aggregate alignment fields default to neutral values.
        assert!(o.alignment_distribution.is_empty());
        assert_eq!(o.alignment_consensus_index, 0.0);
        assert_eq!(o.multi_tf_agreement_pct, 0.0);
    }

    #[test]
    fn all_neutral_advisories_resolve_to_mixed() {
        // Bug-fix #14: removed the `neutral_pct >= 0.6 → Neutral`
        // branch. A market with 100% neutral advisories now falls
        // through to `Mixed` (after `long_count > short_count` and
        // `short_count > long_count` both fail). The empty-input
        // case is still `Neutral` via the early-return default.
        let advs: Vec<AdvisoryMatrix> = (0..5)
            .map(|i| AdvisoryMatrix {
                symbol: format!("X-USD{}", i),
                ..AdvisoryMatrix::empty(&format!("X-USD{}", i))
            })
            .collect();
        let o = compute_overview(&advs, &[], &[]);
        assert!(matches!(o.global_market_bias, GlobalBias::Mixed));
    }

    #[test]
    fn single_bullish_adv_produces_bullish_overview() {
        let adv = AdvisoryMatrix {
            symbol: "BTC-USD".into(),
            directional_guidance: DirectionalGuidance::Long,
            confidence_assessment: 75.0,
            ..AdvisoryMatrix::empty("BTC-USD")
        };
        let instances = vec![InstanceMeta {
            symbol: "BTC-USD".into(),
            timeframe_secs: 300,
            timeframe_label: "slow300".into(),
            is_active: true,
            overall_risk: 50.0,
        }];
        let o = compute_overview(&[adv], &instances, &[]);
        assert!(matches!(o.global_market_bias, GlobalBias::StrongBullish));
        assert_eq!(o.instance_count, 1);
        assert!(!o.regime_distribution.is_empty());
        assert!(!o.opportunity_distribution.is_empty());
    }

    // ── Alignment aggregation tests ───────────────────────────

    fn make_alignment(symbol: &str, mtf_score: f64, label: &str, agreement: f64) -> AlignmentMatrix {
        AlignmentMatrix {
            symbol: symbol.into(),
            timeframes_present: 4,
            dimensions: Vec::new(),
            mtf_trend_alignment: mtf_score / 100.0,
            mtf_momentum_alignment: mtf_score / 100.0,
            mtf_volume_alignment: 0.0,
            mtf_volatility_alignment: 0.0,
            mtf_overall_score: mtf_score,
            mtf_overall_label: label.into(),
            timeframe_alignments: Vec::new(),
            signal_cross_tf_count: 0,
            trend_agreement_pct: agreement,
        }
    }

    #[test]
    fn alignment_distribution_counts_per_label() {
        let advs: Vec<AdvisoryMatrix> = (0..4)
            .map(|i| AdvisoryMatrix {
                symbol: format!("X-USD{}", i),
                ..AdvisoryMatrix::empty(&format!("X-USD{}", i))
            })
            .collect();
        let alignments = vec![
            make_alignment("X-USD0", 80.0, "STRONG_BULL_MTF", 90.0),
            make_alignment("X-USD1", 30.0, "WEAK_BULL_MTF", 70.0),
            make_alignment("X-USD2", 0.0, "NEUTRAL_MTF", 50.0),
            make_alignment("X-USD3", -60.0, "STRONG_BEAR_MTF", 85.0),
        ];
        let o = compute_overview(&advs, &[], &alignments);
        assert_eq!(o.alignment_distribution.get("STRONG_BULL_MTF"), Some(&1));
        assert_eq!(o.alignment_distribution.get("WEAK_BULL_MTF"), Some(&1));
        assert_eq!(o.alignment_distribution.get("NEUTRAL_MTF"), Some(&1));
        assert_eq!(o.alignment_distribution.get("STRONG_BEAR_MTF"), Some(&1));
        assert_eq!(o.alignment_distribution.get("WEAK_BEAR_MTF"), None);
        assert_eq!(o.alignment_distribution.get("NO_DATA"), None);
        assert_eq!(o.alignment_distribution.values().sum::<u32>(), 4);
    }

    #[test]
    fn alignment_consensus_index_is_mean_of_scores() {
        let advs: Vec<AdvisoryMatrix> = (0..3)
            .map(|i| AdvisoryMatrix {
                symbol: format!("X-USD{}", i),
                ..AdvisoryMatrix::empty(&format!("X-USD{}", i))
            })
            .collect();
        let alignments = vec![
            make_alignment("X-USD0", 50.0, "WEAK_BULL_MTF", 60.0),
            make_alignment("X-USD1", 0.0, "NEUTRAL_MTF", 50.0),
            make_alignment("X-USD2", -50.0, "WEAK_BEAR_MTF", 40.0),
        ];
        let o = compute_overview(&advs, &[], &alignments);
        // (50 + 0 + -50) / 3 = 0.0
        assert!((o.alignment_consensus_index - 0.0).abs() < 1e-9);
        // (60 + 50 + 40) / 3 = 50.0
        assert!((o.multi_tf_agreement_pct - 50.0).abs() < 1e-9);
    }

    #[test]
    fn asset_rank_mirrors_alignment_when_present() {
        let adv = AdvisoryMatrix {
            symbol: "BTC-USD".into(),
            directional_guidance: DirectionalGuidance::Long,
            confidence_assessment: 75.0,
            ..AdvisoryMatrix::empty("BTC-USD")
        };
        let instances = vec![InstanceMeta {
            symbol: "BTC-USD".into(),
            timeframe_secs: 300,
            timeframe_label: "slow300".into(),
            is_active: true,
            overall_risk: 50.0,
        }];
        let alignment = make_alignment("BTC-USD", 65.5, "STRONG_BULL_MTF", 80.0);
        let o = compute_overview(&[adv], &instances, &[alignment]);
        assert_eq!(o.asset_ranking.len(), 1);
        assert_eq!(o.asset_ranking[0].symbol, "BTC-USD");
        assert!((o.asset_ranking[0].mtf_score - 65.5).abs() < 1e-9);
        assert_eq!(o.asset_ranking[0].mtf_label, "STRONG_BULL_MTF");
    }

    #[test]
    fn risk_distribution_bins_on_overall_risk_not_cascade() {
        // L7-A (v6.10.13): the risk distribution / risk_environment bin
        // per-asset L5 OVERALL risk — a symbol with LOW cascade risk but
        // HIGH overall risk must bin HIGH (the old cascade-based code and
        // the confidence-based doc disagreed with the dashboard card).
        let adv_low = AdvisoryMatrix {
            symbol: "BTC-USD".into(),
            cascade_risk_score: 10.0,
            ..AdvisoryMatrix::empty("BTC-USD")
        };
        let adv_high = AdvisoryMatrix {
            symbol: "SOL-USD".into(),
            cascade_risk_score: 10.0,
            ..AdvisoryMatrix::empty("SOL-USD")
        };
        let instances = vec![
            InstanceMeta {
                symbol: "BTC-USD".into(),
                timeframe_secs: 300,
                timeframe_label: "slow300".into(),
                is_active: true,
                overall_risk: 20.0,
            },
            InstanceMeta {
                symbol: "SOL-USD".into(),
                timeframe_secs: 300,
                timeframe_label: "slow300".into(),
                is_active: true,
                overall_risk: 85.0,
            },
        ];
        let o = compute_overview(&[adv_low, adv_high], &instances, &[]);
        // Despite cascade 10 on BOTH symbols, the split follows overall:
        // BTC low (20), SOL high (85).
        assert_eq!(o.risk_distribution.low_pct, 50.0);
        assert_eq!(o.risk_distribution.high_pct, 50.0);
        assert_eq!(o.risk_distribution.risk_environment, "HIGH_RISK");
        // L7-B: the cascade index confidence is a 0-100 percentage.
        assert!(
            (o.cascade_risk_index.confidence - 100.0).abs() < 1e-9,
            "cascade_risk_index.confidence must be 100% with a full sample, got {}",
            o.cascade_risk_index.confidence
        );
    }

    #[test]
    fn asset_rank_defaults_when_alignment_missing() {
        // Symbol present in advisories but absent from alignments —
        // must default to neutral without breaking the rest of the
        // aggregation.
        let adv = AdvisoryMatrix {
            symbol: "BTC-USD".into(),
            directional_guidance: DirectionalGuidance::Long,
            confidence_assessment: 75.0,
            ..AdvisoryMatrix::empty("BTC-USD")
        };
        let instances = vec![InstanceMeta {
            symbol: "BTC-USD".into(),
            timeframe_secs: 300,
            timeframe_label: "slow300".into(),
            is_active: true,
            overall_risk: 50.0,
        }];
        let o = compute_overview(&[adv], &instances, &[]);
        assert_eq!(o.asset_ranking.len(), 1);
        assert_eq!(o.asset_ranking[0].mtf_score, 0.0);
        assert_eq!(o.asset_ranking[0].mtf_label, "NO_DATA");
        // Aggregate alignment fields still default to neutral.
        assert!(o.alignment_distribution.is_empty());
        assert_eq!(o.alignment_consensus_index, 0.0);
        assert_eq!(o.multi_tf_agreement_pct, 0.0);
    }

    #[test]
    fn empty_alignments_yield_neutral_aggregates_without_breaking_advisories() {
        // Advisories present but no alignments — the breadth / bias /
        // sync / risk aggregates must still populate correctly; the
        // alignment aggregates must default to neutral without
        // propagating NaN.
        let advs: Vec<AdvisoryMatrix> = (0..3)
            .map(|i| AdvisoryMatrix {
                symbol: format!("X-USD{}", i),
                directional_guidance: DirectionalGuidance::Long,
                confidence_assessment: 80.0,
                ..AdvisoryMatrix::empty(&format!("X-USD{}", i))
            })
            .collect();
        let o = compute_overview(&advs, &[], &[]);
        assert!(matches!(o.global_market_bias, GlobalBias::StrongBullish));
        assert_eq!(o.alignment_distribution.len(), 0);
        assert_eq!(o.alignment_consensus_index, 0.0);
        assert_eq!(o.multi_tf_agreement_pct, 0.0);
    }
}
