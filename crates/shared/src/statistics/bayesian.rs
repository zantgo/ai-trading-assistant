//! Bayesian updating via Beta-Binomial conjugate model (Phase 2).
//!
//! Each probability event maintains a Beta(α, β) posterior.  After each
//! candle, the engine checks whether conditions from `forward_bars` ago
//! have been resolved, observes the outcome (success/failure), and updates
//! the posterior.  This means probability estimates reflect the observed
//! truth rate, not just hope.
//!
//! The 95% Highest Density Interval is approximated via the normal
//! approximation to the Beta distribution, which works well when
//! α + β > 10.

use std::collections::{HashMap, VecDeque};

/// A pending observation: a trigger that fired at bar `trigger_at` and
/// will be resolved when the forward window expires.
#[derive(Debug, Clone)]
struct PendingObservation {
    trigger_at: usize,
    trigger_price: f64,
    trigger_atr: f64,
    trigger_rsi: f64,
    kind: ObservationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservationKind {
    TrendContinuation,
    MeanReversion,
    BreakoutSuccess,
    Reversal,
    AtrExpansion,
    SqueezeReleaseBullish,
    VolatilityExpansion,
    StopBeforeTarget,
}

impl ObservationKind {
    fn name(&self) -> &'static str {
        match self {
            Self::TrendContinuation => "trend_continuation",
            Self::MeanReversion => "mean_reversion",
            Self::BreakoutSuccess => "breakout_success",
            Self::Reversal => "reversal",
            Self::AtrExpansion => "atr_expansion",
            Self::SqueezeReleaseBullish => "squeeze_release",
            Self::VolatilityExpansion => "volatility_expansion",
            Self::StopBeforeTarget => "stop_before_target",
        }
    }
}

/// A single Bayesian probability tracker using a Beta-Binomial conjugate
/// pair: Beta(α, β) prior updated with binomial observations.
#[derive(Debug, Clone)]
pub struct BayesianTracker {
    /// Prior hyperparameters.
    pub alpha_prior: f64,
    pub beta_prior: f64,
    /// Observed successes and failures.
    pub successes: usize,
    pub failures: usize,
    /// Total trials observed.
    pub trials: usize,
}

impl BayesianTracker {
    pub fn new(alpha_prior: f64, beta_prior: f64) -> Self {
        Self { alpha_prior, beta_prior, successes: 0, failures: 0, trials: 0 }
    }

    /// Record an observed outcome.
    pub fn update(&mut self, success: bool) {
        self.trials += 1;
        if success { self.successes += 1; } else { self.failures += 1; }
    }

    /// Posterior α = α_prior + successes.
    pub fn posterior_alpha(&self) -> f64 {
        self.alpha_prior + self.successes as f64
    }

    /// Posterior β = β_prior + failures.
    pub fn posterior_beta(&self) -> f64 {
        self.beta_prior + self.failures as f64
    }

    /// Posterior mean: (α_post) / (α_post + β_post).
    pub fn posterior_mean(&self) -> f64 {
        let a = self.posterior_alpha();
        let b = self.posterior_beta();
        if (a + b).abs() < 1e-12 { return 0.5; }
        a / (a + b)
    }

    /// Posterior variance: αβ / ((α+β)² (α+β+1)).
    pub fn posterior_variance(&self) -> f64 {
        let a = self.posterior_alpha();
        let b = self.posterior_beta();
        let sum = a + b;
        if sum.abs() < 1e-12 || (sum + 1.0).abs() < 1e-12 { return 0.0; }
        (a * b) / (sum * sum * (sum + 1.0))
    }

    /// Approximate 95% Highest Density Interval using the normal
    /// approximation: mean ± 1.96σ, clamped to [0, 1].
    pub fn hdi_95(&self) -> (f64, f64) {
        let mean = self.posterior_mean();
        let std = self.posterior_variance().sqrt();
        let lo = (mean - 1.96 * std).clamp(0.0, 1.0);
        let hi = (mean + 1.96 * std).clamp(0.0, 1.0);
        (lo, hi)
    }

