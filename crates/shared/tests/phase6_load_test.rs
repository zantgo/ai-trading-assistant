//! Phase 6 load test: high-volume liquidation event ingestion.
//!
//! Validates that the LiquidityEventAccumulator handles sustained
//! bursts of events without memory blowup or computation slowdown.

use std::time::Instant;
use shared::liquidity::LiquidityEventAccumulator;
use shared::normalized::{Exchange, LiquidationEvent, LiquidationSide};

fn make_event(price: f64, size: f64, ts_ms: u64) -> LiquidationEvent {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    LiquidationEvent {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USDT".to_string(),
        side: if ts_ms % 2 == 0 {
            LiquidationSide::Long
        } else {
            LiquidationSide::Short
        },
        price: Decimal::from_f64_retain(price).unwrap_or(dec!(50000)),
        size: Decimal::from_f64_retain(size).unwrap_or(dec!(1.0)),
        timestamp_ms: ts_ms,
        venue_order_id: None,
    }
}

#[test]
fn ten_thousand_events_per_minute_throughput() {
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    let start = Instant::now();
    for i in 0..10_000u64 {
        acc.record_event(make_event(50_000.0, 0.5, i * 6));
    }
    let elapsed = start.elapsed();
    println!("10,000 events ingested in {:?}", elapsed);
    assert!(
        elapsed.as_secs() < 5,
        "should ingest 10k events in <5s, took {:?}",
        elapsed
    );
    // Bounded memory: history must cap at 1k (default).
    assert_eq!(acc.buffered_event_count(), 1000);
}

#[test]
fn sustained_burst_with_mixed_sides() {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    // 5,000 long + 5,000 short in alternating batches.
    for i in 0..10_000u64 {
        let side_long = i < 5_000;
        let ev = LiquidationEvent {
            exchange: Exchange::Hyperliquid,
            symbol: "BTC-USDT".to_string(),
            side: if side_long {
                LiquidationSide::Long
            } else {
                LiquidationSide::Short
            },
            price: Decimal::from_f64_retain(50_000.0).unwrap_or(dec!(0)),
            size: Decimal::from_f64_retain(0.1).unwrap_or(dec!(0)),
            timestamp_ms: i * 12,
            venue_order_id: None,
        };
        acc.record_event(ev);
    }
    let flow = acc.flush_to_flow();
    assert!(flow.long_liquidations_usd > 0.0);
    assert!(flow.short_liquidations_usd > 0.0);
}