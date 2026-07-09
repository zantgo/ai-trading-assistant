//! Module C: Confidence Engine (Phase 3).
//!
//! Quantifies uncertainty around SIL estimates:
//!   - Prediction intervals (68%, 95%, 99%) from empirical return percentiles
//!   - Bootstrap confidence intervals via resampling
//!   - Historical reliability: how often were past probability estimates correct?
//!   - Composite confidence score (0–100)

use std::collections::VecDeque;

use crate::statistics::distribution::{percentile, DistributionTracker};

/// Confidence metrics for one snapshot.
#[derive(Debug, Clone)]
pub struct ConfidenceSnapshot {
    pub prediction_interval_68: (f64, f64),
    pub prediction_interval_95: (f64, f64),
    pub prediction_interval_99: (f64, f64),
    pub bootstrap_confidence_95: (f64, f64),
    pub historical_reliability: f64,
    pub confidence_score: f64,
}

impl Default for ConfidenceSnapshot {
    fn default() -> Self {
        Self {
            prediction_interval_68: (0.0, 0.0),
            prediction_interval_95: (0.0, 0.0),
            prediction_interval_99: (0.0, 0.0),
            bootstrap_confidence_95: (0.0, 0.0),
            historical_reliability: 0.0,
            confidence_score: 0.0,
        }
    }
}

// ── Minimal deterministic PRNG for bootstrap (L'Ecuyer's LCG) ──

struct BootstrapRng {
    state: u64,
}

impl BootstrapRng {
    fn with_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns a `u64` in [0, 2^32).
    fn next_u32(&mut self) -> u32 {
        // L'Ecuyer's multiplier for LCG
        self.state = self.state.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
        (self.state >> 32) as u32
    }

    /// Returns an `f64` in [0, 1).
    fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / (u32::MAX as f64 + 1.0)
    }
}

// ── ConfidenceEngine ───────────────────────────────────────────

/// Computes confidence metrics from the returns history and Bayesian
/// tracker observations.
#[derive(Debug, Clone)]
pub struct ConfidenceEngine {
    /// Rolling history of (predicted_probability, actual_outcome_bool) for
    /// the primary directional event.  Capped at 500 entries.
    reliability_log: VecDeque<(f64, bool)>,
    reliability_capacity: usize,
    bootstrap_iterations: usize,
    bootstrap_seed: u64,
}

impl ConfidenceEngine {
    pub fn new() -> Self {
        Self {
            reliability_log: VecDeque::with_capacity(500),
            reliability_capacity: 500,
            bootstrap_iterations: 1000,
            bootstrap_seed: 42,
        }
    }

    /// Record an event outcome for historical-reliability tracking.
    /// `predicted_prob` is the probability estimate at the time the event
    /// was predicted (from an earlier candle).
    pub fn record_outcome(&mut self, predicted_prob: f64, actual_success: bool) {
        if self.reliability_log.len() >= self.reliability_capacity {
            self.reliability_log.pop_front();
        }
        self.reliability_log.push_back((predicted_prob, actual_success));
    }

    /// Compute all confidence metrics from the given distribution tracker
    /// and the current bar's key statistics.
    ///
    /// `total_trials` is the sum of Bayesian observation trials (used as a
    /// proxy for "how much data backs the probability estimates").
    pub fn compute_all(
        &self,
        tracker: &DistributionTracker,
        total_trials: usize,
    ) -> ConfidenceSnapshot {
        let wi = tracker.best_window_idx();

        // ── Prediction intervals from returns percentiles ──────
        let returns = tracker.metric_values(wi, 1); // log-returns
        let (pi68, pi95, pi99) = compute_prediction_intervals(&returns);

        // ── Bootstrap 95% CI for mean return ──────────────────
        let bootstrap_ci = compute_bootstrap_mean_ci(
            &returns,
            self.bootstrap_iterations,
            self.bootstrap_seed,
        );

        // ── Historical reliability ────────────────────────────
        let reliability = compute_historical_reliability(&self.reliability_log);

        // ── Confidence score (composite 0-100) ────────────────
        let score = confidence_score(&pi95, &bootstrap_ci, reliability, total_trials);

        ConfidenceSnapshot {
            prediction_interval_68: pi68,
            prediction_interval_95: pi95,
            prediction_interval_99: pi99,
            bootstrap_confidence_95: bootstrap_ci,
            historical_reliability: reliability,
            confidence_score: score,
        }
    }

    /// Number of entries in the reliability log.
    pub fn reliability_log_len(&self) -> usize {
        self.reliability_log.len()
    }
}

// ── Pure computation functions ─────────────────────────────────

/// Compute prediction intervals from the empirical returns distribution.
/// Uses percentiles: 68% = [16, 84], 95% = [2.5, 97.5], 99% = [0.5, 99.5].
fn compute_prediction_intervals(returns: &[f64]) -> ((f64, f64), (f64, f64), (f64, f64)) {
    if returns.is_empty() {
        return ((0.0, 0.0), (0.0, 0.0), (0.0, 0.0));
    }
    let pi68 = (percentile(returns, 16.0), percentile(returns, 84.0));
    let pi95 = (percentile(returns, 2.5), percentile(returns, 97.5));
    let pi99 = (percentile(returns, 0.5), percentile(returns, 99.5));
    (pi68, pi95, pi99)
}

