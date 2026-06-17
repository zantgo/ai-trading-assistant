use proptest::prelude::*;
use shared::indicators::{DivergenceDetector, DivergenceType, DivergenceStatus};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

proptest! {
    #[test]
    fn divergence_never_jumps_none_to_confirmed(
        prices in proptest::collection::vec(50.0f64..200.0, 10..100),
        rsi_vals in proptest::collection::vec(1.0f64..99.0, 10..100)
    ) {
        let mut det = DivergenceDetector::new(10);
        let n = prices.len().min(rsi_vals.len());
        for i in 0..n {
            let result = det.update_full(dec(prices[i]), dec(rsi_vals[i]), Decimal::ZERO);

            // Status can be None or Potential, never Confirmed from update_full
            prop_assert!(result.rsi_status != DivergenceStatus::Confirmed,
                "RSI divergence must not jump directly to Confirmed");
            prop_assert!(result.macd_status != DivergenceStatus::Confirmed,
                "MACD divergence must not jump directly to Confirmed");

            // If status is Potential, divergence type must not be None
            if result.rsi_status == DivergenceStatus::Potential {
                prop_assert!(result.rsi_divergence != DivergenceType::None,
                    "RSI status Potential requires a divergence type");
            }
            if result.macd_status == DivergenceStatus::Potential {
                prop_assert!(result.macd_divergence != DivergenceType::None,
                    "MACD status Potential requires a divergence type");
            }
        }
    }

    #[test]
    fn divergence_coords_reference_valid_indices(
        prices in proptest::collection::vec(50.0f64..200.0, 10..100),
        rsi_vals in proptest::collection::vec(1.0f64..99.0, 10..100)
    ) {
        let mut det = DivergenceDetector::new(10);
        let n = prices.len().min(rsi_vals.len());
        for i in 0..n {
            let result = det.update_full(dec(prices[i]), dec(rsi_vals[i]), Decimal::ZERO);

            // If coordinates are present, indices must be within the lookback window
            if let Some(ref coords) = result.rsi_coords {
                prop_assert!(coords.first_extreme.index < 10,
                    "RSI first extreme index out of bounds");
                prop_assert!(coords.second_extreme.index < 10,
                    "RSI second extreme index out of bounds");
                prop_assert!(coords.first_extreme.index < coords.second_extreme.index,
                    "RSI first extreme must come before second extreme");
            }
            if let Some(ref coords) = result.macd_coords {
                prop_assert!(coords.first_extreme.index < 10,
                    "MACD first extreme index out of bounds");
                prop_assert!(coords.second_extreme.index < 10,
                    "MACD second extreme index out of bounds");
                prop_assert!(coords.first_extreme.index < coords.second_extreme.index,
                    "MACD first extreme must come before second extreme");
            }
        }
    }

    #[test]
    fn divergence_has_bullish_bearish_consistent(
        prices in proptest::collection::vec(50.0f64..200.0, 10..100),
        rsi_vals in proptest::collection::vec(1.0f64..99.0, 10..100)
    ) {
        let mut det = DivergenceDetector::new(10);
        let n = prices.len().min(rsi_vals.len());
        for i in 0..n {
            let result = det.update_full(dec(prices[i]), dec(rsi_vals[i]), Decimal::ZERO);

            // has_bullish is true iff at least one divergence type is bullish
            let any_bullish = result.rsi_divergence == DivergenceType::RsiBullish
                || result.macd_divergence == DivergenceType::MacdBullish;
            prop_assert_eq!(result.has_bullish, any_bullish,
                "has_bullish must match divergence types");

            let any_bearish = result.rsi_divergence == DivergenceType::RsiBearish
                || result.macd_divergence == DivergenceType::MacdBearish;
            prop_assert_eq!(result.has_bearish, any_bearish,
                "has_bearish must match divergence types");
        }
    }
}
