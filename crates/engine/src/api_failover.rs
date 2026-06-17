use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

#[derive(Debug, Clone, PartialEq)]
pub enum KeySource {
    Primary,
    Backup,
    None,
}

pub struct ApiFailoverState {
    pub primary_key: RwLock<Option<String>>,
    pub backup_key: RwLock<Option<String>>,
    pub active_source: RwLock<KeySource>,
    pub consecutive_failures: AtomicU32,
    pub total_calls: AtomicU32,
    pub total_failures: AtomicU32,
    pub permanently_failed: AtomicBool,

    // Configuration
    pub retry_delay_secs: u64,
    pub max_retries_per_call: u32,
    pub max_consecutive_failures: u32,
}

impl ApiFailoverState {
    pub fn new(
        primary_key: Option<String>,
        backup_key: Option<String>,
        retry_delay_secs: u64,
        max_retries_per_call: u32,
        max_consecutive_failures: u32,
    ) -> Self {
        let source = if primary_key.is_some() {
            KeySource::Primary
        } else if backup_key.is_some() {
            KeySource::Backup
        } else {
            KeySource::None
        };

        Self {
            primary_key: RwLock::new(primary_key),
            backup_key: RwLock::new(backup_key),
            active_source: RwLock::new(source),
            consecutive_failures: AtomicU32::new(0),
            total_calls: AtomicU32::new(0),
            total_failures: AtomicU32::new(0),
            permanently_failed: AtomicBool::new(false),
            retry_delay_secs,
            max_retries_per_call,
            max_consecutive_failures,
        }
    }

    /// Get the currently active API key.
    pub async fn active_key(&self) -> Option<String> {
        let source = self.active_source.read().await.clone();
        match source {
            KeySource::Primary => self.primary_key.read().await.clone(),
            KeySource::Backup => self.backup_key.read().await.clone(),
            KeySource::None => None,
        }
    }

    /// Record a successful API call.
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.total_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed API call. Returns true if the system should halt.
    pub fn record_failure(&self) -> bool {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        let current = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;

        if current >= self.max_consecutive_failures {
            self.permanently_failed.store(true, Ordering::Relaxed);
            true // Halt
        } else {
            false
        }
    }

    /// Attempt to switch to the backup key. Returns true if switch succeeded.
    pub async fn switch_to_backup(&self) -> bool {
        let backup = self.backup_key.read().await.clone();
        if backup.is_some() {
            *self.active_source.write().await = KeySource::Backup;
            println!("🔄 API Failover: Switched to backup API key");
            true
        } else {
            false
        }
    }

    /// Attempt to switch back to primary key.
    pub async fn switch_to_primary(&self) -> bool {
        let primary = self.primary_key.read().await.clone();
        if primary.is_some() {
            *self.active_source.write().await = KeySource::Primary;
            println!("🔄 API Failover: Switched back to primary API key");
            true
        } else {
            false
        }
    }

