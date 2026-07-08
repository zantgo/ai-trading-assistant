use rust_decimal::Decimal;

/// Elder's Force Index — combines price change and volume to measure the
/// strength of buying/selling pressure, smoothed by an EMA for signal quality.
/// Positive FI = bulls in control; negative FI = bears in control.
#[derive(Debug, Clone)]
pub struct ForceIndex {
    prev_close: Option<Decimal>,
    ema: Option<Decimal>,
    alpha: Decimal,
}

impl ForceIndex {
    pub fn new(smoothing_period: usize) -> Self {
        let alpha = Decimal::from(2) / Decimal::from(smoothing_period + 1);
        Self {
            prev_close: None,
            ema: None,
            alpha,
        }
    }

    pub fn update(
        &mut self,
        close: Decimal,
        volume: Decimal,
    ) -> Option<Decimal> {
        let raw = match self.prev_close {
            Some(pc) => (close - pc) * volume,
            None => {
                self.prev_close = Some(close);
                return None;
            }
        };
        self.prev_close = Some(close);
        self.ema = Some(match self.ema {
            Some(e) => e + self.alpha * (raw - e),
            None => raw,
        });
        self.ema
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_none_on_first_bar() {
        let mut fi = ForceIndex::new(13);
        assert!(fi.update(dec!(100), dec!(1000)).is_none());
    }

    #[test]
    fn test_positive_on_uptick() {
        let mut fi = ForceIndex::new(13);
        fi.update(dec!(100), dec!(1000));
        let out = fi.update(dec!(105), dec!(1000)).unwrap();
        assert!(out > dec!(0));
    }

    #[test]
    fn test_negative_on_downtick() {
        let mut fi = ForceIndex::new(13);
        fi.update(dec!(100), dec!(1000));
        let out = fi.update(dec!(95), dec!(1000)).unwrap();
        assert!(out < dec!(0));
    }
}
