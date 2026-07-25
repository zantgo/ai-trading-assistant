use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;

/// Exponential Moving Average
#[derive(Debug, Clone)]
pub struct Ema {
    period: usize,
    current_value: Option<Decimal>,
}

impl Ema {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            current_value: None,
        }
    }

    pub fn update(&mut self, price: f64) -> Decimal {
        let price = Decimal::from_f64_retain(price).unwrap_or(Decimal::ZERO);
        match self.current_value {
            None => {
                self.current_value = Some(price);
                price
            }
            Some(prev_ema) => {
                let p_dec = Decimal::from(self.period);
                let multiplier = Decimal::from(2) / (p_dec + Decimal::ONE);
                let next_ema = (price - prev_ema) * multiplier + prev_ema;
                self.current_value = Some(next_ema);
                next_ema
            }
        }
    }
}

impl Indicator for Ema {
    type Output = Decimal;

    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.close)
    }

    fn reset(&mut self) {
        *self = Ema::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_returns_price() {
        let mut ema = Ema::new(10);
        let result = ema.update(100.0);
        assert_eq!(result, Decimal::from_f64_retain(100.0).unwrap());
    }

    #[test]
    fn test_repeated_same_price_converges() {
        let mut ema = Ema::new(10);
        ema.update(100.0);
        for _ in 0..20 {
            ema.update(100.0);
        }
        let result = ema.update(100.0);
        assert!(
            (result - Decimal::from_f64_retain(100.0).unwrap()).abs()
                < Decimal::from_f64_retain(0.01).unwrap()
        );
    }

    #[test]
    fn test_rising_prices_produce_rising_ema() {
        let mut ema = Ema::new(5);
        ema.update(100.0);
        let v1 = ema.update(110.0);
        let v2 = ema.update(120.0);
        assert!(v2 > v1, "EMA should rise with rising prices");
    }

    #[test]
    fn test_ema_period_2_seeds_correctly() {
        let mut ema = Ema::new(2);
        assert_eq!(ema.update(10.0), Decimal::from_f64_retain(10.0).unwrap());
        let result = ema.update(10.0);
        assert_eq!(result, Decimal::from_f64_retain(10.0).unwrap());
    }
}