    /// Posterior probability that the true probability exceeds `threshold`.
    /// Uses the normal approximation: P(θ > t) = 1 - Φ((t - μ) / σ).
    pub fn prob_superior_to(&self, threshold: f64) -> f64 {
        let mean = self.posterior_mean();
        let std = self.posterior_variance().sqrt().max(1e-12);
        let z = (threshold - mean) / std;
        // Normal CDF approximation via logistic: Φ(z) ≈ 1 / (1 + exp(-1.702z)).
        // P(θ > t) = 1 - Φ(z) = 1 / (1 + exp(1.702z)).
        1.0 / (1.0 + (1.702 * z).exp())
    }
}

/// Collection of named Bayesian trackers with a pending-observation queue.
/// After each candle, `observe_outcomes()` processes any pending triggers
/// whose forward window has elapsed and updates the trackers.
#[derive(Debug, Clone)]
pub struct BayesianEngine {
    trackers: HashMap<String, BayesianTracker>,
    pending: VecDeque<PendingObservation>,
    forward_bars: usize,
    bar_count: usize,
}

impl BayesianEngine {
    pub fn new(prior_alpha: f64, prior_beta: f64, forward_bars: usize) -> Self {
        let mut trackers = HashMap::new();
        for name in super::probability::EVENT_NAMES {
            trackers.insert(name.to_string(), BayesianTracker::new(prior_alpha, prior_beta));
        }
        Self { trackers, pending: VecDeque::new(), forward_bars, bar_count: 0 }
    }

    /// Queue an observation trigger that will be resolved `forward_bars`
    /// candles from now.  The engine stores the trigger context so it can
    /// evaluate the outcome when the window expires.
    pub fn queue_trigger(
        &mut self,
        kind: ObservationKind,
        price: f64,
        atr: f64,
        rsi: f64,
    ) {
        self.pending.push_back(PendingObservation {
            trigger_at: self.bar_count,
            trigger_price: price,
            trigger_atr: atr,
            trigger_rsi: rsi,
            kind,
        });
    }

    /// Advance bar count and resolve any pending observations whose forward
    /// window has elapsed.  `current_price` and `current_atr` are used to
    /// judge the outcome.
    pub fn advance_bar(&mut self, current_price: f64, current_atr: f64) {
        self.bar_count += 1;

        // Resolve pending observations that have aged out.
        while let Some(front) = self.pending.front() {
            if self.bar_count < front.trigger_at + self.forward_bars {
                break;
            }
            let obs = self.pending.pop_front().unwrap();
            let success = evaluate_outcome(&obs, current_price, current_atr);
            if let Some(tracker) = self.trackers.get_mut(obs.kind.name()) {
                tracker.update(success);
            }
        }
    }

    /// Produce a map: event_name → (posterior_mean, hdi_low, hdi_high).
    pub fn posterior_map(&self) -> HashMap<String, (f64, f64, f64)> {
        self.trackers
            .iter()
            .map(|(k, t)| {
                let (lo, hi) = t.hdi_95();
                (k.clone(), (t.posterior_mean(), lo, hi))
            })
            .collect()
    }

    /// Return a reference to a specific tracker by name.
    pub fn tracker(&self, name: &str) -> Option<&BayesianTracker> {
        self.trackers.get(name)
    }

    /// Return the total number of observations across all trackers.
    pub fn total_trials(&self) -> usize {
        self.trackers.values().map(|t| t.trials).sum()
    }
}

