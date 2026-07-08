//! Module B: Empirical Probability Engine.
//!
//! Estimates probabilities from historical frequencies — scanning rolling
//! windows for trigger conditions and counting how often each outcome
//! followed.  Every probability comes from observation counts, not formulas.
//!
//! Fully implemented in Phase 2.

use std::collections::HashMap;

use crate::statistics::distribution::DistributionTracker;

/// Named event types tracked by the probability engine.
pub const EVENT_NAMES: &[&str] = &[
    "trend_continuation",
    "mean_reversion",
    "breakout_success",
    "reversal",
    "atr_expansion",
    "squeeze_release",
    "volatility_expansion",
    "stop_before_target",
];

/// Packed probability estimates for one snapshot.
#[derive(Debug, Clone)]
pub struct ProbabilitySnapshot {
    pub trend_continuation_prob: f64,
    pub mean_reversion_prob: f64,
    pub breakout_success_prob: f64,
    pub reversal_prob: f64,
    pub atr_expansion_prob: f64,
    pub squeeze_release_prob: f64,
    pub volatility_expansion_prob: f64,
    pub stop_before_target_prob: f64,
    pub observation_counts: HashMap<String, usize>,
}

impl Default for ProbabilitySnapshot {
    fn default() -> Self {
        let mut counts = HashMap::new();
        for name in EVENT_NAMES {
            counts.insert(name.to_string(), 0);
        }
        Self {
            trend_continuation_prob: 0.0,
            mean_reversion_prob: 0.0,
            breakout_success_prob: 0.0,
            reversal_prob: 0.0,
            atr_expansion_prob: 0.0,
            squeeze_release_prob: 0.0,
            volatility_expansion_prob: 0.0,
            stop_before_target_prob: 0.0,
            observation_counts: counts,
        }
    }
}

/// Engine that computes empirical probabilities by scanning historical
/// rolling windows.  The scan is O(n × forward_bars) per candle, bounded
/// by the window capacity (max 500).
#[derive(Debug, Clone)]
pub struct ProbabilityEngine {
    min_observations: usize,
    forward_bars: usize,
}

impl ProbabilityEngine {
    pub fn new(min_observations: usize, forward_bars: usize) -> Self {
        Self { min_observations, forward_bars }
    }

    /// Compute all probabilities using the given distribution tracker.
    /// Uses the best (longest warm) window.
    pub fn compute_all(&self, tracker: &DistributionTracker) -> ProbabilitySnapshot {
        let wi = tracker.best_window_idx();

        let prices = tracker.metric_values(wi, 0);
        let atrs = tracker.metric_values(wi, 2);
        let rsis = tracker.metric_values(wi, 3);
        let bbwps = tracker.metric_values(wi, 4);
        let squeeze_vals = tracker.metric_values(wi, 5);

        // Compute a simple SMA(20) from price as a trend proxy.
        let ema_proxy = sma(&prices, 20);

        let tc = self.compute_trend_continuation(&prices, &ema_proxy);
        let mr = self.compute_mean_reversion(&prices, &ema_proxy);
        let bo = self.compute_breakout_success(&prices, &atrs, &bbwps);
        let rv = self.compute_reversal(&rsis, &prices, &atrs);
        let ae = self.compute_atr_expansion(&atrs, &bbwps);
        let sq = self.compute_squeeze_release(&squeeze_vals, &prices);
        let ve = self.compute_volatility_expansion(&atrs, &bbwps);
        let st = self.compute_stop_before_target(&prices, &atrs);

        let mut obs = HashMap::new();
        obs.insert("trend_continuation".into(), tc.1);
        obs.insert("mean_reversion".into(), mr.1);
        obs.insert("breakout_success".into(), bo.1);
        obs.insert("reversal".into(), rv.1);
        obs.insert("atr_expansion".into(), ae.1);
        obs.insert("squeeze_release".into(), sq.1);
        obs.insert("volatility_expansion".into(), ve.1);
        obs.insert("stop_before_target".into(), st.1);

        ProbabilitySnapshot {
            trend_continuation_prob: tc.0,
            mean_reversion_prob: mr.0,
            breakout_success_prob: bo.0,
            reversal_prob: rv.0,
            atr_expansion_prob: ae.0,
            squeeze_release_prob: sq.0,
            volatility_expansion_prob: ve.0,
            stop_before_target_prob: st.0,
            observation_counts: obs,
        }
    }

