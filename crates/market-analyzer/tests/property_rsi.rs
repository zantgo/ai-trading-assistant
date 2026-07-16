use proptest::prelude::*;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use market_analyzer::indicators::Rsi;

fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

proptest! {
    #[test]
    fn rsi_always_in_0_to_100(prices in proptest::collection::vec(0.001f64..1_000_000.0, 2..300)) {
        let mut rsi = Rsi::new(14);
        let mut first = true;
        for window in prices.windows(2) {
            let _change = window[1] - window[0];
            let p = if first { first = false; dec(window[0]) } else { dec(window[1]) };
            if let Some(val) = rsi.update(p) {
                let vf = val.to_f64().unwrap_or(0.0);
                prop_assert!(vf >= 0.0, "RSI {} below 0", vf);
                prop_assert!(vf <= 100.0, "RSI {} above 100", vf);
            }
            // If change is huge, still shouldn't crash
        }
    }

    #[test]
    fn rsi_zero_volatility_no_panic(price in 1.0f64..100000.0) {
        let mut rsi = Rsi::new(14);
        // Feed the same price repeatedly → flatline
        for _ in 0..20 {
            if let Some(val) = rsi.update(dec(price)) {
                let vf = val.to_f64().unwrap_or(0.0);
                prop_assert!((0.0..=100.0).contains(&vf), "RSI on flatline {}", vf);
            }
        }
    }

    #[test]
    fn rsi_massive_spikes_no_panic(ref spikes in proptest::collection::vec((0.001f64..1000.0, 1000.0f64..100_000.0), 1..50)) {
        let mut rsi = Rsi::new(14);
        rsi.update(dec(100.0)); // seed
        for &(low, high) in spikes {
            // After a spike up, spike down
            if let Some(v1) = rsi.update(dec(high)) {
                let vf = v1.to_f64().unwrap_or(0.0);
                prop_assert!((0.0..=100.0).contains(&vf), "RSI spike high {}", vf);
            }
            if let Some(v2) = rsi.update(dec(low)) {
                let vf = v2.to_f64().unwrap_or(0.0);
                prop_assert!((0.0..=100.0).contains(&vf), "RSI spike low {}", vf);
            }
        }
    }
}
