//! AC-L3-5 (03-01-04 §7.1): every completed candle passes `assert_validity()`
//! before reaching L4 — `high ≥ low`, `open/close ∈ [low, high]`,
//! `volume ≥ 0`. Candles built by the L2 generator are structurally valid by
//! construction; hand-built invalid candles are caught.

use core_domain::normalized::{Exchange, NormalizedCandle, NormalizedTrade, TradeSide};
use market_analyzer::candle_generator::CandleGenerator;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn base_candle() -> NormalizedCandle {
    NormalizedCandle {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USDT".to_string(),
        start_time_ms: 0,
        duration_ms: 60_000,
        open: dec!(100),
        high: dec!(110),
        low: dec!(90),
        close: dec!(105),
        volume: dec!(5),
        trades_count: 3,
        reconstructed: None,
    }
}

#[test]
fn valid_candle_passes() {
    assert!(base_candle().assert_validity().is_ok());
}

#[test]
fn inverted_high_low_fails() {
    let mut c = base_candle();
    c.high = dec!(80);
    assert!(c.assert_validity().is_err());
}

#[test]
fn open_outside_bounds_fails() {
    let mut c = base_candle();
    c.open = dec!(120);
    assert!(c.assert_validity().is_err());
}

#[test]
fn close_outside_bounds_fails() {
    let mut c = base_candle();
    c.close = dec!(80);
    assert!(c.assert_validity().is_err());
}

#[test]
fn negative_volume_fails() {
    let mut c = base_candle();
    c.volume = Decimal::from(-1);
    assert!(c.assert_validity().is_err());
}

#[test]
fn generator_output_is_always_valid() {
    // Random-ish tick stream: every completed candle from the L2 generator
    // satisfies the structural invariants.
    let mut generator = CandleGenerator::new("BTC-USDT", 60, Exchange::Hyperliquid);
    let mut completed_count = 0u32;
    for i in 0..2_000u64 {
        let price = dec!(100) + Decimal::from((i * 7919) % 97) - Decimal::from((i * 104729) % 89);
        let trade = NormalizedTrade {
            exchange: Exchange::Hyperliquid,
            symbol: "BTC-USDT".to_string(),
            price,
            size: dec!(0.25),
            side: if i % 2 == 0 { TradeSide::Buy } else { TradeSide::Sell },
            timestamp_ms: i * 3_000,
            trade_id: format!("t{i}"),
        };
        let (completed, live) = generator.process_trade(&trade);
        live.assert_validity().expect("live shadow candle valid");
        if let Some(c) = completed {
            c.assert_validity().expect("completed candle valid");
            completed_count += 1;
        }
    }
    assert!(completed_count > 50, "stream produced completed candles");
}
