use rust_decimal::Decimal;

/// Simple Moving Average
#[derive(Debug, Clone)]
pub struct Sma {
    period: usize,
    values: Vec<Decimal>,
}

impl Sma {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            values: Vec::new(),
        }
    }

    pub fn update(&mut self, val: f64) -> Option<Decimal> {
        let val = Decimal::from_f64_retain(val).unwrap_or(Decimal::ZERO);
        if self.period == 0 {
            return None;
        }
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

    #[test]
    fn test_returns_none_before_full_period() {
        let mut sma = Sma::new(5);
        assert_eq!(sma.update(10.0), None);
        assert_eq!(sma.update(20.0), None);
        assert_eq!(sma.update(30.0), None);
        assert_eq!(sma.update(40.0), None);
    }

    #[test]
    fn test_returns_average_at_period_boundary() {
        let mut sma = Sma::new(3);
        sma.update(10.0);
        sma.update(20.0);
        let result = sma.update(30.0).unwrap();
        let expected = Decimal::from_f64_retain(20.0).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_sliding_window_evicts_oldest() {
        let mut sma = Sma::new(3);
        sma.update(10.0);
        sma.update(20.0);
        sma.update(30.0);
        let result = sma.update(60.0).unwrap();
        let expected = (Decimal::from_f64_retain(20.0).unwrap()
            + Decimal::from_f64_retain(30.0).unwrap()
            + Decimal::from_f64_retain(60.0).unwrap())
            / Decimal::from(3);
        assert_eq!(result, expected);
    }
}
