use rust_decimal::Decimal;

/// Simple Moving Average
#[derive(Debug, Clone)]
pub struct Sma {
    period: usize,
    values: Vec<Decimal>,
}

impl Sma {
    pub fn new(period: usize) -> Self {
        Self { period, values: Vec::new() }
    }

    pub fn update(&mut self, val: Decimal) -> Option<Decimal> {
        self.values.push(val);
        if self.values.len() > self.period {
            self.values.remove(0);
        }
        if self.values.len() == self.period {
            let sum: Decimal = self.values.iter().sum();
            Some(sum / Decimal::from(self.period))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_returns_none_before_full_period() {
        let mut sma = Sma::new(5);
        assert_eq!(sma.update(dec!(10.00)), None);
        assert_eq!(sma.update(dec!(20.00)), None);
        assert_eq!(sma.update(dec!(30.00)), None);
        assert_eq!(sma.update(dec!(40.00)), None);
    }

    #[test]
    fn test_returns_average_at_period_boundary() {
        let mut sma = Sma::new(3);
        sma.update(dec!(10.00));
        sma.update(dec!(20.00));
        let result = sma.update(dec!(30.00)).unwrap();
        assert_eq!(result, dec!(20.00));
    }

    #[test]
    fn test_sliding_window_evicts_oldest() {
        let mut sma = Sma::new(3);
        sma.update(dec!(10.00));
        sma.update(dec!(20.00));
        sma.update(dec!(30.00));
        let result = sma.update(dec!(60.00)).unwrap();
        let expected = (dec!(20.00) + dec!(30.00) + dec!(60.00)) / Decimal::from(3);
        assert_eq!(result, expected);
    }
}
