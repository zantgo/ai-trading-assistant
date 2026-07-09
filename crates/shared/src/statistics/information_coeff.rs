//! Spearman Information Coefficient (IC) — signal quality measurement.
//!
//! The IC measures whether trading signals (confluence scores, indicator
//! values) contain predictive information about future returns.  Uses
//! Spearman rank correlation (robust to outliers and non-linear
//! relationships) between signal values and forward N-bar returns.
//!
//! Interpretation:
//!   IC > 0.10  → strong predictive power (rare)
//!   IC 0.05–0.10 → good
//!   IC 0.02–0.05 → weak
//!   IC < 0.02    → essentially random
//!   IC < 0       → counter-predictive

use std::collections::VecDeque;

/// Rolling tracker for Spearman IC computation.
#[derive(Debug, Clone)]
pub struct IcTracker {
    /// Historical signal values (e.g., confluence scores).
    predictions: VecDeque<f64>,
    /// Corresponding forward returns.
    outcomes: VecDeque<f64>,
    /// Maximum lookback window size.
    lookback: usize,
}

/// Information Coefficient metrics for the current window.
#[derive(Debug, Clone)]
pub struct IcMetrics {
    /// Spearman rank correlation between signals and outcomes.
    pub spearman_ic: f64,
    /// Absolute IC normalized to [0, 1].
    pub rank: f64,
    /// Approximate significance (t-statistic based p-value proxy, [0, 1]).
    pub significance: f64,
}

impl IcTracker {
    pub fn new(lookback: usize) -> Self {
        Self {
            predictions: VecDeque::with_capacity(lookback),
            outcomes: VecDeque::with_capacity(lookback),
            lookback,
        }
    }

    /// Push a new signal-outcome pair.  Evicts oldest if over capacity.
    pub fn push(&mut self, prediction: f64, outcome: f64) {
        self.predictions.push_back(prediction);
        self.outcomes.push_back(outcome);
        while self.predictions.len() > self.lookback {
            self.predictions.pop_front();
        }
        while self.outcomes.len() > self.lookback {
            self.outcomes.pop_front();
        }
    }

    /// Compute the current Spearman IC from the rolling buffer.
    pub fn compute(&self) -> Option<IcMetrics> {
        let n = self.predictions.len();
        if n < 10 {
            return None;
        }

        // Spearman rank correlation = Pearson correlation of ranked values.
        let pred_ranks = rank_values(&self.predictions);
        let outcome_ranks = rank_values(&self.outcomes);

        let mean_pr: f64 = pred_ranks.iter().sum::<f64>() / n as f64;
        let mean_or: f64 = outcome_ranks.iter().sum::<f64>() / n as f64;

        let cov: f64 = pred_ranks
            .iter()
            .zip(outcome_ranks.iter())
            .map(|(p, o)| (p - mean_pr) * (o - mean_or))
            .sum::<f64>()
            / (n - 1) as f64;

        let var_pr: f64 = pred_ranks
            .iter()
            .map(|p| (p - mean_pr).powi(2))
            .sum::<f64>()
            / (n - 1) as f64;

        let var_or: f64 = outcome_ranks
            .iter()
            .map(|o| (o - mean_or).powi(2))
            .sum::<f64>()
            / (n - 1) as f64;

        let denom = (var_pr * var_or).sqrt();
        let ic = if denom > 1e-12 {
            (cov / denom).clamp(-1.0, 1.0)
        } else {
            0.0
        };

        // Approximate significance via t-statistic:
        // t = ic × √((n−2) / (1 − ic²))
        let t_stat = if ic.abs() < 0.999 {
            ic.abs() * ((n as f64 - 2.0) / (1.0 - ic * ic)).sqrt()
        } else {
            10.0
        };
        // Rough p-value approximation (conservative)
        let significance = (1.0 - (-t_stat.abs() * 0.5).exp()).clamp(0.0, 1.0);

        Some(IcMetrics {
            spearman_ic: ic,
            rank: ic.abs(),
            significance,
        })
    }

    pub fn len(&self) -> usize {
        self.predictions.len()
    }
}

