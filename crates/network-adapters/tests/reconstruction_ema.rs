//! AC-DIE-9 (03-01-01 §1.3): EMA reconstruction (sub-minute, ≥ 50 history)
//! converges within `ema_window` ticks of first synthesis.

use core_domain::normalized::{Exchange, ReconstructionMethod};
use network_adapters::adapters::reconstruction::CandleReconstructor;
use rust_decimal::prelude::ToPrimitive;

fn closes(n: usize, base: f64, step: f64) -> Vec<f64> {
    (0..n).map(|i| base + i as f64 * step).collect()
}

#[test]
fn ema_used_when_history_at_least_50() {
    let r = CandleReconstructor::new();
    let history = closes(50, 100.0, 0.1);
    let rc = r
        .reconstruct(Exchange::Hyperliquid, 0, 30_000, 30_000, &history)
        .expect("sub-minute reconstruction with ≥50 closes must synthesize");
    assert_eq!(rc.method, ReconstructionMethod::ExponentialMovingAverage);
    assert_eq!(
        rc.candle.reconstructed,
        Some(ReconstructionMethod::ExponentialMovingAverage)
    );
}

#[test]
fn ema_converges_toward_recent_closes() {
    // A flat series must synthesize exactly the flat value; a trending series
    // must land between the window's min and max and near the recent closes.
    let r = CandleReconstructor::new();

    let flat = vec![250.0; 80];
    let rc_flat = r
        .reconstruct(Exchange::Hyperliquid, 0, 30_000, 30_000, &flat)
        .unwrap();
    let flat_close = rc_flat.candle.close.to_f64().unwrap();
    assert!((flat_close - 250.0).abs() < 1e-9, "flat EMA = flat value");

    let trend = closes(200, 100.0, 0.5); // 100.0 .. 199.5
    let rc_trend = r
        .reconstruct(Exchange::Hyperliquid, 0, 30_000, 30_000, &trend)
        .unwrap();
    let trend_close = rc_trend.candle.close.to_f64().unwrap();
    assert!(trend_close > 100.0 && trend_close < 199.5);
    // The EMA is biased toward the most recent closes.
    assert!(
        trend_close > 150.0,
        "EMA of an uptrend should sit in the upper half (got {trend_close})"
    );
}

#[test]
fn ema_projection_is_flat_ohlc_with_zero_volume_by_default() {
    let r = CandleReconstructor::new();
    let history = closes(60, 500.0, -0.2);
    let rc = r
        .reconstruct(Exchange::Bitget, 60_000, 90_000, 30_000, &history)
        .unwrap();
    assert_eq!(rc.candle.open, rc.candle.close);
    assert_eq!(rc.candle.high, rc.candle.low);
    assert_eq!(rc.candle.volume.to_f64().unwrap(), 0.0);
    assert_eq!(rc.candle.trades_count, 0);
    assert_eq!(rc.candle.start_time_ms, 60_000);
    assert_eq!(rc.candle.duration_ms, 30_000);
}

#[test]
fn sub_50_history_falls_back_to_linear() {
    let r = CandleReconstructor::new();
    let history = closes(10, 100.0, 1.0);
    let rc = r
        .reconstruct(Exchange::Hyperliquid, 0, 30_000, 30_000, &history)
        .unwrap();
    assert_eq!(rc.method, ReconstructionMethod::LinearInterpolation);
}

#[test]
fn one_minute_and_above_defers_to_exchange_rest() {
    let r = CandleReconstructor::new();
    let history = closes(200, 100.0, 0.5);
    assert!(
        r.reconstruct(Exchange::Hyperliquid, 0, 60_000, 60_000, &history)
            .is_none(),
        "≥1m intervals must be filled from exchange REST history"
    );
}