    // ── Individual probability calculators ──────────────────────

    /// P(trend continuation): fraction of times price remained on the same
    /// side of the SMA(20) for `forward_bars` after having been there for
    /// 3 consecutive bars.
    fn compute_trend_continuation(&self, prices: &[f64], sma: &[f64]) -> (f64, usize) {
        let n = prices.len();
        if n < self.forward_bars + 3 { return (0.0, 0); }
        let mut trials = 0usize;
        let mut successes = 0usize;
        for i in 0..n.saturating_sub(self.forward_bars) {
            if sma[i].abs() < 1e-12 || i < 2 { continue; }
            let above = prices[i] > sma[i];
            let prev_above = prices[i - 1] > sma[i - 1];
            let prev2_above = prices[i - 2] > sma[i - 2];
            // Require 3 consecutive bars on the same side (established trend).
            if above != prev_above || above != prev2_above { continue; }
            trials += 1;
            let end = (i + self.forward_bars).min(n);
            let mut still_on_side = true;
            for j in i + 1..end {
                if sma[j].abs() < 1e-12 { still_on_side = false; break; }
                if (prices[j] > sma[j]) != above { still_on_side = false; break; }
            }
            if still_on_side { successes += 1; }
        }
        prob(successes, trials, self.min_observations)
    }

    /// P(mean reversion): fraction of times price returned to within N% of
    /// SMA(20) within `forward_bars` after deviating > 1.5 × rolling stddev.
    fn compute_mean_reversion(&self, prices: &[f64], sma: &[f64]) -> (f64, usize) {
        let n = prices.len();
        if n < self.forward_bars + 5 { return (0.0, 0); }
        let stddev = rolling_stddev(prices, 20);
        let mut trials = 0usize;
        let mut successes = 0usize;
        for i in 0..n.saturating_sub(self.forward_bars) {
            let s = sma[i];
            if s.abs() < 1e-12 || stddev[i].abs() < 1e-12 { continue; }
            let dev = (prices[i] - s).abs();
            if dev < 1.5 * stddev[i] { continue; } // not deviated enough
            trials += 1;
            let end = (i + self.forward_bars).min(n);
            let threshold = s * 0.005; // within 0.5% of SMA
            for j in i + 1..end {
                if sma[j].abs() < 1e-12 { break; }
                if (prices[j] - sma[j]).abs() < threshold { successes += 1; break; }
            }
        }
        prob(successes, trials, self.min_observations)
    }

    /// P(breakout success): fraction of times price extended > 0.5×ATR in
    /// the same direction after BBWP was above the 80th percentile AND the
    /// bar closed outside the recent high/low range.
    fn compute_breakout_success(&self, prices: &[f64], atrs: &[f64], bbwps: &[f64]) -> (f64, usize) {
        let n = prices.len();
        if n < self.forward_bars + 10 { return (0.0, 0); }
        let bbwp_80th = percentile_of_slice(bbwps, 80.0);
        let range_high = rolling_max(prices, 10);
        let range_low = rolling_min(prices, 10);
        let mut trials = 0usize;
        let mut successes = 0usize;
        for i in 10..n.saturating_sub(self.forward_bars) {
            if bbwps[i] < bbwp_80th { continue; }
            if atrs[i].abs() < 1e-12 { continue; }
            let breakout_up = prices[i] > range_high[i - 1];
            let breakout_down = prices[i] < range_low[i - 1];
            if !breakout_up && !breakout_down { continue; }
            trials += 1;
            let atr = atrs[i];
            let target = if breakout_up { prices[i] + 0.5 * atr } else { prices[i] - 0.5 * atr };
            let end = (i + self.forward_bars).min(n);
            for j in i + 1..end {
                if breakout_up && prices[j] >= target { successes += 1; break; }
                if breakout_down && prices[j] <= target { successes += 1; break; }
            }
        }
        prob(successes, trials, self.min_observations)
    }

