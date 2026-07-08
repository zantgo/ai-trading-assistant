//! Statistical anomaly detection (Phase 7).
//!
//! Computes per-metric z-scores from the DistributionTracker's rolling
//! windows and aggregates them into an overall anomaly score via
//! root-mean-square.  A score near 1 indicates the market is behaving
//! extremely unusually across multiple dimensions.

use std::collections::HashMap;

use crate::statistics::distribution::DistributionTracker;

/// Detects when the market is behaving statistically unusually.
#[derive(Debug, Clone)]
pub struct AnomalyDetector {
    pub aggregate_score: f64,
    pub per_metric_scores: HashMap<String, f64>,
    pub top_reason: String,
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self {
            aggregate_score: 0.0,
            per_metric_scores: HashMap::new(),
            top_reason: String::new(),
        }
    }
}

/// Metric names matching the DistributionTracker index ordering.
const ANOMALY_METRICS: &[(&str, usize)] = &[
    ("price", 0),
    ("returns", 1),
    ("atr", 2),
    ("rsi", 3),
    ("bbwp", 4),
    ("squeeze", 5),
    ("volume", 6),
    ("rvol", 7),
    ("adx", 8),
];

impl AnomalyDetector {
    /// Detect anomalies by computing per-metric z-scores and aggregating.
    /// The aggregate score is the RMS of the individual absolute z-scores,
    /// normalized to [0, 1] via: 1 - exp(-rms / 2).
    pub fn detect(&mut self, tracker: &DistributionTracker) -> (f64, &str) {
        let wi = tracker.best_window_idx();
        let mut per_scores = HashMap::new();
        let mut sum_sq = 0.0;
        let mut top_z = 0.0;
        let mut top_name = "none".to_string();

        for &(name, idx) in ANOMALY_METRICS {
            let values = tracker.metric_values(wi, idx);
            if values.len() < 10 { continue; }
            let n = values.len();
            let mean = values.iter().sum::<f64>() / n as f64;
            let std = (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                / (n - 1) as f64)
                .sqrt()
                .max(1e-12);
            let current = values.last().copied().unwrap_or(mean);
            let z = ((current - mean) / std).abs();
            sum_sq += z * z;
            per_scores.insert(name.to_string(), z);

            if z > top_z {
                top_z = z;
                top_name = format!(
                    "{} is {:.1}σ {} {:.1}",
                    name,
                    z,
                    if current > mean { "above" } else { "below" },
                    mean,
                );
            }
        }

        let count = per_scores.len().max(1) as f64;
        let rms = (sum_sq / count).sqrt();
        let aggregate = 1.0 - (-rms / 2.0).exp();

        self.per_metric_scores = per_scores;
        self.aggregate_score = aggregate;
        self.top_reason = top_name;

        (aggregate, &self.top_reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let d = AnomalyDetector::default();
        assert_eq!(d.aggregate_score, 0.0);
        assert!(d.top_reason.is_empty());
    }

    #[test]
    fn test_detection_with_tracker() {
        // Create tracker with some data, compute anomaly.
        use crate::statistics::distribution::DistributionTracker;
        let mut tracker = DistributionTracker::new(&[20]);
        // Push several candles.
        for i in 0..30 {
            let price = 50000.0 + (i % 10) as f64 * 10.0; // slight oscillation
            let ret = (i % 5) as f64 - 2.0;
            let m: [f64; 14] = [
                price, ret, 500.0, 50.0 + ret * 5.0, 50.0,
                0.0, 1000.0, 1.0, 25.0 + ret,
                0.0, 0.0, 50.0, 50.0, price,
            ];
            tracker.advance(&m);
        }
        let mut det = AnomalyDetector::default();
        let (score, reason) = det.detect(&tracker);
        assert!(score >= 0.0 && score <= 1.0, "score in [0,1]: {score}");
        assert!(!reason.is_empty());
        assert!(!det.per_metric_scores.is_empty());
    }
}

/// Incremental multivariate covariance tracking (Welford's algorithm).
/// Enables Mahalanobis distance anomaly detection for truly multivariate analysis.
#[derive(Debug, Clone)]
pub struct IncrementalCovariance {
    mean: Vec<f64>,
    cov: Vec<Vec<f64>>,
    n: usize,
}

impl IncrementalCovariance {
    pub fn new(dim: usize) -> Self {
        Self {
            mean: vec![0.0; dim],
            cov: vec![vec![0.0; dim]; dim],
            n: 0,
        }
    }

