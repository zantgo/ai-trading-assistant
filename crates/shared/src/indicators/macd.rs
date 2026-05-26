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
