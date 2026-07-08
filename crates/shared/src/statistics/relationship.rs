//! Module E: Relationship Engine (Phase 5).
//!
//! Describes relationships between features — pairwise correlations,
//! feature agreement, indicator redundancy, consensus stability, trend
//! consistency, and momentum consistency.
//!
//! All computations derive from the DistributionTracker's rolling metric
//! windows — no new indicators, no AI, no heuristics.

use crate::statistics::distribution::DistributionTracker;

/// Relationship metrics for one snapshot.
#[derive(Debug, Clone)]
pub struct RelationshipSnapshot {
    pub feature_agreement: f64,       // [0, 1]
    pub indicator_redundancy: f64,    // [0, 1]
    pub consensus_stability: f64,     // [0, 1]
    pub trend_consistency: f64,       // [-1, 1]
    pub momentum_consistency: f64,    // [0, 1]
}

impl Default for RelationshipSnapshot {
    fn default() -> Self {
        Self {
            feature_agreement: 0.0,
            indicator_redundancy: 0.0,
            consensus_stability: 0.0,
            trend_consistency: 0.0,
            momentum_consistency: 0.0,
        }
    }
}

/// Pairwise correlation indices into the DistributionTracker's 14-metric
/// layout.  Each pair: (name, metric_a_idx, metric_b_idx).
const CORRELATION_PAIRS: &[(&str, usize, usize)] = &[
    // Momentum alignment
    ("rsi_vs_squeeze", 3, 5),
    ("rsi_vs_macd", 3, 9),
    ("rsi_vs_stochk", 3, 11),
    // Volatility consistency
    ("atr_vs_bbwp", 2, 4),
    // Volume confirmation
    ("volume_vs_rvol", 6, 7),
    ("obv_vs_price", 10, 0),
    // Trend strength alignment
    ("adx_vs_choppiness", 8, 12),
    // Price vs momentum
    ("price_vs_rsi", 0, 3),
];

