use rust_decimal::Decimal;
use super::ema::Ema;

/// Moving Average Convergence Divergence
#[derive(Debug, Clone)]
pub struct Macd {
    fast_ema: Ema,
    slow_ema: Ema,
    signal_ema: Ema,
}

impl Macd {
    pub fn new() -> Self {
        Self {
            fast_ema: Ema::new(12),
            slow_ema: Ema::new(26),
            signal_ema: Ema::new(9),
        }
    }

    pub fn update(&mut self, close: Decimal) -> Option<(Decimal, Decimal, Decimal)> {
        let fast = self.fast_ema.update(close);
        let slow = self.slow_ema.update(close);
        let macd_line = fast - slow;
        let signal_line = self.signal_ema.update(macd_line);
        let histogram = macd_line - signal_line;
        Some((macd_line, signal_line, histogram))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_first_update_seeds_emas() {
        let mut macd = Macd::new();
        let (line, signal, hist) = macd.update(dec!(100.00)).unwrap();
        assert_eq!(line, dec!(0.00), "First update: fast=slow=price, macd_line should be 0");
        assert_eq!(signal, dec!(0.00));
        assert_eq!(hist, dec!(0.00));
    }

    #[test]
    fn test_histogram_sign_matches_macd_line_minus_signal() {
        let mut macd = Macd::new();
        macd.update(dec!(100.00));
        macd.update(dec!(101.00));
        let (line, signal, hist) = macd.update(dec!(102.00)).unwrap();
        assert_eq!(hist, line - signal, "Histogram should equal macd_line - signal_line");
    }
}
