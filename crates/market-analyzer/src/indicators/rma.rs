//! Wilder's Running Moving Average (RMA).
//!
//! The canonical smoothing for ATR and ADX (Wilder, *New Concepts in
//! Technical Trading Systems*, 1978): seed with the simple average of the
//! first `period` values, then recurse:
//!
//! ```text
//! rma_t = (rma_{t-1} × (period - 1) + value_t) / period
//! ```
//!
//! This is NOT the plain EMA used elsewhere in the codebase (`α = 2/(N+1)`,
//! seed on the first value). Using the EMA for ATR/ADX changes the
//! smoothing constant and never converges to the canonical values — that
//! was AUDIT-AIU-003 (the documented D3/D5 deferral, now resolved).

use rust_decimal::Decimal;

/// Wilder's Running Moving Average.
#[derive(Debug, Clone)]
pub struct WilderRma {
    period: usize,
    /// First `period` raw values buffered for the SMA seed.
    seed_buffer: Vec<Decimal>,
    current: Option<Decimal>,
}

impl WilderRma {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            seed_buffer: Vec::with_capacity(period),
            current: None,
        }
    }

    /// Feed one value. Returns the RMA once the seed window is full,
    /// `None` before that.
    pub fn update(&mut self, value: f64) -> Option<Decimal> {
        let value = Decimal::from_f64_retain(value).unwrap_or(Decimal::ZERO);
        if let Some(prev) = self.current {
            let p = Decimal::from(self.period);
            let next = (prev * (p - Decimal::ONE) + value) / p;
            self.current = Some(next);
            return Some(next);
        }
        self.seed_buffer.push(value);
        if self.seed_buffer.len() < self.period {
            return None;
        }
        let sum: Decimal = self.seed_buffer.iter().sum();
        let seed = sum / Decimal::from(self.period);
        self.seed_buffer.clear();
        self.current = Some(seed);
        Some(seed)
    }

    /// Feed a value, seeding on the first sample when the window is not yet
    /// full. Used by callers that need an immediate reading (e.g. ATR).
    /// Returns the RMA value (seeded or smoothed).
    pub fn update_seeded(&mut self, value: f64) -> Decimal {
        match self.update(value) {
            Some(v) => v,
            // Not enough samples for the SMA seed yet — fall back to the
            // running mean so callers never block on warmup.
            None => {
                let sum: Decimal = self.seed_buffer.iter().sum();
                sum / Decimal::from(self.seed_buffer.len().max(1))
            }
        }
    }

    pub fn value(&self) -> Option<Decimal> {
        self.current
    }

    pub fn reset(&mut self) {
        self.seed_buffer.clear();
        self.current = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn seed_is_simple_average_of_first_period_values() {
        let mut rma = WilderRma::new(3);
        assert!(rma.update(1.0).is_none());
        assert!(rma.update(3.0).is_none());
        // (1 + 3 + 5) / 3 = 3.0
        assert_eq!(rma.update(5.0), Some(dec!(3.0)));
    }

    #[test]
    fn recursion_follows_wilders_formula() {
        let mut rma = WilderRma::new(3);
        rma.update(1.0);
        rma.update(3.0);
        assert_eq!(rma.update(5.0), Some(dec!(3.0)));
        // (3 × 2 + 11) / 3 = 17/3 = 5.6666...
        let v = rma.update(11.0).unwrap();
        assert!((v - dec!(5.6666666666666667)).abs() < dec!(0.000001));
    }

    #[test]
    fn update_seeded_returns_running_mean_early() {
        let mut rma = WilderRma::new(3);
        assert_eq!(rma.update_seeded(2.0), dec!(2.0));
        assert_eq!(rma.update_seeded(4.0), dec!(3.0));
        let third = rma.update_seeded(6.0);
        assert_eq!(third, dec!(4.0));
    }
}
