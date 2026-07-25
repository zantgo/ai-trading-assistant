use super::sma::Sma;
use super::traits::{BarInput, Indicator};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct StochasticOutput {
    pub k_value: Decimal,
    pub d_value: Decimal,
}

/// Standard Stochastic Oscillator with slowing (%K smoothing) and %D signal.
#[derive(Debug, Clone)]
pub struct Stochastic {
    k_period: usize,
    d_period: usize,
    s_period: usize,
    high_history: VecDeque<Decimal>,
    low_history: VecDeque<Decimal>,
    k_sma: Sma,
    d_sma: Sma,
    prev_k: Option<Decimal>,
    prev_d: Option<Decimal>,
}

impl Stochastic {
    pub fn new(k_period: usize, d_period: usize, s_period: usize) -> Self {
        Self {
            k_period,
            d_period,
            s_period,
            high_history: VecDeque::with_capacity(k_period),
            low_history: VecDeque::with_capacity(k_period),
            k_sma: Sma::new(s_period),
            d_sma: Sma::new(d_period),
            prev_k: None,
            prev_d: None,
        }
    }

    pub fn update(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
    ) -> Option<StochasticOutput> {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        if self.k_period == 0 {
            return None;
        }
        self.high_history.push_back(high);
        self.low_history.push_back(low);

        if self.high_history.len() > self.k_period {
            self.high_history.pop_front();
            self.low_history.pop_front();
        }

        if self.high_history.len() < self.k_period {
            return None;
        }

        let lowest_low = *self.low_history.iter().min().unwrap_or(&low);
        let highest_high = *self.high_history.iter().max().unwrap_or(&high);
        let range = highest_high - lowest_low;

        let raw_k = if range == Decimal::ZERO {
            Decimal::from(50)
        } else {
            ((close - lowest_low) / range) * Decimal::from(100)
        };

        let slowed_k = self.k_sma.update(raw_k.to_f64().unwrap_or(0.0))?;
        let d = self.d_sma.update(slowed_k.to_f64().unwrap_or(0.0))?;

        self.prev_k = Some(slowed_k);
        self.prev_d = Some(d);

        Some(StochasticOutput {
            k_value: slowed_k,
            d_value: d,
        })
    }
}

impl Indicator for Stochastic {
    type Output = Option<StochasticOutput>;

    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.high, bar.low, bar.close)
    }

    fn reset(&mut self) {
        *self = Stochastic::new(self.k_period, self.d_period, self.s_period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(stoch: &mut Stochastic, high: f64, low: f64, close: f64) -> Option<StochasticOutput> {
        stoch.update(high, low, close)
    }

    #[test]
    fn test_returns_none_before_warmup() {
        let mut stoch = Stochastic::new(5, 3, 3);
        // Not enough bars to fill the k_period window.
        assert!(feed(&mut stoch, 10.0, 9.0, 9.5).is_none());
        assert!(feed(&mut stoch, 11.0, 10.0, 10.5).is_none());
        assert!(feed(&mut stoch, 12.0, 11.0, 11.5).is_none());
    }

    #[test]
    fn test_k_and_d_within_zero_hundred() {
        let mut stoch = Stochastic::new(5, 3, 3);
        let mut price = 100.0;
        for i in 0..60 {
            price += if i % 2 == 0 { 1.5 } else { -0.5 };
            if let Some(out) = feed(&mut stoch, price + 1.0, price - 1.0, price) {
                let k = out.k_value;
                let d = out.d_value;
                assert!(k >= dec!(0) && k <= dec!(100), "K out of range: {}", k);
                assert!(d >= dec!(0) && d <= dec!(100), "D out of range: {}", d);
            }
        }
    }

    #[test]
    fn test_flat_range_yields_midpoint() {
        let mut stoch = Stochastic::new(3, 1, 1);
        // Constant high/low/close → zero range → raw %K defaults to 50.
        for _ in 0..5 {
            let _ = feed(&mut stoch, 10.0, 10.0, 10.0);
        }
        let out = feed(&mut stoch, 10.0, 10.0, 10.0).unwrap();
        assert_eq!(out.k_value, dec!(50));
    }

    #[test]
    fn test_close_at_high_yields_high_k() {
        let mut stoch = Stochastic::new(3, 1, 1);
        for _ in 0..5 {
            let _ = feed(&mut stoch, 12.0, 10.0, 12.0);
        }
        let out = feed(&mut stoch, 12.0, 10.0, 12.0).unwrap();
        assert_eq!(out.k_value, dec!(100));
    }

    #[test]
    fn test_reset_clears_state() {
        let mut stoch = Stochastic::new(3, 1, 1);
        for _ in 0..5 {
            let _ = feed(&mut stoch, 12.0, 10.0, 11.0);
        }
        stoch.reset();
        assert!(feed(&mut stoch, 12.0, 10.0, 11.0).is_none());
    }
}
