use proptest::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use market_analyzer::indicators::Bbwp;

proptest! {
    #[test]
    fn bbwp_always_in_0_to_100(prices in proptest::collection::vec(1.0f64..100_000.0, 25..100)) {
        let mut bbwp = Bbwp::new(20, 20); // smaller lookback for faster test
        for &p in &prices {
            if let Some(percentile) = bbwp.update(p) {
                let pct = percentile.to_f64().unwrap_or(0.0);
                prop_assert!(pct >= 0.0, "BBWP {} below 0", pct);
                prop_assert!(pct <= 100.0, "BBWP {} above 100", pct);
            }
        }
    }

    #[test]
    fn bbwp_extrema_mapping(price in 1.0f64..100_000.0) {
        // All identical prices → bandwidth is zero → BBWP should be near 0
        let mut bbwp = Bbwp::new(20, 20);
        for _ in 0..50 {
            let _ = bbwp.update(price);
        }
        if let Some(pct) = bbwp.update(price) {
            let pct_v = pct.to_f64().unwrap_or(0.0);
            // With all identical prices, percentile should be 0 (narrowest)
            prop_assert!(pct_v < 10.0, "Identical prices should give low BBWP, got {}", pct_v);
        }
    }

    #[test]
    fn bbwp_compression_and_exhaustion_thresholds(
        prices in proptest::collection::vec(50.0f64..200.0, 25..100)
    ) {
        let mut bbwp = Bbwp::new(20, 20);
        for &p in &prices {
            if let Some(pct) = bbwp.update(p) {
                let is_comp = bbwp.is_compression(pct);
                let is_exh = bbwp.is_exhaustion(pct);
                let pct_v = pct.to_f64().unwrap_or(0.0);
                if pct_v < 10.0 { prop_assert!(is_comp); }
                if pct_v > 90.0 { prop_assert!(is_exh); }
                // Compression and exhaustion are mutually exclusive
                if !(10.0..=90.0).contains(&pct_v) {
                    prop_assert!(!(is_comp && is_exh));
                }
            }
        }
    }
}
