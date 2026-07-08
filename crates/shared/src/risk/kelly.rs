//! Kelly Criterion position sizing — pure mathematics (Portfolio Optimization).
//!
//! Given a strategy's win rate `W` and its reward/risk ratio `R` (average win
//! divided by average loss), the Kelly fraction gives the growth-optimal share
//! of capital to allocate. Full Kelly maximizes long-run log-growth but is
//! volatile; fractional Kelly (`lambda` in `(0,1]`) trades growth for a smoother
//! equity curve and robustness to estimation error.

/// Compute full Kelly fraction: `f* = W - (1-W)/R`
/// where `W` = win rate, `R` = reward/risk ratio (avg win / avg loss).
pub fn kelly_full(win_rate: f64, reward_risk_ratio: f64) -> f64 {
    win_rate - (1.0 - win_rate) / reward_risk_ratio.max(0.01)
}

/// Fractional Kelly: `f = lambda * f*`, clamped to `[0, 1]`.
pub fn kelly_fractional(win_rate: f64, reward_risk_ratio: f64, lambda: f64) -> f64 {
    (kelly_full(win_rate, reward_risk_ratio) * lambda).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn full_kelly_matches_formula() {
        // W = 0.6, R = 2.0 -> 0.6 - 0.4/2.0 = 0.4
        assert!(approx(kelly_full(0.6, 2.0), 0.4));
        // W = 0.5, R = 1.0 -> 0.5 - 0.5/1.0 = 0.0
        assert!(approx(kelly_full(0.5, 1.0), 0.0));
        // W = 0.7, R = 3.0 -> 0.7 - 0.3/3.0 = 0.6
        assert!(approx(kelly_full(0.7, 3.0), 0.6));
    }

    #[test]
    fn full_kelly_can_be_negative() {
        // A losing edge yields a negative fraction (do not bet).
        assert!(kelly_full(0.3, 1.0) < 0.0);
    }

    #[test]
    fn full_kelly_guards_zero_ratio() {
        // reward_risk_ratio floored at 0.01 -> no divide-by-zero / infinity.
        let f = kelly_full(0.5, 0.0);
        assert!(f.is_finite());
    }

    #[test]
    fn fractional_scales_full() {
        // Half Kelly of 0.4 = 0.2.
        assert!(approx(kelly_fractional(0.6, 2.0, 0.5), 0.2));
    }

    #[test]
    fn fractional_clamps_low() {
        // Negative edge clamps to 0.
        assert_eq!(kelly_fractional(0.3, 1.0, 0.5), 0.0);
    }

    #[test]
    fn fractional_clamps_high() {
        // lambda > 1 amplifying a strong edge clamps to 1.0.
        assert_eq!(kelly_fractional(0.9, 100.0, 2.0), 1.0);
    }

    #[test]
    fn fractional_within_unit_interval() {
        for &w in &[0.1, 0.3, 0.5, 0.7, 0.9] {
            for &r in &[0.5, 1.0, 2.0, 3.0] {
                let f = kelly_fractional(w, r, 0.5);
                assert!((0.0..=1.0).contains(&f), "out of range W={w} R={r} f={f}");
            }
        }
    }
}
