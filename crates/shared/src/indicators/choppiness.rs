use super::traits::{BarInput, Indicator};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Choppiness Index: quantifies whether the market is trending (low) or
/// consolidating/choppy (high) over N bars. Bounded `[0, 100]`. Non-directional.
#[derive(Debug, Clone)]
pub struct Choppiness {
    period: usize,
    prev_close: Option<Decimal>,
    trs: VecDeque<Decimal>,
    highs: VecDeque<Decimal>,
    lows: VecDeque<Decimal>,
}

impl Choppiness {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prev_close: None,
            trs: VecDeque::with_capacity(period),
            highs: VecDeque::with_capacity(period),
            lows: VecDeque::with_capacity(period),
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<Decimal> {
        if self.period <= 1 {
            return None;
        }
        let tr = match self.prev_close {
            None => high - low,
            Some(pc) => {
                let r1 = high - low;
                let r2 = (high - pc).abs();
                let r3 = (low - pc).abs();
                r1.max(r2).max(r3)
            }
        };
        self.prev_close = Some(close);
        self.trs.push_back(tr);
        self.highs.push_back(high);
        self.lows.push_back(low);
        if self.trs.len() > self.period {
            self.trs.pop_front();
            self.highs.pop_front();
            self.lows.pop_front();
        }
        if self.trs.len() < self.period {
            return None;
        }
        let sum_tr: Decimal = self.trs.iter().sum();
        let max_h = self.highs.iter().copied().max().unwrap_or(high);
        let min_l = self.lows.iter().copied().min().unwrap_or(low);
        let range = max_h - min_l;
        if range <= Decimal::ZERO {
            return Some(Decimal::from(100));
        }
        let ratio = (sum_tr / range).to_f64().unwrap_or(1.0).max(f64::MIN_POSITIVE);
        let chop = 100.0 * ratio.log10() / (self.period as f64).log10();
        Decimal::from_f64_retain(chop.clamp(0.0, 100.0))
    }
}

impl Indicator for Choppiness {
    type Output = Option<Decimal>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.high, bar.low, bar.close)
    }
    fn reset(&mut self) {
        *self = Choppiness::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(c: &mut Choppiness, h: f64, l: f64, cl: f64) -> Option<Decimal> {
        c.update(
            Decimal::from_f64_retain(h).unwrap(),
            Decimal::from_f64_retain(l).unwrap(),
            Decimal::from_f64_retain(cl).unwrap(),
        )
    }

    #[test]
    fn test_bounds() {
        let mut c = Choppiness::new(14);
        let mut p = 100.0;
        for i in 0..40 {
            p += if i % 2 == 0 { 3.0 } else { -2.0 };
            if let Some(v) = feed(&mut c, p + 1.0, p - 1.0, p) {
                assert!(v >= dec!(0) && v <= dec!(100), "CHOP out of range: {}", v);
            }
        }
    }

    #[test]
    fn test_strong_trend_low_chop() {
        let mut c = Choppiness::new(14);
        let mut p = 100.0;
        let mut last = None;
        for _ in 0..30 {
            p += 5.0;
            last = feed(&mut c, p, p - 0.5, p);
        }
        // A clean one-directional march yields a low choppiness reading.
        assert!(last.unwrap() < dec!(50), "trending market → low CHOP");
    }

    #[test]
    fn test_none_before_period() {
        let mut c = Choppiness::new(14);
        assert!(feed(&mut c, 10.0, 9.0, 9.5).is_none());
    }
}
