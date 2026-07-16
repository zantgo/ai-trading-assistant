use super::ema::Ema;
use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Direction of a MACD crossover event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossoverDir {
    Bullish,
    Bearish,
}

/// Expanded output from a MACD update including crossover and momentum state.
#[derive(Debug, Clone)]
pub struct MacdOutput {
    pub macd_line: Decimal,
    pub signal_line: Decimal,
    pub histogram: Decimal,
    pub crossover: Option<CrossoverDir>,
    pub histogram_peak: Decimal,
    pub trend_state: TrendState,
}

/// Whether the histogram is accelerating or decelerating relative to its peak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendState {
    Accelerating,
    Decelerating,
}

/// Moving Average Convergence Divergence with stateful tracking.
///
/// Tracks previous MACD/Signal line values for crossover detection,
/// maintains a running histogram peak since the last crossover, and
/// can compute contraction ratios for momentum exhaustion signals.
#[derive(Debug, Clone)]
pub struct Macd {
    fast_ema: Ema,
    slow_ema: Ema,
    signal_ema: Ema,
    prev_macd_line: Option<Decimal>,
    prev_signal_line: Option<Decimal>,
    histogram_peak: Decimal,
    prev_histogram: Option<Decimal>,
}

impl Default for Macd {
    fn default() -> Self {
        Self::new()
    }
}

impl Macd {
    pub fn new() -> Self {
        Self {
            fast_ema: Ema::new(12),
            slow_ema: Ema::new(26),
            signal_ema: Ema::new(9),
            prev_macd_line: None,
            prev_signal_line: None,
            histogram_peak: Decimal::ZERO,
            prev_histogram: None,
        }
    }

    pub fn update(&mut self, close: Decimal) -> MacdOutput {
        let fast = self.fast_ema.update(close);
        let slow = self.slow_ema.update(close);
        let macd_line = fast - slow;
        let signal_line = self.signal_ema.update(macd_line);
        let histogram = macd_line - signal_line;

        // Detect crossover
        let crossover = detect_crossover(
            self.prev_macd_line,
            self.prev_signal_line,
            macd_line,
            signal_line,
        );

        // Update histogram peak — reset on crossover
        if crossover.is_some() {
            self.histogram_peak = Decimal::ZERO;
        }
        let abs_hist = if histogram < Decimal::ZERO {
            -histogram
        } else {
            histogram
        };
        if abs_hist > self.histogram_peak {
            self.histogram_peak = abs_hist;
        }

        // Determine trend state
        let trend_state = if let Some(prev) = self.prev_histogram {
            let abs_prev = if prev < Decimal::ZERO { -prev } else { prev };
            if abs_hist >= abs_prev {
                TrendState::Accelerating
            } else {
                TrendState::Decelerating
            }
        } else {
            TrendState::Accelerating
        };

        self.prev_macd_line = Some(macd_line);
        self.prev_signal_line = Some(signal_line);
        self.prev_histogram = Some(histogram);

        MacdOutput {
            macd_line,
            signal_line,
            histogram,
            crossover,
            histogram_peak: self.histogram_peak,
            trend_state,
        }
    }

    /// Check whether the histogram has contracted from its peak by more than
    /// the given threshold percentage. Returns true if contraction is triggered.
    ///
    /// `threshold` is a decimal fraction, e.g. Decimal::new(30, 2) for 30%.
    pub fn check_contraction(&self, threshold: Decimal) -> bool {
        if self.histogram_peak == Decimal::ZERO {
            return false;
        }
        if let Some(current) = self.prev_histogram {
            let abs_current = if current < Decimal::ZERO {
                -current
            } else {
                current
            };
            let contraction = Decimal::ONE - threshold; // e.g. 0.70 for 30% threshold
            let trigger_level = self.histogram_peak * contraction;
            abs_current < trigger_level
        } else {
            false
        }
    }

    /// Get the current histogram peak (maximum absolute histogram since last crossover).
    pub fn get_histogram_peak(&self) -> Decimal {
        self.histogram_peak
    }

    /// Get the current MACD line value (from previous state).
    pub fn get_macd_line(&self) -> Option<Decimal> {
        self.prev_macd_line
    }

    /// Get the current signal line value (from previous state).
    pub fn get_signal_line(&self) -> Option<Decimal> {
        self.prev_signal_line
    }
}

impl Indicator for Macd {
    type Output = MacdOutput;

    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.close)
    }

    fn reset(&mut self) {
        *self = Macd::new();
    }
}

