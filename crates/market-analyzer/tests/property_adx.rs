use proptest::prelude::*;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use market_analyzer::indicators::{Adx, TrendRegime};

fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

proptest! {
    #[test]
    fn adx_all_values_in_0_to_100(
        high in 1.0f64..100_000.0,
        low in 0.1f64..100_000.0,
        close in 0.1f64..100_000.0,
        count in 2usize..50
    ) {
        let mut adx = Adx::new(14);
        let mut h = high;
        let mut l = low.min(h);
        let mut c = close.clamp(l, h);

        adx.update(dec(h), dec(l), dec(c));
        for _ in 0..count {
            h += 0.1;
            l += 0.1;
            c += 0.1;
            let hd = dec(h);
            let ld = dec(l);
            let cd = dec(c);
            if let Some(out) = adx.update(hd, ld, cd) {
                let adx_v = out.adx.to_f64().unwrap_or(0.0);
                let plus = out.plus_di.to_f64().unwrap_or(0.0);
                let minus = out.minus_di.to_f64().unwrap_or(0.0);
                prop_assert!((0.0..=100.0).contains(&adx_v), "ADX out of [0,100]: {}", adx_v);
                prop_assert!((0.0..=100.0).contains(&plus), "+DI out of [0,100]: {}", plus);
                prop_assert!((0.0..=100.0).contains(&minus), "-DI out of [0,100]: {}", minus);
            }
        }
    }

    #[test]
    fn adx_di_sum_never_exceeds_200(
        high in 1.0f64..100_000.0,
        low in 0.1f64..100_000.0,
        close in 0.1f64..100_000.0,
        count in 2usize..50
    ) {
        let mut adx = Adx::new(14);
        let mut h = high;
        let mut l = low.min(h);
        let mut c = close.clamp(l, h);
        adx.update(dec(h), dec(l), dec(c));

        for _ in 0..count {
            h += 1.0;
            l += 1.0;
            c += 1.0;
            if let Some(out) = adx.update(dec(h), dec(l), dec(c)) {
                let sum = (out.plus_di + out.minus_di).to_f64().unwrap_or(0.0);
                prop_assert!(sum <= 200.0, "+DI + -DI = {} exceeds 200", sum);
            }
        }
    }

    #[test]
    fn adx_regime_matches_thresholds(
        high in 1.0f64..100_000.0,
        low in 0.1f64..100_000.0,
        close in 0.1f64..100_000.0,
        count in 15usize..50
    ) {
        let mut adx = Adx::new(14);
        let trend_threshold = Decimal::new(20, 0);
        let exhaustion_threshold = Decimal::new(40, 0);
        adx.set_thresholds(trend_threshold, exhaustion_threshold, 3);

        let mut h = high;
        let mut l = low.min(h);
        let mut c = close.clamp(l, h);
        adx.update(dec(h), dec(l), dec(c));

        for _ in 0..count {
            h += 0.1;
            l += 0.1;
            c += 0.1;
            if let Some(out) = adx.update(dec(h), dec(l), dec(c)) {
                match out.trending_regime {
                    TrendRegime::Congestion => prop_assert!(out.adx < trend_threshold),
                    TrendRegime::Emerging => prop_assert!(out.adx >= trend_threshold && out.adx < trend_threshold + Decimal::new(5, 0)),
                    TrendRegime::Strong => prop_assert!(out.adx >= trend_threshold + Decimal::new(5, 0) && out.adx <= exhaustion_threshold),
                    TrendRegime::Extreme => prop_assert!(out.adx > exhaustion_threshold),
                }
            }
        }
    }
}
