use rust_decimal::Decimal;
use super::bollinger::BollingerBands;

/// Bollinger Band Width Percentile — measures volatility compression / exhaustion.
///
/// Tracks the standard deviation of price changes relative to historical bandwidths
/// to identify periods of extreme compression (<10%) and exhaustion (>90%).
#[derive(Debug, Clone)]
pub struct Bbwp {
    bb: BollingerBands,
    width_history: Vec<Decimal>,
    lookback: usize,
    period: usize,
    prices_history: Vec<Decimal>,
}

impl Bbwp {
    pub fn new(lookback: usize, period: usize) -> Self {
        Self {
            bb: BollingerBands::new(),
            width_history: Vec::new(),
            lookback,
            period,
            prices_history: Vec::new(),
        }
    }

    /// Updates the BBWP with a new close price.
    /// Returns the current BBWP percentile value (0-100).
    pub fn update(&mut self, close: Decimal) -> Option<Decimal> {
        self.prices_history.push(close);
        if self.prices_history.len() > self.period + 1 {
            self.prices_history.remove(0);
        }

        let bands = self.bb.update(close)?;
        let (upper, middle, lower) = bands;

        if middle == Decimal::ZERO {
            return None;
        }

        let width = (upper - lower) / middle;

        self.width_history.push(width);
        if self.width_history.len() > self.lookback {
            self.width_history.remove(0);
        }

        if self.width_history.len() < 2 {
            return None;
        }

        let current_width = *self.width_history.last().unwrap();

        let mut count_below: usize = 0;
        let total = self.width_history.len();

        for &w in &self.width_history[..total - 1] {
            if w < current_width {
                count_below += 1;
            }
        }

        let numerator = Decimal::from(count_below) * Decimal::from(100);
        let denominator = Decimal::from(total.saturating_sub(1).max(1));
        Some(numerator / denominator)
    }

    /// Returns true if the BBWP indicates volatility compression (percentile < 10%).
    pub fn is_compression(&self, percentile: Decimal) -> bool {
        percentile < Decimal::from(10)
    }

    /// Returns true if the BBWP indicates volatility exhaustion (percentile > 90%).
    pub fn is_exhaustion(&self, percentile: Decimal) -> bool {
        percentile > Decimal::from(90)
    }

    /// Returns the current Bollinger Band width if available.
    pub fn current_width(&self) -> Option<Decimal> {
        self.width_history.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_returns_none_before_warmup() {
        let mut bbwp = Bbwp::new(252, 20);
        for _ in 0..20 {
            let result = bbwp.update(dec!(100.00));
            assert!(result.is_none(), "Expected None before full warmup");
        }
    }

    #[test]
    fn test_returns_percentile_after_warmup() {
        let mut bbwp = Bbwp::new(50, 20);
        for _ in 0..40 {
            bbwp.update(dec!(100.00));
        }
        let result = bbwp.update(dec!(100.00));
        assert!(result.is_some());
        let percentile = result.unwrap();
        assert!(percentile >= dec!(0.00) && percentile <= dec!(100.00));
    }

    #[test]
    fn test_high_volatility_produces_high_percentile() {
        let mut bbwp = Bbwp::new(50, 20);
        let mut price = dec!(100.00);
        for _ in 0..40 {
            bbwp.update(price);
        }
        for _ in 0..25 {
            price += dec!(10.00);
            bbwp.update(price);
            price -= dec!(10.00);
        }
        let result = bbwp.update(price).unwrap();
        assert!(result > dec!(50.00), "High volatility should produce high percentile, got {}", result);
    }

    #[test]
    fn test_compression_detection() {
        let bbwp = Bbwp::new(50, 20);
        assert!(bbwp.is_compression(dec!(5.00)));
        assert!(!bbwp.is_compression(dec!(15.00)));
    }

    #[test]
    fn test_exhaustion_detection() {
        let bbwp = Bbwp::new(50, 20);
        assert!(bbwp.is_exhaustion(dec!(95.00)));
        assert!(!bbwp.is_exhaustion(dec!(85.00)));
    }
}
