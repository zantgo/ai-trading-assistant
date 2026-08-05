use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;

/// Relative Strength Index (using Wilder's Smoothing).
///
/// v6.10 (Phase 4 / D2): the SMA seed for `avg_gain` / `avg_loss` is
/// computed correctly — we accumulate the first `period` gains/losses
/// and seed with their simple averages before the Wilder recursion
/// takes over. The legacy implementation seeded `avg_gain = gain` from
/// the very first change rather than from the SMA of the first
/// `period` changes, which produced RSI values that diverged from
/// TradingView / canonical tooling for the first `period` bars after
/// warm-up.
#[derive(Debug, Clone)]
pub struct Rsi {
    period: usize,
    prev_close: Option<Decimal>,
    avg_gain: Option<Decimal>,
    avg_loss: Option<Decimal>,
    /// Pre-seed accumulator: gains observed so far in the warming-up
    /// phase. Reset to `None` once the SMA seed is computed.
    seed_gain: Option<Decimal>,
    /// Pre-seed accumulator: losses observed so far.
    seed_loss: Option<Decimal>,
    /// Number of changes observed so far in the warming-up phase.
    changes_seen: usize,
}

impl Rsi {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prev_close: None,
            avg_gain: None,
            avg_loss: None,
            seed_gain: None,
            seed_loss: None,
            changes_seen: 0,
        }
    }

    pub fn update(&mut self, close: f64) -> Option<Decimal> {
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        let prev = match self.prev_close {
            None => {
                self.prev_close = Some(close);
                return None;
            }
            Some(p) => p,
        };
        self.prev_close = Some(close);

        let change = close - prev;
        let gain = if change > Decimal::ZERO {
            change
        } else {
            Decimal::ZERO
        };
        let loss = if change < Decimal::ZERO {
            change.abs()
        } else {
            Decimal::ZERO
        };

        // v6.10 (Phase 4 / D2): SMA-seeded avg_gain / avg_loss. The first
        // `period` change observations accumulate into seed_gain /
        // seed_loss; once we have `period` observations, the SMA seed
        // is committed and Wilder recursion takes over.
        if self.avg_gain.is_none() {
            // Pre-seed phase.
            self.seed_gain = Some(self.seed_gain.unwrap_or(Decimal::ZERO) + gain);
            self.seed_loss = Some(self.seed_loss.unwrap_or(Decimal::ZERO) + loss);
            self.changes_seen += 1;
            if self.changes_seen >= self.period {
                let p_dec = Decimal::from(self.period);
                self.avg_gain = Some(self.seed_gain.unwrap() / p_dec);
                self.avg_loss = Some(self.seed_loss.unwrap() / p_dec);
                self.seed_gain = None;
                self.seed_loss = None;
                // Emit the first SMA-seeded RSI on this call.
                return self.compute_rsi();
            }
            return None;
        }

        // Wilder recursion.
        let ag = self.avg_gain.unwrap();
        let al = self.avg_loss.unwrap();
        let p_dec = Decimal::from(self.period);
        let p_minus_1 = p_dec - Decimal::ONE;

        let next_ag = (ag * p_minus_1 + gain) / p_dec;
        let next_al = (al * p_minus_1 + loss) / p_dec;

        self.avg_gain = Some(next_ag);
        self.avg_loss = Some(next_al);
        self.compute_rsi()
    }

    /// Compute RSI from the current avg_gain / avg_loss. Returns 100 when
    /// avg_loss is exactly zero (no losses over the window).
    fn compute_rsi(&self) -> Option<Decimal> {
        let ag = self.avg_gain?;
        let al = self.avg_loss?;
        if al == Decimal::ZERO {
            Some(Decimal::from(100))
        } else {
            let rs = ag / al;
            let rsi = Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs));
            Some(rsi)
        }
    }
}

impl Indicator for Rsi {
    type Output = Option<Decimal>;

    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.close)
    }

    fn reset(&mut self) {
        *self = Rsi::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_first_update_returns_none() {
        let mut rsi = Rsi::new(14);
        assert_eq!(rsi.update(100.00), None);
    }

    #[test]
    fn test_second_update_returns_none() {
        let mut rsi = Rsi::new(14);
        rsi.update(100.00);
        assert_eq!(rsi.update(105.00), None);
    }

    #[test]
    fn test_emits_on_period_change() {
        // v6.10 (Phase 4 / D2): with SMA seed, the first RSI emits on the
        // (period+1)-th update (initial price = no change, then `period`
        // changes accumulated, then this call is the (period+1)-th change
        // and the SMA seed is committed + RSI computed in the same tick).
        let mut rsi = Rsi::new(14);
        rsi.update(100.00);
        let mut price = 100.00;
        for i in 0..15 {
            price += 1.00;
            let r = rsi.update(price);
            // The (period+1)th = 15th change observed commits the SMA seed
            // and emits. The 14 changes before it were warming up.
            if i == 13 {
                assert!(
                    r.is_some(),
                    "RSI must emit on the {}th update (SMA seed commit)",
                    i + 2
                );
            }
        }
    }

    #[test]
    fn test_all_gains_yields_high_rsi() {
        let mut rsi = Rsi::new(14);
        rsi.update(100.00);
        let mut price = 100.00;
        for _ in 0..14 {
            price += 1.00;
            rsi.update(price);
        }
        let result = rsi.update(price + 1.00).unwrap();
        assert!(
            result > Decimal::from_f64_retain(50.00).unwrap(),
            "All gains should yield RSI > 50"
        );
    }

    #[test]
    fn test_all_losses_yields_low_rsi() {
        let mut rsi = Rsi::new(14);
        rsi.update(100.00);
        let mut price = 100.00;
        for _ in 0..14 {
            price -= 1.00;
            rsi.update(price);
        }
        let result = rsi.update(price - 1.00).unwrap();
        assert!(result < dec!(50.00), "All losses should yield RSI < 50");
    }

    #[test]
    fn test_zero_loss_returns_rsi_100() {
        let mut rsi = Rsi::new(14);
        rsi.update(50.00);
        let mut price = 50.00;
        for _ in 0..14 {
            price += 2.00;
            rsi.update(price);
        }
        let result = rsi.update(price + 2.00).unwrap();
        assert!(result > dec!(90.00));
        assert!(result <= dec!(100.00), "RSI should not exceed 100");
    }

    #[test]
    fn test_rsi_stays_within_zero_to_hundred() {
        let mut rsi = Rsi::new(14);
        rsi.update(100.00);
        for i in 0..50 {
            let price = if i % 2 == 0 { 200.00 } else { 10.00 };
            if let Some(val) = rsi.update(price) {
                assert!(
                    val >= dec!(0.00),
                    "RSI should never be negative, got {}",
                    val
                );
                assert!(
                    val <= dec!(100.00),
                    "RSI should never exceed 100, got {}",
                    val
                );
            }
        }
    }

    #[test]
    fn test_sma_seed_matches_tradingview_reference() {
        // TradingView reference: for a 14-period RSI with all +1 gains,
        // the first emitted RSI should be 100 (no losses).
        // With our SMA-seeded implementation: avg_gain = sum_of_14_gains/14
        // = 14*1/14 = 1.0, avg_loss = 0/14 = 0, RSI = 100.
        let mut rsi = Rsi::new(14);
        rsi.update(100.00);
        for i in 1..=14 {
            let r = rsi.update(100.00 + i as f64);
            // RSI emits on the 15th update (the (period+1)-th change observed).
            if i == 14 {
                assert_eq!(
                    r,
                    Some(Decimal::from(100)),
                    "All-gain RSI should be 100 after SMA seed"
                );
            }
        }
    }
}