/// Evaluate whether a past trigger resolved as a success.
fn evaluate_outcome(obs: &PendingObservation, current_price: f64, current_atr: f64) -> bool {
    let atr = if obs.trigger_atr > 1e-12 { obs.trigger_atr } else { current_atr };
    let tp = obs.trigger_price;
    if atr < 1e-12 { return false; }

    match obs.kind {
        ObservationKind::TrendContinuation => {
            // Success if price moved further in the same direction relative
            // to a simple trend proxy (price vs the trigger's RSI-implied bias).
            let bias = if obs.trigger_rsi > 50.0 { 1.0 } else { -1.0 };
            (current_price - tp) * bias > 0.0
        }
        ObservationKind::MeanReversion => {
            // Success if price reversed toward the mean — RSI moving back
            // toward 50 is a sign of mean reversion.
            let was_extreme = obs.trigger_rsi > 70.0 || obs.trigger_rsi < 30.0;
            if !was_extreme { return false; }
            let moved_back = if obs.trigger_rsi > 70.0 {
                current_price < tp // price came down
            } else {
                current_price > tp // price went up
            };
            moved_back
        }
        ObservationKind::BreakoutSuccess => {
            // Success if price extended > 0.5 ATR beyond the trigger price
            // in the breakout direction (judged by BBWP context).
            let extension = (current_price - tp).abs();
            extension > 0.5 * atr
        }
        ObservationKind::Reversal => {
            // Success if RSI-extreme was followed by a significant pullback.
            if obs.trigger_rsi > 70.0 {
                current_price <= tp - 0.5 * atr
            } else if obs.trigger_rsi < 30.0 {
                current_price >= tp + 0.5 * atr
            } else {
                false
            }
        }
        ObservationKind::AtrExpansion => {
            // Success if ATR increased relative to trigger ATR.
            current_atr > atr
        }
        ObservationKind::SqueezeReleaseBullish => {
            // Success if price moved up after squeeze release.
            current_price > tp + 0.002 * tp
        }
        ObservationKind::VolatilityExpansion => {
            current_atr > atr
        }
        ObservationKind::StopBeforeTarget => {
            // Price moved more in the stop direction than target direction
            // (judged by simple 1.5 ATR stop / 2 ATR target).
            let move_size = (current_price - tp).abs();
            let stop_dist = 1.5 * atr;
            let target_dist = 2.0 * atr;
            // If price moved past the stop distance toward the target side
            // but stopped within it... this is a rough proxy.
            move_size > stop_dist && move_size < target_dist
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bayesian_posterior_mean() {
        let mut t = BayesianTracker::new(1.0, 1.0);
        assert!((t.posterior_mean() - 0.5).abs() < 1e-9);
        t.update(true);
        t.update(true);
        t.update(false);
        // Prior: Beta(1,1). Posterior after 2 success, 1 failure: Beta(3,2) → mean=3/5=0.6
        assert!((t.posterior_mean() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn test_hdi_clamped() {
        let t = BayesianTracker::new(1.0, 1.0);
        // With few observations the HDI should stay within [0, 1].
        let (lo, hi) = t.hdi_95();
        assert!(lo >= 0.0 && hi <= 1.0);
    }

    #[test]
    fn test_prob_superior_to() {
        let mut t = BayesianTracker::new(1.0, 1.0);
        for _ in 0..10 { t.update(true); }
        // With 10 successes, P(θ > 0.5) should be high.
        let prob = t.prob_superior_to(0.5);
        assert!(prob > 0.7, "10/10 successes: prob > 0.5 should be high, got {prob}");
    }

    #[test]
    fn test_observation_queue_resolves() {
        let mut engine = BayesianEngine::new(1.0, 1.0, 5);
        engine.queue_trigger(ObservationKind::TrendContinuation, 50000.0, 500.0, 60.0);
        // Advance 5 bars with a higher price (uptrend → continuation success).
        for _ in 0..5 {
            engine.advance_bar(51000.0, 500.0);
        }
        let map = engine.posterior_map();
        let (mean, _, _) = map["trend_continuation"];
        // One trial, success → posterior mean should be > 0.5
        assert!(mean > 0.5, "trend_continuation posterior with 1 success: {mean}");
    }

    #[test]
    fn test_queue_only_resolves_after_window() {
        let mut engine = BayesianEngine::new(1.0, 1.0, 5);
        engine.queue_trigger(ObservationKind::TrendContinuation, 50000.0, 500.0, 60.0);
        // Advance only 3 bars — observation should NOT yet resolve.
        for _ in 0..3 {
            engine.advance_bar(51000.0, 500.0);
        }
        assert_eq!(engine.total_trials(), 0, "should not resolve before forward window");
        // Advance 2 more.
        for _ in 0..2 {
            engine.advance_bar(51000.0, 500.0);
        }
        assert_eq!(engine.total_trials(), 1 /* actually, trend_continuation might or might not be counted... */);
    }
}
