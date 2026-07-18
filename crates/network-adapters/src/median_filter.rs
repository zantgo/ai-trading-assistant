//! DIE L3 median price filter (03-01-04 §4.1).
//!
//! Rolling median outlier rejection for the raw tick stream. Warm-up accepts
//! every tick for the first `median_window_size` ticks; from tick `N + 1`
//! onward a tick is rejected when `|p − median| / median > outlier_tolerance`.
//! The current tick is appended to the window **after** the filter check.
//! When the rolling median is exactly zero (venue reset) and
//! `bypass_on_zero_median` is set, the filter is bypassed for that tick and
//! the bypass is counted separately (`outliers_bypassed`).

use config_models::QualityConfig;
use std::collections::VecDeque;

/// Outcome of one median-filter evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterVerdict {
    /// Tick accepted (warm-up or within tolerance).
    Accepted,
    /// Median was exactly zero and `bypass_on_zero_median` is set: the filter
    /// is skipped for this tick, which is accepted and counted separately.
    Bypassed,
    /// Tick rejected as an outlier.
    Rejected,
}

pub struct MedianPriceFilter {
    window: VecDeque<f64>,
    window_size: usize,
    tolerance: f64,
    bypass_on_zero: bool,
    warmup_remaining: usize,
    outliers_rejected: u32,
    outliers_bypassed: u32,
}

impl MedianPriceFilter {
    pub fn new(config: &QualityConfig) -> Self {
        Self {
            window: VecDeque::new(),
            window_size: config.median_window_size,
            tolerance: config.outlier_tolerance,
            bypass_on_zero: config.bypass_on_zero_median,
            warmup_remaining: config.median_window_size,
            outliers_rejected: 0,
            outliers_bypassed: 0,
        }
    }

    /// Evaluate one tick price against the rolling median.
    pub fn evaluate(&mut self, price: f64) -> FilterVerdict {
        if self.warmup_remaining > 0 {
            self.warmup_remaining -= 1;
            self.window.push_back(price);
            return FilterVerdict::Accepted;
        }
        let mut sorted: Vec<f64> = self.window.iter().copied().collect();
        sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted[sorted.len() / 2];
        let verdict = if median == 0.0 && self.bypass_on_zero {
            FilterVerdict::Bypassed
        } else if median == 0.0 {
            if price == 0.0 {
                FilterVerdict::Accepted
            } else {
                FilterVerdict::Rejected
            }
        } else if ((price - median).abs() / median) <= self.tolerance {
            FilterVerdict::Accepted
        } else {
            FilterVerdict::Rejected
        };
        match verdict {
            FilterVerdict::Accepted | FilterVerdict::Bypassed => {
                if !self.window.is_empty() {
                    self.window.pop_front();
                }
                self.window.push_back(price);
            }
            FilterVerdict::Rejected => {
                self.outliers_rejected += 1;
            }
        }
        if verdict == FilterVerdict::Bypassed {
            self.outliers_bypassed += 1;
        }
        verdict
    }

    pub fn outliers_rejected(&self) -> u32 {
        self.outliers_rejected
    }

    pub fn outliers_bypassed(&self) -> u32 {
        self.outliers_bypassed
    }

    /// Number of samples currently held in the rolling window.
    pub fn window_len(&self) -> usize {
        self.window.len()
    }

    /// Configured window size (`median_window_size`).
    pub fn window_size(&self) -> usize {
        self.window_size
    }
}
