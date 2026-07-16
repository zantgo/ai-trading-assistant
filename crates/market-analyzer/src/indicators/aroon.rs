use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct AroonOutput {
    pub up: Decimal,
    pub down: Decimal,
    /// Aroon Oscillator = up - down, range [-100, 100].
    pub oscillator: Decimal,
}

/// Aroon: measures periods since the highest high / lowest low over the window,
/// classifying trend emergence vs consolidation.
#[derive(Debug, Clone)]
pub struct Aroon {
    period: usize,
    highs: VecDeque<Decimal>,
    lows: VecDeque<Decimal>,
}

impl Aroon {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            highs: VecDeque::with_capacity(period + 1),
            lows: VecDeque::with_capacity(period + 1),
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal) -> Option<AroonOutput> {
        if self.period == 0 {
            return None;
        }
        self.highs.push_back(high);
        self.lows.push_back(low);
        let window = self.period + 1;
        if self.highs.len() > window {
            self.highs.pop_front();
            self.lows.pop_front();
        }
        if self.highs.len() < window {
            return None;
        }
        let n = self.highs.len();
        // Most-recent extreme wins ties (>= / <=).
        let mut hi_idx = 0;
        let mut hi_val = self.highs[0];
        let mut lo_idx = 0;
        let mut lo_val = self.lows[0];
        for i in 0..n {
            if self.highs[i] >= hi_val {
                hi_val = self.highs[i];
                hi_idx = i;
            }
            if self.lows[i] <= lo_val {
                lo_val = self.lows[i];
                lo_idx = i;
            }
        }
        let bars_since_high = (n - 1 - hi_idx) as u64;
        let bars_since_low = (n - 1 - lo_idx) as u64;
        let p = Decimal::from(self.period);
        let up = Decimal::from(100) * (p - Decimal::from(bars_since_high)) / p;
        let down = Decimal::from(100) * (p - Decimal::from(bars_since_low)) / p;
        Some(AroonOutput {
            up,
            down,
            oscillator: up - down,
        })
    }
}

impl Indicator for Aroon {
    type Output = Option<AroonOutput>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.high, bar.low)
    }
    fn reset(&mut self) {
        *self = Aroon::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(a: &mut Aroon, h: f64, l: f64) -> Option<AroonOutput> {
        a.update(
            Decimal::from_f64_retain(h).unwrap(),
            Decimal::from_f64_retain(l).unwrap(),
        )
    }

    #[test]
    fn test_rising_highs_aroon_up_100() {
        let mut a = Aroon::new(5);
        let mut p = 100.0;
        let mut last = None;
        for _ in 0..8 {
            p += 1.0;
            last = feed(&mut a, p, p - 1.0);
        }
        let o = last.unwrap();
        assert_eq!(o.up, dec!(100), "new highs each bar → Aroon Up = 100");
        assert_eq!(o.oscillator, dec!(100));
    }

    #[test]
    fn test_falling_lows_aroon_down_100() {
        let mut a = Aroon::new(5);
        let mut p = 100.0;
        let mut last = None;
        for _ in 0..8 {
            p -= 1.0;
            last = feed(&mut a, p + 1.0, p);
        }
        assert_eq!(last.unwrap().down, dec!(100));
    }

    #[test]
    fn test_none_before_window_and_zero_period() {
        let mut a = Aroon::new(5);
        assert!(feed(&mut a, 10.0, 9.0).is_none());
        let mut z = Aroon::new(0);
        assert!(feed(&mut z, 10.0, 9.0).is_none());
    }
}