    /// P(reversal): fraction of times RSI > 70 was followed by > 0.5×ATR
    /// pullback within `forward_bars`.  Symmetric for RSI < 30.
    fn compute_reversal(&self, rsis: &[f64], prices: &[f64], atrs: &[f64]) -> (f64, usize) {
        let n = rsis.len();
        if n < self.forward_bars + 1 { return (0.0, 0); }
        let mut trials = 0usize;
        let mut successes = 0usize;
        for i in 0..n.saturating_sub(self.forward_bars) {
            if atrs[i].abs() < 1e-12 { continue; }
            let overbought = rsis[i] > 70.0;
            let oversold = rsis[i] < 30.0;
            if !overbought && !oversold { continue; }
            trials += 1;
            let atr = atrs[i];
            let end = (i + self.forward_bars).min(n);
            for j in i + 1..end {
                let pullback = if overbought {
                    prices[j] <= prices[i] - 0.5 * atr
                } else {
                    prices[j] >= prices[i] + 0.5 * atr
                };
                if pullback { successes += 1; break; }
            }
        }
        prob(successes, trials, self.min_observations)
    }

    /// P(atr expansion): fraction of times ATR increased on the next bar
    /// when BBWP was above 80.
    fn compute_atr_expansion(&self, atrs: &[f64], bbwps: &[f64]) -> (f64, usize) {
        let n = atrs.len();
        if n < 2 { return (0.0, 0); }
        let mut trials = 0usize;
        let mut successes = 0usize;
        for i in 0..n - 1 {
            if bbwps[i] < 80.0 { continue; }
            trials += 1;
            if atrs[i + 1] > atrs[i] { successes += 1; }
        }
        prob(successes, trials, self.min_observations)
    }

    /// P(squeeze release direction): what fraction of squeeze releases
    /// moved up (bullish) within `forward_bars`?
    fn compute_squeeze_release(&self, squeeze_vals: &[f64], prices: &[f64]) -> (f64, usize) {
        let n = squeeze_vals.len();
        if n < self.forward_bars + 3 { return (0.0, 0); }
        let mut trials = 0usize;
        let mut bullish = 0usize;
        // Detect squeeze release: momentum was near-zero (coiling) then
        // becomes significantly non-zero (release).
        for i in 3..n.saturating_sub(self.forward_bars) {
            let was_coiling = squeeze_vals[i - 1].abs() < 0.2
                && squeeze_vals[i - 2].abs() < 0.2
                && squeeze_vals[i - 3].abs() < 0.2;
            let released = squeeze_vals[i].abs() > 0.3;
            if !was_coiling || !released { continue; }
            trials += 1;
            let end = (i + self.forward_bars).min(n);
            let mut net_up = false;
            for j in i + 1..end {
                if prices[j] > prices[i] + 0.002 * prices[i] { net_up = true; break; }
                if prices[j] < prices[i] - 0.002 * prices[i] { net_up = false; break; }
            }
            if net_up { bullish += 1; }
        }
        // Return the probability of bullish direction.
        prob(bullish, trials, self.min_observations)
    }

    /// P(volatility expansion): P(next bar ATR > current ATR | BBWP band).
    fn compute_volatility_expansion(&self, atrs: &[f64], bbwps: &[f64]) -> (f64, usize) {
        let n = atrs.len();
        if n < 2 { return (0.0, 0); }
        // Bucket by BBWP quintiles and compute conditional probability.
        let mut buckets: HashMap<usize, (usize, usize)> = HashMap::new(); // quintile -> (successes, trials)
        for i in 0..n - 1 {
            if bbwps[i] <= 0.0 { continue; }
            let q = ((bbwps[i] / 20.0) as usize).min(4);
            let entry = buckets.entry(q).or_insert((0, 0));
            entry.1 += 1;
            if atrs[i + 1] > atrs[i] { entry.0 += 1; }
        }
        // Return the bucket with the most trials as the primary estimate.
        let mut best_trials = 0usize;
        let mut best_prob = 0.0;
        for (_, (succ, trials)) in &buckets {
            if *trials > best_trials {
                best_trials = *trials;
                best_prob = if *trials > 0 { *succ as f64 / *trials as f64 } else { 0.0 };
            }
        }
        if best_trials < self.min_observations { (0.0, best_trials) }
        else { (best_prob, best_trials) }
    }

