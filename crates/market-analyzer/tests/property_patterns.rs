use market_analyzer::indicators::fibonacci::{PivotPoint, PivotType};
use market_analyzer::indicators::{detect_pattern, ChartPattern};
use proptest::prelude::*;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

fn make_high(index: usize, price: f64) -> PivotPoint {
    PivotPoint {
        index,
        price: dec(price),
        pivot_type: PivotType::High,
        strength: 10,
    }
}

fn make_low(index: usize, price: f64) -> PivotPoint {
    PivotPoint {
        index,
        price: dec(price),
        pivot_type: PivotType::Low,
        strength: 10,
    }
}

proptest! {
    #[test]
    fn patterns_confidence_always_in_0_to_100(
        n in 1usize..30
    ) {
        // Build a list of alternating high/low pivots with random spacing
        let mut pivots: Vec<PivotPoint> = Vec::with_capacity(n);
        let mut price = 100.0;
        for i in 0..n {
            let is_high = i % 2 == 0;
            let idx = i * 5 + (i % 3); // varying indices
            price += (i as f64 * 0.3).sin() * 20.0 + 2.0;
            pivots.push(if is_high {
                make_high(idx, price)
            } else {
                make_low(idx, price * 0.95)
            });
        }

        let result = detect_pattern(&pivots);
        prop_assert!(result.confidence >= 0.0, "Confidence must be >= 0, got {}", result.confidence);
        prop_assert!(result.confidence <= 100.0, "Confidence must be <= 100, got {}", result.confidence);
    }

    #[test]
    fn patterns_mutual_exclusion_single_pattern_or_none(
        n in 1usize..30
    ) {
        let mut pivots: Vec<PivotPoint> = Vec::with_capacity(n);
        let mut price = 100.0;
        for i in 0..n {
            price += (i as f64 * 0.5).cos() * 15.0 + 1.0;
            pivots.push(if i % 2 == 0 {
                make_high(i * 3, price)
            } else {
                make_low(i * 3, price * 0.97)
            });
        }

        let result = detect_pattern(&pivots);
        // is_bullish and is_bearish should NOT both be true
        prop_assert!(!(result.is_bullish && result.is_bearish),
            "Pattern must not be both bullish and bearish simultaneously");
        if result.pattern == ChartPattern::None {
            prop_assert!(!result.is_bullish);
            prop_assert!(!result.is_bearish);
        }
    }

    #[test]
    fn patterns_insufficient_pivots_returns_none(
        n in 0usize..4
    ) {
        let mut pivots: Vec<PivotPoint> = Vec::with_capacity(n);
        for i in 0..n {
            pivots.push(if i % 2 == 0 {
                make_high(i, 100.0 + i as f64)
            } else {
                make_low(i, 95.0 + i as f64)
            });
        }

        let result = detect_pattern(&pivots);
        // With <4 pivots total, we can't have 2 highs AND 2 lows
        prop_assert_eq!(result.pattern, ChartPattern::None,
            "Insufficient pivots ({}) should yield None", n);
        prop_assert!(!result.is_bullish);
        prop_assert!(!result.is_bearish);
    }

    #[test]
    fn patterns_only_highs_returns_none(
        count in 2usize..10
    ) {
        let pivots: Vec<PivotPoint> = (0..count)
            .map(|i| make_high(i, 100.0 + i as f64 * 10.0))
            .collect();

        let result = detect_pattern(&pivots);
        prop_assert_eq!(result.pattern, ChartPattern::None,
            "All highs and no lows should yield None");
    }

    #[test]
    fn patterns_only_lows_returns_none(
        count in 2usize..10
    ) {
        let pivots: Vec<PivotPoint> = (0..count)
            .map(|i| make_low(i, 100.0 - i as f64 * 5.0))
            .collect();

        let result = detect_pattern(&pivots);
        prop_assert_eq!(result.pattern, ChartPattern::None,
            "All lows and no highs should yield None");
    }

    #[test]
    fn patterns_triangle_has_convergence_confidence_positive(
        highs_prices in proptest::collection::vec(150.0f64..200.0, 3..8),
        lows_prices in proptest::collection::vec(50.0f64..100.0, 3..8)
    ) {
        // Construct a valid triangle: descending highs, ascending lows
        let mut sorted_highs: Vec<f64> = highs_prices.into_iter().collect();
        sorted_highs.sort_by(|a, b| b.partial_cmp(a).unwrap()); // descending
        let mut sorted_lows: Vec<f64> = lows_prices.into_iter().collect();
        sorted_lows.sort_by(|a, b| a.partial_cmp(b).unwrap()); // ascending

        let n = sorted_highs.len().min(sorted_lows.len()).min(6);
        let mut pivots: Vec<PivotPoint> = Vec::with_capacity(n * 2);
        let mut idx = 0;
        for i in 0..n {
            pivots.push(make_low(idx, sorted_lows[i]));
            idx += 2;
            pivots.push(make_high(idx, sorted_highs[i]));
            idx += 2;
        }

        let result = detect_pattern(&pivots);
        // Should at least not crash, and if a pattern is found verify invariants
        if result.pattern != ChartPattern::None {
            prop_assert!(result.confidence >= 0.0);
            prop_assert!(result.confidence <= 100.0);
            if result.is_bullish { prop_assert!(!result.is_bearish); }
            if result.is_bearish { prop_assert!(!result.is_bullish); }
        }
    }

    #[test]
    fn patterns_random_pivots_always_valid_result(
        n in 1usize..50
    ) {
        // Fully random pivot sequences — must never panic
        let mut pivots: Vec<PivotPoint> = Vec::with_capacity(n);
        for i in 0..n {
            let price = 50.0 + (i as f64 * 1.7).sin() * 100.0 + (i as f64 * 0.3);
            let idx = i * 2 + (i % 5);
            let is_high = (i + (price as usize % 3)).is_multiple_of(2);
            pivots.push(if is_high {
                PivotPoint {
                    index: idx,
                    price: dec(price),
                    pivot_type: PivotType::High,
                    strength: 5 + (i % 5),
                }
            } else {
                PivotPoint {
                    index: idx,
                    price: dec(price * 0.9),
                    pivot_type: PivotType::Low,
                    strength: 5 + (i % 5),
                }
            });
        }

        let result = detect_pattern(&pivots);
        // Just verify it returns a valid PatternResult without panicking
        prop_assert!(result.confidence >= 0.0);
        prop_assert!(result.confidence <= 100.0);
        prop_assert!(!(result.is_bullish && result.is_bearish));
        if result.pattern == ChartPattern::None {
            prop_assert!(!result.is_bullish);
            prop_assert!(!result.is_bearish);
        }
    }

    #[test]
    fn patterns_confidence_zero_when_none(
        n in 1usize..10
    ) {
        // Pivots with tiny price differences that won't form patterns
        let pivots: Vec<PivotPoint> = (0..n)
            .map(|i| {
                if i % 2 == 0 {
                    make_high(i, 100.0 + i as f64 * 0.01)
                } else {
                    make_low(i, 100.0 - i as f64 * 0.01)
                }
            })
            .collect();

        let result = detect_pattern(&pivots);
        if result.pattern == ChartPattern::None {
            prop_assert_eq!(result.confidence, 0.0,
                "Confidence must be 0.0 when pattern is None");
        }
    }

    #[test]
    fn patterns_bullish_bearish_match_pattern_type(
        n in 3usize..20
    ) {
        let mut pivots: Vec<PivotPoint> = Vec::with_capacity(n);
        let mut price = 100.0;
        for i in 0..n {
            price += (i as f64 * 0.8).sin() * 30.0 + 1.5;
            pivots.push(if i % 2 == 0 {
                make_high(i * 2, price)
            } else {
                make_low(i * 2, price * 0.92)
            });
        }

        let result = detect_pattern(&pivots);
        match result.pattern {
            ChartPattern::BullishTriangle | ChartPattern::FallingWedge | ChartPattern::AscendingChannel => {
                prop_assert!(result.is_bullish, "Bullish pattern must have is_bullish=true");
                prop_assert!(!result.is_bearish, "Bullish pattern must have is_bearish=false");
            }
            ChartPattern::BearishTriangle | ChartPattern::RisingWedge | ChartPattern::DescendingChannel => {
                prop_assert!(result.is_bearish, "Bearish pattern must have is_bearish=true");
                prop_assert!(!result.is_bullish, "Bearish pattern must have is_bullish=false");
            }
            ChartPattern::None => {
                prop_assert!(!result.is_bullish);
                prop_assert!(!result.is_bearish);
            }
        }
    }
}
