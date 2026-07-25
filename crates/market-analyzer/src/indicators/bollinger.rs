use super::sma::Sma;
use super::traits::{BarInput, Indicator};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;

/// Bollinger Bands Indicator (SMA +/- 2 Standard Deviations)
#[derive(Debug, Clone)]
pub struct BollingerBands {
    sma: Sma,
    period: usize,
    prices_history: Vec<Decimal>,
}

impl BollingerBands {
    pub fn new(period: usize) -> Self {
        Self {
            sma: Sma::new(period),
            period,
            prices_history: Vec::new(),
        }
    }

    pub fn update(&mut self, close: f64) -> Option<(Decimal, Decimal, Decimal)> {
        let close_d = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        self.prices_history.push(close_d);
        if self.prices_history.len() > self.period {
            self.prices_history.remove(0);
        }

        let sma = self.sma.update(close)?;

        if self.prices_history.len() < self.period {
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
            let variance = sum_sq / self.period as f64;
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
        *self = BollingerBands::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_returns_none_before_20_values() {
        let mut bb = BollingerBands::new(20);
        for _ in 0..19 {
            assert_eq!(bb.update(100.00), None);
        }
    }

    #[test]
    fn test_returns_bands_at_20_values() {
        let mut bb = BollingerBands::new(20);
        for _ in 0..19 {
            bb.update(100.00);
        }
        let result = bb.update(100.00).unwrap();
        assert!(result.0 >= result.1);
        assert!(result.1 >= result.2);
    }

    #[test]
    fn test_upper_band_widens_with_volatility() {
        let mut bb = BollingerBands::new(20);
        for _ in 0..20 {
            bb.update(100.00);
        }
        let narrow = bb.update(100.00).unwrap();

        let mut bb2 = BollingerBands::new(20);
        let mut price = 100.00;
        for _ in 0..10 {
            bb2.update(price);
            price += 10.00;
        }
        for _ in 0..10 {
            bb2.update(price);
            price -= 10.00;
        }
        let wide = bb2.update(100.00).unwrap();
        assert!(wide.0 > narrow.0, "Volatile prices should widen upper band");
    }

    #[test]
    fn test_bandwidth_calculation() {
        let mut bb = BollingerBands::new(20);
        // Feed alternating prices to create a non-zero standard deviation
        for i in 0..20 {
            if i % 2 == 0 {
                bb.update(100.00);
            } else {
                bb.update(110.00);
            }
        }
        let (upper, middle, lower) = bb.update(105.00).unwrap();
        let bandwidth = (upper - lower) / middle;
        assert!(
            bandwidth > Decimal::from_f64_retain(0.00).unwrap(),
            "Bandwidth should be positive with varying prices"
        );
        assert!(upper > middle, "Upper band must be above middle");
        assert!(middle > lower, "Middle band must be above lower band");
    }

    #[test]
    fn test_percent_b_stays_in_bounds() {
        let mut bb = BollingerBands::new(20);
        // Feed prices with variance to get non-zero bandwidth
        for i in 0..20 {
            if i % 2 == 0 {
                bb.update(95.00);
            } else {
                bb.update(105.00);
            }
        }
        let close = Decimal::from_f64_retain(100.00).unwrap();
        let (upper, _middle, lower) = bb.update(100.00).unwrap();
        assert!(upper > lower, "Bands must have non-zero width");
        let pct_b = (close - lower) / (upper - lower);
        assert!(
            pct_b >= Decimal::from_f64_retain(0.00).unwrap() && pct_b <= Decimal::from_f64_retain(1.00).unwrap(),
            "%B should stay in [0,1], got {}",
            pct_b
        );
        // At the middle of the price range, %B should be near 0.5
        assert!(
            (pct_b - Decimal::from_f64_retain(0.5).unwrap()).abs() < Decimal::from_f64_retain(0.4).unwrap(),
            "%B should be near 0.5 at mid-range, got {}",
            pct_b
        );
    }
}
