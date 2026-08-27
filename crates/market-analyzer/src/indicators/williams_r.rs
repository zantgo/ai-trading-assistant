use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Williams %R — measures where the close sits relative to the high-low range
/// over a lookback period, normalized to [-100, 0]. Readings above -20 are
/// overbought, below -80 are oversold.
#[derive(Debug, Clone)]
pub struct WilliamsR {
    period: usize,
    highs: VecDeque<Decimal>,
    lows: VecDeque<Decimal>,
}

impl WilliamsR {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            highs: VecDeque::with_capacity(period + 1),
            lows: VecDeque::with_capacity(period + 1),
        }
    }

    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<Decimal> {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        self.highs.push_back(high);
        self.lows.push_back(low);
        while self.highs.len() > self.period {
            self.highs.pop_front();
            self.lows.pop_front();
        }
        if self.highs.len() < self.period {
            return None;
        }
        if self.highs.is_empty() || self.lows.is_empty() {
            return None;
        }
        let hh = self
            .highs
            .iter()
            .copied()
            .fold(Decimal::MIN, |a, b| a.max(b));
        let ll = self
            .lows
            .iter()
            .copied()
            .fold(Decimal::MAX, |a, b| a.min(b));
        if hh == Decimal::MIN || ll == Decimal::MAX {
            return Some(Decimal::from(-50));
        }
        let range = hh - ll;
        if range <= Decimal::ZERO {
            return Some(Decimal::from(-50));
        }
        let wr = (hh - close) / range * Decimal::NEGATIVE_ONE * Decimal::from(100);
        Some(wr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_before_period() {
        let mut wr = WilliamsR::new(14);
        assert!(wr.update(110.0, 90.0, 100.0).is_none());
    }

    #[test]
    fn test_close_at_high_is_zero() {
        let mut wr = WilliamsR::new(14);
        for _ in 0..14 {
            wr.update(110.0, 90.0, 110.0);
        }
        let out = wr.update(110.0, 90.0, 110.0).unwrap();
        assert!(
            out > Decimal::from_f64_retain(-1.0).unwrap_or_default()
                && out <= Decimal::from_f64_retain(0.0).unwrap_or_default()
        );
    }

    #[test]
    fn test_close_at_low_is_minus_100() {
        let mut wr = WilliamsR::new(14);
        for _ in 0..14 {
            wr.update(110.0, 90.0, 90.0);
        }
        let out = wr.update(110.0, 90.0, 90.0).unwrap();
        assert!(out < Decimal::from_f64_retain(-98.0).unwrap_or_default());
    }
}