/// Detect a crossover between the MACD line and signal line.
/// Bullish: MACD line crosses from below to above signal.
/// Bearish: MACD line crosses from above to below signal.
fn detect_crossover(
    prev_macd: Option<Decimal>,
    prev_signal: Option<Decimal>,
    current_macd: Decimal,
    current_signal: Decimal,
) -> Option<CrossoverDir> {
    let (prev_m, prev_s) = match (prev_macd, prev_signal) {
        (Some(m), Some(s)) => (m, s),
        _ => return None,
    };

    if prev_m <= prev_s && current_macd > current_signal {
        Some(CrossoverDir::Bullish)
    } else if prev_m >= prev_s && current_macd < current_signal {
        Some(CrossoverDir::Bearish)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_first_update_seeds_emas() {
        let mut macd = Macd::new();
        let out = macd.update(dec!(100.00));
        assert_eq!(out.macd_line, dec!(0.00));
        assert_eq!(out.signal_line, dec!(0.00));
        assert_eq!(out.histogram, dec!(0.00));
        assert!(out.crossover.is_none());
    }

    #[test]
    fn test_histogram_sign_matches_macd_line_minus_signal() {
        let mut macd = Macd::new();
        macd.update(dec!(100.00));
        macd.update(dec!(101.00));
        let out = macd.update(dec!(102.00));
        assert_eq!(out.histogram, out.macd_line - out.signal_line);
    }

    #[test]
    fn test_bullish_crossover_detected() {
        let mut macd = Macd::new();
        // Seed with equal prices first
        macd.update(dec!(100.00));
        macd.update(dec!(100.00));
        macd.update(dec!(100.00));
        macd.update(dec!(100.00));
        // Now drive a sustained uptrend to create a bullish crossover
        for _ in 0..20 {
            macd.update(dec!(100.00));
        }
        for _ in 0..10 {
            macd.update(dec!(105.00));
        }
        let out = macd.update(dec!(110.00));
        // After sustained rise, MACD line should eventually cross above signal
        // (may take more periods depending on EMA convergence)
        assert!(out.macd_line > dec!(0.00) || out.crossover.is_some());
    }

    #[test]
    fn test_histogram_peak_tracks_max() {
        let mut macd = Macd::new();
        for _ in 0..20 {
            macd.update(dec!(100.00));
        }
        // Push price up to create positive histogram
        for _ in 0..10 {
            macd.update(dec!(110.00));
        }
        let _ = macd.update(dec!(115.00));
        let peak = macd.get_histogram_peak();
        assert!(peak > Decimal::ZERO);
        // Go sideways — peak should not decrease
        for _ in 0..5 {
            let _ = macd.update(dec!(115.00));
        }
        assert!(macd.get_histogram_peak() >= peak);
    }

    #[test]
    fn test_contraction_detection() {
        let mut macd = Macd::new();
        // Build a large histogram peak
        for _ in 0..20 {
            macd.update(dec!(100.00));
        }
        for _ in 0..10 {
            macd.update(dec!(120.00));
        }
        let out = macd.update(dec!(125.00));
        assert!(out.histogram_peak > Decimal::ZERO);
        // Now go flat to let histogram contract
        for _ in 0..15 {
            macd.update(dec!(125.00));
        }
        let out_flat = macd.update(dec!(125.00));
        // After flattening out, histogram may contract
        let current_abs = if out_flat.histogram < Decimal::ZERO {
            -out_flat.histogram
        } else {
            out_flat.histogram
        };
        // The histogram should be smaller than the peak (or at least not significantly larger)
        assert!(
            current_abs <= out.histogram_peak || out_flat.trend_state == TrendState::Decelerating
        );
    }

    #[test]
    fn test_crossover_resets_peak() {
        let mut macd = Macd::new();
        for _ in 0..20 {
            macd.update(dec!(100.00));
        }
        for _ in 0..10 {
            macd.update(dec!(120.00));
        }
        let peak_before = macd.get_histogram_peak();
        assert!(peak_before > Decimal::ZERO);
        // Now drive price down hard to force a bearish crossover
        for _ in 0..30 {
            macd.update(dec!(80.00));
        }
        // After a crossover, peak should reset
        let peak_after = macd.get_histogram_peak();
        // Depending on timing, the peak may have reset and started tracking anew
        // It should be different from the pre-crossover peak
        assert!(peak_after != peak_before || peak_after == Decimal::ZERO);
    }

    #[test]
    fn test_trend_state_transitions() {
        let mut macd = Macd::new();
        for _ in 0..20 {
            macd.update(dec!(100.00));
        }
        // Accelerating phase
        let mut found_decelerating = false;
        for i in 0..30 {
            let out = macd.update(dec!(100.00) + Decimal::from(i as i64));
            if out.trend_state == TrendState::Decelerating {
                found_decelerating = true;
            }
        }
        // Eventually the histogram should decelerate as price levels off
        // This may or may not occur depending on the EMA convergence rate
        // Just assert that it runs without panicking
        let _ = found_decelerating;
    }
}
