use rust_decimal::Decimal;
use super::ema::Ema;

/// Average True Range
#[derive(Debug, Clone)]
pub struct Atr {
    prev_close: Option<Decimal>,
    tr_ema: Ema,
}

impl Atr {
    pub fn new(period: usize) -> Self {
        Self {
            prev_close: None,
            tr_ema: Ema::new(period),
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<Decimal> {
        let tr = match self.prev_close {
            None => high - low,
            Some(prev) => {
                let r1 = high - low;
                let r2 = (high - prev).abs();
                let r3 = (low - prev).abs();
                r1.max(r2).max(r3)
            }
        };
        self.prev_close = Some(close);
        Some(self.tr_ema.update(tr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_first_call_uses_simple_high_low() {
        let mut atr = Atr::new(14);
        let result = atr.update(dec!(110.00), dec!(100.00), dec!(105.00)).unwrap();
        assert_eq!(result, dec!(10.00), "First call: TR = H - L = 10");
    }

    #[test]
    fn test_subsequent_calls_use_true_range() {
        let mut atr = Atr::new(14);
        atr.update(dec!(110.00), dec!(100.00), dec!(105.00));
        let result = atr.update(dec!(108.00), dec!(102.00), dec!(104.00)).unwrap();
        assert!(result > dec!(0.00));
    }

    #[test]
    fn test_atr_increases_with_volatility_spike() {
        let mut atr = Atr::new(5);
        for _ in 0..6 {
            atr.update(dec!(101.00), dec!(99.00), dec!(100.00));
        }
        let normal_atr = atr.update(dec!(101.00), dec!(99.00), dec!(100.00)).unwrap();
        let spike_atr = atr.update(dec!(120.00), dec!(80.00), dec!(100.00)).unwrap();
        assert!(spike_atr > normal_atr, "ATR should increase on volatility spike");
    }
}
