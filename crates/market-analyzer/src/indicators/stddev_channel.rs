use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Standard Deviation Channel — linear regression centerline with ±2σ bands.
/// Price near upper band = overextended (bearish), near lower band = oversold
/// (bullish). Similar to Bollinger Bands but uses linear regression instead
/// of a simple moving average as the centerline.
#[derive(Debug, Clone)]
pub struct StdDevChannel {
    period: usize,
    closes: VecDeque<Decimal>,
}

#[derive(Debug, Clone, Copy)]
pub struct SdChannelOutput {
    pub center: Decimal,
    pub upper: Decimal,
    pub lower: Decimal,
    pub slope: Decimal,
}

impl StdDevChannel {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            closes: VecDeque::with_capacity(period + 1),
        }
    }

    pub fn update(&mut self, close: Decimal) -> Option<SdChannelOutput> {
        self.closes.push_back(close);
        while self.closes.len() > self.period {
            self.closes.pop_front();
        }
        let n = self.closes.len();
        if n < self.period {
            return None;
        }
        let sum_x: i64 = (0..n).sum::<usize>() as i64;
        let sum_x2: i64 = (0..n).map(|i| (i * i) as i64).sum();
        let sum_y: Decimal = self.closes.iter().sum();
        let sum_xy: Decimal = self
            .closes
            .iter()
            .enumerate()
            .map(|(i, c)| Decimal::from(i) * *c)
            .sum();
        let n_d = Decimal::from(n);
        let denom = Decimal::from(n) * Decimal::from(sum_x2) - Decimal::from(sum_x * sum_x);
        let slope = if denom != Decimal::ZERO && denom.abs() > Decimal::from(1) {
            (n_d * sum_xy - Decimal::from(sum_x) * sum_y) / denom
        } else {
            Decimal::ZERO
        };
        if n_d == Decimal::ZERO {
            return None;
        }
        let intercept = (sum_y - slope * Decimal::from(sum_x)) / n_d;
        let center = intercept + slope * Decimal::from(n - 1);
        let mut sq_sum = Decimal::ZERO;
        for (i, c) in self.closes.iter().enumerate() {
            let fitted = intercept + slope * Decimal::from(i);
            sq_sum += (*c - fitted) * (*c - fitted);
        }
        let std = {
            let v = ((sq_sum / n_d).to_f64().unwrap_or(0.0)).sqrt();
            Decimal::from_f64_retain(v).unwrap_or(Decimal::ZERO)
        };
        Some(SdChannelOutput {
            center,
            upper: center + Decimal::from(2) * std,
            lower: center - Decimal::from(2) * std,
            slope,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_none_before_period() {
        let mut sd = StdDevChannel::new(20);
        for _ in 0..19 {
            assert!(sd.update(dec!(100)).is_none());
        }
    }

    #[test]
    fn test_produces_output_after_period() {
        let mut sd = StdDevChannel::new(20);
        for _ in 0..20 {
            sd.update(dec!(100));
        }
        assert!(sd.update(dec!(100)).is_some());
    }

    #[test]
    fn test_upper_above_center() {
        let mut sd = StdDevChannel::new(20);
        for _ in 0..20 {
            sd.update(dec!(100));
        }
        let out = sd.update(dec!(100)).unwrap();
        assert!(out.upper >= out.center);
        assert!(out.center >= out.lower);
    }
}
