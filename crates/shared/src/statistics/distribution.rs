//! Module A: Distribution Statistics.
//!
//! Computes rolling mean, median, variance, standard deviation, percentile,
//! z-score, interquartile range, and median absolute deviation for price,
//! returns, ATR, RSI, BBWP, squeeze momentum, volume, RVOL, and ADX.
//!
//! All statistics come from pure mathematical operations on the rolling
//! window buffers — no heuristics, no AI, no manual tuning.

use crate::statistics::rolling_window::RollingWindow;
use crate::statistics::statistical_object::StatisticValue;
use crate::statistics::WINDOW_SIZES;

/// Number of metrics tracked.  Index layout:
///
///   0 → price
///   1 → log-returns (×100 for readability)
///   2 → ATR
///   3 → RSI
///   4 → BBWP
///   5 → squeeze momentum
///   6 → volume
///   7 → RVOL
///   8 → ADX
///   9 → MACD
///  10 → OBV
///  11 → StochK
///  12 → Choppiness
///  13 → EMA_50
pub(crate) const METRIC_COUNT: usize = 14;

// ── Pure distribution functions (work on any &[f64]) ──────────

/// Median value. Sorts a clone — O(n log n) per call.
pub fn median(data: &[f64]) -> f64 {
    if data.is_empty() { return 0.0; }
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

/// Percentile (0–100) of the given data. Linear interpolation between
/// adjacent order-statistics.
pub fn percentile(data: &[f64], p: f64) -> f64 {
    if data.is_empty() { return 0.0; }
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let k = (p / 100.0 * (n - 1) as f64).clamp(0.0, (n - 1) as f64);
    let lo = k.floor() as usize;
    let hi = k.ceil() as usize;
    if lo == hi { return sorted[lo]; }
    let frac = k - lo as f64;
    sorted[lo] + frac * (sorted[hi] - sorted[lo])
}

/// Percentile rank of `value` within `data`. Returns a value in [0, 100].
pub fn percentile_rank(data: &[f64], value: f64) -> f64 {
    if data.is_empty() { return 50.0; }
    let n = data.len();
    let below = data.iter().filter(|&&x| x < value).count() as f64;
    let equal = data.iter().filter(|&&x| (x - value).abs() < 1e-12).count() as f64;
    (below + 0.5 * equal) / n as f64 * 100.0
}

/// Z-score: (value - mean) / stddev.  Returns 0.0 when stddev ≈ 0.
pub fn z_score(value: f64, mean: f64, stddev: f64) -> f64 {
    if stddev.abs() < 1e-12 { 0.0 } else { (value - mean) / stddev }
}

/// Interquartile range (Q3 – Q1).
pub fn iqr(data: &[f64]) -> f64 { percentile(data, 75.0) - percentile(data, 25.0) }

/// Median absolute deviation (from median). MAD = median(|xi – median(x)|).
pub fn mad(data: &[f64]) -> f64 {
    let med = median(data);
    let abs_devs: Vec<f64> = data.iter().map(|x| (x - med).abs()).collect();
    median(&abs_devs)
}

// ── DistributionTracker ───────────────────────────────────────

/// Maintains 5 rolling windows (one per configured horizon) for each of 9
/// key metrics. Updated incrementally on every completed candle.
#[derive(Debug, Clone)]
pub struct DistributionTracker {
    /// windows[w][m] = RollingWindow for window-size index w, metric index m.
    windows: Vec<Vec<RollingWindow>>,
    window_sizes: Vec<usize>,
}

impl DistributionTracker {
    pub fn new(window_sizes: &[usize]) -> Self {
        let ws = if window_sizes.is_empty() { WINDOW_SIZES.to_vec() } else { window_sizes.to_vec() };
        let windows: Vec<Vec<RollingWindow>> = ws
            .iter()
            .map(|&cap| (0..METRIC_COUNT).map(|_| RollingWindow::new(cap)).collect())
            .collect();
        Self { windows, window_sizes: ws }
    }

    /// Push one new value per metric. Called on each completed candle.
    pub fn advance(&mut self, metrics: &[f64; METRIC_COUNT]) {
        for w in 0..self.window_sizes.len() {
            for m in 0..METRIC_COUNT {
                self.windows[w][m].push(metrics[m]);
            }
        }
    }

    /// Return the number of window sizes configured.
    pub fn num_windows(&self) -> usize { self.window_sizes.len() }

    /// Return the window size for the given index.
    pub fn window_size(&self, idx: usize) -> usize { self.window_sizes[idx] }

    /// Produce a `StatisticValue` for the given metric across the given
    /// window-size index.
    pub fn statistic(&self, window_idx: usize, metric_idx: usize) -> StatisticValue {
        let win = &self.windows[window_idx][metric_idx];
        let current = win.latest().unwrap_or(0.0);
        let mean = win.mean();
        let stddev = win.stddev();
        let sorted = win.sorted_values();
        StatisticValue {
            current,
            mean,
            stddev,
            percentile: percentile_rank(&sorted, current),
            z_score: z_score(current, mean, stddev),
            confidence: if mean.abs() > 1e-12 {
                (1.0 / (1.0 + (stddev / mean.abs()))).clamp(0.0, 1.0)
            } else {
                0.0
            },
            trend: trend_label(&sorted, current),
        }
    }

    /// Produce multi-window statistics for the 5 key metrics exposed in the
    /// `StatisticalContext`. Uses the **longest warm window** available.
    pub fn key_statistics(&self) -> KeyStatistics {
        let best_window = self.best_window_idx();
        let m = |mi: usize| self.statistic(best_window, mi);
        KeyStatistics {
            price: m(0),
            returns: m(1),
            atr: m(2),
            rsi: m(3),
            bbwp: m(4),
        }
    }

    /// Return the index of the longest warm window, falling back to 0.
    pub fn best_window_idx(&self) -> usize {
        for w in (0..self.window_sizes.len()).rev() {
            if self.windows[w][0].is_warm() { return w; }
        }
        0
    }

    /// Returns the sorted values for the price window at the given index.
    pub fn price_values(&self, window_idx: usize) -> Vec<f64> {
        self.windows[window_idx][0].sorted_values()
    }

    /// Returns the sorted values for the returns window at the given index.
    pub fn returns_values(&self, window_idx: usize) -> Vec<f64> {
        self.windows[window_idx][1].sorted_values()
    }

    /// Return a flat `Vec<f64>` of the values for metric `metric_idx` in
    /// window `window_idx`, in insertion order (oldest → newest).
    pub fn metric_values(&self, window_idx: usize, metric_idx: usize) -> Vec<f64> {
        self.windows[window_idx][metric_idx]
            .as_slice()
            .iter()
            .copied()
            .collect()
    }
}

/// Packed statistics for the 5 key metrics, using one window.
#[derive(Debug, Clone)]
pub struct KeyStatistics {
    pub price: StatisticValue,
    pub returns: StatisticValue,
    pub atr: StatisticValue,
    pub rsi: StatisticValue,
    pub bbwp: StatisticValue,
}

// ── helpers ────────────────────────────────────────────────────

fn trend_label(sorted: &[f64], current: f64) -> String {
    if sorted.len() < 10 { return "stable".to_string(); }
    let mid = sorted.len() / 2;
    let median_recent = median(&sorted[mid..]);
    let median_older = median(&sorted[..mid]);
    let delta = median_recent - median_older;
    let rel = if current.abs() > 1e-12 { delta / current.abs() } else { delta };
    if rel > 0.05 { "increasing".into() }
    else if rel < -0.05 { "decreasing".into() }
    else { "stable".into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_odd() {
        assert!((median(&[1.0, 3.0, 2.0]) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_median_even() {
        assert!((median(&[1.0, 2.0, 3.0, 4.0]) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn test_median_empty() {
        assert_eq!(median(&[]), 0.0);
    }

    #[test]
    fn test_percentile_50_is_median() {
        let data = vec![5.0, 1.0, 4.0, 2.0, 3.0];
        assert!((percentile(&data, 50.0) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_percentile_rank() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile_rank(&data, 3.0) - 50.0).abs() < 1.0);
        assert!(percentile_rank(&data, 1.0) < 20.0);
        assert!(percentile_rank(&data, 5.0) > 80.0);
    }

    #[test]
    fn test_z_score() {
        assert!((z_score(2.0, 0.0, 1.0) - 2.0).abs() < 1e-9);
        assert_eq!(z_score(5.0, 5.0, 0.0), 0.0);
    }

    #[test]
    fn test_iqr() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        // Q1 = 3.25, Q3 = 7.75, IQR = 4.5
        assert!((iqr(&data) - 4.5).abs() < 1e-9);
    }

    #[test]
    fn test_mad() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 100.0];
        // median = 3, abs devs = [2,1,0,1,97], median of devs = 1
        assert!((mad(&data) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_tracker_advance() {
        let mut tracker = DistributionTracker::new(&[20, 50]);
        let metrics = [50000.0, 0.0, 100.0, 50.0, 50.0, 0.0, 1000.0, 1.0, 25.0, 0.0, 0.0, 50.0, 50.0, 50000.0];
        tracker.advance(&metrics);
        assert_eq!(tracker.windows[0][0].len(), 1);
        assert_eq!(tracker.windows[1][0].len(), 1);
    }

    #[test]
    fn test_key_statistics_no_panic_empty() {
        let tracker = DistributionTracker::new(&[20]);
        let ks = tracker.key_statistics();
        assert_eq!(ks.price.current, 0.0);
    }

    #[test]
    fn test_best_window_idx() {
        let tracker = DistributionTracker::new(&[20, 50]);
        // Neither is warm yet.
        assert_eq!(tracker.best_window_idx(), 0);
    }
}
