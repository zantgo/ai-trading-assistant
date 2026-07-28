//! AC-L4-4 (03-01-05 §5.1): a lagged consumer receives `RecvError::Lagged(n)`
//! within 1 frame of falling behind and can resubscribe at the current head;
//! the producer is never blocked (AC-L4-3 non-blocking fan-out).

use core_domain::normalized::{Exchange, NormalizedCandle};
use rust_decimal_macros::dec;
use tokio::sync::broadcast;

fn candle(i: u64) -> NormalizedCandle {
    NormalizedCandle {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USDT".to_string(),
        start_time_ms: i * 60_000,
        duration_ms: 60_000,
        open: dec!(100),
        high: dec!(101),
        low: dec!(99),
        close: dec!(100),
        volume: dec!(1),
        trades_count: 1,
        reconstructed: None,
    }
}

#[tokio::test]
async fn lagged_consumer_gets_lagged_error_and_resyncs() {
    let (tx, mut slow_rx) = broadcast::channel::<NormalizedCandle>(4);

    // Producer outruns the capacity-4 channel while the consumer sleeps.
    for i in 0..10u64 {
        tx.send(candle(i)).expect("send never blocks");
    }

    // First recv surfaces the explicit lag signal.
    match slow_rx.recv().await {
        Err(broadcast::error::RecvError::Lagged(n)) => {
            assert!(n >= 1, "lag count reported (got {n})");
        }
        other => panic!("expected Lagged, got {other:?}"),
    }

    // After the lag signal the consumer resumes from the oldest retained
    // frame and can drain to the head.
    let mut received = Vec::new();
    while let Ok(c) = slow_rx.try_recv() {
        received.push(c.start_time_ms / 60_000);
    }
    assert!(!received.is_empty(), "consumer resynchronized");
    assert_eq!(*received.last().unwrap(), 9, "caught up to the head");
    // Frames are in production order.
    assert!(
        received.windows(2).all(|w| w[0] < w[1]),
        "ordering preserved"
    );
}

#[tokio::test]
async fn slow_subscriber_never_blocks_producer_or_peers() {
    let (tx, mut fast_rx) = broadcast::channel::<NormalizedCandle>(4);
    let _slow_rx = tx.subscribe(); // never polled — permanently slow

    let start = std::time::Instant::now();
    let producer = tokio::spawn({
        let tx = tx.clone();
        async move {
            for i in 0..1_000u64 {
                let _ = tx.send(candle(i));
            }
        }
    });

    // The fast consumer keeps receiving (possibly with lag skips) while the
    // slow subscriber exists.
    let mut seen = 0u32;
    loop {
        match fast_rx.recv().await {
            Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {
                seen += 1;
                if seen >= 4 {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    producer.await.unwrap();
    assert!(
        start.elapsed() < std::time::Duration::from_secs(1),
        "producer completed without ever blocking on the dead subscriber"
    );
}

#[tokio::test]
async fn resubscribe_starts_at_current_head() {
    let (tx, _keepalive) = broadcast::channel::<NormalizedCandle>(4);
    for i in 0..100u64 {
        let _ = tx.send(candle(i));
    }
    // A fresh subscription sees only frames sent after it was created.
    let mut fresh = tx.subscribe();
    let _ = tx.send(candle(1_000));
    let got = fresh.recv().await.expect("head frame");
    assert_eq!(got.start_time_ms, 1_000 * 60_000);
}
