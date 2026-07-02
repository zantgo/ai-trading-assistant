use proptest::prelude::*;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use shared::indicators::{MomentumDirection, SqueezeMomentum};

fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

proptest! {
    #[test]
    fn squeeze_duration_only_increments_when_on(
        prices in proptest::collection::vec(50.0f64..200.0, 40..100)
    ) {
        let mut sqz = SqueezeMomentum::new(20);
        let mut prev_on_known: Option<bool> = None;

        for (i, &p) in prices.iter().enumerate() {
            let high = p + (i as f64 * 0.1);
            let low = p - (i as f64 * 0.1);
            if let Some(out) = sqz.update(dec(high), dec(low), dec(p)) {
                if !out.squeeze_on {
                    // Squeeze OFF → duration must be 0
                    prop_assert_eq!(out.squeeze_duration, 0,
                        "Duration must drop to 0 when squeeze is OFF");
                    if prev_on_known == Some(true) {
                        prop_assert!(out.squeeze_release_trigger,
                            "Release trigger must fire on ON→OFF transition");
                    }
                }
                prev_on_known = Some(out.squeeze_on);
            }
        }
    }

    #[test]
    fn squeeze_momentum_direction_matches_sign(
        prices in proptest::collection::vec(50.0f64..200.0, 40..100)
    ) {
        let mut sqz = SqueezeMomentum::new(20);
        for &p in prices.iter() {
            let high = p + 1.0;
            let low = p - 1.0;
            if let Some(out) = sqz.update(dec(high), dec(low), dec(p)) {
                match out.momentum_direction {
                    MomentumDirection::BullishAcceleration | MomentumDirection::BullishDeceleration => {
                        prop_assert!(out.momentum_value > Decimal::ZERO,
                            "Bullish direction requires positive momentum");
                    }
                    MomentumDirection::BearishAcceleration | MomentumDirection::BearishDeceleration => {
                        prop_assert!(out.momentum_value < Decimal::ZERO,
                            "Bearish direction requires negative momentum");
                    }
                    MomentumDirection::Flat => {
                        // Near zero
                    }
                }
            }
        }
    }

    #[test]
    fn squeeze_nesting_state_consistent(
        prices in proptest::collection::vec(50.0f64..200.0, 40..100)
    ) {
        let mut sqz = SqueezeMomentum::new(20);
        for (i, &p) in prices.iter().enumerate() {
            let high = p + (i as f64 * 0.5).sin() * 10.0;
            let low = p - (i as f64 * 0.5).cos() * 10.0;
            if let Some(out) = sqz.update(dec(high), dec(low), dec(p)) {
                // Verify structural integrity: state transitions are consistent
                prop_assert!(out.momentum_value.to_f64().is_some(),
                    "Momentum value should be a valid float");
                // Release trigger only fires when transitioning ON→OFF
                if out.squeeze_release_trigger {
                    prop_assert!(!out.squeeze_on, "Release trigger requires squeeze OFF");
                    prop_assert_eq!(out.squeeze_duration, 0,
                        "After release, duration must be 0");
                }
            }
        }
    }
}
