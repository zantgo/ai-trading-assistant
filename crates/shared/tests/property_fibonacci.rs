use proptest::prelude::*;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use shared::indicators::FibonacciRange;

fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

proptest! {
    #[test]
    fn fibonacci_retracements_within_swing_bounds(
        swing_low in 1.0f64..50_000.0,
        swing_high_diff in 1.0f64..50_000.0
    ) {
        let low = dec(swing_low);
        let high = low + dec(swing_high_diff);
        let coeffs: &[f64] = &[0.236, 0.382, 0.500, 0.618, 0.660, 0.786];
        let exts: &[f64] = &[1.272, 1.618, 2.000, 2.618];

        let fib = FibonacciRange::compute_bullish(low, high, coeffs, exts);

        // All retracement levels must be within [swing_low, swing_high]
        for level in &fib.retracement_levels {
            prop_assert!(*level >= low, "Retracement {} below swing_low {}", level, low);
            prop_assert!(*level <= high, "Retracement {} above swing_high {}", level, high);
        }
    }

    #[test]
    fn fibonacci_bullish_retracement_order(
        swing_low in 1.0f64..50_000.0,
        swing_high_diff in 1.0f64..50_000.0
    ) {
        let low = dec(swing_low);
        let high = low + dec(swing_high_diff);
        let coeffs: &[f64] = &[0.236, 0.382, 0.500, 0.618, 0.660, 0.786];
        let exts: &[f64] = &[1.272, 1.618, 2.000, 2.618];

        let fib = FibonacciRange::compute_bullish(low, high, coeffs, exts);

        // Bullish retracements must be strictly descending
        for i in 1..fib.retracement_levels.len() {
            prop_assert!(fib.retracement_levels[i - 1] > fib.retracement_levels[i],
                "Bullish retracement at {} not descending: {} >= {}",
                i, fib.retracement_levels[i-1], fib.retracement_levels[i]);
        }
    }

    #[test]
    fn fibonacci_bullish_extensions_above_swing_high(
        swing_low in 1.0f64..50_000.0,
        swing_high_diff in 1.0f64..50_000.0
    ) {
        let low = dec(swing_low);
        let high = low + dec(swing_high_diff);
        let coeffs: &[f64] = &[0.618, 0.660];
        let exts: &[f64] = &[1.272, 1.618, 2.000, 2.618];

        let fib = FibonacciRange::compute_bullish(low, high, coeffs, exts);

        // All extension levels must be strictly above swing_high for bullish
        for level in &fib.extension_levels {
            prop_assert!(*level > high,
                "Bullish extension {} not above swing_high {}", level, high);
        }
    }

    #[test]
    fn fibonacci_golden_pocket_between_618_and_660(
        swing_low in 1.0f64..50_000.0,
        swing_high_diff in 1.0f64..50_000.0
    ) {
        let low = dec(swing_low);
        let high = low + dec(swing_high_diff);
        let coeffs: &[f64] = &[0.618, 0.660];
        let exts: &[f64] = &[1.618, 2.618];

        let fib = FibonacciRange::compute_bullish(low, high, coeffs, exts);

        if let (Some(gp_low), Some(gp_high)) = (fib.golden_pocket_low, fib.golden_pocket_high) {
            prop_assert!(gp_low <= gp_high,
                "Golden pocket: low {} must be <= high {}", gp_low, gp_high);
            prop_assert!(gp_low >= low && gp_low <= high,
                "Golden pocket low {} must be within [{}, {}]", gp_low, low, high);
            prop_assert!(gp_high >= low && gp_high <= high,
                "Golden pocket high {} must be within [{}, {}]", gp_high, low, high);
        }
    }
}
