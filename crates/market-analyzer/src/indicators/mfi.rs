use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Money Flow Index: volume-weighted RSI over N bars. Bounded `[0, 100]`.
#[derive(Debug, Clone)]
pub struct Mfi {
    period: usize,
    prev_tp: Option<Decimal>,
    pos_flows: VecDeque<Decimal>,
    neg_flows: VecDeque<Decimal>,
}

impl Mfi {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prev_tp: None,
            pos_flows: VecDeque::with_capacity(period),
            neg_flows: VecDeque::with_capacity(period),
        }
    }

    pub fn update(&mut self, high: f64, low: f64, close: f64, volume: f64) -> Option<Decimal> {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        let volume = Decimal::from_f64_retain(volume).unwrap_or(Decimal::ZERO);
        if self.period == 0 {
            return None;
        }
        let tp = (high + low + close) / Decimal::from(3);
        let raw_flow = tp * volume;
        let prev = match self.prev_tp {
            None => {
                self.prev_tp = Some(tp);
                return None;
            }
            Some(p) => p,
        };
        self.prev_tp = Some(tp);

        let (pos, neg) = if tp > prev {
            (raw_flow, Decimal::ZERO)
        } else if tp < prev {
            (Decimal::ZERO, raw_flow)
        } else {
            (Decimal::ZERO, Decimal::ZERO)
        };
        self.pos_flows.push_back(pos);
        self.neg_flows.push_back(neg);
        if self.pos_flows.len() > self.period {
            self.pos_flows.pop_front();
            self.neg_flows.pop_front();
        }
        if self.pos_flows.len() < self.period {
            return None;
        }
        let sum_pos: Decimal = self.pos_flows.iter().sum();
        let sum_neg: Decimal = self.neg_flows.iter().sum();
        if sum_neg == Decimal::ZERO {
            return Some(Decimal::from(100));
        }
        let ratio = sum_pos / sum_neg;
        let mfi = Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + ratio));
        Some(mfi)
    }
}

impl Indicator for Mfi {
    type Output = Option<Decimal>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.high, bar.low, bar.close, bar.volume)
    }
    fn reset(&mut self) {
        *self = Mfi::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(m: &mut Mfi, h: f64, l: f64, c: f64, v: f64) -> Option<Decimal> {
        m.update(h, l, c, v)
    }

    #[test]
    fn test_all_rising_high_mfi() {
        let mut m = Mfi::new(5);
        let mut p = 100.0;
        feed(&mut m, p, p, p, 10.0);
        let mut last = None;
        for _ in 0..6 {
            p += 1.0;
            last = feed(&mut m, p, p, p, 10.0);
        }
        assert_eq!(last.unwrap(), dec!(100));
    }

    #[test]
    fn test_bounds() {
        let mut m = Mfi::new(14);
        let mut p = 100.0;
        feed(&mut m, p, p, p, 10.0);
        for i in 0..40 {
            p += if i % 2 == 0 { 2.0 } else { -1.5 };
            if let Some(v) = feed(&mut m, p + 1.0, p - 1.0, p, 10.0) {
                assert!(v >= dec!(0) && v <= dec!(100), "MFI out of range: {}", v);
            }
        }
    }
}
