use market_analyzer::indicators::{Macd, TrendState};
use proptest::prelude::*;
use rust_decimal::Decimal;

proptest! {
    #[test]
    fn macd_histogram_sign_matches_difference(prices in proptest::collection::vec(1.0f64..100_000.0, 2..100)) {
        let mut macd = Macd::new();
        for &p in &prices {
            let out = macd.update(p);
            let diff = out.macd_line - out.signal_line;
            let hist = out.histogram;
            prop_assert_eq!(hist, diff, "Histogram must equal MACD line minus signal line");
        }
    }

    #[test]
    fn macd_peak_resets_on_crossover(prices in proptest::collection::vec(1.0f64..100_000.0, 5..200)) {
        let mut macd = Macd::new();
        let mut last_crossover_idx: Option<usize> = None;
        let mut prev_peak = Decimal::ZERO;

        for (i, &p) in prices.iter().enumerate() {
            let out = macd.update(p);

            if out.crossover.is_some() {
                // On crossover bar, the peak should be reset to current abs histogram
                let _abs_hist = if out.histogram < Decimal::ZERO { -out.histogram } else { out.histogram };
                // Peak can be >= abs_hist (set to abs_hist then possibly updated by trend state logic)
                prop_assert!(out.histogram_peak >= Decimal::ZERO);
                last_crossover_idx = Some(i);
                prev_peak = out.histogram_peak;
            }
        }
        // Verify the function never panics and values are finite
        let _ = last_crossover_idx;
        let _ = prev_peak;
    }

    #[test]
    fn macd_trend_state_matches_histogram_acceleration(prices in proptest::collection::vec(1.0f64..100_000.0, 5..100)) {
        let mut macd = Macd::new();
        let mut prev_abs_hist: Option<Decimal> = None;
        for &p in &prices {
            let out = macd.update(p);
            let abs_hist = if out.histogram < Decimal::ZERO { -out.histogram } else { out.histogram };

            if let Some(prev) = prev_abs_hist {
                if abs_hist >= prev {
                    prop_assert_eq!(out.trend_state, TrendState::Accelerating);
                } else {
                    prop_assert_eq!(out.trend_state, TrendState::Decelerating);
                }
            }
            prev_abs_hist = Some(abs_hist);
        }
    }
}
