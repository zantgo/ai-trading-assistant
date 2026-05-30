use rust_decimal::Decimal;

/// Relative Strength Index (using Wilder's Smoothing)
#[derive(Debug, Clone)]
pub struct Rsi {
    period: usize,
    prev_close: Option<Decimal>,
    avg_gain: Option<Decimal>,
    avg_loss: Option<Decimal>,
}

impl Rsi {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prev_close: None,
            avg_gain: None,
            avg_loss: None,
        }
    }

    pub fn update(&mut self, close: Decimal) -> Option<Decimal> {
        let prev = match self.prev_close {
            None => {
                self.prev_close = Some(close);
                return None;
            }
            Some(p) => p,
        };
        self.prev_close = Some(close);

        let change = close - prev;
        let gain = if change > Decimal::ZERO { change } else { Decimal::ZERO };
        let loss = if change < Decimal::ZERO { change.abs() } else { Decimal::ZERO };

        match (self.avg_gain, self.avg_loss) {
            (Some(ag), Some(al)) => {
                let p_dec = Decimal::from(self.period);
                let p_minus_1 = p_dec - Decimal::ONE;

                let next_ag = (ag * p_minus_1 + gain) / p_dec;
                let next_al = (al * p_minus_1 + loss) / p_dec;

                self.avg_gain = Some(next_ag);
                self.avg_loss = Some(next_al);

                if next_al == Decimal::ZERO {
                    Some(Decimal::from(100))
                } else {
                    let rs = next_ag / next_al;
                    let rsi = Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs));
                    Some(rsi)
                }
            }
            _ => {
                self.avg_gain = Some(gain);
                self.avg_loss = Some(loss);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_first_update_returns_none() {
        let mut rsi = Rsi::new(14);
        assert_eq!(rsi.update(dec!(100.00)), None);
    }

    #[test]
    fn test_second_update_returns_none() {
        let mut rsi = Rsi::new(14);
        rsi.update(dec!(100.00));
        assert_eq!(rsi.update(dec!(105.00)), None);
    }

    #[test]
    fn test_all_gains_yields_high_rsi() {
        let mut rsi = Rsi::new(14);
        rsi.update(dec!(100.00));
        let mut price = dec!(100.00);
        for _ in 0..14 {
            price += dec!(1.00);
            rsi.update(price);
        }
        let result = rsi.update(price + dec!(1.00)).unwrap();
        assert!(result > dec!(50.00), "All gains should yield RSI > 50");
    }

    #[test]
    fn test_all_losses_yields_low_rsi() {
        let mut rsi = Rsi::new(14);
        rsi.update(dec!(100.00));
        let mut price = dec!(100.00);
        for _ in 0..14 {
            price -= dec!(1.00);
            rsi.update(price);
        }
        let result = rsi.update(price - dec!(1.00)).unwrap();
        assert!(result < dec!(50.00), "All losses should yield RSI < 50");
    }
}
