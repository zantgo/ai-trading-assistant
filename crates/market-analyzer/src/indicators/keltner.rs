use super::atr::Atr;
use super::ema::Ema;
use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct KeltnerOutput {
    pub upper: Decimal,
    pub middle: Decimal,
    pub lower: Decimal,
}

/// Keltner Channels: EMA middle band ± (multiplier × ATR).
#[derive(Debug, Clone)]
pub struct Keltner {
    ema_period: usize,
    atr_period: usize,
    multiplier: Decimal,
    ema: Ema,
    atr: Atr,
}

impl Keltner {
    pub fn new(ema_period: usize, atr_period: usize, multiplier: f64) -> Self {
        Self {
            ema_period,
            atr_period,
            multiplier: Decimal::from_f64_retain(multiplier).unwrap_or(Decimal::from(2)),
            ema: Ema::new(ema_period),
            atr: Atr::new(atr_period),
        }
    }

    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<KeltnerOutput> {
        if self.ema_period == 0 || self.atr_period == 0 {
            return None;
        }
        let middle = self.ema.update(close);
        let atr = self.atr.update(high, low, close)?.atr_value;
        let band = self.multiplier * atr;
        Some(KeltnerOutput {
            upper: middle + band,
            middle,
            lower: middle - band,
        })
    }
}

impl Indicator for Keltner {
    type Output = Option<KeltnerOutput>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.high, bar.low, bar.close)
    }
    fn reset(&mut self) {
        let m = self.multiplier.try_into().unwrap_or(2.0);
        *self = Keltner::new(self.ema_period, self.atr_period, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn feed(k: &mut Keltner, h: f64, l: f64, c: f64) -> Option<KeltnerOutput> {
        k.update(h, l, c)
    }

    #[test]
    fn test_bands_ordered() {
        let mut k = Keltner::new(20, 10, 2.0);
        let mut out = None;
        for i in 0..40 {
            let p = 100.0 + (i as f64) * 0.5;
            out = feed(&mut k, p + 1.0, p - 1.0, p);
        }
        let o = out.unwrap();
        assert!(o.upper > o.middle && o.middle > o.lower, "upper>mid>lower");
    }

    #[test]
    fn test_zero_period_none() {
        let mut k = Keltner::new(0, 10, 2.0);
        assert!(feed(&mut k, 10.0, 9.0, 9.5).is_none());
    }

    #[test]
    fn test_flat_market_narrow_band() {
        let mut k = Keltner::new(5, 5, 2.0);
        let mut out = None;
        for _ in 0..15 {
            out = feed(&mut k, 100.0, 100.0, 100.0);
        }
        let o = out.unwrap();
        assert_eq!(
            o.middle,
            Decimal::from_f64_retain(100.0).unwrap_or_default()
        );
        assert_eq!(o.upper, o.lower);
    }
}
