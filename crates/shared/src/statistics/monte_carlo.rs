//! Module F: Monte Carlo Price-Path Simulation (Phase 6).
//!
//! Simulates possible future price paths by resampling historical log-returns
//! with replacement.  Each path starts at the current price and evolves for N
//! steps.  Aggregate statistics report target-hit and stop-hit probabilities,
//! expected drawdown, and confidence ranges.
//!
//! The simulation is deterministic when seeded: same inputs produce same
//! outputs.  It runs synchronously — async orchestration is handled in the
//! engine crate during Phase 9.

/// Summary of a Monte Carlo simulation run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonteCarloPriceOutcome {
    /// Fraction of paths that reached target before stop.
    pub target_hit_prob: f64,
    /// Fraction of paths that reached stop before target.
    pub stop_hit_prob: f64,
    /// 95th percentile of maximum drawdown across paths (positive number, %).
    pub max_drawdown_95: f64,
    /// 95th percentile of maximum favorable excursion (%).
    pub max_favorable_excursion_95: f64,
    /// Mean final price movement (% of starting price).
    pub expected_movement: f64,
    /// 95th percentile of final outcomes (%).
    pub best_case: f64,
    /// 5th percentile of final outcomes (%).
    pub worst_case: f64,
    /// 50th percentile of final outcomes (%).
    pub median_outcome: f64,
    /// (5th, 95th) percentile interval of final outcomes (%).
    pub confidence_95_range: (f64, f64),
}

impl Default for MonteCarloPriceOutcome {
    fn default() -> Self {
        Self {
            target_hit_prob: 0.0,
            stop_hit_prob: 0.0,
            max_drawdown_95: 0.0,
            max_favorable_excursion_95: 0.0,
            expected_movement: 0.0,
            best_case: 0.0,
            worst_case: 0.0,
            median_outcome: 0.0,
            confidence_95_range: (0.0, 0.0),
        }
    }
}

impl MonteCarloPriceOutcome {
    /// Run a Kalman-drift-aware Monte Carlo simulation.
    ///
    /// Instead of resampling raw returns, this resamples from de-drifted
    /// residuals and adds back the Kalman-estimated drift.  Produces
    /// directionally-aware paths that respect the current trend estimate.
    ///
    /// # Arguments
    ///
    /// * `price`          — current close price
    /// * `atr`            — current ATR value
    /// * `drift`          — Kalman-estimated per-bar return in percent
    /// * `residuals`      — de-drifted return series (%) for resampling noise
    /// * `target_atr_mult`— target distance in ATR multiples
    /// * `stop_atr_mult`  — stop distance in ATR multiples
    /// * `num_paths`      — number of paths to simulate
    /// * `num_steps`      — forward bars per path
    /// * `seed`           — fixed seed for reproducibility
    pub fn compute_with_kalman(
        price: f64,
        atr: f64,
        drift: f64,
        residuals: &[f64],
        target_atr_mult: f64,
        stop_atr_mult: f64,
        num_paths: usize,
        num_steps: usize,
        seed: Option<u64>,
    ) -> Self {
        if residuals.len() < 3 || price < 1e-12 || atr < 1e-12 || num_paths == 0 || num_steps == 0 {
            return Self::compute(price, atr, residuals, target_atr_mult, stop_atr_mult, num_paths, num_steps, seed);
        }

        let atr_pct = atr / price;
        let target_price = price * (1.0 + target_atr_mult * atr_pct);
        let stop_price = price * (1.0 - stop_atr_mult * atr_pct);
        let bias_up = target_price > stop_price;

        let mut rng = SimpleRng::new(seed.unwrap_or(42));
        let n_residuals = residuals.len();

        let mut target_hits = 0usize;
        let mut stop_hits = 0usize;
        let mut max_drawdowns: Vec<f64> = Vec::with_capacity(num_paths);
        let mut max_fav_excs: Vec<f64> = Vec::with_capacity(num_paths);
        let mut final_outcomes: Vec<f64> = Vec::with_capacity(num_paths);

        for _ in 0..num_paths {
            let mut px = price;
            let mut peak = price;
            let mut trough = price;
            let mut hit_target = false;
            let mut hit_stop = false;

            for _ in 0..num_steps {
                let noise_idx = (rng.next_f64() * n_residuals as f64) as usize;
                let noise = residuals[noise_idx.min(n_residuals - 1)];
                // Step return = Kalman drift + resampled residual noise
                let step_ret = drift + noise;
                px = px * ((step_ret / 100.0).exp());
                if px > peak { peak = px; }
                if px < trough { trough = px; }

                if bias_up {
                    if px >= target_price { hit_target = true; break; }
                    if px <= stop_price { hit_stop = true; break; }
                } else {
                    if px <= target_price { hit_target = true; break; }
                    if px >= stop_price { hit_stop = true; break; }
                }
            }

            if hit_target { target_hits += 1; }
            else if hit_stop { stop_hits += 1; }

            let dd_pct = (peak - trough) / peak * 100.0;
            let mfe_pct = if hit_target || hit_stop {
                let reached = if hit_target { target_price } else { stop_price };
                (reached - price).abs() / price * 100.0
            } else {
                (peak - price).max(price - trough) / price * 100.0
            };
            let final_pct = (px - price) / price * 100.0;

            max_drawdowns.push(dd_pct);
            max_fav_excs.push(mfe_pct);
            final_outcomes.push(final_pct);
        }

        max_drawdowns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        max_fav_excs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        final_outcomes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx_05 = (0.05 * num_paths as f64) as usize;
        let idx_50 = (0.50 * num_paths as f64) as usize;
        let idx_95 = ((0.95 * num_paths as f64) as usize).min(num_paths - 1);

        let expected_movement = final_outcomes.iter().sum::<f64>() / num_paths as f64;

        Self {
            target_hit_prob: target_hits as f64 / num_paths as f64,
            stop_hit_prob: stop_hits as f64 / num_paths as f64,
            max_drawdown_95: max_drawdowns[idx_95],
            max_favorable_excursion_95: max_fav_excs[idx_95],
            expected_movement,
            best_case: final_outcomes[idx_95],
            worst_case: final_outcomes[idx_05],
            median_outcome: final_outcomes[idx_50],
            confidence_95_range: (final_outcomes[idx_05], final_outcomes[idx_95]),
        }
    }

