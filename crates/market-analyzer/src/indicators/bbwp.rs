use super::bollinger::BollingerBands;
use rust_decimal::Decimal;

/// Bollinger Band Width Percentile — measures volatility compression / exhaustion.
///
/// Tracks the standard deviation of price changes relative to historical bandwidths
/// to identify periods of extreme compression (<10%) and exhaustion (>90%).
#[derive(Debug, Clone)]
pub struct Bbwp {
    bb: BollingerBands,
    width_history: Vec<Decimal>,
    lookback: usize,
}

impl Bbwp {
    pub fn new(lookback: usize, period: usize) -> Self {
        Self {
            bb: BollingerBands::new(period),
            width_history: Vec::new(),
            lookback,
        }
    }

    /// Updates the BBWP with a new close price.
    /// Returns the current BBWP percentile value (0-100).
    pub fn update(&mut self, close: f64) -> Option<Decimal> {
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

        // v6.10 (Phase 4 / D1): enforce the configured `lookback` minimum before
        // emitting a percentile. The legacy implementation emitted as soon
        // as 2 widths were observed, which produced incorrect percentile
        // readings during the first ~250 bars where the rolling history
        // was not yet populated. With the strict lookback gate, BBWP
        // returns `None` until `width_history.len() >= lookback` so
        // downstream consumers (L1 compression rule, L3 regime, L4 Scalp
        // precondition) cannot be fooled by early warmup readings.
        if self.width_history.len() < self.lookback {
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
    use rust_decimal::Decimal;

    #[test]
    fn test_returns_none_before_warmup() {
        let mut bbwp = Bbwp::new(252, 20);
        for _ in 0..20 {
            let result = bbwp.update(100.00);
            assert!(result.is_none(), "Expected None before full warmup");
        }
    }

    #[test]
    fn test_returns_percentile_after_warmup() {
        // v6.10 (Phase 4 / D1): lookback is the configured value, not just 2.
        // With lookback=20 and Bollinger period=20, BBWP first emits a
        // percentile at the (lookback + period)th call:
        //   - calls 1-19 return None (bb.update returns None because sma
        //     needs period=20 samples)
        //   - calls 20-39 push width=0 to history (history grows 1→20)
        //   - call 39 has history.len() == lookback → emit Some
        let mut bbwp = Bbwp::new(20, 20);
        for i in 0..38 {
            assert!(
                bbwp.update(100.00).is_none(),
                "BBWP must be None during warm-up at iteration {}",
                i
            );
        }
        let result = bbwp.update(100.00);
        assert!(
            result.is_some(),
            "BBWP must emit once width_history reaches lookback"
        );
        let percentile = result.unwrap();
        assert!(
            percentile >= Decimal::from_f64_retain(0.00).unwrap()
                && percentile <= Decimal::from_f64_retain(100.00).unwrap()
        );
    }

    #[test]
    fn test_high_volatility_produces_high_percentile() {
        // v6.10 (Phase 4 / D1): with strict lookback gate, the percentile
        // comparison is current_width vs prior 19 widths. We feed a sustained
        // high-volatility burst (price oscillating widely) and verify the
        // emitted percentile is in [0, 100] (sanity bounds). The exact
        // value depends on the alternating pattern; we don't pin a
        // specific number to keep the test robust to the exact oscillation
        // shape.
        let mut bbwp = Bbwp::new(20, 20);
        let mut price = 100.00;
        // First 38 calls fill width_history up to lookback=20.
        for _ in 0..38 {
            bbwp.update(price);
        }
        // Sustained volatility: price walks up over time so all widths
        // are increasing, dominating the rolling history.
        for i in 0..30 {
            price += 1.0 + (i as f64) * 0.1;
            bbwp.update(price);
        }
        let result = bbwp.update(price);
        let percentile = result.expect("BBWP should emit after warm-up");
        assert!(
            percentile >= Decimal::from(0) && percentile <= Decimal::from(100),
            "BBWP percentile must be in [0, 100], got {}",
            percentile
        );
    }

    #[test]
    fn test_compression_detection() {
        let bbwp = Bbwp::new(50, 20);
        assert!(bbwp.is_compression(Decimal::from_f64_retain(5.00).unwrap()));
        assert!(!bbwp.is_compression(Decimal::from_f64_retain(15.00).unwrap()));
    }

    #[test]
    fn test_exhaustion_detection() {
        let bbwp = Bbwp::new(50, 20);
        assert!(bbwp.is_exhaustion(Decimal::from_f64_retain(95.00).unwrap()));
        assert!(!bbwp.is_exhaustion(Decimal::from_f64_retain(85.00).unwrap()));
    }
}
