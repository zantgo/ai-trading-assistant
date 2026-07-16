use super::sma::Sma;
use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct ObvOutput {
    /// Cumulative On-Balance Volume.
    pub obv: Decimal,
    /// Smoothed OBV (SMA over `smoothing`) — used for directional slope.
    pub obv_sma: Decimal,
}

/// On-Balance Volume: running cumulative volume signed by close direction.
/// The paired SMA provides a directional slope reference (OBV is unbounded, so
/// normalization keys off `obv` vs `obv_sma`).
#[derive(Debug, Clone)]
pub struct Obv {
    prev_close: Option<Decimal>,
    obv: Decimal,
    sma: Sma,
}

impl Obv {
    pub fn new(smoothing: usize) -> Self {
        Self {
            prev_close: None,
            obv: Decimal::ZERO,
            sma: Sma::new(smoothing.max(1)),
        }
    }

    pub fn update(&mut self, close: Decimal, volume: Decimal) -> Option<ObvOutput> {
        match self.prev_close {
            None => {
                self.prev_close = Some(close);
                // Seed OBV at 0; no directional info yet.
                let _ = self.sma.update(self.obv);
                None
            }
            Some(prev) => {
                if close > prev {
                    self.obv += volume;
                } else if close < prev {
                    self.obv -= volume;
                }
                self.prev_close = Some(close);
                let sma = self.sma.update(self.obv)?;
                Some(ObvOutput {
                    obv: self.obv,
                    obv_sma: sma,
                })
            }
        }
    }
}

impl Indicator for Obv {
    type Output = Option<ObvOutput>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.close, bar.volume)
    }
    fn reset(&mut self) {
        // period recovered from the internal SMA is not exposed; reinit at 20.
        *self = Obv::new(20);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(o: &mut Obv, c: f64, v: f64) -> Option<ObvOutput> {
        o.update(
            Decimal::from_f64_retain(c).unwrap(),
            Decimal::from_f64_retain(v).unwrap(),
        )
    }

    #[test]
    fn test_rising_closes_increase_obv() {
        let mut o = Obv::new(3);
        feed(&mut o, 100.0, 10.0);
        feed(&mut o, 101.0, 10.0);
        feed(&mut o, 102.0, 10.0);
        let out = feed(&mut o, 103.0, 10.0).unwrap();
        assert_eq!(out.obv, dec!(30));
        assert!(out.obv > out.obv_sma, "rising OBV should exceed its SMA");
    }

    #[test]
    fn test_falling_closes_decrease_obv() {
        let mut o = Obv::new(3);
        feed(&mut o, 100.0, 10.0);
        feed(&mut o, 99.0, 10.0);
        feed(&mut o, 98.0, 10.0);
        let out = feed(&mut o, 97.0, 10.0).unwrap();
        assert_eq!(out.obv, dec!(-30));
    }
}
