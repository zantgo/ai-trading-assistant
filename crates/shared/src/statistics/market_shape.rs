//! Module D: Market Distribution Engine (Phase 4).
//!
//! Describes the shape of the historical returns distribution within a
//! rolling window: skewness (third moment), kurtosis (fourth moment),
//! Shannon entropy (disorder), tail risk, distribution symmetry,
//! volatility percentile, and compression percentile.
//!
//! Every value is computed from the raw returns in the DistributionTracker
//! — no new indicators, no AI, no heuristics.

use crate::statistics::distribution::{DistributionTracker, percentile, percentile_rank};

const ENTROPY_BINS: usize = 20;

/// Computed market-shape metrics for a single rolling window.
#[derive(Debug, Clone)]
pub struct MarketShape {
    pub skewness: f64,
    pub kurtosis: f64,             // excess kurtosis (0 = normal)
    pub entropy: f64,              // [0, 1], 1 = maximum disorder
    pub tail_risk: f64,            // |P1 loss| / mean_loss
    pub distribution_symmetry: f64, // [0, 1], 0 = symmetric
    pub volatility_percentile: f64, // current HV %ile in historical HV
    pub compression_percentile: f64, // current BBWP %ile in historical BBWP
    pub shape_label: String,       // normal|compressed|explosive|chaotic|rare
}

impl Default for MarketShape {
    fn default() -> Self {
        Self {
            skewness: 0.0,
            kurtosis: 0.0,
            entropy: 0.0,
            tail_risk: 0.0,
            distribution_symmetry: 0.0,
            volatility_percentile: 0.0,
            compression_percentile: 0.0,
            shape_label: "unknown".into(),
        }
    }
}

impl MarketShape {
    /// Compute all market-shape metrics from the DistributionTracker using
    /// the best (longest warm) window.
    pub fn compute(tracker: &DistributionTracker) -> Self {
        let wi = tracker.best_window_idx();
        let returns = tracker.metric_values(wi, 1); // log-returns
        let atrs = tracker.metric_values(wi, 2);
        let bbwps = tracker.metric_values(wi, 4);

        if returns.len() < 10 {
            return Self::default();
        }

        let (_mean, _variance, skewness, kurtosis) = compute_moments(&returns);
        let entropy = compute_entropy(&returns, ENTROPY_BINS);
        let tail_risk = compute_tail_risk(&returns);
        let symmetry = if skewness.abs() > 1e-12 {
            skewness.abs() / (skewness.abs() + 1.0)
        } else {
            0.0
        };

        // Current HV-based volatility: ATR at latest position relative to
        // historical ATR distribution.
        let current_atr = atrs.last().copied().unwrap_or(0.0);
        let vol_pct = if atrs.len() >= 10 && current_atr > 1e-12 {
            percentile_rank(&atrs, current_atr)
        } else {
            50.0
        };

        let current_bbwp = bbwps.last().copied().unwrap_or(50.0);
        let comp_pct = if bbwps.len() >= 10 {
            percentile_rank(&bbwps, current_bbwp)
        } else {
            50.0
        };

        let squeeze_on = current_bbwp > 0.0; // rough proxy — full squeeze detection in later phases
        let label = classify_shape(skewness, kurtosis, entropy, tail_risk, vol_pct, comp_pct, squeeze_on);

        MarketShape {
            skewness,
            kurtosis,
            entropy,
            tail_risk,
            distribution_symmetry: symmetry,
            volatility_percentile: vol_pct,
            compression_percentile: comp_pct,
            shape_label: label,
        }
    }
}

// ── Statistical computations ───────────────────────────────────

/// Compute the first four standardized moments of the sample.
/// Returns (mean, variance, skewness, excess_kurtosis).
fn compute_moments(data: &[f64]) -> (f64, f64, f64, f64) {
    let n = data.len() as f64;
    if n < 3.0 { return (0.0, 0.0, 0.0, 0.0); }
    let mean = data.iter().sum::<f64>() / n;
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std = variance.sqrt();
    if std < 1e-12 { return (mean, variance, 0.0, 0.0); }

    let skewness = data.iter()
        .map(|x| ((x - mean) / std).powi(3))
        .sum::<f64>() * n / ((n - 1.0) * (n - 2.0));

    let kurtosis_raw = data.iter()
        .map(|x| ((x - mean) / std).powi(4))
        .sum::<f64>() * n * (n + 1.0) / ((n - 1.0) * (n - 2.0) * (n - 3.0))
        - 3.0 * (n - 1.0).powi(2) / ((n - 2.0) * (n - 3.0));

    (mean, variance, skewness, kurtosis_raw)
}

/// Shannon entropy of a histogram with `num_bins` equal-width bins,
/// normalized to [0, 1] (divided by log(num_bins)).
fn compute_entropy(data: &[f64], num_bins: usize) -> f64 {
    if data.is_empty() { return 0.0; }
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range < 1e-12 { return 0.0; }

    let bin_width = range / num_bins as f64;
    let mut bins = vec![0usize; num_bins];
    for &x in data {
        let idx = ((x - min) / bin_width) as usize;
        bins[idx.min(num_bins - 1)] += 1;
    }

    let n = data.len() as f64;
    let mut entropy = 0.0;
    for count in bins.iter() {
        if *count > 0 {
            let p = *count as f64 / n;
            entropy -= p * p.ln();
        }
    }

    let max_entropy = (num_bins as f64).ln();
    if max_entropy < 1e-12 { 0.0 } else { (entropy / max_entropy).clamp(0.0, 1.0) }
}

