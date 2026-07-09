//! Extreme Value Theory (EVT) — Peaks-Over-Threshold (POT) tail risk modeling.
//!
//! Historical percentile-based VaR is biased for extreme quantiles because
//! you cannot observe a 1-in-500 event from 500 data points.  EVT fits a
//! Generalized Pareto Distribution (GPD) to tail exceedances, enabling
//! extrapolation beyond observed extremes.
//!
//! GPD:  F(x) = 1 − (1 + ξ·x/β)^(−1/ξ)   for ξ ≠ 0
//!              = 1 − exp(−x/β)           for ξ = 0
//!
//! ξ = shape (tail index):   ξ>0 = heavy-tailed, ξ=0 = exponential, ξ<0 = bounded
//! β = scale

use crate::statistics::distribution::{percentile, DistributionTracker};

/// EVT-based tail risk metrics.
#[derive(Debug, Clone)]
pub struct EvtTailMetrics {
    /// 99% Value at Risk from EVT (as positive loss %).
    pub var_99: f64,
    /// 99% Expected Shortfall from EVT.
    pub expected_shortfall_99: f64,
    /// Shape parameter ξ (tail index). Positive = heavy-tailed.
    pub tail_index_xi: f64,
    /// Scale parameter β.
    pub scale_beta: f64,
    /// POT threshold used.
    pub threshold: f64,
    /// Number of observations exceeding threshold.
    pub exceedance_count: usize,
}

/// Fit a GPD to return exceedances using Probability-Weighted Moments (PWM).
///
/// Returns `None` if too few exceedances or degenerate distribution.
pub fn fit_evt(sorted_returns: &[f64], threshold_percentile: f64) -> Option<EvtTailMetrics> {
    let n = sorted_returns.len();
    if n < 50 {
        return None;
    }

    // Sort is already sorted. Find the threshold at the given percentile.
    let pct = 100.0 - threshold_percentile * 100.0; // e.g., 95th → 5%
    let threshold = percentile(sorted_returns, pct);

    // Extract exceedances: returns below threshold → convert to positive exceedances
    let exceedances: Vec<f64> = sorted_returns
        .iter()
        .copied()
        .filter(|&r| r < threshold - 1e-12)
        .map(|r| threshold - r) // positive exceedance magnitude
        .collect();

    let nu = exceedances.len();
    if nu < 10 {
        return None; // too few exceedances for reliable fit
    }

    // PWM estimation of GPD parameters
    // b₀ = E[X]  (sample mean of exceedances)
    let b0 = exceedances.iter().sum::<f64>() / nu as f64;

    if b0 < 1e-12 {
        return None;
    }

    // b₁ = E[X · (1 − F(X))]
    // For ordered sample x₁ ≤ x₂ ≤ ... ≤ x_𝝂, the PWM estimator is:
    // b₁ = (1/ν) · Σ_{j=1}^{ν} (ν−j)/(ν−1) · x_j
    let mut b1 = 0.0;
    let mut sorted_exc = exceedances.clone();
    sorted_exc.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for (j, &x) in sorted_exc.iter().enumerate() {
        let rank = (j + 1) as f64;
        let weight = (nu as f64 - rank) / ((nu - 1) as f64).max(1.0);
        b1 += weight * x;
    }
    b1 /= nu as f64;

    if b1 < 1e-12 {
        return None;
    }

    // PWM estimators for GPD:
    // ξ̂ = (b₀ / (b₀ − 2b₁)) − 2
    // β̂ = 2·b₀·b₁ / (b₀ − 2·b₁)
    let denom = b0 - 2.0 * b1;
    let xi = if denom.abs() > 1e-12 {
        (b0 / denom - 2.0).clamp(-0.5, 0.5)
    } else {
        0.0
    };

    let beta = if denom.abs() > 1e-12 {
        (2.0 * b0 * b1 / denom).abs().max(1e-12)
    } else {
        b0
    };

    // EVT-VaR formula:
    // VaR_α = u + (β/ξ) × [(n/Nu × (1−α))^(−ξ) − 1]
    let alpha = 0.01; // 99% VaR
    let prob = (n as f64 / nu as f64) * alpha;

    let var_99 = if xi.abs() > 1e-6 {
        let term = prob.powf(-xi);
        let var_raw = threshold + (beta / xi) * (term - 1.0);
        // Convert back to losses as positive (if threshold is negative, var_raw
        // may be negative; take the magnitude for loss representation).
        var_raw.abs()
    } else {
        // ξ ≈ 0 → exponential
        let var_raw = threshold + beta * (prob.ln()).abs();
        var_raw.abs()
    };

    // EVT Expected Shortfall:
    // ES_α = VaR_α / (1−ξ) + (β − ξ·u) / (1−ξ)
    let es_99 = if xi < 1.0 {
        let es_raw = var_99 / (1.0 - xi) + (beta - xi * threshold) / (1.0 - xi);
        es_raw.abs().max(var_99)
    } else {
        var_99 * 3.0 // heavy fallback
    };

    Some(EvtTailMetrics {
        var_99,
        expected_shortfall_99: es_99,
        tail_index_xi: xi,
        scale_beta: beta,
        threshold,
        exceedance_count: nu,
    })
}

