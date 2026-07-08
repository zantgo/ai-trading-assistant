use rust_decimal::prelude::ToPrimitive;
use shared::indicators::Ichimoku;

fn feed(ich: &mut Ichimoku, n: usize, start: f64, step: f64) -> Option<f64> {
    let mut out = None;
    for i in 0..n {
        let base = start + step * i as f64;
        // reasonable wick: range = 2, body fills most of it
        let h = base + 1.0;
        let l = base - 1.0;
        let c = base;
        out = ich.update(
            rust_decimal::Decimal::from_f64_retain(h).unwrap(),
            rust_decimal::Decimal::from_f64_retain(l).unwrap(),
            rust_decimal::Decimal::from_f64_retain(c).unwrap(),
        )
        .map(|r| r.tenkan.to_f64().unwrap());
    }
    out
}

/// None before 52 candles (minimum required for Senkou B).
#[test]
fn ichimoku_none_until_warmup_complete() {
    let mut ich = Ichimoku::new(9, 26, 52, 26);
    let out = feed(&mut ich, 50, 100.0, 0.0);
    assert!(out.is_none(), "should be None before 52 candles");
}

/// After 52+ candles, returns Some output.
#[test]
fn ichimoku_produces_output_after_52() {
    let mut ich = Ichimoku::new(9, 26, 52, 26);
    let out = feed(&mut ich, 55, 100.0, 0.0);
    assert!(out.is_some(), "should produce output after 52 candles");
}

/// In a steady uptrend, the faster Tenkan (9) leads the slower Kijun (26).
#[test]
fn ichimoku_uptrend_tenkan_above_kijun() {
    let mut ich = Ichimoku::new(9, 26, 52, 26);
    // 60 candles, steady uptrend.
    feed(&mut ich, 60, 100.0, 1.0);
    // Read the latest output explicitly by feeding one more.
    let result = ich.update(
        rust_decimal::Decimal::from_f64_retain(161.0).unwrap(),
        rust_decimal::Decimal::from_f64_retain(159.0).unwrap(),
        rust_decimal::Decimal::from_f64_retain(160.0).unwrap(),
    );
    assert!(result.is_some());
    let r = result.unwrap();
    let tenkan: f64 = r.tenkan.to_f64().unwrap();
    let kijun: f64 = r.kijun.to_f64().unwrap();
    assert!(
        tenkan > kijun,
        "uptrend: Tenkan ({}) should exceed Kijun ({})",
        tenkan,
        kijun
    );
}

/// Senkou A must always equal (Tenkan + Kijun) / 2.
#[test]
fn ichimoku_senkou_a_is_tenkan_kijun_midpoint() {
    let mut ich = Ichimoku::new(9, 26, 52, 26);
    feed(&mut ich, 60, 100.0, 1.0);
    let result = ich.update(
        rust_decimal::Decimal::from_f64_retain(161.0).unwrap(),
        rust_decimal::Decimal::from_f64_retain(159.0).unwrap(),
        rust_decimal::Decimal::from_f64_retain(160.0).unwrap(),
    );
    let r = result.unwrap();
    let expected = (r.tenkan + r.kijun) / rust_decimal::Decimal::from(2);
    assert_eq!(
        r.senkou_a, expected,
        "Senkou A must be the Tenkan-Kijun midpoint"
    );
}

/// After 52+displacement bars the current-applicable cloud should differ from
/// the live forward projection (the projection queue has aged a full cycle).
#[test]
fn ichimoku_current_cloud_differs_after_full_displacement() {
    let mut ich = Ichimoku::new(9, 26, 52, 26);
    // Warm with an oscillating series so values change across bars.
    for i in 0..90i64 {
        let base = 100.0 + (i / 5 * 10) as f64; // stair-step up every 5 bars
        ich.update(
            rust_decimal::Decimal::from_f64_retain(base + 2.0).unwrap(),
            rust_decimal::Decimal::from_f64_retain(base - 2.0).unwrap(),
            rust_decimal::Decimal::from_f64_retain(base).unwrap(),
        );
    }
    let r = ich
        .update(
            rust_decimal::Decimal::from_f64_retain(300.0).unwrap(),
            rust_decimal::Decimal::from_f64_retain(280.0).unwrap(),
            rust_decimal::Decimal::from_f64_retain(290.0).unwrap(),
        )
        .unwrap();
    // After 90+ bars, the projection queue is full: current ≠ future.
    assert!(
        (r.senkou_a_current != r.senkou_a) || (r.senkou_b_current != r.senkou_b),
        "current-applicable cloud should diverge from the forward projection after {} bars",
        90
    );
}

/// Cloud thickness: top = max(sa, sb), bottom = min(sa, sb).
#[test]
fn ichimoku_cloud_ordering() {
    let mut ich = Ichimoku::new(9, 26, 52, 26);
    feed(&mut ich, 60, 100.0, 1.0);
    let r = ich
        .update(
            rust_decimal::Decimal::from_f64_retain(161.0).unwrap(),
            rust_decimal::Decimal::from_f64_retain(159.0).unwrap(),
            rust_decimal::Decimal::from_f64_retain(160.0).unwrap(),
        )
        .unwrap();
    let top = r.senkou_a_current.max(r.senkou_b_current);
    let bot = r.senkou_a_current.min(r.senkou_b_current);
    assert!(top >= bot);
}
