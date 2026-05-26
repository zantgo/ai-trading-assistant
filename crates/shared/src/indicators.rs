//! # Technical Indicators Module
//!
//! Implements analytical math structures designed to process sequential data streams.
//! To maintain absolute financial precision and avoid IEEE-754 floating point rounding
//! inaccuracies, all metrics are processed using high-precision fractional `Decimal` values.

use rust_decimal::Decimal;

/// An Exponential Moving Average (EMA) state tracker.
/// It dynamically adjusts the weighted average of sequential values, prioritizing newer inputs.
#[derive(Debug, Clone)]
pub struct Ema {
    /// The calculation lookback window length (e.g., 9 or 21 ticks).
    period: usize,
    /// The current moving average value, if initialized.
    current_value: Option<Decimal>,
}

impl Ema {
    /// Creates an uninitialized EMA structure configured for a specific window.
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "EMA period must be greater than zero");
        Self {
            period,
            current_value: None,
        }
    }

    /// Inputs a new price tick to update the internal average state, returning the new average.
    pub fn update(&mut self, price: Decimal) -> Decimal {
        match self.current_value {
            None => {
                // Cold-start fallback: The first EMA value defaults to the raw first price.
                self.current_value = Some(price);
                price
            }
            Some(prev_ema) => {
                // Smoothing constant (α) formula: 2 / (period + 1)
                let period_decimal = Decimal::from(self.period);
                let two = Decimal::from(2);
                let one = Decimal::from(1);
                
                let smoothing_multiplier = two / (period_decimal + one);
                
                // EMA calculation: (Current Price - Previous EMA) * α + Previous EMA
                let next_ema = (price - prev_ema) * smoothing_multiplier + prev_ema;
                self.current_value = Some(next_ema);
                next_ema
            }
        }
    }

    /// Returns the current state of the EMA, if it has been updated with at least one tick.
    pub fn value(&self) -> Option<Decimal> {
        self.current_value
    }
}
