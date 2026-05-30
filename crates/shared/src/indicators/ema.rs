use rust_decimal::Decimal;

/// Exponential Moving Average
#[derive(Debug, Clone)]
pub struct Ema {
    period: usize,
    current_value: Option<Decimal>,
}

impl Ema {
    pub fn new(period: usize) -> Self {
        Self { period, current_value: None }
    }

    pub fn update(&mut self, price: Decimal) -> Decimal {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_seed_returns_price() {
        let mut ema = Ema::new(10);
        let result = ema.update(dec!(100.00));
        assert_eq!(result, dec!(100.00));
    }

    #[test]
    fn test_repeated_same_price_converges() {
        let mut ema = Ema::new(10);
        ema.update(dec!(100.00));
        for _ in 0..20 {
            ema.update(dec!(100.00));
        }
        let result = ema.update(dec!(100.00));
        assert!((result - dec!(100.00)).abs() < dec!(0.01));
    }

    #[test]
    fn test_rising_prices_produce_rising_ema() {
        let mut ema = Ema::new(5);
        ema.update(dec!(100.00));
        let v1 = ema.update(dec!(110.00));
        let v2 = ema.update(dec!(120.00));
        assert!(v2 > v1, "EMA should rise with rising prices");
    }

    #[test]
    fn test_ema_period_2_seeds_correctly() {
        let mut ema = Ema::new(2);
        assert_eq!(ema.update(dec!(10.00)), dec!(10.00));
        let result = ema.update(dec!(10.00));
        assert_eq!(result, dec!(10.00));
    }
}
