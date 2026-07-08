use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use engine::analyzer::warm_indicators_for_timeframe;
use engine::config::{CandlesConfig, FibonacciConfig, IndicatorsConfig, TimeframeConfig};
use shared::normalized::NormalizedCandle;

/// Build a synthetic candle whose OHLC frames a clear swing structure so the
/// pivot detector produces support (swing lows) and resistance (swing highs).
fn candle(start_ms: u64, o: Decimal, h: Decimal, l: Decimal, c: Decimal) -> NormalizedCandle {
    NormalizedCandle {
        symbol: "SRT".to_string(),
        start_time_ms: start_ms,
        duration_ms: 60_000,
        open: o,
        high: h,
        low: l,
        close: c,
        volume: dec!(100.0),
        trades_count: 10,
    }
}

/// Phase 1 verification: Support & Resistance zones are wired into the
/// normalization pipeline. Before wiring, `support_levels`/`resistance_levels`
/// were always `&[]`, so the `support_resistance` indicator was permanently
/// `INACTIVE`. After wiring, warming a series with clear swing pivots must
/// yield an active `support_resistance` entry with a non-`INACTIVE` label.
#[test]
fn test_support_resistance_zones_activate_from_warm() {
    let indicators = IndicatorsConfig {
        ema_fast: 5,
        ema_medium: 10,
        ema_slow: 20,
        ema_long: 30,
        rsi_period: 5,
        adx_period: 5,
        adx_trend_threshold: 20,
        adx_exhaustion_threshold: 40,
        adx_slope_lookback: 3,
        squeeze_period: 5,
        squeeze_min_duration: 2,
        bbwp_lookback: 10,
        bbwp_period: 5,
        atr_period: 5,
        ..Default::default()
    };

    let fib_config = FibonacciConfig {
        swing_lookback: 3,
        swing_scan_range: 60,
        retracement_coefficients: vec![0.618, 0.660],
        extension_coefficients: vec![1.618, 2.618],
    };

    let tf = TimeframeConfig {
        candles: CandlesConfig {
            duration_seconds: 60,
            analysis_limit: 200,
        },
        indicators,
    };

    // Build a clean triangle-wave zig-zag between 90 and 110 (leg length 5).
    // Each turning bar is a strict local extremum, so the pivot detector marks
    // repeated swing highs (~110) and swing lows (~90).
    let mut candles: Vec<NormalizedCandle> = Vec::new();
    let mut t = 60_000u64;
    let mut price = dec!(90.0);
    let mut dir: i64 = 1;
    let step = dec!(5.0);
    for _ in 0..100u64 {
        let high = price + dec!(0.5);
        let low = price - dec!(0.5);
        candles.push(candle(t, price, high, low, price));
        t += 60_000;
        if price >= dec!(110.0) {
            dir = -1;
        } else if price <= dec!(90.0) {
            dir = 1;
        }
        price += step * Decimal::from(dir);
    }

    let warmed = warm_indicators_for_timeframe(candles, &tf, &fib_config, "SRT", 60);

    let snap = warmed
        .latest_snapshot
        .expect("warm pass should produce a latest snapshot");

    let sr = snap
        .indicators
        .get("support_resistance")
        .expect("support_resistance indicator must be present in the map");

    // The core Phase-1 assertion: S/R is no longer permanently INACTIVE.
    // Depending on where price sits relative to the marked swing levels it may
    // be a demand/supply zone, a confirmed flip, or structurally neutral — but
    // it must NOT be the INACTIVE placeholder that meant "no levels wired".
    assert_ne!(
        sr.state_label, "INACTIVE",
        "S/R must be active once zones are wired (got INACTIVE — levels not flowing)"
    );

    // Normalized must be a finite value within the unit interval.
    assert!(
        sr.normalized >= -1.0 && sr.normalized <= 1.0,
        "S/R normalized out of range: {}",
        sr.normalized
    );
}