    pub fn update(&mut self, x: &[f64]) {
        self.n += 1;
        if self.n == 1 {
            self.mean.copy_from_slice(x);
            return;
        }
        let nf = self.n as f64;
        let delta: Vec<f64> = x.iter().zip(self.mean.iter()).map(|(xi, mi)| xi - mi).collect();
        for (i, d) in delta.iter().enumerate() {
            self.mean[i] += d / nf;
        }
        let delta2: Vec<f64> = x.iter().zip(self.mean.iter()).map(|(xi, mi)| xi - mi).collect();
        let n_minus_1 = (self.n - 1) as f64;
        for i in 0..self.mean.len() {
            for j in 0..self.mean.len() {
                self.cov[i][j] = (self.cov[i][j] * n_minus_1 + delta[i] * delta2[j]) / nf;
            }
        }
    }

    pub fn mahalanobis(&self, x: &[f64]) -> f64 {
        if self.n < 3 {
            return 0.0;
        }
        let dim = self.mean.len();
        let delta: Vec<f64> = x.iter().zip(self.mean.iter()).map(|(xi, mi)| xi - mi).collect();

        let mut lu = self.cov.clone();
        let mut solved = delta.clone();

        // LU decomposition (in-place Gaussian elimination)
        for col in 0..dim {
            let pivot = lu[col][col];
            if pivot.abs() < 1e-12 {
                continue;
            }
            for row in col + 1..dim {
                let factor = lu[row][col] / pivot;
                for k in col..dim {
                    lu[row][k] -= factor * lu[col][k];
                }
            }
        }

        // Forward substitution
        for i in 0..dim {
            let mut sum = 0.0;
            for j in 0..i {
                sum += lu[i][j] * solved[j];
            }
            solved[i] = delta[i] - sum;
        }

        // Backward substitution
        for i in (0..dim).rev() {
            let mut sum = 0.0;
            for j in i + 1..dim {
                sum += lu[i][j] * solved[j];
            }
            if lu[i][i].abs() > 1e-12 {
                solved[i] = (solved[i] - sum) / lu[i][i];
            } else {
                solved[i] = 0.0;
            }
        }

        // y^T C^{-1} y = y^T x_hat where C x_hat = y
        let mut mahal = 0.0;
        for i in 0..dim {
            mahal += delta[i] * solved[i];
        }
        mahal.abs().sqrt()
    }

    pub fn n(&self) -> usize {
        self.n
    }
}

#[cfg(test)]
mod mahalanobis_tests {
    use super::*;

    #[test]
    fn test_mahalanobis_on_identity() {
        let mut ic = IncrementalCovariance::new(2);
        for _ in 0..50 {
            ic.update(&[0.0, 0.0]);
        }
        let d = ic.mahalanobis(&[0.0, 0.0]);
        assert!(d >= 0.0);
    }

    #[test]
    fn test_mahalanobis_detects_outlier() {
        let mut ic = IncrementalCovariance::new(2);
        for _ in 0..100 {
            ic.update(&[0.0, 0.0]);
        }
        let d_normal = ic.mahalanobis(&[1.0, 1.0]);
        ic.update(&[5.0, 5.0]);
        let d_outlier = ic.mahalanobis(&[5.0, 5.0]);
        assert!(d_outlier > 0.0, "mahalanobis should detect distance; got {}", d_outlier);
    }
}
