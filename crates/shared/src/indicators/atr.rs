use rust_decimal::Decimal;
use std::collections::VecDeque;
use rust_decimal::prelude::FromPrimitive;
use super::ema::Ema;
use serde::{Deserialize, Serialize};

/// Volatility regime classification based on ATR slope vs its SMA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolatilityRegime {
    Expanding,
    Contracting,
    Stable,
}

/// Expanded output from an ATR update including slope and regime.
#[derive(Debug, Clone)]
pub struct AtrOutput {
    pub atr_value: Decimal,
    pub atr_slope: Decimal,
    pub volatility_regime: VolatilityRegime,
}

/// Average True Range with stateful tracking.
///
/// Maintains a 5-value history buffer for regime classification
/// and tracks previous ATR for slope calculation.
#[derive(Debug, Clone)]
pub struct Atr {
    prev_close: Option<Decimal>,
    tr_ema: Ema,
    atr_history: VecDeque<Decimal>,
    prev_atr: Option<Decimal>,
    regime_history_len: usize,
}

impl Atr {
    pub fn new(period: usize) -> Self {
        Self {
            prev_close: None,
            tr_ema: Ema::new(period),
            atr_history: VecDeque::with_capacity(5),
            prev_atr: None,
            regime_history_len: 5,
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<AtrOutput> {
        let tr = match self.prev_close {
            None => high - low,
            Some(prev) => {
                let r1 = high - low;
                let r2 = (high - prev).abs();
                let r3 = (low - prev).abs();
                r1.max(r2).max(r3)
            }
        };
        self.prev_close = Some(close);
        let atr = self.tr_ema.update(tr);

        // Maintain ATR history for regime classification
        self.atr_history.push_back(atr);
        while self.atr_history.len() > self.regime_history_len {
            self.atr_history.pop_front();
        }

        // Compute slope
        let atr_slope = match self.prev_atr {
            Some(prev) => atr - prev,
            None => Decimal::ZERO,
        };
        self.prev_atr = Some(atr);

        // Classify volatility regime
        let volatility_regime = classify_regime(&self.atr_history);

        Some(AtrOutput {
            atr_value: atr,
            atr_slope,
            volatility_regime,
        })
    }

    /// Get the current ATR value.
    pub fn get_atr(&self) -> Option<Decimal> {
        self.prev_atr
    }

    /// Get the current volatility regime.
    pub fn get_regime(&self) -> Option<VolatilityRegime> {
        if self.atr_history.len() < self.regime_history_len {
            None
        } else {
            Some(classify_regime(&self.atr_history))
        }
    }
}

/// Classify volatility regime: Expanding (>2% above SMA), Contracting (>2% below),
/// or Stable (within ±2% of SMA).
fn classify_regime(history: &VecDeque<Decimal>) -> VolatilityRegime {
    if history.is_empty() {
        return VolatilityRegime::Stable;
    }

    let current = history.back().copied().unwrap_or(Decimal::ZERO);
    let count = Decimal::from(history.len());

    let sum: Decimal = history.iter().sum();
    if sum == Decimal::ZERO || count == Decimal::ZERO {
        return VolatilityRegime::Stable;
    }

    let avg = sum / count;
    if avg == Decimal::ZERO {
        return VolatilityRegime::Stable;
    }

    let ratio = current / avg;
    let two_pct = Decimal::new(102, 2); // 1.02
    let neg_two_pct = Decimal::new(98, 2); // 0.98

    if ratio > two_pct {
        VolatilityRegime::Expanding
    } else if ratio < neg_two_pct {
        VolatilityRegime::Contracting
    } else {
        VolatilityRegime::Stable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_first_call_uses_simple_high_low() {
        let mut atr = Atr::new(14);
        let out = atr.update(dec!(110.00), dec!(100.00), dec!(105.00)).unwrap();
        assert_eq!(out.atr_value, dec!(10.00));
    }

    #[test]
    fn test_subsequent_calls_use_true_range() {
        let mut atr = Atr::new(14);
        atr.update(dec!(110.00), dec!(100.00), dec!(105.00));
        let out = atr.update(dec!(108.00), dec!(102.00), dec!(104.00)).unwrap();
        assert!(out.atr_value > dec!(0.00));
    }

    #[test]
    fn test_atr_increases_with_volatility_spike() {
        let mut atr = Atr::new(5);
        for _ in 0..6 {
            atr.update(dec!(101.00), dec!(99.00), dec!(100.00));
        }
        let normal = atr.update(dec!(101.00), dec!(99.00), dec!(100.00)).unwrap().atr_value;
        let spike = atr.update(dec!(120.00), dec!(80.00), dec!(100.00)).unwrap().atr_value;
        assert!(spike > normal, "ATR should increase on volatility spike");
    }

    #[test]
    fn test_slope_computation() {
        let mut atr = Atr::new(14);
        atr.update(dec!(101.00), dec!(99.00), dec!(100.00));
        atr.update(dec!(101.00), dec!(99.00), dec!(100.00));
        // Wide bar should create positive slope
        let out = atr.update(dec!(120.00), dec!(80.00), dec!(100.00)).unwrap();
        assert!(out.atr_slope > Decimal::ZERO, "Slope should be positive after volatility spike");
    }

    #[test]
    fn test_regime_classification_expanding() {
        let mut history = VecDeque::new();
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.0));
        history.push_back(dec!(15.0)); // 50% above average → Expanding
        let regime = classify_regime(&history);
        assert_eq!(regime, VolatilityRegime::Expanding);
    }

    #[test]
    fn test_regime_classification_contracting() {
        let mut history = VecDeque::new();
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.0));
        history.push_back(dec!(5.0)); // 50% below average → Contracting
        let regime = classify_regime(&history);
        assert_eq!(regime, VolatilityRegime::Contracting);
    }

    #[test]
    fn test_regime_classification_stable() {
        let mut history = VecDeque::new();
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.0));
        history.push_back(dec!(10.1)); // 1% above → Stable
        let regime = classify_regime(&history);
        assert_eq!(regime, VolatilityRegime::Stable);
    }

    #[test]
    fn test_get_regime_returns_none_early() {
        let mut atr = Atr::new(14);
        assert!(atr.get_regime().is_none());
        for _ in 0..6 {
            atr.update(dec!(101.00), dec!(99.00), dec!(100.00));
        }
        assert!(atr.get_regime().is_some());
    }
}
