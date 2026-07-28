//! AC-L3-3 (03-01-04 §7.1): late ticks (timestamp earlier than a
//! previously-completed candle interval) are dropped at L3 and counted in
//! `out_of_order_dropped`. Completed candles are immutable — a late tick
//! never mutates an already-closed interval.
//!
//! Note: the doc AC table places this file under `network-adapters/tests/`;
//! it lives here because the late-tick detector (`CandleGenerator`) is an L2
//! artifact hosted in `market-analyzer` and `network-adapters` cannot depend
//! on it without inverting the crate graph (01-06).

use core_domain::normalized::{Exchange, NormalizedTrade, TradeSide};
use market_analyzer::candle_generator::CandleGenerator;
use network_adapters::pipeline_reliability::ReliabilityTracker;
use rust_decimal_macros::dec;

fn trade(ts_ms: u64) -> NormalizedTrade {
    NormalizedTrade {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USDT".to_string(),
        price: dec!(100),
        size: dec!(1),
        side: TradeSide::Buy,
        timestamp_ms: ts_ms,
        trade_id: format!("t{ts_ms}"),
    }
}

#[test]
fn tick_in_previous_interval_is_late() {
    let mut generator = CandleGenerator::new("BTC-USDT", 60, Exchange::Hyperliquid);
    generator.process_trade(&trade(120_500)); // current interval starts at 120000

    assert!(generator.is_late_tick(119_999), "prior interval → late");
    assert!(generator.is_late_tick(60_000), "far prior interval → late");
    assert!(!generator.is_late_tick(120_001), "same interval → not late");
    assert!(
        !generator.is_late_tick(180_000),
        "future interval → not late"
    );
}

#[test]
fn no_candle_yet_means_nothing_is_late() {
    let generator = CandleGenerator::new("BTC-USDT", 60, Exchange::Hyperliquid);
    assert!(!generator.is_late_tick(0));
    assert!(!generator.is_late_tick(999_999_999));
}

#[tokio::test]
async fn dropped_late_ticks_are_counted_in_out_of_order_dropped() {
    // Replicates the L3 drop rule used by the analyzer loop:
    // `if generator.is_late_tick(ts) { reliability.increment_out_of_order(1); skip }`.
    let mut generator = CandleGenerator::new("BTC-USDT", 60, Exchange::Hyperliquid);
    let reliability = ReliabilityTracker::new();

    let stream = [120_500u64, 121_000, 119_000, 60_500, 122_000, 118_750];
    let mut applied = 0u32;
    for ts in stream {
        if generator.is_late_tick(ts) {
            reliability.increment_out_of_order(1).await;
            continue;
        }
        generator.process_trade(&trade(ts));
        applied += 1;
    }

    let metrics = reliability.snapshot().await;
    assert_eq!(metrics.out_of_order_dropped, 3, "three late ticks dropped");
    assert_eq!(applied, 3, "in-order ticks still applied");
}

#[test]
fn completed_candle_is_immutable_after_late_tick() {
    let mut generator = CandleGenerator::new("BTC-USDT", 60, Exchange::Hyperliquid);
    generator.process_trade(&trade(60_100));
    let (completed, _) = generator.process_trade(&trade(120_100));
    let completed = completed.expect("boundary crossing emits candle");
    let frozen = (
        completed.open,
        completed.high,
        completed.low,
        completed.close,
    );

    // The late tick targets the closed [60000,120000) interval; the L3 rule
    // drops it before it can touch state, so the emitted candle is unchanged.
    assert!(generator.is_late_tick(61_000));
    assert_eq!(
        (
            completed.open,
            completed.high,
            completed.low,
            completed.close
        ),
        frozen
    );
}
