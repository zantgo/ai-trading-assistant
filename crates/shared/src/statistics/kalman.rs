//! Module F.2: Kalman Filter Drift Estimation.
//!
//! A 1D local linear trend Kalman filter that decomposes the log-price
//! series into signal (drift / trend slope) and noise (residuals).
//!
//! State vector:  [level (estimated log-price), slope (per-bar log-return)]
//! Measurement:   observed log-price at each candle close
//!
//! The filtered drift feeds into the Monte Carlo engine to produce
//! directionally-aware path simulations.  Residuals (return − drift)
//! provide the stochastic dispersion component with lower variance
//! than raw returns.
//!
//! All computations are incremental — O(1) per candle.

use std::collections::VecDeque;

/// 1D local linear trend Kalman filter for log-price drift estimation.
#[derive(Debug, Clone)]
pub struct KalmanFilter {
    /// Filtered drift (expected per-bar return in percent, annualized).
    pub drift: f64,
    /// Rolling standard deviation of residuals (return − drift) in percent.
    pub noise_vol: f64,
    /// Signal-to-noise ratio: |drift| / noise_vol.  > 1 = trending.
    pub trend_strength: f64,

    // ── Internal state ──
    x: [f64; 2],                // state: [level, slope]
    p: [[f64; 2]; 2],           // covariance matrix
    q: f64,                      // process noise (for slope)
    r: f64,                      // measurement noise
    residual_window: usize,
    residuals: VecDeque<f64>,    // last N residuals (percent form)
    initialized: bool,
    bar_count: u64,
}

impl KalmanFilter {
    /// Create a new Kalman filter.
    ///
    /// `process_noise` controls how responsive the filter is — higher values
    /// let the slope adapt more quickly to regime changes.
    /// `measurement_noise` controls smoothing — higher values treat more of
    /// each price observation as noise.
    /// `residual_window` is the number of recent residuals kept for noise_vol
    /// estimation.
    pub fn new(process_noise: f64, measurement_noise: f64, residual_window: usize) -> Self {
        Self {
            drift: 0.0,
            noise_vol: 0.0,
            trend_strength: 0.0,
            x: [0.0, 0.0],
            p: [[1.0, 0.0], [0.0, 1.0]],
            q: process_noise,
            r: measurement_noise,
            residual_window: residual_window.max(10),
            residuals: VecDeque::with_capacity(residual_window.max(10)),
            initialized: false,
            bar_count: 0,
        }
    }

    /// Advance the filter by one candle.  Returns the residual (actual return −
    /// drift) in percent for accumulation into the residual history.
    ///
    /// `close` is the completed candle close price.
    /// `prev_close` is the previous candle close (used for log-return calc).
    pub fn update(&mut self, close: f64, prev_close: f64) {
        if close < 1e-12 {
            return;
        }

        let log_price = close.ln();
        let log_return = if prev_close > 1e-12 {
            (close / prev_close).ln()
        } else {
            0.0
        };

        // ── Predict ──
        let (x_pred, p_pred) = self.predict();

        if !self.initialized {
            // First observation: set level directly, zero slope.
            self.x = [log_price, 0.0];
            self.p = [[self.r, 0.0], [0.0, 1.0]];
            self.initialized = true;
            self.bar_count = 1;
            return;
        }

        // ── Update ──
        let innovation = log_price - x_pred[0];
        let s = p_pred[0][0] + self.r;
        if s.abs() < 1e-15 {
            return;
        }
        let k0 = p_pred[0][0] / s;
        let k1 = p_pred[0][1] / s;

        self.x[0] = x_pred[0] + k0 * innovation;
        self.x[1] = x_pred[1] + k1 * innovation;

        let ikh_00 = 1.0 - k0;
        let ikh_01 = -k0;
        let ikh_10 = -k1;
        let ikh_11 = 1.0 - k1;

        self.p[0][0] = ikh_00 * p_pred[0][0] + ikh_01 * p_pred[1][0];
        self.p[0][1] = ikh_00 * p_pred[0][1] + ikh_01 * p_pred[1][1];
        self.p[1][0] = ikh_10 * p_pred[0][0] + ikh_11 * p_pred[1][0];
        self.p[1][1] = ikh_10 * p_pred[0][1] + ikh_11 * p_pred[1][1];

        // ── Residual: actual log-return − filtered slope ──
        let residual_pct = (log_return - self.x[1]) * 100.0;

        self.residuals.push_back(residual_pct);
        while self.residuals.len() > self.residual_window {
            self.residuals.pop_front();
        }

        // ── Derived statistics ──
        self.drift = self.x[1] * 100.0; // per-bar drift in percent

        let n = self.residuals.len() as f64;
        if n > 1.0 {
            let mean = self.residuals.iter().sum::<f64>() / n;
            let var = self.residuals.iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>() / (n - 1.0).max(1.0);
            self.noise_vol = var.sqrt();
            if self.noise_vol > 1e-12 {
                self.trend_strength = self.drift.abs() / self.noise_vol;
            } else {
                self.trend_strength = 0.0;
            }
        }

        self.bar_count = self.bar_count.wrapping_add(1);
    }

