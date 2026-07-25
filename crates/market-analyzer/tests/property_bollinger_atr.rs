use proptest::prelude::*;
use rust_decimal::Decimal;
use market_analyzer::indicators::{Atr, BollingerBands};

proptest! {
    #[test]
    fn bollinger_band_ordering(prices in proptest::collection::vec(1.0f64..100_000.0, 20..100)) {
        let mut bb = BollingerBands::new(20);
        for &p in &prices {
            if let Some((upper, middle, lower)) = bb.update(p) {
                prop_assert!(upper >= middle, "Upper({}) >= Middle({})", upper, middle);
                prop_assert!(middle >= lower, "Middle({}) >= Lower({})", middle, lower);
            }
        }
    }

    #[test]
    fn bollinger_bands_with_high_low(
        highs in proptest::collection::vec(1.0f64..100_000.0, 20..100),
        lows in proptest::collection::vec(0.1f64..50_000.0, 20..100)
    ) {
        let mut bb = BollingerBands::new(20);
        let n = highs.len().min(lows.len());
        for i in 0..n {
            let close = (highs[i] + lows[i]) / 2.0;
            if let Some((upper, _middle, lower)) = bb.update(close) {
                // Standard deviation must be non-negative → upper >= lower
                let width = upper - lower;
                prop_assert!(width >= Decimal::ZERO,
                    "Band width must be non-negative: upper={}, lower={}", upper, lower);
            }
        }
    }

    #[test]
    fn atr_never_negative(
        high in 1.0f64..100_000.0,
        low in 0.1f64..100_000.0,
        close in 0.1f64..100_000.0,
        count in 1usize..30
    ) {
        let mut atr = Atr::new(14);
        let h = high;
        let l = low.min(h);
        let c = close.clamp(l, h);
        atr.update(h, l, c);

        for _ in 0..count {
            if let Some(out) = atr.update(h, l, c) {
                prop_assert!(out.atr_value >= Decimal::ZERO,
                    "ATR must be non-negative: {}", out.atr_value);
            }
        }
    }
}
