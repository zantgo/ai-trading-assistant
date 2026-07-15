use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Live funding rate tracker. Computes annualized rate and rolling average.
/// Non-directional gate — extreme funding signals potential reversal.
#[derive(Debug, Clone)]
pub struct FundingRate {
    pub current: Option<Decimal>,
    history: VecDeque<f64>,
    lookback: usize,
    /// Annualization factor (365 days × 3 periods/day = 1095 for 8h funding intervals).
    pub annualize_factor: f64,
}

impl FundingRate {
    pub fn new(lookback: usize, annualize_factor: f64) -> Self {
        Self {
            current: None,
            history: VecDeque::with_capacity(lookback),
            lookback,
            annualize_factor,
        }
    }

    pub fn update(&mut self, rate: Decimal) {
        self.current = Some(rate);
        if let Some(v) = rate.to_f64() {
            self.history.push_back(v);
            if self.history.len() > self.lookback {
                self.history.pop_front();
            }
        }
    }

    /// Raw funding rate (per 8h, as decimals — 0.0001 = 0.01%).
    pub fn raw(&self) -> Option<Decimal> {
        self.current
    }

    /// Annualized funding rate as percentage.
    pub fn annualized_pct(&self) -> Option<Decimal> {
        let r = self.current?.to_f64()?;
        Decimal::from_f64_retain(r * self.annualize_factor * 100.0)
    }

    /// Rolling average funding rate.
    pub fn average(&self) -> Option<Decimal> {
        if self.history.is_empty() {
            return None;
        }
        let avg = self.history.iter().sum::<f64>() / self.history.len() as f64;
        Decimal::from_f64_retain(avg)
    }

    /// Check if current funding rate is at an extreme level (> threshold %)
    pub fn is_extreme(&self, threshold_pct: f64) -> Option<bool> {
        let ann = self.annualized_pct()?.to_f64()?;
        Some(ann.abs() > threshold_pct)
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.lookback, self.annualize_factor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_funding_annualized() {
        let mut f = FundingRate::new(20, 1095.0);
        f.update(dec!(0.0001)); // 0.01% per 8h
        let ann = f.annualized_pct().unwrap();
        let ann_f = ann.to_f64().unwrap();
        assert!(
            (ann_f - 10.95).abs() < 0.01,
            "expected ~10.95, got {}",
            ann_f
        );
    }

    #[test]
    fn test_funding_extreme_detection() {
        let mut f = FundingRate::new(20, 1095.0);
        f.update(dec!(0.001)); // 0.1% per 8h → ~109.5% annualized
        assert!(f.is_extreme(50.0).unwrap());
    }

    #[test]
    fn test_funding_not_extreme() {
        let mut f = FundingRate::new(20, 1095.0);
        f.update(dec!(0.00005)); // 0.005% per 8h → ~5.5% annualized
        assert!(!f.is_extreme(50.0).unwrap());
    }

    #[test]
    fn test_funding_negative_rate() {
        let mut f = FundingRate::new(20, 1095.0);
        f.update(dec!(-0.0002)); // -0.02% per 8h
        let ann = f.annualized_pct().unwrap();
        assert!(ann < dec!(0));
    }
}