    /// P(stop before target): fraction of times price reached a 1.5×ATR
    /// stop before a 2.0×ATR target, given a directional bias from the
    /// recent trend (using SMA slope).
    fn compute_stop_before_target(&self, prices: &[f64], atrs: &[f64]) -> (f64, usize) {
        let n = prices.len();
        if n < self.forward_bars + 5 { return (0.0, 0); }
        let sma = sma(prices, 20);
        let mut trials = 0usize;
        let mut stop_first = 0usize;
        for i in 5..n.saturating_sub(self.forward_bars) {
            if sma[i].abs() < 1e-12 || atrs[i].abs() < 1e-12 { continue; }
            // Use SMA slope for directional bias.
            let bias = sma[i] - sma[i.saturating_sub(5)];
            let (target, stop) = if bias > 0.0 {
                (prices[i] + 2.0 * atrs[i], prices[i] - 1.5 * atrs[i])
            } else {
                (prices[i] - 2.0 * atrs[i], prices[i] + 1.5 * atrs[i])
            };
            trials += 1;
            let end = (i + self.forward_bars).min(n);
            for j in i + 1..end {
                if bias > 0.0 {
                    if prices[j] >= target { break; } // target hit first
                    if prices[j] <= stop { stop_first += 1; break; }
                } else {
                    if prices[j] <= target { break; } // target hit first
                    if prices[j] >= stop { stop_first += 1; break; }
                }
            }
        }
        prob(stop_first, trials, self.min_observations)
    }
}

// ── Helper: probability with minimum-observation gate ─────────

fn prob(successes: usize, trials: usize, min_obs: usize) -> (f64, usize) {
    if trials < min_obs { (0.0, trials) }
    else { (successes as f64 / trials as f64, trials) }
}

// ── Lightweight indicator-like helpers (computed from price only) ──

/// Simple moving average of `prices` with the given period.
fn sma(prices: &[f64], period: usize) -> Vec<f64> {
    let n = prices.len();
    let mut out = vec![0.0; n];
    if period == 0 || n < period { return out; }
    let mut sum: f64 = prices[..period].iter().sum();
    out[period - 1] = sum / period as f64;
    for i in period..n {
        sum += prices[i] - prices[i - period];
        out[i] = sum / period as f64;
    }
    out
}

/// Rolling sample standard deviation with the given period.
fn rolling_stddev(prices: &[f64], period: usize) -> Vec<f64> {
    let n = prices.len();
    let mut out = vec![0.0; n];
    if period < 2 || n < period { return out; }
    for i in period - 1..n {
        let slice = &prices[i + 1 - period..=i];
        let mean = slice.iter().sum::<f64>() / period as f64;
        let var = slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (period - 1) as f64;
        out[i] = var.sqrt();
    }
    out
}

