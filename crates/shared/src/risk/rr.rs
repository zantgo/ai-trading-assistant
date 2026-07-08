//! Adaptive Reward/Risk recommendation — pure mathematics (Section 12).
//!
//! Given realized win/loss counts, produces a data-driven reward multiple `R`
//! (expressed as the ratio `1 : R`) that keeps long-run expectancy positive.
//! Win rate is Beta-smoothed so the recommendation starts exactly at the 50%
//! baseline (1:1) and updates smoothly as evidence accumulates.

use serde::{Deserialize, Serialize};

/// Beta-smoothed win-rate estimate.
///
/// `W_est = (a0 + wins) / (a0 + b0 + wins + losses)`.
/// With the default prior `a0 = b0 = 5` this encodes the "5 of 10" neutral
/// anchor: zero observations → exactly 0.5.
pub fn win_rate_beta(wins: u32, losses: u32, prior_wins: f64, prior_losses: f64) -> f64 {
    let a = prior_wins + wins as f64;
    let b = prior_losses + losses as f64;
    let denom = a + b;
    if denom <= 0.0 {
        return 0.5;
    }
    (a / denom).clamp(0.0, 1.0)
}

/// Breakeven reward multiple `R = (1 - W) / W` (per-trade expectancy = 0).
/// Returns a large finite value as `W -> 0` to avoid division blow-ups.
pub fn breakeven_ratio(win_rate: f64) -> f64 {
    let w = win_rate.clamp(1e-6, 1.0 - 1e-6);
    (1.0 - w) / w
}

/// Recommended reward multiple `R = k * (1 - W) / W` with safety margin `k > 1`,
/// guaranteeing positive per-trade expectancy `E = (k - 1)(1 - W) > 0`.
pub fn recommended_ratio(win_rate: f64, safety_margin: f64) -> f64 {
    breakeven_ratio(win_rate) * safety_margin.max(1.0)
}

/// Confidence in the recommendation from the Beta posterior variance.
/// `Var = a*b / ((a+b)^2 (a+b+1))`, `confidence = clamp(1 - 2*sqrt(Var), 0, 1)`.
pub fn rr_confidence(wins: u32, losses: u32, prior_wins: f64, prior_losses: f64) -> f64 {
    let a = prior_wins + wins as f64;
    let b = prior_losses + losses as f64;
    let n = a + b;
    if n <= 0.0 {
        return 0.0;
    }
    let var = (a * b) / (n * n * (n + 1.0));
    (1.0 - 2.0 * var.sqrt()).clamp(0.0, 1.0)
}

/// The complete adaptive R:R recommendation for a pair (Section 12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RewardRiskRecommendation {
    /// Beta-smoothed win-rate estimate `[0,1]`.
    pub win_rate_estimate: f64,
    /// Breakeven reward multiple `(1-W)/W`.
    pub breakeven_ratio: f64,
    /// Recommended reward multiple `k*(1-W)/W` (positive-expectancy margin).
    pub recommended_ratio: f64,
    /// Confidence from posterior variance `[0,1]`.
    pub confidence: f64,
    /// Number of realized trades in the evaluation window.
    pub sample_size: u32,
}

impl Default for RewardRiskRecommendation {
    fn default() -> Self {
        // Zero observations → neutral 50% / 1:1 anchor.
        Self {
            win_rate_estimate: 0.5,
            breakeven_ratio: 1.0,
            recommended_ratio: 1.25,
            confidence: 0.0,
            sample_size: 0,
        }
    }
}

impl RewardRiskRecommendation {
    /// Derive the recommendation from realized wins/losses and the priors.
    pub fn compute(
        wins: u32,
        losses: u32,
        prior_wins: f64,
        prior_losses: f64,
        safety_margin: f64,
    ) -> Self {
        let w = win_rate_beta(wins, losses, prior_wins, prior_losses);
        Self {
            win_rate_estimate: w,
            breakeven_ratio: breakeven_ratio(w),
            recommended_ratio: recommended_ratio(w, safety_margin),
            confidence: rr_confidence(wins, losses, prior_wins, prior_losses),
            sample_size: wins + losses,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn breakeven_matches_spec_table() {
        assert!(approx(breakeven_ratio(0.50), 1.00));
        assert!(approx(breakeven_ratio(0.70), 0.4286));
        assert!(approx(breakeven_ratio(0.60), 0.6667));
        assert!(approx(breakeven_ratio(0.40), 1.50));
        assert!(approx(breakeven_ratio(0.30), 2.3333));
        assert!(approx(breakeven_ratio(0.20), 4.00));
    }

    #[test]
    fn block_zero_is_exactly_one_to_one() {
        let w = win_rate_beta(0, 0, 5.0, 5.0);
        assert!(approx(w, 0.5));
        assert!(approx(breakeven_ratio(w), 1.0));
    }

    #[test]
    fn beta_smoothing_matches_spec_examples() {
        // 7W/3L -> 12/20 = 0.6
        assert!(approx(win_rate_beta(7, 3, 5.0, 5.0), 0.6));
        // 3W/7L -> 8/20 = 0.4
        assert!(approx(win_rate_beta(3, 7, 5.0, 5.0), 0.4));
        // 70W/30L -> 75/110
        assert!(approx(win_rate_beta(70, 30, 5.0, 5.0), 75.0 / 110.0));
        // 30W/70L -> 35/110
        assert!(approx(win_rate_beta(30, 70, 5.0, 5.0), 35.0 / 110.0));
    }

    #[test]
    fn recommended_yields_positive_expectancy() {
        // E = W*R - (1-W) must be > 0 for k > 1 across win rates.
        for &w in &[0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8] {
            let r = recommended_ratio(w, 1.25);
            let e = w * r - (1.0 - w);
            assert!(e > 0.0, "expectancy not positive for W={w}: E={e}");
            // Closed form: E = (k-1)(1-W) = 0.25*(1-W)
            assert!(approx(e, 0.25 * (1.0 - w)));
        }
    }

    #[test]
    fn confidence_grows_with_sample() {
        let c_small = rr_confidence(1, 1, 5.0, 5.0);
        let c_large = rr_confidence(200, 200, 5.0, 5.0);
        assert!(c_large > c_small);
        assert!((0.0..=1.0).contains(&c_small));
        assert!((0.0..=1.0).contains(&c_large));
    }
}