    /// Predict step: project state and covariance forward by one bar.
    fn predict(&self) -> ([f64; 2], [[f64; 2]; 2]) {
        // State transition:  level' = level + slope,  slope' = slope
        let x_pred = [self.x[0] + self.x[1], self.x[1]];

        // Covariance transition:  P' = F·P·Fᵀ + Q
        // F = [[1, 1], [0, 1]]
        let p00 = self.p[0][0] + self.p[0][1] + self.p[1][0] + self.p[1][1];
        let p01 = self.p[0][1] + self.p[1][1];
        let p10 = self.p[1][0] + self.p[1][1];
        let p11 = self.p[1][1];

        // Q adds process noise to slope variance only
        let p_pred = [
            [p00, p01],
            [p10, p11 + self.q],
        ];

        (x_pred, p_pred)
    }

    /// Access the residual history for Monte Carlo resampling.
    pub fn residuals_slice(&self) -> Vec<f64> {
        self.residuals.iter().copied().collect()
    }

    /// Number of residual observations collected.
    pub fn residual_count(&self) -> usize {
        self.residuals.len()
    }

    /// Whether the filter has been initialized with at least one observation.
    pub fn is_ready(&self) -> bool {
        self.initialized && self.residuals.len() >= 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_initializes_on_first_bar() {
        let mut kf = KalmanFilter::new(1e-5, 1e-3, 100);
        assert!(!kf.is_ready());
        assert_eq!(kf.drift, 0.0);

        kf.update(50000.0, 49900.0);
        assert!(kf.initialized);
        assert_eq!(kf.bar_count, 1);
        // After one bar with significant price move, drift should be near zero
        // because we set slope=0 on init and haven't had an update yet.
    }

    #[test]
    fn test_kalman_drift_converges_in_uptrend() {
        let mut kf = KalmanFilter::new(1e-5, 1e-3, 50);
        let base = 50000.0;
        let step = 10.0; // +0.02% per bar

        // Feed 200 bars of steady uptrend.
        let mut prev = base;
        for i in 0..200 {
            let close = base + (i as f64 + 1.0) * step;
            kf.update(close, prev);
            prev = close;
        }

        assert!(kf.is_ready());
        // Drift should be positive in an uptrend.
        assert!(kf.drift > 0.0, "drift {} should be positive", kf.drift);
        // Trend strength should be meaningful.
        assert!(kf.trend_strength > 0.5, "trend_strength {} should be > 0.5", kf.trend_strength);
    }

    #[test]
    fn test_kalman_drift_converges_in_downtrend() {
        let mut kf = KalmanFilter::new(1e-5, 1e-3, 50);
        let base = 50000.0;
        let step = 10.0;

        let mut prev = base;
        for i in 0..200 {
            let close = base - (i as f64 + 1.0) * step;
            kf.update(close, prev);
            prev = close;
        }

        assert!(kf.is_ready());
        assert!(kf.drift < 0.0, "drift {} should be negative", kf.drift);
    }

    #[test]
    fn test_kalman_noise_vol_stable_in_clean_trend() {
        let mut kf = KalmanFilter::new(1e-5, 1e-3, 100);
        let base = 50000.0;
        let step = 5.0;

        let mut prev = base;
        for i in 0..300 {
            let close = base + (i as f64 + 1.0) * step;
            kf.update(close, prev);
            prev = close;
        }

        assert!(kf.is_ready());
        // In a perfectly clean trend, noise vol should be very small.
        assert!(kf.noise_vol < 0.05, "noise_vol {} should be tiny in clean trend", kf.noise_vol);
    }

    #[test]
    fn test_kalman_residuals_populated() {
        let mut kf = KalmanFilter::new(1e-5, 1e-3, 100);

        let mut prev = 50000.0;
        for i in 0..150 {
            let close = 50000.0 + (i as f64 * 3.0) + (i as f64).sin() * 100.0;
            kf.update(close, prev);
            prev = close;
        }

        assert!(kf.residual_count() > 0);
        assert!(kf.residual_count() <= 100);
        let slice = kf.residuals_slice();
        assert_eq!(slice.len(), kf.residual_count());
    }

    #[test]
    fn test_kalman_trend_strength_zero_in_noise() {
        let mut kf = KalmanFilter::new(1e-5, 1e-3, 50);

        // Mean-reverting noise around 50000 — no trend.
        let mut prev: f64 = 50000.0;
        let mut rng = SimpleRng::new(42);
        for _ in 0..200 {
            let noise = (rng.next_f64() - 0.5) * 200.0; // ±100
            let close: f64 = (50000.0 + noise).max(1.0);
            let p: f64 = prev.max(1.0);
            kf.update(close, p);
            prev = close;
        }

        assert!(kf.is_ready());
        // In pure noise, trend_strength should be low.
        assert!(kf.trend_strength < 0.5, "trend_strength {} should be low in noise", kf.trend_strength);
    }

    // ── Minimal PRNG for tests ──
    struct SimpleRng {
        state: u64,
    }

    impl SimpleRng {
        fn new(seed: u64) -> Self {
            Self { state: seed.wrapping_add(1) }
        }

        fn next_f64(&mut self) -> f64 {
            self.state = self.state.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
            (self.state >> 32) as u32 as f64 / (u32::MAX as f64 + 1.0)
        }
    }
}
