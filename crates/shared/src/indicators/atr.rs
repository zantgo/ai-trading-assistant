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
