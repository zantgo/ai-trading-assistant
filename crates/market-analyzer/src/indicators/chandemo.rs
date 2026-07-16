use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Chande Momentum Oscillator: raw momentum ratio tracking summed gains vs
/// losses over the lookback window, natively bounded in `[-100, 100]`.
#[derive(Debug, Clone)]
pub struct ChandeMO {
    period: usize,
    prev_close: Option<Decimal>,
    gains: VecDeque<Decimal>,
    losses: VecDeque<Decimal>,
    prev_value: Option<Decimal>,
}

impl ChandeMO {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prev_close: None,
            gains: VecDeque::with_capacity(period),
            losses: VecDeque::with_capacity(period),
            prev_value: None,
        }
    }

    pub fn update(&mut self, close: Decimal) -> Option<Decimal> {
        if self.period == 0 {
            return None;
        }
        let prev = match self.prev_close {
            None => {
                self.prev_close = Some(close);
                return None;
            }
            Some(p) => p,
        };
        self.prev_close = Some(close);

        let change = close - prev;
        let gain = if change > Decimal::ZERO {
            change
        } else {
            Decimal::ZERO
        };
        let loss = if change < Decimal::ZERO {
            change.abs()
        } else {
            Decimal::ZERO
        };

        self.gains.push_back(gain);
        self.losses.push_back(loss);

        if self.gains.len() > self.period {
            self.gains.pop_front();
            self.losses.pop_front();
        }

        if self.gains.len() < self.period {
            return None;
        }

        let sum_gains: Decimal = self.gains.iter().sum();
        let sum_losses: Decimal = self.losses.iter().sum();
        let divisor = sum_gains + sum_losses;

        let value = if divisor == Decimal::ZERO {
            Decimal::ZERO
        } else {
            ((sum_gains - sum_losses) / divisor) * Decimal::from(100)
        };

        self.prev_value = Some(value);
        Some(value)
    }
}

impl Indicator for ChandeMO {
    type Output = Option<Decimal>;

    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.close)
    }

    fn reset(&mut self) {
        *self = ChandeMO::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(cmo: &mut ChandeMO, close: f64) -> Option<Decimal> {
        cmo.update(Decimal::from_f64_retain(close).unwrap())
    }

    #[test]
    fn test_first_update_returns_none() {
        let mut cmo = ChandeMO::new(9);
        assert_eq!(feed(&mut cmo, 100.0), None);
    }

    #[test]
    fn test_returns_none_before_full_period() {
        let mut cmo = ChandeMO::new(5);
        feed(&mut cmo, 100.0);
        assert!(feed(&mut cmo, 101.0).is_none());
        assert!(feed(&mut cmo, 102.0).is_none());
        assert!(feed(&mut cmo, 103.0).is_none());
    }

    #[test]
    fn test_all_gains_yields_plus_100() {
        let mut cmo = ChandeMO::new(5);
        let mut price = 100.0;
        feed(&mut cmo, price);
        let mut last = None;
        for _ in 0..6 {
            price += 1.0;
            last = feed(&mut cmo, price);
        }
        assert_eq!(last.unwrap(), dec!(100));
    }

    #[test]
    fn test_all_losses_yields_minus_100() {
        let mut cmo = ChandeMO::new(5);
        let mut price = 100.0;
        feed(&mut cmo, price);
        let mut last = None;
        for _ in 0..6 {
            price -= 1.0;
            last = feed(&mut cmo, price);
        }
        assert_eq!(last.unwrap(), dec!(-100));
    }

    #[test]
    fn test_flat_prices_yield_zero() {
        let mut cmo = ChandeMO::new(5);
        let mut last = None;
        for _ in 0..8 {
            last = feed(&mut cmo, 100.0);
        }
        assert_eq!(last.unwrap(), dec!(0));
    }

    #[test]
    fn test_stays_within_bounds() {
        let mut cmo = ChandeMO::new(9);
        feed(&mut cmo, 100.0);
        for i in 0..50 {
            let price = if i % 3 == 0 { 120.0 } else { 90.0 };
            if let Some(v) = feed(&mut cmo, price) {
                assert!(v >= dec!(-100) && v <= dec!(100), "CMO out of range: {}", v);
            }
        }
    }
}
