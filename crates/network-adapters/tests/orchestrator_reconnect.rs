use network_adapters::adapters::resilience::{
    apply_jitter, compute_backoff, run_with_reconnect, ReconnectPolicy, ReconnectState,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[test]
fn backoff_progression_is_exponential() {
    let policy = ReconnectPolicy {
        jitter_pct: 0.0,
        ..ReconnectPolicy::default()
    };

    let delays: Vec<Duration> = (0..8)
        .map(|attempt| compute_backoff(attempt, &policy))
        .collect();

    assert_eq!(
        delays,
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(8),
            Duration::from_secs(16),
            Duration::from_secs(30),
            Duration::from_secs(30),
            Duration::from_secs(30),
        ]
    );
}

#[test]
fn jitter_stays_within_bounds() {
    let delay = Duration::from_millis(1_000);
    let jitter_pct = 0.2;
    let lower = Duration::from_millis(800);
    let upper = Duration::from_millis(1_200);
    let mut observed_jitter = false;

    for _ in 0..100 {
        let result = apply_jitter(delay, jitter_pct);
        assert!(
            result >= lower,
            "jittered delay {result:?} below lower bound {lower:?}"
        );
        assert!(
            result <= upper,
            "jittered delay {result:?} above upper bound {upper:?}"
        );
        if result != delay {
            observed_jitter = true;
        }
    }

    assert!(observed_jitter, "expected at least one jittered value different from base");
}

#[test]
fn jitter_zero_pct_returns_unchanged() {
    assert_eq!(
        apply_jitter(Duration::from_secs(5), 0.0),
        Duration::from_secs(5)
    );
}

#[test]
fn jitter_zero_delay_returns_unchanged() {
    assert_eq!(
        apply_jitter(Duration::ZERO, 0.2),
        Duration::ZERO
    );
}

#[tokio::test]
async fn max_attempts_returns_failed() {
    let policy = ReconnectPolicy {
        initial_backoff: Duration::from_millis(1),
        max_backoff: Duration::from_millis(1),
        jitter_pct: 0.0,
        max_attempts: Some(5),
    };

    let reconnect = run_with_reconnect(
        "ws://127.0.0.1:1",
        policy,
        |_| {},
        || {},
        |_| {},
        CancellationToken::new(),
    );

    let result = tokio::time::timeout(Duration::from_secs(2), reconnect)
        .await
        .expect("reconnect loop did not stop at max attempts");

    assert!(matches!(
        &result.final_state,
        ReconnectState::Failed { attempts, .. } if *attempts == 5
    ));
    assert_eq!(result.total_reconnects, 4);
    assert_eq!(result.messages_received, 0);
}

#[tokio::test]
async fn permanent_disable_after_5_consecutive_failures() {
    let policy = ReconnectPolicy {
        initial_backoff: Duration::from_millis(1),
        max_backoff: Duration::from_millis(1),
        jitter_pct: 0.0,
        max_attempts: Some(5),
    };

    let reconnect = run_with_reconnect(
        "ws://127.0.0.1:1",
        policy,
        |_| {},
        || {},
        |_| {},
        CancellationToken::new(),
    );

    let result = tokio::time::timeout(Duration::from_secs(2), reconnect)
        .await
        .expect("reconnect loop did not stop at max attempts");

    if let ReconnectState::Failed {
        ref last_error,
        attempts,
    } = &result.final_state
    {
        assert_eq!(*attempts, 5, "should be permanently disabled after exactly 5 consecutive failures");
        assert!(!last_error.is_empty(), "failed state should contain an error message");
    } else {
        panic!(
            "expected ReconnectState::Failed after 5 consecutive failures, got {:?}",
            result.final_state
        );
    }

    assert_eq!(
        result.total_reconnects, 4,
        "should have 4 reconnect attempts (max_attempts - 1)"
    );
    assert_eq!(
        result.messages_received, 0,
        "should have zero messages received (never successfully connected)"
    );
}