    /// Set a new primary key.
    pub async fn set_primary_key(&self, key: String) {
        *self.primary_key.write().await = Some(key);
        if *self.active_source.read().await == KeySource::None {
            *self.active_source.write().await = KeySource::Primary;
        }
        self.permanently_failed.store(false, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    /// Set a new backup key.
    pub async fn set_backup_key(&self, key: Option<String>) {
        *self.backup_key.write().await = key;
    }

    /// Check if the failover state is healthy (can make API calls).
    pub async fn is_healthy(&self) -> bool {
        if self.permanently_failed.load(Ordering::Relaxed) {
            return false;
        }
        self.active_key().await.is_some()
    }

    /// Execute a fallible operation with automatic retry and key failover.
    /// `make_call` receives the active API key and returns Result<T, String>.
    pub async fn execute_with_failover<F, Fut, T>(
        &self,
        call_name: &str,
        make_call: F,
    ) -> Result<T, String>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        if self.permanently_failed.load(Ordering::Relaxed) {
            return Err(format!(
                "[{}] API failover permanently halted due to {} consecutive failures",
                call_name, self.max_consecutive_failures
            ));
        }

        let mut last_error: Option<String> = None;

        // Try with current active key, with per-call retries
        for attempt in 0..self.max_retries_per_call + 1 {
            let key = self.active_key().await;
            let key = match key {
                Some(k) => k,
                None => return Err(format!("[{}] No API key configured", call_name)),
            };

            match make_call(key).await {
                Ok(result) => {
                    self.record_success();
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries_per_call {
                        eprintln!(
                            "⚠️  API Failover: [{}] attempt {}/{} failed. Retrying in {}s...",
                            call_name,
                            attempt + 1,
                            self.max_retries_per_call,
                            self.retry_delay_secs,
                        );
                        sleep(Duration::from_secs(self.retry_delay_secs)).await;
                    }
                }
            }
        }

        // Exhausted per-call retries. Try switching to backup.
        let switched = self.switch_to_backup().await;
        if switched {
            // Try with backup key (no retries for the fallback key)
            let key = self.active_key().await;
            if let Some(k) = key {
                match make_call(k).await {
                    Ok(result) => {
                        self.record_success();
                        return Ok(result);
                    }
                    Err(e) => {
                        last_error = Some(e);
                    }
                }
            }
        }

        // All failed. Record final failure and check if we should halt.
        let should_halt = self.record_failure();
        let msg = format!(
            "[{}] All API calls failed after {} retries. {}",
            call_name,
            self.max_retries_per_call,
            last_error.unwrap_or_else(|| "Unknown error".into()),
        );

        if should_halt {
            eprintln!("🛑 API Failover: Instance halted after {} consecutive failures", self.max_consecutive_failures);
        }

        Err(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicU32;

    #[tokio::test]
    async fn test_success_on_first_attempt() {
        let state = ApiFailoverState::new(
            Some("pk-test".into()),
            None,
            1,
            3,
            10,
        );

        let call_count = AtomicU32::new(0);
        let result = state.execute_with_failover("test", |key| {
            call_count.fetch_add(1, Ordering::Relaxed);
            async move {
                assert_eq!(key, "pk-test");
                Ok("success")
            }
        }).await;

        assert_eq!(result.unwrap(), "success");
        assert_eq!(call_count.load(Ordering::Relaxed), 1);
        assert_eq!(state.consecutive_failures.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_retries_before_failover() {
        let state = ApiFailoverState::new(
            Some("pk-test".into()),
            Some("bk-test".into()),
            0, // No delay for test
            2,
            10,
        );

        let call_count = AtomicU32::new(0);
        let result = state.execute_with_failover("test", |key| {
            let n = call_count.fetch_add(1, Ordering::Relaxed) + 1;
            async move {
                if n <= 2 {
                    Err(format!("fail {}", n))
                } else {
                    Ok(format!("ok via {}", key))
                }
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::Relaxed), 3); // 2 fail + 1 success
    }

    #[tokio::test]
    async fn test_failover_to_backup_key() {
        let state = ApiFailoverState::new(
            Some("pk-bad".into()),
            Some("bk-good".into()),
            0,
            1, // Only 1 retry per call -> 2 attempts
            10,
        );

        let call_count = AtomicU32::new(0);
        let seen_keys = Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_keys_clone = seen_keys.clone();

        let result = state.execute_with_failover("test", move |key| {
            seen_keys_clone.lock().unwrap().push(key.clone());
            let _n = call_count.fetch_add(1, Ordering::Relaxed) + 1;
            async move {
                if key == "pk-bad" {
                    Err("bad key".into())
                } else {
                    Ok("ok")
                }
            }
        }).await;

        assert!(result.is_ok());
        let keys = seen_keys.lock().unwrap();
        assert!(keys.iter().any(|k| k == "bk-good"), "Should have tried backup key");
    }

    #[tokio::test]
    async fn test_permanent_halt_after_max_failures() {
        let state = ApiFailoverState::new(
            Some("bad".into()),
            None,
            0,
            0, // no retries per call
            3, // halt after 3 consecutive failures
        );

        for i in 0..3 {
            let r = state.execute_with_failover("test", |_| async {
                Err::<&str, _>("fail".into())
            }).await;
            if i < 2 {
                assert!(r.is_err());
            }
        }
        // Third failure triggers halt
        let r = state.execute_with_failover("test", |_| async {
            Err::<&str, _>("fail".into())
        }).await;
        assert!(r.is_err());

        // Subsequent calls should fail immediately
        let r = state.execute_with_failover("test", |_| async {
            Ok::<&str, _>("should not be called")
        }).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("permanently halted"));
    }
}
