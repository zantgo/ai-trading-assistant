use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct DonchianOutput {
    pub upper: Decimal,
    pub middle: Decimal,
    pub lower: Decimal,
}

/// Donchian Channels: highest-high / lowest-low over the lookback window.
#[derive(Debug, Clone)]
pub struct Donchian {
    period: usize,
    highs: VecDeque<Decimal>,
    lows: VecDeque<Decimal>,
}

impl Donchian {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            highs: VecDeque::with_capacity(period),
            lows: VecDeque::with_capacity(period),
        }
    }

    pub fn update(&mut self, high: f64, low: f64) -> Option<DonchianOutput> {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        if self.period == 0 {
            return None;
        }
        self.highs.push_back(high);
        self.lows.push_back(low);
        if self.highs.len() > self.period {
            self.highs.pop_front();
            self.lows.pop_front();
        }
        if self.highs.len() < self.period {
            return None;
        }
        let upper = self.highs.iter().copied().max().unwrap_or(high);
        let lower = self.lows.iter().copied().min().unwrap_or(low);
        Some(DonchianOutput {
            upper,
            middle: (upper + lower) / Decimal::from(2),
            lower,
        })
    }
}

impl Indicator for Donchian {
    type Output = Option<DonchianOutput>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.high, bar.low)
    }
    fn reset(&mut self) {
        *self = Donchian::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(d: &mut Donchian, h: f64, l: f64) -> Option<DonchianOutput> {
        d.update(h, l)
    }

    #[test]
    fn test_none_before_full_period() {
        let mut d = Donchian::new(3);
        assert!(feed(&mut d, 10.0, 9.0).is_none());
        assert!(feed(&mut d, 11.0, 10.0).is_none());
    }

    #[test]
    fn test_channel_bounds() {
        let mut d = Donchian::new(3);
        feed(&mut d, 12.0, 8.0);
        feed(&mut d, 14.0, 10.0);
        let o = feed(&mut d, 13.0, 11.0).unwrap();
        assert_eq!(o.upper, dec!(14));
        assert_eq!(o.lower, dec!(8));
        assert_eq!(o.middle, dec!(11));
    }

    #[test]
    fn test_zero_period_none() {
        let mut d = Donchian::new(0);
        assert!(feed(&mut d, 10.0, 9.0).is_none());
    }
}
