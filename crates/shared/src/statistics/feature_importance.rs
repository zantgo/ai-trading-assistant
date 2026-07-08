//! Feature importance — mutual-information-based indicator ranking (Phase 7).
//!
//! Discretizes each metric's value into 3 bins (high/neutral/low) and
//! computes the mutual information between that feature and the forward
//! N-bar return direction.  Higher MI → feature has more predictive power
//! for the current market.
//!
//! The tracker consumes the rolling (features, forward_return) history
//! maintained by the `OnlineLearner`.

use std::collections::HashMap;

/// Metric index layout in the 9-tracker DistributionTracker.
pub const FEATURE_LABELS: &[(&str, usize)] = &[
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

/// Scores indicators by their predictive power.
#[derive(Debug, Clone)]
pub struct FeatureImportanceTracker {
    pub scores: Vec<(String, f64)>, // sorted descending by importance
    pub top_n: usize,
}

impl FeatureImportanceTracker {
    pub fn new(top_n: usize) -> Self {
        Self { scores: Vec::new(), top_n }
    }

    /// Compute feature importance from a history of (feature_vector,
    /// forward_return) pairs.  Each feature vector is indexed as per the
    /// DistributionTracker's metric layout (9 elements).
    ///
    /// Returns the top-N features by mutual information with the forward
    /// return direction (up/down).
    pub fn compute(&mut self, history: &[(Vec<f64>, f64)]) -> Vec<(String, f64)> {
        if history.len() < 10 {
            self.scores = Vec::new();
            return Vec::new();
        }

        let num_features = history.first().map(|h| h.0.len()).unwrap_or(9);
        let n = history.len();

        // Discretize forward returns: positive → 1, negative → 0.
        let directions: Vec<usize> = history
            .iter()
            .map(|(_, r)| if *r > 0.0 { 1 } else { 0 })
            .collect();

        let mut mi_scores: Vec<(String, f64)> = Vec::with_capacity(num_features);

        for fi in 0..num_features {
            // Extract feature values and discretize into 3 bins.
            let feature_vals: Vec<f64> = history.iter().map(|(f, _)| f[fi]).collect();
            let mean = feature_vals.iter().sum::<f64>() / n as f64;
            let std = (feature_vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                / (n - 1) as f64)
                .sqrt()
                .max(1e-12);

            // 0 = low (< mean - 0.5σ), 1 = neutral, 2 = high (> mean + 0.5σ).
            let bins: Vec<usize> = feature_vals
                .iter()
                .map(|&v| {
                    if v < mean - 0.5 * std { 0 }
                    else if v > mean + 0.5 * std { 2 }
                    else { 1 }
                })
                .collect();

            let mi = mutual_information_3bins(&bins, &directions, n);
            let label = FEATURE_LABELS
                .iter()
                .find(|(_, idx)| *idx == fi)
                .map(|(name, _)| name.to_string())
                .unwrap_or_else(|| format!("metric_{fi}"));
            mi_scores.push((label, mi));
        }

        mi_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        mi_scores.truncate(self.top_n);
        self.scores = mi_scores.clone();
        mi_scores
    }

    /// Last computed top-N scores.
    pub fn top_scores(&self) -> &[(String, f64)] {
        &self.scores
    }
}

impl Default for FeatureImportanceTracker {
    fn default() -> Self {
        Self::new(5)
    }
}

// ── Mutual information for 3 bins × 2 classes ──────────────────

fn mutual_information_3bins(
    feature_bins: &[usize], // 0, 1, 2
    class_labels: &[usize], // 0 or 1
    n: usize,
) -> f64 {
    // Count joint distribution.
    let mut joint = [[0usize; 2]; 3];
    for i in 0..n {
        let b = feature_bins[i].min(2);
        let c = class_labels[i].min(1);
        joint[b][c] += 1;
    }

    let nf = n as f64;
    let mut mi = 0.0;

    let mut colspan = HashMap::new();
    for b in 0..3 {
        for c in 0..2 {
            *colspan.entry(("col", c)).or_insert(0usize) += joint[b][c];
        }
    }
    // Actually let me compute MI directly:
    // MI = Σ p(x,y) log(p(x,y) / (p(x) * p(y)))

    // Marginal probabilities.
    let mut p_feat = [0usize; 3];
    let mut p_class = [0usize; 2];
    for b in 0..3 {
        for c in 0..2 {
            p_feat[b] += joint[b][c];
            p_class[c] += joint[b][c];
        }
    }

    for b in 0..3 {
        for c in 0..2 {
            let j = joint[b][c] as f64;
            if j == 0.0 { continue; }
            let p_j = j / nf;
            let p_f = p_feat[b] as f64 / nf;
            let p_c = p_class[c] as f64 / nf;
            if p_f > 0.0 && p_c > 0.0 {
                mi += p_j * (p_j / (p_f * p_c)).ln();
            }
        }
    }

    (mi / (2.0_f64).ln()).clamp(0.0, 10.0) // normalize by log(2) for bits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_history() {
        let mut tracker = FeatureImportanceTracker::new(3);
        let scores = tracker.compute(&[]);
        assert!(scores.is_empty());
    }

    #[test]
    fn test_perfect_predictor() {
        // Feature 0 perfectly predicts direction: high → up, low → down.
        let mut history: Vec<(Vec<f64>, f64)> = Vec::new();
        for i in 0..100 {
            let val = if i % 2 == 0 { 1.0 } else { -1.0 };
            let ret = if i % 2 == 0 { 0.5 } else { -0.5 };
            history.push((vec![val, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], ret));
        }
        let mut tracker = FeatureImportanceTracker::new(3);
        let scores = tracker.compute(&history);
        assert!(!scores.is_empty(), "should have scores");
        // Feature 0 should be the top-ranked.
        assert_eq!(scores[0].0, "price", "price should be top predictor");
        assert!(scores[0].1 > 0.1, "MI should be positive: {}", scores[0].1);
    }

    #[test]
    fn test_noise_feature_zero_mi() {
        let mut history: Vec<(Vec<f64>, f64)> = Vec::new();
        for i in 0..50 {
            history.push((vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], if i % 2 == 0 { 1.0 } else { -1.0 }));
        }
        let mut tracker = FeatureImportanceTracker::new(3);
        let scores = tracker.compute(&history);
        // All features are constant → MI should be near zero.
        for (_, mi) in &scores {
            assert!(mi.abs() < 1e-6, "constant feature should have zero MI");
        }
    }
}
