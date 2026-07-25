use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Chaikin Money Flow: volume-weighted accumulation/distribution over N bars.
/// Natively bounded in `[-1, 1]`.
#[derive(Debug, Clone)]
pub struct Cmf {
    period: usize,
    mfv: VecDeque<Decimal>,
    vol: VecDeque<Decimal>,
}

impl Cmf {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            mfv: VecDeque::with_capacity(period),
            vol: VecDeque::with_capacity(period),
        }
    }

    pub fn update(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Option<Decimal> {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        let volume = Decimal::from_f64_retain(volume).unwrap_or(Decimal::ZERO);
        if self.period == 0 {
            return None;
        }
        let range = high - low;
        let mf_mult = if range == Decimal::ZERO {
            Decimal::ZERO
        } else {
            ((close - low) - (high - close)) / range
        };
        self.mfv.push_back(mf_mult * volume);
        self.vol.push_back(volume);
        if self.mfv.len() > self.period {
            self.mfv.pop_front();
            self.vol.pop_front();
        }
        if self.mfv.len() < self.period {
            return None;
        }
        let sum_mfv: Decimal = self.mfv.iter().sum();
        let sum_vol: Decimal = self.vol.iter().sum();
        if sum_vol == Decimal::ZERO {
            Some(Decimal::ZERO)
        } else {
            Some(sum_mfv / sum_vol)
        }
    }
}

impl Indicator for Cmf {
    type Output = Option<Decimal>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.high, bar.low, bar.close, bar.volume)
    }
    fn reset(&mut self) {
        *self = Cmf::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(c: &mut Cmf, h: f64, l: f64, cl: f64, v: f64) -> Option<Decimal> {
        c.update(h, l, cl, v)
    }

    #[test]
    fn test_closes_at_high_positive() {
        let mut c = Cmf::new(3);
        feed(&mut c, 10.0, 8.0, 10.0, 100.0);
        feed(&mut c, 10.0, 8.0, 10.0, 100.0);
        let v = feed(&mut c, 10.0, 8.0, 10.0, 100.0).unwrap();
        assert_eq!(v, dec!(1));
    }

    #[test]
    fn test_closes_at_low_negative() {
        let mut c = Cmf::new(3);
        feed(&mut c, 10.0, 8.0, 8.0, 100.0);
        feed(&mut c, 10.0, 8.0, 8.0, 100.0);
        let v = feed(&mut c, 10.0, 8.0, 8.0, 100.0).unwrap();
        assert_eq!(v, dec!(-1));
    }

    #[test]
    fn test_stays_in_bounds() {
        let mut c = Cmf::new(5);
        for i in 0..30 {
            let cl = if i % 2 == 0 { 9.5 } else { 8.5 };
            if let Some(v) = feed(&mut c, 10.0, 8.0, cl, 50.0 + i as f64) {
                assert!(v >= dec!(-1) && v <= dec!(1), "CMF out of range: {}", v);
            }
        }
    }
}
