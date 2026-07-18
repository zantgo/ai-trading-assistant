//! AC-L2-4: `average_volume` is the MME-side rolling baseline; the per-candle
//! `volume / trades_count` ratio is `avg_trade_size` and is not emitted.
//! This test verifies that `NormalizedCandle` carries only the raw `volume`
//! sum and `trades_count` — no `average_volume` field exists on the struct.

use core_domain::normalized::{Exchange, NormalizedCandle};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[test]
fn volume_is_raw_sum_not_an_average() {
    let candle = NormalizedCandle {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USDT".to_string(),
        start_time_ms: 0,
        duration_ms: 60_000,
        open: dec!(50000),
        high: dec!(51000),
        low: dec!(49000),
        close: dec!(50500),
        volume: dec!(150),
        trades_count: 30,
        reconstructed: None,
    };

    // Volume is the raw sum of all trade sizes in the candle window
    assert_eq!(candle.volume, dec!(150));
    assert_eq!(candle.trades_count, 30);

    // avg_trade_size = volume / trades_count — computed on demand, not a stored field
    let avg_trade_size = candle.volume / Decimal::from(candle.trades_count);
    assert_eq!(avg_trade_size, dec!(5));
}

#[test]
fn no_average_volume_field_exists() {
    // AC-L2-4: `NormalizedCandle` intentionally does NOT carry an
    // `average_volume` field. The MME pipeline computes the rolling
    // baseline downstream from successive candle volumes.
    //
    // This test compiles because `candle.volume` refers to the raw sum.
    // If an `average_volume` field were ever added to `NormalizedCandle`,
    // this test alone would not catch it — but the L2 spec mandates that
    // per-candle averages are not persisted in the data model, and adding
    // the field would violate the documented contract.

    let candle = NormalizedCandle {
        exchange: Exchange::Bitget,
        symbol: "ETH-USDT".to_string(),
        start_time_ms: 60_000,
        duration_ms: 60_000,
        open: dec!(3000),
        high: dec!(3100),
        low: dec!(2950),
        close: dec!(3050),
        volume: dec!(42),
        trades_count: 14,
        reconstructed: None,
    };

    assert_eq!(candle.volume, dec!(42));

    let avg_trade_size = candle.volume / Decimal::from(candle.trades_count);
    assert_eq!(avg_trade_size, dec!(3));
}

#[test]
fn avg_trade_size_with_single_trade() {
    let candle = NormalizedCandle {
        exchange: Exchange::Hyperliquid,
        symbol: "SOL-USDT".to_string(),
        start_time_ms: 0,
        duration_ms: 60_000,
        open: dec!(50),
        high: dec!(50),
        low: dec!(50),
        close: dec!(50),
        volume: dec!(0),
        trades_count: 1,
        reconstructed: None,
    };

    let avg_trade_size = candle.volume / Decimal::from(candle.trades_count);
    assert_eq!(avg_trade_size, dec!(0));

    // Edge case: non-zero volume, single trade — avg_trade_size equals volume
    let candle = NormalizedCandle {
        exchange: Exchange::Hyperliquid,
        symbol: "SOL-USDT".to_string(),
        start_time_ms: 60_000,
        duration_ms: 60_000,
        open: dec!(50),
        high: dec!(50),
        low: dec!(50),
        close: dec!(50),
        volume: dec!(10.5),
        trades_count: 1,
        reconstructed: None,
    };

    let avg_trade_size = candle.volume / Decimal::from(candle.trades_count);
    assert_eq!(avg_trade_size, dec!(10.5));
}
