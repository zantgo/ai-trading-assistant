use rust_decimal::Decimal;

/// Unified bar/candle input that all indicators accept.
/// Indicators that need fewer fields simply ignore the rest.
#[derive(Debug, Clone, Copy)]
pub struct BarInput {
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

impl BarInput {
    pub fn from_close(close: Decimal) -> Self {
        Self {
            open: close,
            high: close,
            low: close,
            close,
            volume: Decimal::ZERO,
        }
    }

    pub fn ohlcv(
        open: Decimal,
        high: Decimal,
        low: Decimal,
        close: Decimal,
        volume: Decimal,
    ) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
        }
    }
}

/// Common interface for all technical indicators.
///
/// Implementors feed bars through `update()` and return their computed value.
/// `reset()` returns the indicator to its initial state (e.g. for re-warming).
pub trait Indicator {
    /// The type of value this indicator produces per bar.
    type Output;

    /// Apply a new bar/candle to the indicator.
    fn update(&mut self, bar: &BarInput) -> Self::Output;

    /// Reset internal history to a pristine state.
    fn reset(&mut self);
}
