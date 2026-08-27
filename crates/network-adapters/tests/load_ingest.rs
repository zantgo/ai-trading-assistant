use core_domain::normalized::{Exchange, NormalizedEvent, NormalizedTrade, TradeSide};
use rust_decimal::Decimal;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration, Instant};

const CAPACITY: usize = 10_000;
const EVENT_COUNT: usize = 10_000;
const MIN_THROUGHPUT: f64 = 1_000.0;

fn make_trade(i: usize) -> NormalizedEvent {
    NormalizedEvent::Trade(NormalizedTrade {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USD".into(),
        price: Decimal::from(50_000u64 + (i % 500) as u64),
        size: Decimal::from(1u64),
        side: if i % 2 == 0 {
            TradeSide::Buy
        } else {
            TradeSide::Sell
        },
        timestamp_ms: i as u64,
        trade_id: format!("trade_{}", i),
    })
}

#[tokio::test]
async fn sustained_ingestion_1000_trades_per_sec_no_channel_saturation() {
    let (tx, mut rx) = mpsc::channel::<NormalizedEvent>(CAPACITY);

    let consumer = tokio::spawn(async move {
        let mut received = 0usize;
        loop {
            tokio::select! {
                maybe = rx.recv() => {
                    match maybe {
                        Some(_) => received += 1,
                        None => break,
                    }
                }
            }
            if received % 500 == 0 {
                tokio::task::yield_now().await;
            }
        }
        received
    });

    let start = Instant::now();

    let tx_prod = tx.clone();
    let producer_timeout = Duration::from_secs(20);

    let producer = tokio::spawn(async move {
        let outcome = timeout(producer_timeout, async {
            for i in 0..EVENT_COUNT {
                tx_prod
                    .send(make_trade(i))
                    .await
                    .expect("channel receiver dropped");
            }
        })
        .await;

        match outcome {
            Ok(()) => true,
            Err(_elapsed) => false,
        }
    });

    let completed = producer.await.expect("producer panicked");
    assert!(
        completed,
        "producer timed out after {:?} — channel may be saturated",
        Duration::from_secs(20)
    );

    let elapsed = start.elapsed();
    drop(tx);

    let received = consumer.await.expect("consumer panicked");

    assert_eq!(
        received, EVENT_COUNT,
        "expected {} events received, got {}",
        EVENT_COUNT, received
    );

    let throughput = EVENT_COUNT as f64 / elapsed.as_secs_f64();

    assert!(
        throughput >= MIN_THROUGHPUT,
        "throughput {:.0} trades/sec below minimum threshold of {:.0} (elapsed {:.3}s with backpressure via channel capacity {})",
        throughput,
        MIN_THROUGHPUT,
        elapsed.as_secs_f64(),
        CAPACITY,
    );
}