impl RelationshipSnapshot {
    /// Compute all relationship metrics from the DistributionTracker using
    /// the best (longest warm) window.
    pub fn compute(tracker: &DistributionTracker) -> Self {
        let wi = tracker.best_window_idx();

        let mut correlations = Vec::with_capacity(CORRELATION_PAIRS.len());
        let mut corr_sum = 0.0;
        let mut active_pairs = 0usize;

        for &(_, a, b) in CORRELATION_PAIRS {
            let series_a = tracker.metric_values(wi, a);
            let series_b = tracker.metric_values(wi, b);
            let n = series_a.len().min(series_b.len());
            if n < 5 {
                correlations.push(0.0);
                continue;
            }
            let r = pearson_correlation(&series_a, &series_b);
            correlations.push(r);
            corr_sum += r.abs();
            active_pairs += 1;
        }

        let feature_agreement = if active_pairs > 0 {
            corr_sum / active_pairs as f64
        } else {
            0.0
        };

        // Redundancy: how strongly correlated are the pairs on average?
        // Higher correlation → more redundancy.
        let indicator_redundancy = if active_pairs > 0 {
            let mean_corr = corr_sum / active_pairs as f64;
            // Use a sigmoid-like mapping: redundancy saturates at ~80% mean correlation.
            (mean_corr / (mean_corr + 0.3)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Consensus stability: from the consistency of returns.
        // Stable returns (low CV) → consensus is not flipping wildly.
        let returns = tracker.metric_values(wi, 1);
        let consensus_stability = consensus_stability_from_returns(&returns);

        // Trend consistency: lag-1 autocorrelation of returns.
        // Positive autocorrelation → trend is consistent (follows itself).
        let trend_consistency = lag1_autocorrelation(&returns);

        // Momentum consistency: how well does price align with ATR/volatility
        // direction?  Compute correlation between price change and ATR change.
        let prices = tracker.metric_values(wi, 0);
        let atrs = tracker.metric_values(wi, 2);
        let momentum_consistency = momentum_consistency_from_price_atr(&prices, &atrs);

        RelationshipSnapshot {
            feature_agreement,
            indicator_redundancy,
            consensus_stability,
            trend_consistency,
            momentum_consistency,
        }
    }
}

// ── Statistical computation helpers ────────────────────────────

/// Pearson correlation coefficient between two equal-length slices.
/// Returns NaN → 0.0 sentinel when stddev of either series is zero.
pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 { return 0.0; }
    let mean_x = x.iter().take(n).sum::<f64>() / n as f64;
    let mean_y = y.iter().take(n).sum::<f64>() / n as f64;
    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    let denom = (var_x * var_y).sqrt();
    if denom < 1e-12 { 0.0 } else { (cov / denom).clamp(-1.0, 1.0) }
}

/// Consensus stability: derived from the coefficient of variation of
/// returns.  Low CV → stable market → consensus is not changing much.
/// Maps CV to [0, 1] where 1 = very stable.
fn consensus_stability_from_returns(returns: &[f64]) -> f64 {
    if returns.len() < 5 { return 0.5; }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let var = returns.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (returns.len() - 1) as f64;
    let std = var.sqrt();
    let cv = if mean.abs() > 1e-12 { std / mean.abs() } else { std.max(0.01) };
    // Low CV → high stability.
    (1.0 / (1.0 + cv)).clamp(0.0, 1.0)
}

/// Lag-1 autocorrelation of a series.  Measures how well the series
/// predicts its next value — a proxy for trend consistency.
fn lag1_autocorrelation(data: &[f64]) -> f64 {
    if data.len() < 3 { return 0.0; }
    let x: Vec<f64> = data[..data.len() - 1].to_vec();
    let y: Vec<f64> = data[1..].to_vec();
    pearson_correlation(&x, &y)
}

/// Momentum consistency: correlation between price changes (returns) and
/// ATR changes.  When price direction and volatility expansion align,
/// momentum is consistent.
fn momentum_consistency_from_price_atr(prices: &[f64], atrs: &[f64]) -> f64 {
    if prices.len() < 3 || atrs.len() < 3 { return 0.0; }
    let n = prices.len().min(atrs.len());
    let mut price_deltas = Vec::with_capacity(n - 1);
    let mut atr_deltas = Vec::with_capacity(n - 1);
    for i in 1..n {
        price_deltas.push(prices[i] - prices[i - 1]);
        atr_deltas.push(atrs[i] - atrs[i - 1]);
    }
    let r = pearson_correlation(&price_deltas, &atr_deltas);
    // Map to [0, 1]: strong positive/negative correlation both indicate
    // structure (volatility expanding with trend or contracting with
    // consolidation).
    (r.abs()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pearson_perfect_positive() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // y = 2x
        let r = pearson_correlation(&x, &y);
        assert!((r - 1.0).abs() < 1e-9, "perfect positive: {r}");
    }

    #[test]
    fn test_pearson_perfect_negative() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0]; // y = 12-2x
        let r = pearson_correlation(&x, &y);
        assert!((r + 1.0).abs() < 1e-9, "perfect negative: {r}");
    }

    #[test]
    fn test_pearson_no_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 1.0, 4.0, 2.0, 3.0]; // shuffled
        let r = pearson_correlation(&x, &y);
        assert!(r.abs() < 0.5, "uncorrelated should have low |r|: {r}");
    }

    #[test]
    fn test_pearson_empty() {
        assert_eq!(pearson_correlation(&[], &[]), 0.0);
    }

    #[test]
    fn test_consensus_stability_constant_returns() {
        let r = vec![0.01; 20];
        let s = consensus_stability_from_returns(&r);
        assert!(s > 0.9, "constant returns = stable: {s}");
    }

    #[test]
    fn test_consensus_stability_volatile() {
        let r: Vec<f64> = (0..20).map(|i| if i % 2 == 0 { 10.0 } else { -10.0 }).collect();
        let s = consensus_stability_from_returns(&r);
        assert!(s < 0.3, "wild returns = unstable: {s}");
    }

    #[test]
    fn test_lag1_acf_positive() {
        // Strong trend: each value is previous + small increment.
        let data: Vec<f64> = (0..50).map(|i| i as f64 * 0.5).collect();
        let acf = lag1_autocorrelation(&data);
        assert!(acf > 0.9, "strong trend should have high ACF: {acf}");
    }

    #[test]
    fn test_lag1_acf_mean_reverting() {
        // Alternating series → negative autocorrelation.
        let data: Vec<f64> = (0..50).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let acf = lag1_autocorrelation(&data);
        assert!(acf < -0.5, "oscillating should have negative ACF: {acf}");
    }

    #[test]
    fn test_momentum_consistency() {
        // Both trending up with correlated fluctuations.
        let prices: Vec<f64> = (0..30).map(|i| 50000.0 + i as f64 * 100.0 + (i % 3) as f64 * 20.0).collect();
        let atrs: Vec<f64> = (0..30).map(|i| 500.0 + i as f64 * 10.0 + (i % 3) as f64 * 5.0).collect();
        let mc = momentum_consistency_from_price_atr(&prices, &atrs);
        // With aligned variation, we should see positive correlation.
        assert!(mc > 0.2, "aligned variation should show correlation: {mc}");
    }

    #[test]
    fn test_default_snapshot() {
        let snap = RelationshipSnapshot::default();
        assert_eq!(snap.feature_agreement, 0.0);
    }
}