/// Tail risk: magnitude of the 1st-percentile loss divided by the mean
/// absolute loss.  Higher values indicate fatter left tail.
fn compute_tail_risk(data: &[f64]) -> f64 {
    if data.len() < 50 { return 0.0; }
    let p1 = percentile(data, 1.0);       // 1st %ile (usually negative in returns)
    let mean_abs_loss = data.iter().map(|x| x.abs()).sum::<f64>() / data.len() as f64;
    if mean_abs_loss < 1e-12 { return 0.0; }
    (p1.abs() / mean_abs_loss).clamp(0.0, 10.0)
}

/// Classify the market shape based on distribution moment thresholds.
///
/// | Label       | Condition                                              |
/// |-------------|--------------------------------------------------------|
/// | normal      | \|skewness\| < 0.5, kurtosis < 2, entropy < 0.6      |
/// | compressed  | compression %ile > 90 OR squeeze-like (low ATR %ile) |
/// | explosive   | volatility %ile > 90 AND compression %ile < 20        |
/// | chaotic     | kurtosis > 3 OR tail_risk > 3.0                       |
/// | rare        | price volatility %ile > 95 OR compression %ile > 95   |
fn classify_shape(
    skewness: f64,
    kurtosis: f64,
    entropy: f64,
    tail_risk: f64,
    vol_pct: f64,
    comp_pct: f64,
    _squeeze_on: bool,
) -> String {
    if kurtosis > 3.0 || tail_risk > 3.0 {
        return "chaotic".into();
    }
    if vol_pct > 95.0 || comp_pct > 95.0 {
        return "rare".into();
    }
    if vol_pct > 90.0 && comp_pct < 20.0 {
        return "explosive".into();
    }
    if comp_pct > 90.0 {
        return "compressed".into();
    }
    if skewness.abs() < 0.5 && kurtosis < 2.0 && entropy < 0.6 {
        return "normal".into();
    }
    // Default: whichever dimension dominates.
    if skewness.abs() > 0.5 { "asymmetric" } else { "normal" }.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skewness_symmetric() {
        let data = vec![1.0, 1.5, 2.0, 2.0, 2.5, 3.0, 3.0, 2.5, 2.0, 2.0, 1.5, 1.0];
        let (_, _, sk, _) = compute_moments(&data);
        assert!(sk.abs() < 0.8, "nearly symmetric should have low skew: {sk}");
    }

    #[test]
    fn test_skewness_right_tail() {
        // All small values + one large outlier → positive skew.
        let data = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 100.0];
        let (_, _, sk, _) = compute_moments(&data);
        assert!(sk > 0.5, "right-tailed should have positive skew: {sk}");
    }

    #[test]
    fn test_kurtosis_fat_tails() {
        // Lots of values near the mean + very far outliers → high kurtosis.
        let data: Vec<f64> = vec![0.0; 90]
            .into_iter()
            .chain([-10.0, -10.0, -10.0, -10.0, -10.0, 10.0, 10.0, 10.0, 10.0, 10.0])
            .collect();
        let (_, _, _, kurt) = compute_moments(&data);
        assert!(kurt > 2.0, "fat tails should have high kurtosis: {kurt}");
    }

    #[test]
    fn test_entropy_uniform_max() {
        let data: Vec<f64> = (0..200).map(|i| i as f64 / 10.0).collect();
        let h = compute_entropy(&data, 20);
        assert!(h > 0.9, "uniform distribution should have high entropy: {h}");
    }

    #[test]
    fn test_entropy_spike_low() {
        let data = vec![1.0; 100];
        let h = compute_entropy(&data, 20);
        assert_eq!(h, 0.0, "single value should have zero entropy");
    }

    #[test]
    fn test_tail_risk_no_data() {
        assert_eq!(compute_tail_risk(&[]), 0.0);
    }

    #[test]
    fn test_classify_normal() {
        let label = classify_shape(0.2, 1.0, 0.4, 1.0, 50.0, 50.0, false);
        assert_eq!(label, "normal");
    }

    #[test]
    fn test_classify_chaotic() {
        let label = classify_shape(0.0, 4.0, 0.5, 1.0, 50.0, 50.0, false);
        assert_eq!(label, "chaotic");
    }

    #[test]
    fn test_classify_compressed() {
        let label = classify_shape(0.0, 1.0, 0.5, 1.0, 30.0, 92.0, false);
        assert_eq!(label, "compressed");
    }

    #[test]
    fn test_classify_explosive() {
        let label = classify_shape(0.0, 1.0, 0.5, 1.0, 92.0, 15.0, false);
        assert_eq!(label, "explosive");
    }

    #[test]
    fn test_classify_rare() {
        let label = classify_shape(0.0, 1.0, 0.3, 1.0, 97.0, 50.0, false);
        assert_eq!(label, "rare");
    }

    #[test]
    fn test_default_shape() {
        let shape = MarketShape::default();
        assert_eq!(shape.shape_label, "unknown");
    }
}
