//! `RollingWindow<T>` — fixed-capacity circular buffer with incremental
//! mean/variance tracking via Welford's online algorithm.
//!
//! Every `push` is O(1). Mean, variance, and standard deviation are available
//! in O(1) without iterating. Order-statistics (median, percentile) require
//! a temporary sort — O(n log n) per query, amortized by infrequent access.

use std::collections::VecDeque;

/// Running statistics maintained incrementally (Welford 1962).
#[derive(Debug, Clone, Default)]
struct RunningStats {
    count: usize,
    mean: f64,
    m2: f64,
}

impl RunningStats {
    fn push(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    fn pop(&mut self, value: f64) {
        if self.count <= 1 {
            self.count = 0;
            self.mean = 0.0;
            self.m2 = 0.0;
            return;
        }
        let prev_mean = (self.mean * self.count as f64 - value) / (self.count - 1) as f64;
        let delta = value - prev_mean;
        let old_mean = self.mean;
        let delta2 = old_mean - prev_mean;
        self.m2 = (self.m2 - delta * (value - old_mean) - delta2 * (value - prev_mean)).max(0.0);
        self.mean = prev_mean;
        self.count -= 1;
    }

    fn variance(&self) -> f64 {
        if self.count < 2 { 0.0 } else { self.m2 / (self.count - 1) as f64 }
    }

    fn stddev(&self) -> f64 { self.variance().sqrt() }
}

/// A fixed-capacity rolling buffer of `f64` values that tracks the
/// incremental running mean and variance of its contents. When the buffer
/// is at capacity, the oldest element is evicted — and its contribution
/// is efficiently removed from the running statistics via Welford's
/// algorithm.
#[derive(Debug, Clone)]
pub struct RollingWindow {
    buffer: VecDeque<f64>,
    capacity: usize,
    stats: RunningStats,
}

impl RollingWindow {
    pub fn new(capacity: usize) -> Self {
        Self { buffer: VecDeque::with_capacity(capacity), capacity, stats: RunningStats::default() }
    }

    /// Push a new value. If the buffer is full, the oldest entry is evicted
    /// and its contribution removed from the running statistics. Returns the
    /// evicted value, if any.
    pub fn push(&mut self, value: f64) -> Option<f64> {
        let evicted = if self.buffer.len() >= self.capacity {
            self.buffer.pop_front()
        } else {
            None
        };
        if let Some(v) = evicted {
            self.stats.pop(v);
        }
        self.stats.push(value);
        self.buffer.push_back(value);
        evicted
    }

    /// Number of elements currently in the buffer.
    pub fn len(&self) -> usize { self.buffer.len() }

    /// Whether the buffer has reached its configured capacity.
    pub fn is_full(&self) -> bool { self.buffer.len() >= self.capacity }

    /// Whether the buffer has enough observations for meaningful statistics
    /// (at least half the capacity or 10 observations, whichever is smaller).
    pub fn is_warm(&self) -> bool {
        self.buffer.len() >= self.capacity.min(20) / 2
    }

    /// Access the underlying buffer as a reference.
    pub fn as_slice(&self) -> &VecDeque<f64> { &self.buffer }

    /// Return a sorted vector of the current values (O(n log n)).
    pub fn sorted_values(&self) -> Vec<f64> {
        let mut v: Vec<f64> = self.buffer.iter().copied().collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    /// Rolling arithmetic mean (O(1)).
    pub fn mean(&self) -> f64 {
        if self.buffer.is_empty() { 0.0 } else { self.stats.mean }
    }

    /// Rolling (sample) variance (O(1)).
    pub fn variance(&self) -> f64 { self.stats.variance() }

    /// Rolling standard deviation (O(1)).
    pub fn stddev(&self) -> f64 { self.stats.stddev() }

    /// Return the most recently pushed value, if any.
    pub fn latest(&self) -> Option<f64> { self.buffer.back().copied() }

    pub fn capacity(&self) -> usize { self.capacity }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_len() {
        let mut rw = RollingWindow::new(5);
        assert!(rw.push(1.0).is_none());
        assert!(rw.push(2.0).is_none());
        assert_eq!(rw.len(), 2);
    }

    #[test]
    fn test_eviction() {
        let mut rw = RollingWindow::new(3);
        rw.push(1.0);
        rw.push(2.0);
        rw.push(3.0);
        let ev = rw.push(4.0);
        assert_eq!(ev, Some(1.0));
        assert_eq!(rw.len(), 3);
        assert_eq!(rw.latest(), Some(4.0));
    }

    #[test]
    fn test_mean_variance_known() {
        let mut rw = RollingWindow::new(5);
        for x in [2.0, 4.0, 6.0] { rw.push(x); }
        assert!((rw.mean() - 4.0).abs() < 1e-9);
        // sample variance of [2,4,6] = 4.0
        assert!((rw.variance() - 4.0).abs() < 1e-9);
        assert!((rw.stddev() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_pop_maintains_stats() {
        let mut rw = RollingWindow::new(3);
        rw.push(1.0);
        rw.push(2.0);
        rw.push(3.0);
        let ev = rw.push(4.0);
        assert_eq!(ev, Some(1.0));
        // remaining: [2, 3, 4] → mean = 3.0
        assert!((rw.mean() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_is_warm() {
        let mut rw = RollingWindow::new(100);
        assert!(!rw.is_warm());
        for _ in 0..9 { rw.push(1.0); }
        assert!(!rw.is_warm());
        rw.push(1.0);
        assert!(rw.is_warm()); // 10 >= 10 (capacity.min(20) / 2 = 10)
    }

    #[test]
    fn test_sorted_values() {
        let mut rw = RollingWindow::new(5);
        for x in [5.0, 1.0, 3.0, 4.0, 2.0] { rw.push(x); }
        assert_eq!(rw.sorted_values(), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_empty_stats() {
        let rw = RollingWindow::new(10);
        assert_eq!(rw.mean(), 0.0);
        assert_eq!(rw.variance(), 0.0);
        assert_eq!(rw.stddev(), 0.0);
    }
}
