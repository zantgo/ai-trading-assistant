use engine::api_failover::ApiFailoverState;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[tokio::test]
async fn test_primary_to_backup_switch_on_failure() {
    let state = Arc::new(ApiFailoverState::new(
        Some("primary-bad".into()),
        Some("backup-good".into()),
        0,   // zero delay for fast test
        1,   // 1 retry per call
        10,  // halt after 10
    ));

    let call_count = AtomicU32::new(0);
    let state_ref = state.clone();

    let result = state_ref.execute_with_failover("test", move |key| {
        call_count.fetch_add(1, Ordering::Relaxed);
        async move {
            if key == "primary-bad" {
                Err::<&str, _>("primary key failed".into())
            } else {
                Ok::<&str, _>("ok via backup")
            }
        }
    }).await;

    assert!(result.is_ok(), "Should succeed via backup key");
    assert_eq!(result.unwrap(), "ok via backup");

    // Verify active source switched to backup
    let active = state.active_source.read().await.clone();
    assert_eq!(active, engine::api_failover::KeySource::Backup, "Should have switched to backup");
    assert_eq!(state.consecutive_failures.load(Ordering::Relaxed), 0,
        "Consecutive failures should reset after success");
}

#[tokio::test]
async fn test_permanent_halt_blocks_subsequent_calls() {
    let state = ApiFailoverState::new(
        Some("doomed".into()),
        None,   // no backup
        0,      // zero delay
        0,      // no retries per call
        3,      // halt after 3 consecutive
    );

    // First 3 calls all fail
    for _ in 0..3 {
        let result = state.execute_with_failover("test", |_| async {
            Err::<&str, _>("always fails".into())
        }).await;
        assert!(result.is_err());
    }

    // Verify permanently failed
    assert!(state.permanently_failed.load(Ordering::Relaxed),
        "Should be permanently failed after max consecutive failures");

    // Next call should fail immediately with "permanently halted"
    let final_result = state.execute_with_failover("test", |_| async {
        Ok::<&str, _>("should never be called")
    }).await;
    assert!(final_result.is_err());
    assert!(final_result.unwrap_err().contains("permanently halted"),
        "Should return permanent halt error");
}

#[tokio::test]
async fn test_success_resets_consecutive_failures() {
    let state = ApiFailoverState::new(
        Some("good-key".into()),
        None,
        0,
        1,
        10,
    );

    let call_count = Arc::new(AtomicU32::new(0));

    // Fail twice
    for _ in 0..2 {
        let count = call_count.clone();
        let result = state.execute_with_failover("test", move |_| {
            count.fetch_add(1, Ordering::Relaxed);
            async move {
                Err::<&str, _>("fail".into())
            }
        }).await;
        assert!(result.is_err());
    }

    assert_eq!(state.consecutive_failures.load(Ordering::Relaxed), 2);

    // Now succeed
    let count = call_count.clone();
    let result = state.execute_with_failover("test", move |_| {
        count.fetch_add(1, Ordering::Relaxed);
        async move {
            Ok::<&str, _>("recovered")
        }
    }).await;
    assert!(result.is_ok());

    // Consecutive failures should reset to 0
    assert_eq!(state.consecutive_failures.load(Ordering::Relaxed), 0,
        "Consecutive failures should reset after success");
    assert!(!state.permanently_failed.load(Ordering::Relaxed),
        "Should not be permanently failed after recovery");
}
