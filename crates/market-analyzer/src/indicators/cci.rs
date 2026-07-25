use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Commodity Channel Index — measures the deviation of typical price from its
/// statistical mean, producing an oscillator that identifies cyclical turns.
#[derive(Debug, Clone)]
pub struct Cci {
    period: usize,
    typicals: VecDeque<Decimal>,
}

impl Cci {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            typicals: VecDeque::with_capacity(period + 1),
        }
    }

    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<Decimal> {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        if self.period == 0 {
            return None;
        }
        let tp = (high + low + close) / Decimal::from(3);
        self.typicals.push_back(tp);
        while self.typicals.len() > self.period {
            self.typicals.pop_front();
        }
        if self.typicals.len() < self.period {
            return None;
        }
        let sum: Decimal = self.typicals.iter().sum();
        let sma = sum / Decimal::from(self.period);
        let mean_dev: Decimal = self
            .typicals
            .iter()
            .map(|t| (*t - sma).abs())
            .sum::<Decimal>()
            / Decimal::from(self.period);
        if mean_dev < Decimal::from_f64_retain(1e-9).unwrap() {
            return Some(Decimal::ZERO);
        }
        let cci = (tp - sma) / (Decimal::from_f64_retain(0.015).unwrap() * mean_dev);
        Some(cci)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(cci: &mut Cci, n: usize, base: f64) {
        for i in 0..n {
            let h = base + i as f64 + 1.0;
            let l = base + i as f64 - 1.0;
            let c = base + i as f64;
            cci.update(h, l, c);
        }
    }

    #[test]
    fn test_none_before_period() {
        let mut cci = Cci::new(20);
        let out = cci.update(100.0, 98.0, 99.0);
        assert!(out.is_none());
    }

    #[test]
    fn test_produces_output_after_period() {
        let mut cci = Cci::new(20);
        feed(&mut cci, 21, 100.0);
        let out = cci.update(121.0, 119.0, 120.0);
        assert!(out.is_some());
    }

    #[test]
    fn test_flat_prices_near_zero() {
        let mut cci = Cci::new(20);
        // True flat prices (no trend) → CCI near 0.
        for _ in 0..20 {
            cci.update(101.0, 99.0, 100.0);
        }
        let out = cci.update(101.0, 99.0, 100.0).unwrap();
        assert!(
            out.abs() < Decimal::from_f64_retain(1.0).unwrap(),
            "flat prices should produce near-zero CCI, got {}",
            out
        );
    }

    #[test]
    fn test_uptrend_positive_cci() {
        let mut cci = Cci::new(20);
        feed(&mut cci, 20, 100.0);
        // Strong uptrend spike at the end → positive CCI.
        let out = cci.update(140.0, 119.0, 135.0).unwrap();
        assert!(out > Decimal::from_f64_retain(0.0).unwrap());
    }
}
