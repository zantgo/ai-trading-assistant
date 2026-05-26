use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};
use super::sma::Sma;

/// Bollinger Bands Indicator (20 SMA +/- 2 Standard Deviations)
#[derive(Debug, Clone)]
pub struct BollingerBands {
    sma_20: Sma,
    prices_history: Vec<Decimal>,
}

impl BollingerBands {
    pub fn new() -> Self {
        Self {
            sma_20: Sma::new(20),
            prices_history: Vec::new(),
        }
    }

    /// Update with a closed price, returning (upper, middle, lower)
    pub fn update(&mut self, close: Decimal) -> Option<(Decimal, Decimal, Decimal)> {
        self.prices_history.push(close);
        if self.prices_history.len() > 20 {
            self.prices_history.remove(0);
        }

        let sma = self.sma_20.update(close)?;

        if self.prices_history.len() < 20 {
            return None;
        }

        let std_dev = {
            let sum_sq: f64 = self.prices_history.iter()
                .map(|&p| {
                    let diff = (p - sma).to_f64().unwrap_or(0.0);
                    diff * diff
                })
                .sum();
            let variance = sum_sq / 20.0;
            Decimal::from_f64(variance.sqrt()).unwrap_or(Decimal::ZERO)
        };

        let upper = sma + std_dev * Decimal::from(2);
        let lower = sma - std_dev * Decimal::from(2);

        Some((upper, sma, lower))
    }
}
