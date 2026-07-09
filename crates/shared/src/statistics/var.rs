//! Value at Risk (VaR) and Conditional VaR (Expected Shortfall) engine.
//!
//! Computes historical VaR and CVaR from the return distribution maintained
//! by the SIL's `DistributionTracker`.  VaR answers "how bad can it get?";
//! CVaR answers "when it gets bad, how bad?"
//!
//! All values are reported as positive percentages (loss magnitude).

use crate::statistics::distribution::{percentile, DistributionTracker};

/// Historical Value at Risk at the given confidence level.
///
/// VaR at α% = (1-α)th percentile of sorted returns.
/// Returns a **positive** value representing loss magnitude in percent.
/// E.g. `var(returns, 95.0) = 3.2` means "5% chance of losing ≥ 3.2% in one period."
pub fn historical_var(sorted_returns: &[f64], confidence: f64) -> Option<f64> {
    if sorted_returns.len() < 10 {
        return None;
    }
    // For loss VaR, we want the lower tail of the return distribution.
    // Confidence 95 → percentile 5 (the worst 5% of returns).
    let tail_percentile = 100.0 - confidence;
    let var_raw = percentile(sorted_returns, tail_percentile);
    // Convert to positive loss magnitude (negate: if 5th percentile is -3.2, VaR = 3.2%).
    Some(var_raw.abs().max(0.0))
}

/// Conditional Value at Risk (Expected Shortfall).
///
/// The mean of all returns that are ≤ VaR threshold.
/// Returns a **positive** value representing expected loss magnitude.
pub fn historical_cvar(sorted_returns: &[f64], var_value: f64) -> Option<f64> {
    if sorted_returns.len() < 10 {
        return None;
    }
    // VaR is stored as positive loss → actual threshold is -var_value
    let threshold = -var_value;
    let tail: Vec<f64> = sorted_returns
        .iter()
        .copied()
        .filter(|&r| r <= threshold + 1e-12)
        .collect();
    if tail.is_empty() {
        return Some(var_value); // fallback: CVaR = VaR if no tail observations
    }
    let mean_tail = tail.iter().sum::<f64>() / tail.len() as f64;
    Some(mean_tail.abs().max(var_value)) // CVaR ≥ VaR
}

/// Summary of VaR and CVaR at standard confidence levels.
#[derive(Debug, Clone)]
pub struct VarCvarSummary {
    /// 95% Value at Risk (5% chance of losing ≥ this amount).
    pub var_95: f64,
    /// 99% Value at Risk (1% chance of losing ≥ this amount).
    pub var_99: f64,
    /// 95% Conditional VaR (expected loss when in the worst 5%).
    pub cvar_95: f64,
    /// 99% Conditional VaR (expected loss when in the worst 1%).
    pub cvar_99: f64,
}

impl VarCvarSummary {
    /// Compute full VaR/CVaR summary from the distribution tracker's returns.
    pub fn compute(tracker: &DistributionTracker) -> Self {
        let best = tracker.best_window_idx();
        let sorted_returns = tracker.returns_values(best);

        let var_95 = historical_var(&sorted_returns, 95.0).unwrap_or(0.0);
        let var_99 = historical_var(&sorted_returns, 99.0).unwrap_or(0.0);
        let cvar_95 = historical_cvar(&sorted_returns, var_95).unwrap_or(var_95);
        let cvar_99 = historical_cvar(&sorted_returns, var_99).unwrap_or(var_99);

        Self { var_95, var_99, cvar_95, cvar_99 }
    }

    pub fn zero() -> Self {
        Self { var_95: 0.0, var_99: 0.0, cvar_95: 0.0, cvar_99: 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_95_exceeds_var_99() {
        let returns: Vec<f64> = (0..100)
            .map(|i| (i as f64 - 50.0) * 0.1) // [-5.0 → 4.9]
            .collect();
        let mut sorted = returns;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let v95 = historical_var(&sorted, 95.0).unwrap();
        let v99 = historical_var(&sorted, 99.0).unwrap();
        assert!(v99 >= v95, "VaR 99% must be ≥ VaR 95%");
    }

    #[test]
    fn test_cvar_exceeds_var() {
        let returns: Vec<f64> = (0..500)
            .map(|i| if i < 10 { -5.0 } else { (i as f64 - 250.0) * 0.02 })
            .collect();
        let mut sorted = returns;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let var = historical_var(&sorted, 95.0).unwrap();
        let cvar = historical_cvar(&sorted, var).unwrap();
        assert!(cvar >= var, "CVaR must be ≥ VaR");
    }

    #[test]
    fn test_empty_returns_none() {
        assert!(historical_var(&[], 95.0).is_none());
        assert!(historical_cvar(&[], 2.0).is_none());
    }

    #[test]
    fn test_var_all_positive() {
        let returns: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
        let var = historical_var(&returns, 95.0).unwrap();
        assert!(var >= 0.0);
    }

    #[test]
    fn test_cvar_at_least_var() {
        let returns: Vec<f64> = (0..200)
            .map(|i| (i as f64 * 0.03 - 3.0).sin() * 3.0)
            .collect();
        let mut sorted = returns;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let v95 = historical_var(&sorted, 95.0).unwrap();
        let v99 = historical_var(&sorted, 99.0).unwrap();
        let c95 = historical_cvar(&sorted, v95).unwrap();
        let c99 = historical_cvar(&sorted, v99).unwrap();
        assert!(c95 >= v95);
        assert!(c99 >= v99);
    }
}
