use rust_decimal::Decimal;

/// Elder's Force Index — combines price change and volume to measure the
/// strength of buying/selling pressure, smoothed by an EMA for signal quality.
/// Positive FI = bulls in control; negative FI = bears in control.
///
/// AUDIT-AIU-043: additionally tracks a rolling EMA of |raw| so the
/// extreme-threshold detector can be scale-relative (|fi| > 30× mean) instead
/// of an absolute ±1000 that was meaningless across assets.
#[derive(Debug, Clone)]
pub struct ForceIndex {
    prev_close: Option<Decimal>,
    ema: Option<Decimal>,
    mean_abs_ema: Option<Decimal>,
    alpha: Decimal,
}

impl ForceIndex {
    /// AUDIT-AIU-042: `smoothing_period` is floored at 1 so a 0 (or
    /// degenerate) config cannot produce α = 2/1 = 2.0 — a wildly unstable
    /// EMA that oscillates instead of smoothing.
    pub fn new(smoothing_period: usize) -> Self {
        let p = smoothing_period.max(1);
        let alpha = Decimal::from(2) / Decimal::from(p + 1);
        Self {
            prev_close: None,
            ema: None,
            mean_abs_ema: None,
            alpha,
        }
    }

    pub fn update(&mut self, close: f64, volume: f64) -> Option<Decimal> {
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        let volume = Decimal::from_f64_retain(volume).unwrap_or(Decimal::ZERO);
        let raw = match self.prev_close {
            Some(pc) => (close - pc) * volume,
            None => {
                self.prev_close = Some(close);
                return None;
            }
        };
        self.prev_close = Some(close);
        self.mean_abs_ema = Some(match self.mean_abs_ema {
            Some(m) => m + self.alpha * (raw.abs() - m),
            None => raw.abs(),
        });
        self.ema = Some(match self.ema {
            Some(e) => e + self.alpha * (raw - e),
            None => raw,
        });
        self.ema
    }

    /// Rolling mean of |raw FI| — the scale-relative baseline for the
    /// extreme threshold detector.
    pub fn mean_abs(&self) -> Option<Decimal> {
        self.mean_abs_ema
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_on_first_bar() {
        let mut fi = ForceIndex::new(13);
        assert!(fi.update(100.0, 1000.0).is_none());
    }

    #[test]
    fn test_positive_on_uptick() {
        let mut fi = ForceIndex::new(13);
        fi.update(100.0, 1000.0);
        let out = fi.update(105.0, 1000.0).unwrap();
        assert!(out > Decimal::from_f64_retain(0.0).unwrap());
    }

    #[test]
    fn test_negative_on_downtick() {
        let mut fi = ForceIndex::new(13);
        fi.update(100.0, 1000.0);
        let out = fi.update(95.0, 1000.0).unwrap();
        assert!(out < Decimal::from_f64_retain(0.0).unwrap());
    }
}
