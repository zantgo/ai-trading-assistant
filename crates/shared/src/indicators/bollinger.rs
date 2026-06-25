use super::sma::Sma;
use super::traits::{BarInput, Indicator};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;

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
            let sum_sq: f64 = self
                .prices_history
                .iter()
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

impl Indicator for BollingerBands {
    type Output = Option<(Decimal, Decimal, Decimal)>;

    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.close)
    }

    fn reset(&mut self) {
        *self = BollingerBands::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_returns_none_before_20_values() {
        let mut bb = BollingerBands::new();
        for _ in 0..19 {
            assert_eq!(bb.update(dec!(100.00)), None);
        }
    }

    #[test]
    fn test_returns_bands_at_20_values() {
        let mut bb = BollingerBands::new();
        for _ in 0..19 {
            bb.update(dec!(100.00));
        }
        let result = bb.update(dec!(100.00)).unwrap();
        assert!(result.0 >= result.1);
        assert!(result.1 >= result.2);
    }

    #[test]
    fn test_upper_band_widens_with_volatility() {
        let mut bb = BollingerBands::new();
        for _ in 0..20 {
            bb.update(dec!(100.00));
        }
        let narrow = bb.update(dec!(100.00)).unwrap();

        let mut bb2 = BollingerBands::new();
        let mut price = dec!(100.00);
        for _ in 0..10 {
            bb2.update(price);
            price += dec!(10.00);
        }
        for _ in 0..10 {
            bb2.update(price);
            price -= dec!(10.00);
        }
        let wide = bb2.update(dec!(100.00)).unwrap();
        assert!(wide.0 > narrow.0, "Volatile prices should widen upper band");
    }

    #[test]
    fn test_bandwidth_calculation() {
        let mut bb = BollingerBands::new();
        // Feed alternating prices to create a non-zero standard deviation
        for i in 0..20 {
            if i % 2 == 0 {
                bb.update(dec!(100.00));
            } else {
                bb.update(dec!(110.00));
            }
        }
        let (upper, middle, lower) = bb.update(dec!(105.00)).unwrap();
        let bandwidth = (upper - lower) / middle;
        assert!(
            bandwidth > dec!(0.00),
            "Bandwidth should be positive with varying prices"
        );
        assert!(upper > middle, "Upper band must be above middle");
        assert!(middle > lower, "Middle band must be above lower band");
    }

    #[test]
    fn test_percent_b_stays_in_bounds() {
        let mut bb = BollingerBands::new();
        // Feed prices with variance to get non-zero bandwidth
        for i in 0..20 {
            if i % 2 == 0 {
                bb.update(dec!(95.00));
            } else {
                bb.update(dec!(105.00));
            }
        }
        let close = dec!(100.00);
        let (upper, _middle, lower) = bb.update(close).unwrap();
        assert!(upper > lower, "Bands must have non-zero width");
        let pct_b = (close - lower) / (upper - lower);
        assert!(
            pct_b >= dec!(0.00) && pct_b <= dec!(1.00),
            "%B should stay in [0,1], got {}",
            pct_b
        );
        // At the middle of the price range, %B should be near 0.5
        assert!(
            (pct_b - dec!(0.5)).abs() < dec!(0.4),
            "%B should be near 0.5 at mid-range, got {}",
            pct_b
        );
    }
}
