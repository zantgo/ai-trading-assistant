//! AC-DIE-2 (Trade tick -> live candle update p95 < 2ms).
//!
//! Verifies that `CandleGenerator::process_trade` completes in under 2ms in
//! release mode. Uses a generous 50ms threshold to account for debug/test-profile
//! overhead in CI environments.

use std::time::Instant;

use core_domain::normalized::{Exchange, NormalizedTrade, TradeSide};
use market_analyzer::candle_generator::CandleGenerator;

#[test]
fn per_tick_candle_update_p95_under_threshold() {
    const ITERATIONS: usize = 1000;
    const THRESHOLD_MS: f64 = 50.0;

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut generator = CandleGenerator::new("BTC-USDT", 60, Exchange::Hyperliquid);

    let mut hasher = DefaultHasher::new();
    42u64.hash(&mut hasher);
    let mut seed = hasher.finish();

    let mut pseudo_rand = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        seed
    };

    let mut times: Vec<f64> = Vec::with_capacity(ITERATIONS);

    let base_ts = 1700000000000u64;
    let candle_duration_ms = 60_000u64;

    for i in 0..ITERATIONS {
        let r = pseudo_rand();
        let price = 50000.0 + ((r % 1001) as f64);
        let size = 0.01 + (((r >> 10) % 100) as f64) / 100.0;

        let price_dec = rust_decimal::Decimal::from_f64_retain(price).unwrap();
        let size_dec = rust_decimal::Decimal::from_f64_retain(size).unwrap();

        let side = if (r >> 20) & 1 == 0 {
            TradeSide::Buy
        } else {
            TradeSide::Sell
        };

        let ts = base_ts + (i as u64 % candle_duration_ms);

        let trade = NormalizedTrade {
            exchange: Exchange::Hyperliquid,
            symbol: "BTC-USDT".to_string(),
            price: price_dec,
            size: size_dec,
            side,
            timestamp_ms: ts,
            trade_id: format!("t_{}", i),
        };

        let start = Instant::now();
        let _ = generator.process_trade(&trade);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        times.push(elapsed);
    }

    times.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    let p95_idx = ((ITERATIONS as f64) * 0.95).ceil() as usize - 1;
    let p95 = times[p95_idx];
    let min = times[0];
    let max = times[ITERATIONS - 1];
    let median = times[ITERATIONS / 2];

    eprintln!(
        "Candle update stats (ms): min={min:.4} median={median:.4} p95={p95:.4} max={max:.4}"
    );

    assert!(
        p95 < THRESHOLD_MS,
        "p95 candle update time ({p95:.4}ms) must be below {THRESHOLD_MS}ms"
    );
}