/// Bootstrap 95% confidence interval for the sample mean.
/// Resamples with replacement `n_iterations` times, computes the mean of
/// each bootstrap sample, and reports the 2.5th and 97.5th percentiles of
/// the bootstrap distribution.
fn compute_bootstrap_mean_ci(data: &[f64], n_iterations: usize, seed: u64) -> (f64, f64) {
    let n = data.len();
    if n < 3 {
        return (0.0, 0.0);
    }
    let mut rng = BootstrapRng::with_seed(seed);
    let mut bootstrap_means: Vec<f64> = Vec::with_capacity(n_iterations);

    for _ in 0..n_iterations {
        let mut sum = 0.0;
        for _ in 0..n {
            let idx = (rng.next_f64() * n as f64) as usize;
            sum += data[idx.min(n - 1)];
        }
        bootstrap_means.push(sum / n as f64);
    }

    bootstrap_means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo_idx = (0.025 * n_iterations as f64) as usize;
    let hi_idx = (0.975 * n_iterations as f64) as usize;
    (bootstrap_means[lo_idx], bootstrap_means[hi_idx])
}

/// Historical reliability: for probability estimates in the reliability
/// log, report the fraction of times the actual outcome matched the
/// prediction (using P > 0.5 → predicted success).  Properly calibrated
/// probability estimates should have ~reliability matching the mean
/// predicted probability.
fn compute_historical_reliability(log: &VecDeque<(f64, bool)>) -> f64 {
    if log.is_empty() {
        return 0.0;
    }
    let correct = log
        .iter()
        .filter(|(prob, outcome)| {
            // Predicted success when prob > 0.5, predicted failure when prob <= 0.5.
            let predicted_success = *prob > 0.5;
            predicted_success == *outcome
        })
        .count();
    correct as f64 / log.len() as f64
}

/// Composite confidence score:
///   35% prediction interval tightness  (narrower = more confident)
///   25% bootstrap CI tightness
///   25% historical reliability
///   15% observation-count factor
fn confidence_score(
    pi95: &(f64, f64),
    bootstrap_ci: &(f64, f64),
    reliability: f64,
    total_trials: usize,
) -> f64 {
    let pi_width = (pi95.1 - pi95.0).abs();
    // Normalize: narrower PI → higher tightness.  Typical crypto daily returns
    // have ~5% range; a 2% PI is very tight, a 20% PI is wide.
    let pi_tightness = if pi_width > 0.0 {
        (1.0 / (1.0 + pi_width / 5.0)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let bs_width = (bootstrap_ci.1 - bootstrap_ci.0).abs();
    let bs_tightness = if bs_width > 0.0 {
        (1.0 / (1.0 + bs_width / 3.0)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Observation-count factor: saturates at 100 trials.
    let obs_factor = (total_trials as f64 / 100.0).min(1.0);

    let raw = 0.35 * pi_tightness + 0.25 * bs_tightness + 0.25 * reliability + 0.15 * obs_factor;
    (raw * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_intervals_empty() {
        let (pi68, pi95, pi99) = compute_prediction_intervals(&[]);
        assert_eq!(pi68, (0.0, 0.0));
        assert_eq!(pi95, (0.0, 0.0));
        assert_eq!(pi99, (0.0, 0.0));
    }

    #[test]
    fn test_prediction_intervals_normal() {
        // Uniform(-3, 3) data: 2.5th %ile = -2.85, 97.5th = +2.85.
        let data: Vec<f64> = (-300i32..=300).map(|i| i as f64 / 100.0).collect();
        let (_, pi95, _) = compute_prediction_intervals(&data);
        assert!((pi95.0 + 2.85).abs() < 0.05, "95% PI lower: {}", pi95.0);
        assert!((pi95.1 - 2.85).abs() < 0.05, "95% PI upper: {}", pi95.1);
    }

    #[test]
    fn test_bootstrap_mean_ci_deterministic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let ci1 = compute_bootstrap_mean_ci(&data, 500, 42);
        let ci2 = compute_bootstrap_mean_ci(&data, 500, 42);
        assert!((ci1.0 - ci2.0).abs() < 1e-9, "deterministic bootstrap");
        // Mean of 1..10 = 5.5. CI should bracket it.
        assert!(ci1.0 < 5.5 && ci1.1 > 5.5, "CI must bracket mean");
    }

    #[test]
    fn test_historical_reliability_perfect() {
        let mut log = VecDeque::new();
        for _ in 0..20 {
            log.push_back((0.9, true)); // high probability, all correct
        }
        let r = compute_historical_reliability(&log);
        assert!(r > 0.9, "perfect predictions: {r}");
    }

    #[test]
    fn test_historical_reliability_bad() {
        let mut log = VecDeque::new();
        for _ in 0..20 {
            log.push_back((0.9, false)); // high probability, all wrong
        }
        let r = compute_historical_reliability(&log);
        assert!(r < 0.1, "bad predictions: {r}");
    }

    #[test]
    fn test_confidence_score_range() {
        let score = confidence_score(&(-1.0, 1.0), &(-0.5, 0.5), 0.8, 50);
        assert!(score >= 0.0 && score <= 100.0);
    }

    #[test]
    fn test_recording_outcome_capacity() {
        let mut engine = ConfidenceEngine::new();
        for i in 0..600 {
            engine.record_outcome(0.5 + i as f64 * 0.001, i % 2 == 0);
        }
        // Should be capped.
        assert!(engine.reliability_log_len() <= 500);
    }
}
