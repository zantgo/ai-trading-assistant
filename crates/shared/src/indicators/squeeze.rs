use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};
use super::sma::Sma;
use super::ema::Ema;
use super::atr::Atr;

/// Squeeze Momentum Indicator (John Carter / LazyBear implementation)
#[derive(Debug, Clone)]
pub struct SqueezeMomentum {
    sma_20: Sma,
    ema_20: Ema,
    atr_20: Atr,
    prices_history: Vec<Decimal>,
    high_history: Vec<Decimal>,
    low_history: Vec<Decimal>,
    val_history: Vec<Decimal>,
}

impl SqueezeMomentum {
    pub fn new(period: usize) -> Self {
        Self {
            sma_20: Sma::new(period),
            ema_20: Ema::new(period),
            atr_20: Atr::new(period),
            prices_history: Vec::new(),
            high_history: Vec::new(),
            low_history: Vec::new(),
            val_history: Vec::new(),
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<(bool, Decimal)> {
        self.prices_history.push(close);
        self.high_history.push(high);
        self.low_history.push(low);

        if self.prices_history.len() > 20 {
            self.prices_history.remove(0);
            self.high_history.remove(0);
            self.low_history.remove(0);
        }

        // Keep underlying SMA, EMA, and ATR instances updated on every iteration
        let sma = self.sma_20.update(close);
        let ema = self.ema_20.update(close);
        let atr = self.atr_20.update(high, low, close);

        // Always compute and track momentum metrics to warm up linear regression
        let highest_high = self.high_history.iter().max().copied().unwrap_or(high);
        let lowest_low = self.low_history.iter().min().copied().unwrap_or(low);
        let avg = ((highest_high + lowest_low) / Decimal::from(2) + ema) / Decimal::from(2);
        let val = close - avg;

        self.val_history.push(val);
        if self.val_history.len() > 20 {
            self.val_history.remove(0);
        }

        // We require at least 20 historical candles to form valid bands and channels
        if self.prices_history.len() < 20 {
            return None;
        }

        let sma_val = sma?;
        let atr_val = atr?;

        let std_dev = {
            let sum_sq: f64 = self.prices_history.iter()
                .map(|&p| {
                    let diff = (p - sma_val).to_f64().unwrap_or(0.0);
                    diff * diff
                })
                .sum();
            let variance = sum_sq / 20.0;
            Decimal::from_f64(variance.sqrt()).unwrap_or(Decimal::ZERO)
        };

        let bb_upper = sma_val + std_dev * Decimal::from(2);
        let bb_lower = sma_val - std_dev * Decimal::from(2);

        let kc_upper = ema + atr_val * Decimal::new(15, 1); // 1.5 multiplier
        let kc_lower = ema - atr_val * Decimal::new(15, 1);

        // Squeeze active: Bollinger Bands are compressed inside Keltner Channels
        let squeeze_on = bb_lower > kc_lower && bb_upper < kc_upper;

        if self.val_history.len() == 20 {
            // Linear regression of the last 20 'val' points (x: 0..19)
            let n = 20.0;
            let sum_x: f64 = 190.0;
            let sum_x_sq: f64 = 2470.0;

            let mut sum_y = 0.0;
            let mut sum_xy = 0.0;

            for (x, &y_dec) in self.val_history.iter().enumerate() {
                let y = y_dec.to_f64().unwrap_or(0.0);
                sum_y += y;
                sum_xy += (x as f64) * y;
            }

            let denominator = n * sum_x_sq - (sum_x * sum_x);
            let b = if denominator != 0.0 {
                (n * sum_xy - sum_x * sum_y) / denominator
            } else {
                0.0
            };

            let a = (sum_y - b * sum_x) / n;
            let momentum_val_f64 = a + b * 19.0;
            let momentum_val = Decimal::from_f64(momentum_val_f64).unwrap_or(Decimal::ZERO);

            Some((squeeze_on, momentum_val))
        } else {
            None
        }
    }
}