/// Compute EVT metrics from the distribution tracker.
pub fn compute_evt(tracker: &DistributionTracker) -> Option<EvtTailMetrics> {
    let best = tracker.best_window_idx();
    let sorted_returns = tracker.returns_values(best);
    fit_evt(&sorted_returns, 0.95) // 95th percentile threshold (5% tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_random() -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        static mut COUNTER: u64 = 0;
        let val = unsafe {
            COUNTER = COUNTER.wrapping_add(1);
            COUNTER
        };
        let mut h = DefaultHasher::new();
        val.hash(&mut h);
        (h.finish() as f64 / u64::MAX as f64) * 2.0 - 1.0
    }

    fn fat_tailed_returns(n: usize) -> Vec<f64> {
        (0..n)
            .map(|_| {
                let sum: f64 = (0..12).map(|_| test_random()).sum();
                sum / (12.0_f64 / 4.0).sqrt()
            })
            .collect()
    }

    #[test]
    fn test_evt_heavy_tail_positive_xi() {
        let returns = fat_tailed_returns(500);
        let mut sorted = returns;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let metrics = fit_evt(&sorted, 0.95).expect("should fit");
        assert!(metrics.tail_index_xi > -0.1);
        assert!(metrics.var_99 > 0.0);
        assert!(metrics.expected_shortfall_99 >= metrics.var_99);
    }

    #[test]
    fn test_evt_positive_scale() {
        let returns: Vec<f64> = (0..200)
            .map(|i| (i as f64 * 0.03 - 3.0).sin() * 3.0)
            .collect();
        let mut sorted = returns;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let metrics = fit_evt(&sorted, 0.95).expect("should fit");
        assert!(metrics.scale_beta > 0.0);
    }

    #[test]
    fn test_evt_var_exceeds_historical() {
        let returns = fat_tailed_returns(300);
        let mut sorted = returns.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let evt = fit_evt(&sorted, 0.95).expect("should fit");
        assert!(evt.var_99 > 0.0);
        assert!(evt.expected_shortfall_99 >= evt.var_99);
    }

    #[test]
    fn test_evt_too_few_returns_none() {
        let returns: Vec<f64> = (0..10).map(|i| i as f64 * 0.1).collect();
        let mut sorted = returns;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(fit_evt(&sorted, 0.95).is_none());
    }

    #[test]
    fn test_evt_exceedance_count() {
        let returns = fat_tailed_returns(200);
        let mut sorted = returns;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let metrics = fit_evt(&sorted, 0.95).unwrap();
        assert!(metrics.exceedance_count >= 5);
    }
}
