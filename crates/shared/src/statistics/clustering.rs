//! Regime discovery via online k-means clustering (Phase 7).
//!
//! Maintains 5 centroids corresponding to regime archetypes and, on each
//! candle, assigns the current feature vector to the nearest centroid,
//! nudging the centroid toward the new observation with a decaying
//! learning rate (stochastic gradient descent on the k-means objective).
//!
//! Feature vector: [adx_norm, rsi_norm, bbwp_norm, chop_norm, return_norm]
//! where each is z-score-normalized within its rolling window.

/// Number of regime centroids.
const K: usize = 5;

/// Initial centroids seeded to archetype positions in [-1, 1] normalized
/// feature space.  Order: ADX, RSI, BBWP, Choppiness, Returns.
const INITIAL_CENTROIDS: [[f64; 5]; K] = [
    [0.8,  0.6, -0.5, -0.8,  0.5], // Trending Up
    [0.8, -0.6, -0.5, -0.8, -0.5], // Trending Down
    [-0.8, 0.0, -0.2,  0.8,  0.0], // Ranging
    [0.3,  0.0,  0.8,  0.0,  0.3], // Volatile / Explosive
    [-0.5, 0.0,  0.9, -0.3,  0.0], // Compressed
];

const REGIME_LABELS: [&str; K] = [
    "trending_up",
    "trending_down",
    "ranging",
    "volatile",
    "compressed",
];

/// Identifies the current market regime by assigning the feature vector
/// to the nearest centroid.  Centroids drift slowly toward observations
/// (online learning rate α = 0.02).
#[derive(Debug, Clone)]
pub struct RegimeClusterer {
    centroids: [[f64; 5]; K],
    labels: [&'static str; K],
    pub current_regime: String,
    pub regime_stability: f64,
    bars_since_change: usize,
    window_size: usize,
    update_count: usize,
}

impl RegimeClusterer {
    pub fn new() -> Self {
        Self {
            centroids: INITIAL_CENTROIDS,
            labels: REGIME_LABELS,
            current_regime: "unknown".into(),
            regime_stability: 0.0,
            bars_since_change: 0,
            window_size: 20,
            update_count: 0,
        }
    }

    /// Assign the feature vector to the nearest centroid, update the
    /// centroid location, and return the current regime label and
    /// stability score.
    ///
    /// Feature order: [adx_norm, rsi_norm, bbwp_norm, chop_norm, return_norm].
    pub fn classify_and_update(&mut self, features: &[f64; 5]) -> (&str, f64) {
        let mut best_k = 0usize;
        let mut best_dist = f64::INFINITY;

        for k in 0..K {
            let d = euclidean_sq(features, &self.centroids[k]);
            if d < best_dist {
                best_dist = d;
                best_k = k;
            }
        }

        let label = self.labels[best_k];

        // Track regime changes.
        if self.current_regime != label {
            self.bars_since_change = 0;
        } else {
            self.bars_since_change += 1;
        }
        self.current_regime = label.to_string();
        self.regime_stability = (self.bars_since_change as f64 / self.window_size as f64)
            .min(1.0);

        // Online centroid update with decaying learning rate.
        self.update_count += 1;
        let alpha = (0.02 / (1.0 + self.update_count as f64 * 0.001)).max(0.005);
        for d in 0..5 {
            self.centroids[best_k][d] += alpha * (features[d] - self.centroids[best_k][d]);
        }

        (&self.current_regime, self.regime_stability)
    }

    /// Pure classify — no centroid update.
    pub fn classify(&self) -> (&str, f64) {
        (&self.current_regime, self.regime_stability)
    }
}

impl Default for RegimeClusterer {
    fn default() -> Self {
        Self::new()
    }
}

/// Squared Euclidean distance between two 5-dimensional vectors.
fn euclidean_sq(a: &[f64; 5], b: &[f64; 5]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_classification() {
        let mut rc = RegimeClusterer::new();
        // Trending-up profile: high ADX, positive RSI, low BBWP, low chop, positive returns.
        let features = [0.8, 0.6, -0.5, -0.7, 0.4];
        let (label, _) = rc.classify_and_update(&features);
        assert_eq!(label, "trending_up");
    }

    #[test]
    fn test_ranging_classification() {
        let mut rc = RegimeClusterer::new();
        let features = [-0.7, 0.0, -0.1, 0.8, 0.0];
        let (label, _) = rc.classify_and_update(&features);
        assert_eq!(label, "ranging");
    }

    #[test]
    fn test_stability_increases() {
        let mut rc = RegimeClusterer::new();
        let features = [0.8, 0.6, -0.5, -0.7, 0.4];
        for _ in 0..15 {
            rc.classify_and_update(&features);
        }
        assert!(rc.regime_stability > 0.5, "stability should grow after repeated assignments");
    }

    #[test]
    fn test_regime_change_resets_stability() {
        let mut rc = RegimeClusterer::new();
        let trending = [0.8, 0.6, -0.5, -0.7, 0.4];
        let ranging = [-0.7, 0.0, -0.1, 0.8, 0.0];
        for _ in 0..10 {
            rc.classify_and_update(&trending);
        }
        assert!(rc.regime_stability > 0.3);
        rc.classify_and_update(&ranging);
        assert!(rc.regime_stability < 0.2, "stability should reset on regime change");
    }
}
