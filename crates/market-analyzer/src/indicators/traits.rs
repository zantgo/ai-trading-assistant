/// Unified bar/candle input that all indicators accept.
/// Indicators that need fewer fields simply ignore the rest.
#[derive(Debug, Clone, Copy)]
pub struct BarInput {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl BarInput {
    pub fn from_close(close: f64) -> Self {
        Self {
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
        }
    }

    pub fn ohlcv(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
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
