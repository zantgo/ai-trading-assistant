use std::time::{Duration, Instant};
use tokio::sync::broadcast;

const MSG_COUNT: usize = 100;
const FAST_SUB_COUNT: usize = 4;
const BUFFER_SIZE: usize = 16;

#[tokio::test]
async fn broadcast_fanout_is_nonblocking() {
    let (tx, _keepalive) = broadcast::channel::<String>(BUFFER_SIZE);

    let mut fast_handles = Vec::with_capacity(FAST_SUB_COUNT);
    for sub_id in 0..FAST_SUB_COUNT {
        let mut rx = tx.subscribe();
        let handle = tokio::spawn(async move {
            let mut received = 0usize;
            let mut total_lagged = 0usize;
            loop {
                match rx.recv().await {
                    Ok(_) => received += 1,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        total_lagged += n as usize;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            (sub_id, received, total_lagged)
        });
        fast_handles.push(handle);
    }

    let mut slow_rx = tx.subscribe();
    let slow_handle = tokio::spawn(async move {
        let mut got_lagged = false;
        loop {
            match slow_rx.recv().await {
                Ok(_) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    assert!(
                        n > 0,
                        "sleeping subscriber must report positive lag count, got {n}"
                    );
                    got_lagged = true;
                    break;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
        got_lagged
    });

    let start = Instant::now();

    {
        let tx = tx.clone();
        tokio::spawn(async move {
            for i in 0..MSG_COUNT {
                let _ = tx.send(format!("msg-{i}"));
            }
        })
        .await
        .expect("producer panicked");
    }

    drop(tx);

    for handle in fast_handles {
        let (sub_id, received, lagged) = handle.await.expect("fast subscriber panicked");
        let total = received + lagged;
        assert_eq!(
            total, MSG_COUNT,
            "fast subscriber {sub_id}: received {received} + lagged {lagged} = {total}, expected {MSG_COUNT}"
        );
    }

    let lagged = slow_handle.await.expect("slow subscriber panicked");
    assert!(
        lagged,
        "sleeping subscriber must receive RecvError::Lagged after 1 s sleep"
    );

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "fan-out completed in {elapsed:?} — producer and fast subscribers not blocked by sleeping peer"
    );
}