    /// Run a standard (non-Kalman) Monte Carlo simulation of price paths.
    ///
    /// # Arguments
    ///
    /// * `price`          — current close price
    /// * `atr`            — current ATR value
    /// * `returns`        — historical log-returns (%) to resample from
    /// * `target_atr_mult`— target distance in ATR multiples (e.g. 2.0)
    /// * `stop_atr_mult`  — stop distance in ATR multiples (e.g. 1.5)
    /// * `num_paths`      — number of independent paths to simulate
    /// * `num_steps`      — number of forward bars per path
    /// * `seed`           — fixed seed for reproducibility; None for pseudo-entropy
    pub fn compute(
        price: f64,
        atr: f64,
        returns: &[f64],
        target_atr_mult: f64,
        stop_atr_mult: f64,
        num_paths: usize,
        num_steps: usize,
        seed: Option<u64>,
    ) -> Self {
        if returns.len() < 3 || price < 1e-12 || atr < 1e-12 || num_paths == 0 || num_steps == 0 {
            return Self::default();
        }

        let atr_pct = atr / price;
        let target_price = price * (1.0 + target_atr_mult * atr_pct);
        let stop_price = price * (1.0 - stop_atr_mult * atr_pct);
        // Also support bearish: target below, stop above.  We default to a
        // symmetric bullish simulation.  The actual direction is controlled by
        // the caller via the sign of target/stop offsets.
        let bias_up = target_price > stop_price; // true for standard long setup

        let mut rng = SimpleRng::new(seed.unwrap_or(42));
        let n_returns = returns.len();

        let mut target_hits = 0usize;
        let mut stop_hits = 0usize;
        let mut max_drawdowns: Vec<f64> = Vec::with_capacity(num_paths);
        let mut max_fav_excs: Vec<f64> = Vec::with_capacity(num_paths);
        let mut final_outcomes: Vec<f64> = Vec::with_capacity(num_paths);

        for _ in 0..num_paths {
            let mut px = price;
            let mut peak = price;
            let mut trough = price;
            let mut hit_target = false;
            let mut hit_stop = false;

            for _ in 0..num_steps {
                let idx = (rng.next_f64() * n_returns as f64) as usize;
                let ret = returns[idx.min(n_returns - 1)];
                // Price evolves multiplicatively: P_next = P * exp(r).
                // Returns are logged as percentage (e.g. 0.5 means 0.5%).
                px = px * ((ret / 100.0).exp());
                if px > peak { peak = px; }
                if px < trough { trough = px; }

                if bias_up {
                    if px >= target_price { hit_target = true; break; }
                    if px <= stop_price { hit_stop = true; break; }
                } else {
                    if px <= target_price { hit_target = true; break; }
                    if px >= stop_price { hit_stop = true; break; }
                }
            }

            if hit_target { target_hits += 1; }
            else if hit_stop { stop_hits += 1; }

            let dd_pct = (peak - trough) / peak * 100.0;
            let mfe_pct = if hit_target || hit_stop {
                // If we hit a boundary, the excursion is the distance to that boundary.
                let reached = if hit_target { target_price } else { stop_price };
                (reached - price).abs() / price * 100.0
            } else {
                (peak - price).max(price - trough) / price * 100.0
            };
            let final_pct = (px - price) / price * 100.0;

            max_drawdowns.push(dd_pct);
            max_fav_excs.push(mfe_pct);
            final_outcomes.push(final_pct);
        }

        max_drawdowns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        max_fav_excs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        final_outcomes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx_05 = (0.05 * num_paths as f64) as usize;
        let idx_50 = (0.50 * num_paths as f64) as usize;
        let idx_95 = ((0.95 * num_paths as f64) as usize).min(num_paths - 1);

        let expected_movement = final_outcomes.iter().sum::<f64>() / num_paths as f64;

        Self {
            target_hit_prob: target_hits as f64 / num_paths as f64,
            stop_hit_prob: stop_hits as f64 / num_paths as f64,
            max_drawdown_95: max_drawdowns[idx_95],
            max_favorable_excursion_95: max_fav_excs[idx_95],
            expected_movement,
            best_case: final_outcomes[idx_95],
            worst_case: final_outcomes[idx_05],
            median_outcome: final_outcomes[idx_50],
            confidence_95_range: (final_outcomes[idx_05], final_outcomes[idx_95]),
        }
    }
}