/// Rolling maximum over the last `period` bars.
fn rolling_max(values: &[f64], period: usize) -> Vec<f64> {
    let n = values.len();
    let mut out = vec![0.0; n];
    for i in 0..n {
        let start = i.saturating_sub(period - 1);
        out[i] = values[start..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    }
    out
}

/// Rolling minimum over the last `period` bars.
fn rolling_min(values: &[f64], period: usize) -> Vec<f64> {
    let n = values.len();
    let mut out = vec![0.0; n];
    for i in 0..n {
        let start = i.saturating_sub(period - 1);
        out[i] = values[start..=i].iter().cloned().fold(f64::INFINITY, f64::min);
    }
    out
}

/// Compute the nth percentile of a slice (0-100).
fn percentile_of_slice(data: &[f64], p: f64) -> f64 {
    crate::statistics::distribution::percentile(data, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_uptrend(n: usize, base: f64, step: f64) -> Vec<f64> {
        (0..n).map(|i| base + i as f64 * step).collect()
    }

    fn make_downtrend(n: usize, base: f64, step: f64) -> Vec<f64> {
        (0..n).map(|i| base - i as f64 * step).collect()
    }

    #[test]
    fn test_trend_continuation_uptrend() {
        let prices = make_uptrend(100, 50000.0, 10.0);
        let sma = sma(&prices, 20);
        let engine = ProbabilityEngine::new(5, 5);
        let (prob, trials) = engine.compute_trend_continuation(&prices, &sma);
        // In a strong uptrend, continuation should be high.
        assert!(trials > 10, "should have enough trials");
        assert!(prob > 0.5, "uptrend should continue: got {prob}");
    }

    #[test]
    fn test_reversal_overbought() {
        // Create a series where RSI oscillates around 70-30, prices
        // mean-revert after extremes.
        let n = 100;
        let mut rsis = vec![50.0; n];
        let mut prices = vec![50000.0; n];
        let mut atrs = vec![500.0; n];
        for i in 0..n {
            let phase = (i as f64 * 0.3).sin();
            rsis[i] = 50.0 + phase * 30.0;
            prices[i] = 50000.0 - phase * 1000.0;
        }
        let engine = ProbabilityEngine::new(5, 5);
        let (prob, _) = engine.compute_reversal(&rsis, &prices, &atrs);
        // We don't assert a specific value since it depends on the data,
        // but the function must not panic.
        assert!(prob >= 0.0 && prob <= 1.0);
    }

    #[test]
    fn test_atr_expansion_basic() {
        let n = 50;
        let atrs: Vec<f64> = (0..n).map(|i| if i % 2 == 0 { 100.0 } else { 150.0 }).collect();
        let bbwps: Vec<f64> = vec![85.0; n];
        let engine = ProbabilityEngine::new(5, 5);
        let (prob, trials) = engine.compute_atr_expansion(&atrs, &bbwps);
        assert!(trials > 10);
        // Alternating low→high→low→high: expansion only on low→high bars,
        // contraction on high→low bars.  Expect ~50% expansion rate.
        assert!((prob - 0.5).abs() < 0.15, "alternating ATR should give ~50% expansion, got {prob}");
    }

    #[test]
    fn test_probability_snapshot_default() {
        let snap = ProbabilitySnapshot::default();
        assert_eq!(snap.observation_counts.len(), 8);
    }

    #[test]
    fn test_sma_basic() {
        let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let s = sma(&prices, 3);
        // period=3 → first at index 2: (1+2+3)/3=2.0, then (2+3+4)/3=3.0, (3+4+5)/3=4.0
        assert!((s[2] - 2.0).abs() < 1e-9);
        assert!((s[3] - 3.0).abs() < 1e-9);
        assert!((s[4] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_squeeze_release_bullish() {
        let n = 120;
        let mut squeeze_vals = vec![0.0; n];
        let mut prices = vec![50000.0; n];
        // Create 6 squeeze-release events, all followed by strong rallies.
        for event in 0..6 {
            let base = 15 + event * 16;
            squeeze_vals[base] = 0.05;
            squeeze_vals[base + 1] = 0.05;
            squeeze_vals[base + 2] = 0.05;
            squeeze_vals[base + 3] = 0.6; // release
            for j in base + 4..base + 10 {
                prices[j] = 50000.0 + (j - base - 3) as f64 * 200.0;
            }
        }
        let engine = ProbabilityEngine::new(5, 5);
        let (prob, trials) = engine.compute_squeeze_release(&squeeze_vals, &prices);
        assert!(trials >= 5, "should detect >= 5 releases, got {trials}");
        // All releases were followed by strong rallies.
        assert!(prob > 0.5, "strong rallies after release: prob {prob}");
    }
}
