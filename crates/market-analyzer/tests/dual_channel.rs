//! AC-L4-5 (03-01-05 §5.2): two broadcast channels per (symbol, timeframe)
//! (NormalizedCandle + MarketSnapshot) operate independently; lag in one does
//! not affect the other.

use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::test]
async fn dual_channel_independence_lag_in_one_does_not_block_the_other() {
    let (candle_tx, _) = broadcast::channel::<String>(64);
    let (snapshot_tx, _) = broadcast::channel::<String>(64);
    let mut candle_rx = candle_tx.subscribe();
    let mut snapshot_rx = snapshot_tx.subscribe();
    let go = Arc::new(tokio::sync::Barrier::new(3));

    let slow_handle = {
        let go = go.clone();
        tokio::spawn(async move {
            let mut received = Vec::new();
            go.wait().await;
            loop {
                match candle_rx.recv().await {
                    Ok(msg) => {
                        received.push(msg);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            received
        })
    };

    let fast_handle = {
        let go = go.clone();
        tokio::spawn(async move {
            let mut received = Vec::new();
            go.wait().await;
            loop {
                match snapshot_rx.recv().await {
                    Ok(msg) => {
                        received.push(msg);
                        if received.len() >= 40 {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            received
        })
    };

    go.wait().await;

    let start = std::time::Instant::now();
    for i in 0..40u32 {
        candle_tx
            .send(format!("candle-{i}"))
            .expect("candle channel send never blocks");
        snapshot_tx
            .send(format!("snapshot-{i}"))
            .expect("snapshot channel send never blocks");
    }

    let fast_received = fast_handle.await.unwrap();
    let fast_elapsed = start.elapsed();

    drop(candle_tx);
    drop(snapshot_tx);

    let slow_received = slow_handle.await.unwrap();

    assert_eq!(fast_received.len(), 40);
    for (i, msg) in fast_received.iter().enumerate() {
        assert_eq!(msg, &format!("snapshot-{i}"));
    }
    assert!(
        fast_elapsed < std::time::Duration::from_secs(2),
        "fast subscriber received all 40 messages within {fast_elapsed:?}"
    );

    assert!(
        slow_received.len() <= 40,
        "slow subscriber received {} messages",
        slow_received.len()
    );
}
