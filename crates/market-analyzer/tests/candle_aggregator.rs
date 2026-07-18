//! AC-L2-2 (03-01-03 §6.1): multi-timeframe rollup preserves OHLCV
//! invariants: `high = max(highs)`, `low = min(lows)`, `close = last close`,
//! `volume = Σ volumes`, `trades_count = Σ counts`.

use core_domain::normalized::{Exchange, NormalizedCandle, ReconstructionMethod};
use market_analyzer::candle_aggregator::CandleAggregator;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn candle(start_ms: u64, o: Decimal, h: Decimal, l: Decimal, c: Decimal, v: Decimal, n: u64) -> NormalizedCandle {
    NormalizedCandle {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USDT".to_string(),
        start_time_ms: start_ms,
        duration_ms: 60_000,
        open: o,
        high: h,
        low: l,
        close: c,
        volume: v,
        trades_count: n,
        reconstructed: None,
    }
}

#[test]
fn rollup_preserves_ohlcv_invariants() {
    let mut aggregator = CandleAggregator::new("BTC-USDT", &[180]);

    let sources = [
        candle(0, dec!(100), dec!(105), dec!(99), dec!(101), dec!(10), 5),
        candle(60_000, dec!(101), dec!(110), dec!(100), dec!(108), dec!(20), 7),
        candle(120_000, dec!(108), dec!(109), dec!(95), dec!(96), dec!(30), 11),
    ];
    for s in &sources {
        assert!(aggregator.process_candle(s).is_empty(), "no rollover yet");
    }

    // Crossing into the next 180 s bucket emits the completed rollup.
    let next = candle(180_000, dec!(96), dec!(97), dec!(94), dec!(95), dec!(1), 1);
    let completed = aggregator.process_candle(&next);
    assert_eq!(completed.len(), 1);
    let agg = &completed[0].candle;

    assert_eq!(agg.open, dec!(100), "open = first open");
    assert_eq!(agg.high, dec!(110), "high = max(highs)");
    assert_eq!(agg.low, dec!(95), "low = min(lows)");
    assert_eq!(agg.close, dec!(96), "close = last close");
    assert_eq!(agg.volume, dec!(60), "volume = Σ volumes");
    assert_eq!(agg.trades_count, 23, "trades_count = Σ counts");
    assert_eq!(agg.start_time_ms, 0);
    assert_eq!(agg.duration_ms, 180_000);
    agg.assert_validity().expect("rolled-up candle must be valid");
}

#[test]
fn multi_target_ladder_emits_independent_rollups() {
    // Default ladder rollups: fast 180 s and slow 300 s from micro 60 s.
    let mut aggregator = CandleAggregator::new("BTC-USDT", &[180, 300]);
    let mut completed_180 = 0u32;
    let mut completed_300 = 0u32;

    for i in 0..11u64 {
        let c = candle(
            i * 60_000,
            dec!(100),
            dec!(101),
            dec!(99),
            dec!(100),
            dec!(1),
            1,
        );
        for done in aggregator.process_candle(&c) {
            match done.timeframe_secs {
                180 => completed_180 += 1,
                300 => completed_300 += 1,
                other => panic!("unexpected timeframe {other}"),
            }
            assert_eq!(
                done.candle.start_time_ms % (done.timeframe_secs * 1000),
                0,
                "rollup start aligned to its own tier"
            );
        }
    }

    // 11 micro candles (0..=10 min): 180s buckets completed at 3,6,9 → 3;
    // 300s buckets completed at 5,10 → 2.
    assert_eq!(completed_180, 3);
    assert_eq!(completed_300, 2);
}

#[test]
fn reconstruction_provenance_propagates_to_rollup() {
    let mut aggregator = CandleAggregator::new("BTC-USDT", &[180]);
    let mut tagged = candle(0, dec!(100), dec!(101), dec!(99), dec!(100), dec!(1), 1);
    tagged.reconstructed = Some(ReconstructionMethod::ExponentialMovingAverage);

    aggregator.process_candle(&tagged);
    aggregator.process_candle(&candle(60_000, dec!(100), dec!(101), dec!(99), dec!(100), dec!(1), 1));
    aggregator.process_candle(&candle(120_000, dec!(100), dec!(101), dec!(99), dec!(100), dec!(1), 1));
    let completed = aggregator.process_candle(&candle(180_000, dec!(100), dec!(101), dec!(99), dec!(100), dec!(1), 1));

    assert_eq!(
        completed[0].candle.reconstructed,
        Some(ReconstructionMethod::ExponentialMovingAverage),
        "a rollup containing any reconstructed source carries the provenance flag"
    );
}
