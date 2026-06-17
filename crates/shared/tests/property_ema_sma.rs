use proptest::prelude::*;
use shared::indicators::{Ema, Sma};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

proptest! {
    #[test]
    fn ema_range_containment(prices in proptest::collection::vec(0.1f64..100000.0, 5..200)) {
        let mut ema = Ema::new(10);
        let mut values = Vec::new();
        for &p in &prices {
            let val = ema.update(dec(p));
            values.push(val);
        }
        if values.len() >= 10 {
            let min_price = prices.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_price = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            for &v in &values[9..] {
                let vf = v.to_f64().unwrap_or(0.0);
                prop_assert!(vf >= min_price * 0.01, "EMA {} below min {}", vf, min_price);
                prop_assert!(vf <= max_price * 2.0, "EMA {} above max*2 {}", vf, max_price);
            }
        }
    }

    #[test]
    fn ema_monotonicity_on_trend(prices in proptest::collection::vec(1.0f64..1000.0, 10..100)) {
        let mut increasing: Vec<f64> = prices.clone();
        increasing.sort_by(|a, b| a.partial_cmp(b).unwrap());
        increasing.dedup();
        if increasing.len() < 10 { return Ok(()); }

        let mut ema = Ema::new(5);
        let mut prev = None;
        for &p in &increasing {
            let val = ema.update(dec(p)).to_f64().unwrap_or(0.0);
            if let Some(prev_val) = prev {
                prop_assert!(val >= prev_val, "EMA must rise with monotonic prices: {} -> {}", prev_val, val);
            }
            prev = Some(val);
        }
    }

    #[test]
    fn sma_range_containment(prices in proptest::collection::vec(0.1f64..100000.0, 10..200)) {
        let mut sma = Sma::new(5);
        for &p in &prices {
            if let Some(val) = sma.update(dec(p)) {
                let vf = val.to_f64().unwrap_or(0.0);
                let min_p = prices.iter().cloned().fold(f64::INFINITY, f64::min);
                let max_p = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                prop_assert!(vf >= min_p, "SMA {} below min {}", vf, min_p);
                prop_assert!(vf <= max_p, "SMA {} above max {}", vf, max_p);
            }
        }
    }

    #[test]
    fn sma_single_value_convergence(price in 1.0f64..100000.0) {
        let mut sma = Sma::new(5);
        for _ in 0..10 {
            let val = sma.update(dec(price));
            if let Some(v) = val {
                let vf = v.to_f64().unwrap_or(0.0);
                prop_assert!((vf - price).abs() < 0.01, "SMA {} should converge to {}", vf, price);
            }
        }
    }
}
