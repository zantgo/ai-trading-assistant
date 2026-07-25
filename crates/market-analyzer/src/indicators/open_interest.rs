use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Open Interest tracker with rolling average and percentile computation.
/// Non-directional unless combined with price for OI-Price Divergence.
#[derive(Debug, Clone)]
pub struct OpenInterest {
    pub current: Option<Decimal>,
    history: VecDeque<f64>,
    lookback: usize,
}

impl OpenInterest {
    pub fn new(lookback: usize) -> Self {
        Self {
            current: None,
            history: VecDeque::with_capacity(lookback),
            lookback,
        }
    }

    pub fn update(&mut self, oi: f64) {
        self.current = Some(Decimal::from_f64_retain(oi).unwrap_or(Decimal::ZERO));
        self.history.push_back(oi);
        if self.history.len() > self.lookback {
            self.history.pop_front();
        }
    }

    pub fn raw(&self) -> Option<Decimal> {
        self.current
    }

    pub fn average(&self) -> Option<Decimal> {
        if self.history.is_empty() {
            return None;
        }
        let avg = self.history.iter().sum::<f64>() / self.history.len() as f64;
        Decimal::from_f64_retain(avg)
    }

    /// Percentile of current OI within the rolling history (0-100).
    pub fn percentile(&self) -> Option<f64> {
        let cur = self.current?.to_f64()?;
        if self.history.is_empty() {
            return None;
        }
        let n = self.history.len();
        let below = self.history.iter().filter(|&&v| v < cur).count();
        Some((below as f64 / n as f64) * 100.0)
    }

    /// OI delta over the full history window (current - oldest).
    pub fn delta_window(&self) -> Option<Decimal> {
        if self.history.len() < 2 {
            return None;
        }
        let oldest = self.history.front()?;
        let cur = self.history.back()?;
        Decimal::from_f64_retain(cur - oldest)
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.lookback);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_oi_average() {
        let mut oi = OpenInterest::new(100);
        for i in 1..=5 {
            oi.update((i * 100) as f64);
        }
        let avg = oi.average().unwrap();
        assert_eq!(avg, dec!(300));
    }

    #[test]
    fn test_oi_delta() {
        let mut oi = OpenInterest::new(100);
        oi.update(10000.0);
        oi.update(10500.0);
        oi.update(10300.0);
        let delta = oi.delta_window().unwrap();
        assert_eq!(delta, dec!(300));
    }

    #[test]
    fn test_oi_percentile() {
        let mut oi = OpenInterest::new(100);
        for v in [100, 200, 300, 400, 500] {
            oi.update(v as f64);
        }
        let pct = oi.percentile().unwrap();
        assert!(pct > 75.0);
    }
}