// ── Minimal deterministic PRNG ─────────────────────────────────

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_add(1) } // avoid zero state
    }

    fn next_u32(&mut self) -> u32 {
        // L'Ecuyer's multiplier
        self.state = self.state.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
        (self.state >> 32) as u32
    }

    fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / (u32::MAX as f64 + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns() {
        let outcome = MonteCarloPriceOutcome::compute(
            50000.0, 500.0, &[], 2.0, 1.5, 100, 10, Some(42),
        );
        assert_eq!(outcome.target_hit_prob, 0.0);
        assert_eq!(outcome.stop_hit_prob, 0.0);
    }

    #[test]
    fn test_deterministic_reproducible() {
        let returns: Vec<f64> = (-100..=100).map(|i| i as f64 / 10.0).collect();
        let o1 = MonteCarloPriceOutcome::compute(
            100.0, 2.0, &returns, 2.0, 1.5, 500, 20, Some(42),
        );
        let o2 = MonteCarloPriceOutcome::compute(
            100.0, 2.0, &returns, 2.0, 1.5, 500, 20, Some(42),
        );
        assert!((o1.target_hit_prob - o2.target_hit_prob).abs() < 1e-9);
        assert!((o1.median_outcome - o2.median_outcome).abs() < 1e-9);
    }

    #[test]
    fn test_uptrend_bias() {
        // Positive-biased returns should produce more target hits than stops.
        let returns: Vec<f64> = (0..100).map(|_| 0.5).collect(); // always +0.5%
        let outcome = MonteCarloPriceOutcome::compute(
            100.0, 2.0, &returns, 2.0, 1.5, 200, 10, Some(42),
        );
        // Target is 100*(1+2*0.02) = 104, stop = 100*(1-1.5*0.02) = 97
        // With +0.5% per step for 10 steps, we hit target quickly.
        assert!(outcome.target_hit_prob > 0.8, "uptrend favors target, got {}", outcome.target_hit_prob);
    }

    #[test]
    fn test_downtrend_bias() {
        let returns: Vec<f64> = (0..100).map(|_| -0.5).collect(); // always -0.5%
        let outcome = MonteCarloPriceOutcome::compute(
            100.0, 2.0, &returns, 2.0, 1.5, 200, 10, Some(42),
        );
        assert!(outcome.stop_hit_prob > 0.8, "downtrend favors stop, got {}", outcome.stop_hit_prob);
    }

    #[test]
    fn test_range_bounds() {
        let returns: Vec<f64> = (-50..=50).map(|i| i as f64 / 10.0).collect();
        let outcome = MonteCarloPriceOutcome::compute(
            50000.0, 500.0, &returns, 2.0, 1.5, 200, 30, Some(99),
        );
        assert!(outcome.target_hit_prob >= 0.0 && outcome.target_hit_prob <= 1.0);
        assert!(outcome.stop_hit_prob >= 0.0 && outcome.stop_hit_prob <= 1.0);
        assert!(outcome.max_drawdown_95 >= 0.0);
        assert!((outcome.target_hit_prob + outcome.stop_hit_prob) <= 1.0 + 1e-9);
    }

    #[test]
    fn test_median_between_worst_best() {
        let returns: Vec<f64> = (-100..=100).map(|i| i as f64 / 5.0).collect();
        let outcome = MonteCarloPriceOutcome::compute(
            100.0, 5.0, &returns, 3.0, 2.0, 300, 15, Some(7),
        );
        assert!(outcome.worst_case <= outcome.median_outcome);
        assert!(outcome.median_outcome <= outcome.best_case);
    }
}
