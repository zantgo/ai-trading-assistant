use super::traits::{BarInput, Indicator};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Historical Volatility: annualized standard deviation of log returns over N
/// bars, expressed as a percentage. Non-directional volatility magnitude.
#[derive(Debug, Clone)]
pub struct HistoricalVolatility {
    period: usize,
    prev_close: Option<f64>,
    returns: VecDeque<f64>,
}

impl HistoricalVolatility {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prev_close: None,
            returns: VecDeque::with_capacity(period),
        }
    }

    pub fn update(&mut self, close: Decimal) -> Option<Decimal> {
        if self.period == 0 {
            return None;
        }
        let c = close.to_f64().unwrap_or(0.0);
        let prev = match self.prev_close {
            None => {
                self.prev_close = Some(c);
                return None;
            }
            Some(p) => p,
        };
        self.prev_close = Some(c);
        if prev <= 0.0 || c <= 0.0 {
            return None;
        }
        self.returns.push_back((c / prev).ln());
        if self.returns.len() > self.period {
            self.returns.pop_front();
        }
        if self.returns.len() < self.period {
            return None;
        }
        let n = self.returns.len() as f64;
        let mean = self.returns.iter().sum::<f64>() / n;
        let var = self.returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
        // Annualize assuming ~365d crypto sessions; expressed as percent.
        let hv = var.sqrt() * (365.0_f64).sqrt() * 100.0;
        Decimal::from_f64_retain(hv)
    }
}

impl Indicator for HistoricalVolatility {
    type Output = Option<Decimal>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.close)
    }
    fn reset(&mut self) {
        *self = HistoricalVolatility::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(hv: &mut HistoricalVolatility, c: f64) -> Option<Decimal> {
        hv.update(Decimal::from_f64_retain(c).unwrap())
    }

    #[test]
    fn test_none_before_period() {
        let mut hv = HistoricalVolatility::new(5);
        feed(&mut hv, 100.0);
        assert!(feed(&mut hv, 101.0).is_none());
    }

    #[test]
    fn test_constant_price_zero_vol() {
        let mut hv = HistoricalVolatility::new(5);
        let mut last = None;
        for _ in 0..8 {
            last = feed(&mut hv, 100.0);
        }
        assert_eq!(last.unwrap(), dec!(0));
    }

    #[test]
    fn test_volatile_series_positive() {
        let mut hv = HistoricalVolatility::new(5);
        feed(&mut hv, 100.0);
        let mut last = None;
        for i in 0..10 {
            let p = if i % 2 == 0 { 120.0 } else { 90.0 };
            last = feed(&mut hv, p);
        }
        assert!(last.unwrap() > dec!(0), "volatile series → positive HV");
    }
}
