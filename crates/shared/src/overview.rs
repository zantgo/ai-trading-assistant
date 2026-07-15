//! # Overview Matrix — Market Synthesis Layer
//!
//! The Overview Matrix aggregates all Advisory Matrices and instance metadata
//! to provide a unified representation of the observed market environment.
//! It summarizes the collective state of all analyzed assets.
//!
//! Layer: L5.5 in the architecture (Market Synthesis).

use crate::advisory::{AdvisoryMatrix, DirectionalGuidance};
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
    /// Phase 3: cross-symbol aggregate cascade risk (mean of all
    /// per-symbol cascade_risk scores, 0..100).
    #[serde(default)]
    pub cascade_risk_index: f64,
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
            risk_distribution: RiskDistribution { low_pct: 0.0, moderate_pct: 100.0, high_pct: 0.0, risk_environment: "NO_DATA".into() },
            asset_ranking: Vec::new(),
            market_synchronization: SyncLevel::HighlyFragmented,
            market_health: HealthLevel::Neutral,
            cascade_risk_index: 0.0,
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
}

/// Compute the Overview Matrix from all Advisory Matrices and instance metadata.
pub fn compute_overview(
    advisories: &[AdvisoryMatrix],
    instances: &[InstanceMeta],
) -> OverviewMatrix {
    if advisories.is_empty() && instances.iter().all(|i| !i.is_active) {
        return OverviewMatrix::empty();
    }

    let active_instances: Vec<&InstanceMeta> = instances.iter().filter(|i| i.is_active).collect();
    let instance_count = active_instances.len() as u32;

    let mut symbols_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for inst in &active_instances { symbols_set.insert(inst.symbol.clone()); }
    for a in advisories { symbols_set.insert(a.symbol.clone()); }
    let mut active_symbols: Vec<String> = symbols_set.into_iter().collect();
    active_symbols.sort();

    // Global bias: mode of all advisory directional guidances
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
    let total = (long_count + short_count + neutral_count).max(1) as f64;
    let global_bias = if long_count as f64 / total >= 0.6 { GlobalBias::Bullish }
        else if short_count as f64 / total >= 0.6 { GlobalBias::Bearish }
        else if neutral_count as f64 / total >= 0.6 { GlobalBias::Neutral }
        else if long_count > short_count { GlobalBias::Bullish }
        else if short_count > long_count { GlobalBias::Bearish }
        else { GlobalBias::Mixed };

    // Market breadth
    let breadth_pct = (long_count as f64 - short_count as f64) / total * 100.0;
    let breadth = if breadth_pct > 60.0 { MarketBreadth::StrongPositive }
        else if breadth_pct > 20.0 { MarketBreadth::Positive }
        else if breadth_pct < -60.0 { MarketBreadth::StrongNegative }
        else if breadth_pct < -20.0 { MarketBreadth::Negative }
        else if breadth_pct.abs() < 10.0 { MarketBreadth::Balanced }
        else if breadth_pct > 0.0 { MarketBreadth::Weak }
        else { MarketBreadth::VeryWeak };

    // Asset ranking
    let mut rankings: Vec<AssetRank> = advisories.iter().map(|a| {
        let score = a.confidence_assessment * 0.5 + (100.0 - a.confidence_assessment.min(50.0) * 0.5);
        AssetRank {
            symbol: a.symbol.clone(),
            score,
            bias: format!("{:?}", a.directional_guidance),
            confidence: a.confidence_assessment,
            regime: format!("{:?}", a.strategy_environment),
            risk_level: "MODERATE".into(),
        }
    }).collect();
    rankings.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Synchronization
    let sync = if breadth_pct.abs() > 75.0 { SyncLevel::HighlySynchronized }
        else if breadth_pct.abs() > 50.0 { SyncLevel::Synchronized }
        else if breadth_pct.abs() > 25.0 { SyncLevel::Mixed }
        else if breadth_pct.abs() > 10.0 { SyncLevel::Fragmented }
        else { SyncLevel::HighlyFragmented };

    let health = match global_bias {
        GlobalBias::StrongBullish | GlobalBias::StrongBearish => HealthLevel::Strong,
        GlobalBias::Bullish | GlobalBias::Bearish => HealthLevel::Healthy,
        GlobalBias::Mixed => HealthLevel::Weak,
        _ => HealthLevel::Neutral,
    };

    let summary = format!(
        "{} active instances across {} symbols. Global bias: {} with {} market breadth.",
        instance_count, active_symbols.len(), global_bias, match breadth {
            MarketBreadth::StrongPositive | MarketBreadth::Positive => "positive",
            MarketBreadth::StrongNegative | MarketBreadth::Negative => "negative",
            _ => "balanced",
        }
    );

    // Risk distribution: approximate from advisories
    let total_adv = advisories.len().max(1) as f64;
    let low_risk = advisories.iter().filter(|a| a.confidence_assessment > 70.0).count() as f64 / total_adv * 100.0;
    let high_risk = advisories.iter().filter(|a| a.confidence_assessment < 30.0).count() as f64 / total_adv * 100.0;

    OverviewMatrix {
        global_market_bias: global_bias,
        market_breadth: breadth,
        regime_distribution: HashMap::new(),
        opportunity_distribution: HashMap::new(),
        risk_distribution: RiskDistribution {
            low_pct: low_risk.round(),
            moderate_pct: (100.0 - low_risk - high_risk).max(0.0).round(),
            high_pct: high_risk.round(),
            risk_environment: if high_risk > 50.0 { "HIGH_RISK".into() } else if low_risk > 50.0 { "LOW_RISK".into() } else { "MODERATE".into() },
        },
        asset_ranking: rankings,
        market_synchronization: sync,
        market_health: health,
        cascade_risk_index: 0.0, // populated by Phase 3 cross-symbol aggregator
        global_summary: summary,
        instance_count,
        active_symbols,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_default() {
        let o = compute_overview(&[], &[]);
        assert!(matches!(o.global_market_bias, GlobalBias::Neutral));
        assert_eq!(o.instance_count, 0);
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
            symbol: "BTC-USD".into(), timeframe_secs: 180,
            timeframe_label: "fast180".into(), is_active: true,
        }];
        let o = compute_overview(&[adv], &instances);
        assert!(matches!(o.global_market_bias, GlobalBias::Bullish));
        assert_eq!(o.instance_count, 1);
    }
}