/// Compute ranks of values (1-based, average rank for ties).
fn rank_values(values: &VecDeque<f64>) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }
    // Create index-value pairs, sort by value
    let mut indexed: Vec<(usize, f64)> = values.iter().enumerate().map(|(i, &v)| (i, v)).collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        // Find all values equal to this one
        while j + 1 < n && (indexed[j].1 - indexed[j + 1].1).abs() < 1e-12 {
            j += 1;
        }
        let avg_rank = (i + j) as f64 / 2.0 + 1.0; // 1-based
        for k in i..=j {
            ranks[indexed[k].0] = avg_rank;
        }
        i = j + 1;
    }
    ranks
}

/// Spearman correlation of two slices directly (used for testing and one-shot computation).
pub fn spearman_correlation(x: &[f64], y: &[f64]) -> f64 {
    if x.len() != y.len() || x.len() < 3 {
        return 0.0;
    }
    let n = x.len();

    // Rank x
    let mut x_idx: Vec<(usize, f64)> = x.iter().enumerate().map(|(i, &v)| (i, v)).collect();
    x_idx.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut x_rank = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && (x_idx[j].1 - x_idx[j + 1].1).abs() < 1e-12 { j += 1; }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j { x_rank[x_idx[k].0] = avg; }
        i = j + 1;
    }

    // Rank y
    let mut y_idx: Vec<(usize, f64)> = y.iter().enumerate().map(|(i, &v)| (i, v)).collect();
    y_idx.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut y_rank = vec![0.0; n];
    i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && (y_idx[j].1 - y_idx[j + 1].1).abs() < 1e-12 { j += 1; }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j { y_rank[y_idx[k].0] = avg; }
        i = j + 1;
    }

    let mr = x_rank.iter().sum::<f64>() / n as f64;
    let ms = y_rank.iter().sum::<f64>() / n as f64;
    let cov: f64 = x_rank.iter().zip(y_rank.iter())
        .map(|(a, b)| (a - mr) * (b - ms))
        .sum::<f64>() / (n - 1) as f64;
    let vr: f64 = x_rank.iter().map(|a| (a - mr).powi(2)).sum::<f64>() / (n - 1) as f64;
    let vs: f64 = y_rank.iter().map(|b| (b - ms).powi(2)).sum::<f64>() / (n - 1) as f64;
    let denom = (vr * vs).sqrt();
    if denom > 1e-12 { (cov / denom).clamp(-1.0, 1.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ic_perfect_positive() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let ic = spearman_correlation(&x, &y);
        assert!((ic - 1.0).abs() < 1e-6, "perfect monotonic → IC=1, got {}", ic);
    }

    #[test]
    fn test_ic_perfect_negative() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let ic = spearman_correlation(&x, &y);
        assert!((ic + 1.0).abs() < 1e-6, "perfect inverse → IC=−1, got {}", ic);
    }

    #[test]
    fn test_ic_independent_approx_zero() {
        // Independent data: x increasing, y as random scatter — should have low IC
        let x: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let y: Vec<f64> = vec![1.2, 3.1, 2.4, 5.0, 4.3, 6.1, 7.8, 8.2, 9.5, 10.0,
                               0.1, 2.5, 4.0, 5.5, 7.0, 8.5, 9.0, 10.5, 11.0, 12.2];
        let ic = spearman_correlation(&x, &y);
        // Should be imperfectly correlated
        assert!(ic.abs() < 0.95);
    }

    #[test]
    fn test_ic_tracker_push_and_compute() {
        let mut tracker = IcTracker::new(20);
        // Feed perfectly correlated data
        for i in 0..20 {
            tracker.push(i as f64, i as f64 * 2.0);
        }
        let metrics = tracker.compute().unwrap();
        assert!((metrics.spearman_ic - 1.0).abs() < 1e-3);
        assert!(metrics.significance > 0.9);
    }

    #[test]
    fn test_ic_tracker_too_few() {
        let tracker = IcTracker::new(20);
        assert!(tracker.compute().is_none());
    }
}
