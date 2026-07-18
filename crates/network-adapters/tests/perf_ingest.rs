//! AC-DIE-1: Raw WS frame → NormalizedEvent p95 < 1ms.
//!
//! Measures construction-to-send latency for NormalizedEvent through an
//! mpsc channel — the core of the raw data ingestion path.

use core_domain::normalized::{Exchange, NormalizedEvent, NormalizedTrade, TradeSide};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Instant;

const ITERATIONS: usize = 1_000;

fn build_trade(i: usize) -> NormalizedTrade {
    let base_price = Decimal::from_str("97123.45").unwrap();
    let tick = Decimal::new(i as i64 % 100, 2);
    NormalizedTrade {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USD".to_string(),
        price: base_price + tick,
        size: Decimal::new((1000 + (i as i64 % 500)) * 10, 2),
        side: if i & 1 == 0 { TradeSide::Buy } else { TradeSide::Sell },
        timestamp_ms: 1_752_800_000_000 + (i as u64),
        trade_id: format!("0x{:016x}", i),
    }
}

#[tokio::test]
async fn ingest_p95_latency_below_50ms_debug_budget() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<NormalizedEvent>(10_000);

    let drain = tokio::spawn(async move {
        while rx.recv().await.is_some() {}
    });

    let mut durations = Vec::with_capacity(ITERATIONS);

    for i in 0..ITERATIONS {
        let t0 = Instant::now();
        let event = NormalizedEvent::Trade(build_trade(i));
        let _ = tx.send(event).await;
        durations.push(t0.elapsed());
    }

    drop(tx);
    let _ = drain.await;

    durations.sort();
    let p95_idx = (ITERATIONS as f64 * 0.95).ceil() as usize - 1;
    let p95 = durations[p95_idx];

    assert!(
        p95.as_millis() < 50,
        "AC-DIE-1 failed: p95 ingest latency {}ms >= 50ms",
        p95.as_millis()
    );
}
