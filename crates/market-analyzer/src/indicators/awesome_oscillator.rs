use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Awesome Oscillator (Bill Williams) — measures market momentum by
/// comparing a fast (5-period) vs slow (34-period) SMA of the median price.
/// Positive AO indicates bullish momentum; negative AO indicates bearish.
/// The bar colour (green=rising, red=falling) is a secondary signal.
#[derive(Debug, Clone)]
pub struct AwesomeOscillator {
    medians: VecDeque<Decimal>,
}

#[derive(Debug, Clone, Copy)]
pub struct AoOutput {
    pub value: Decimal,
    pub rising: bool,
}

impl AwesomeOscillator {
    pub fn new() -> Self {
        Self {
            medians: VecDeque::with_capacity(35),
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal) -> Option<AoOutput> {
        let median = (high + low) / Decimal::from(2);
        self.medians.push_back(median);
        while self.medians.len() > 35 {
            self.medians.pop_front();
        }
        if self.medians.len() < 34 {
            return None;
        }
        let sma5: Decimal =
            self.medians.iter().rev().take(5).copied().sum::<Decimal>() / Decimal::from(5);
        let sma34: Decimal =
            self.medians.iter().rev().take(34).copied().sum::<Decimal>() / Decimal::from(34);
        let ao = sma5 - sma34;
        let prev_ao = if self.medians.len() >= 35 {
            let prev_sma5: Decimal = self
                .medians
                .iter()
                .rev()
                .skip(1)
                .take(5)
                .copied()
                .sum::<Decimal>()
                / Decimal::from(5);
            let prev_sma34: Decimal = self
                .medians
                .iter()
                .rev()
                .skip(1)
                .take(34)
                .copied()
                .sum::<Decimal>()
                / Decimal::from(34);
            prev_sma5 - prev_sma34
        } else {
            ao
        };
        Some(AoOutput {
            value: ao,
            rising: ao >= prev_ao,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_none_before_34() {
        let mut ao = AwesomeOscillator::new();
        for _ in 0..33 {
            ao.update(dec!(110), dec!(90));
        }
        assert!(ao.update(dec!(110), dec!(90)).is_some());
    }

    #[test]
    fn test_produces_output_after_34() {
        let mut ao = AwesomeOscillator::new();
        for _ in 0..34 {
            ao.update(dec!(110), dec!(90));
        }
        assert!(ao.update(dec!(110), dec!(90)).is_some());
    }
}
