use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub jitter_pct: f64,
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            jitter_pct: 0.2,
            max_attempts: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReconnectState {
    Connected { since: Instant },
    Reconnecting { attempt: u32, next_retry: Instant },
    Failed { last_error: String, attempts: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReconnectResult {
    pub final_state: ReconnectState,
    pub total_reconnects: u32,
    pub messages_received: u64,
}

pub fn compute_backoff(attempt: u32, policy: &ReconnectPolicy) -> Duration {
    let mut delay = policy.initial_backoff.min(policy.max_backoff);

    if delay.is_zero() || delay == policy.max_backoff {
        return apply_jitter(delay, policy.jitter_pct);
    }

    for _ in 0..attempt {
        delay = delay
            .checked_mul(2)
            .unwrap_or(policy.max_backoff)
            .min(policy.max_backoff);
        if delay == policy.max_backoff {
            break;
        }
    }

    apply_jitter(delay, policy.jitter_pct)
}

fn apply_jitter(delay: Duration, jitter_pct: f64) -> Duration {
    if delay.is_zero() || !jitter_pct.is_finite() || jitter_pct <= 0.0 {
        return delay;
    }

    let jitter_pct = jitter_pct.min(1.0);
    let jitter = rand::thread_rng().gen_range(-jitter_pct..=jitter_pct);
    delay.mul_f64(1.0 + jitter)
}

fn transition<S>(current: &mut ReconnectState, next: ReconnectState, state_callback: &S)
where
    S: Fn(ReconnectState),
{
    match &next {
        ReconnectState::Connected { since } => {
            info!(since = ?since, "WebSocket connected");
        }
        ReconnectState::Reconnecting {
            attempt,
            next_retry,
        } => {
            warn!(attempt = *attempt, next_retry = ?next_retry, "WebSocket reconnect scheduled");
        }
        ReconnectState::Failed {
            last_error,
            attempts,
        } => {
            error!(attempts = *attempts, last_error = %last_error, "WebSocket reconnect failed");
        }
    }

    *current = next.clone();
    state_callback(next);
}

fn make_result(
    final_state: ReconnectState,
    total_reconnects: u32,
    messages_received: u64,
) -> ReconnectResult {
    ReconnectResult {
        final_state,
        total_reconnects,
        messages_received,
    }
}

pub async fn run_with_reconnect<F, R, S>(
    url: &str,
    policy: ReconnectPolicy,
    on_message: F,
    on_resume: R,
    state_callback: S,
    cancel: CancellationToken,
) -> ReconnectResult
where
    F: Fn(Vec<u8>) + Send + Sync + 'static,
    R: Fn() + Send + Sync + 'static,
    S: Fn(ReconnectState) + Send + Sync + 'static,
{
    let mut current_state = ReconnectState::Reconnecting {
        attempt: 0,
        next_retry: Instant::now(),
    };
    let mut total_reconnects = 0u32;
    let mut messages_received = 0u64;
    let mut failed_attempts = 0u32;
    let mut reconnect_attempt = 0u32;
    let mut has_attempted_connection = false;

    if policy.max_attempts == Some(0) {
        let failed = ReconnectState::Failed {
            last_error: "maximum reconnect attempts is zero".to_string(),
            attempts: 0,
        };
        transition(&mut current_state, failed, &state_callback);
        return make_result(current_state, total_reconnects, messages_received);
    }

    loop {
        if cancel.is_cancelled() {
            info!("WebSocket reconnect loop cancelled");
            return make_result(current_state, total_reconnects, messages_received);
        }

        let is_reconnect = has_attempted_connection;
        if is_reconnect {
            total_reconnects = total_reconnects.saturating_add(1);
        }
        has_attempted_connection = true;

        let connect_result = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                info!("WebSocket connection attempt cancelled");
                return make_result(current_state, total_reconnects, messages_received);
            }
            connect_result = tokio_tungstenite::connect_async(url) => connect_result,
        };

        let ws_stream = match connect_result {
            Ok((ws_stream, _)) => ws_stream,
            Err(connect_error) => {
                failed_attempts = failed_attempts.saturating_add(1);
                let last_error = connect_error.to_string();
                warn!(attempts = failed_attempts, error = %last_error, "WebSocket connection attempt failed");

                if policy
                    .max_attempts
                    .is_some_and(|max_attempts| failed_attempts >= max_attempts)
                {
                    let failed = ReconnectState::Failed {
                        last_error,
                        attempts: failed_attempts,
                    };
                    transition(&mut current_state, failed, &state_callback);
                    return make_result(current_state, total_reconnects, messages_received);
                }

                reconnect_attempt = reconnect_attempt.saturating_add(1).max(1);
                let delay = compute_backoff(reconnect_attempt - 1, &policy);
                let next_retry = Instant::now() + delay;
                let reconnecting = ReconnectState::Reconnecting {
                    attempt: reconnect_attempt,
                    next_retry,
                };
                transition(&mut current_state, reconnecting, &state_callback);

                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => {
                        info!("WebSocket reconnect backoff cancelled");
                        return make_result(current_state, total_reconnects, messages_received);
                    }
                    _ = tokio::time::sleep(delay) => {}
                }
                continue;
            }
        };

        failed_attempts = 0;
        let connected = ReconnectState::Connected {
            since: Instant::now(),
        };
        transition(&mut current_state, connected, &state_callback);

        if is_reconnect {
            on_resume();
        }

        let (mut write, mut read) = ws_stream.split();
        let disconnect_reason = loop {
            let next_message = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    info!("WebSocket connection cancelled");
                    return make_result(current_state, total_reconnects, messages_received);
                }
                next_message = read.next() => next_message,
            };

            match next_message {
                Some(Ok(Message::Text(text))) => {
                    messages_received = messages_received.saturating_add(1);
                    on_message(text.as_bytes().to_vec());
                }
                Some(Ok(Message::Binary(data))) => {
                    messages_received = messages_received.saturating_add(1);
                    on_message(data.to_vec());
                }
                Some(Ok(Message::Ping(payload))) => {
                    let pong_result = tokio::select! {
                        biased;
                        _ = cancel.cancelled() => {
                            info!("WebSocket connection cancelled");
                            return make_result(current_state, total_reconnects, messages_received);
                        }
                        pong_result = write.send(Message::Pong(payload)) => pong_result,
                    };
                    if let Err(pong_error) = pong_result {
                        break format!("failed to send pong: {pong_error}");
                    }
                }
                Some(Ok(Message::Close(frame))) => {
                    break format!("peer closed connection: {frame:?}");
                }
                Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {}
                Some(Err(socket_error)) => break socket_error.to_string(),
                None => break "WebSocket stream ended".to_string(),
            }
        };

        warn!(reason = %disconnect_reason, "WebSocket disconnected");
        reconnect_attempt = 1;
        let delay = compute_backoff(0, &policy);
        let next_retry = Instant::now() + delay;
        let reconnecting = ReconnectState::Reconnecting {
            attempt: reconnect_attempt,
            next_retry,
        };
        transition(&mut current_state, reconnecting, &state_callback);

        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                info!("WebSocket reconnect backoff cancelled");
                return make_result(current_state, total_reconnects, messages_received);
            }
            _ = tokio::time::sleep(delay) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_progression_is_exponential() {
        let policy = ReconnectPolicy {
            jitter_pct: 0.0,
            ..ReconnectPolicy::default()
        };
        let delays: Vec<_> = (0..8)
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
    fn jitter_within_bounds() {
        let policy = ReconnectPolicy::default();
        let lower_bound = Duration::from_millis(800);
        let upper_bound = Duration::from_millis(1_200);
        let mut observed_jitter = false;

        for _ in 0..100 {
            let delay = compute_backoff(0, &policy);
            assert!(delay >= lower_bound);
            assert!(delay <= upper_bound);
            observed_jitter |= delay != Duration::from_secs(1);
        }

        assert!(observed_jitter);
    }

    #[tokio::test]
    async fn cancel_stops_loop() {
        let cancel = CancellationToken::new();
        let cancel_trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            cancel_trigger.cancel();
        });

        let policy = ReconnectPolicy {
            initial_backoff: Duration::from_secs(60),
            max_backoff: Duration::from_secs(60),
            jitter_pct: 0.0,
            max_attempts: None,
        };
        let reconnect =
            run_with_reconnect("ws://127.0.0.1:1", policy, |_| {}, || {}, |_| {}, cancel);
        let result = tokio::time::timeout(Duration::from_secs(1), reconnect)
            .await
            .expect("reconnect loop did not stop after cancellation");

        assert!(!matches!(result.final_state, ReconnectState::Failed { .. }));
    }

    #[tokio::test]
    async fn max_attempts_returns_failed() {
        let policy = ReconnectPolicy {
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(1),
            jitter_pct: 0.0,
            max_attempts: Some(3),
        };
        let reconnect = run_with_reconnect(
            "ws://127.0.0.1:1",
            policy,
            |_| {},
            || {},
            |_| {},
            CancellationToken::new(),
        );
        let result = tokio::time::timeout(Duration::from_secs(1), reconnect)
            .await
            .expect("reconnect loop did not stop at max attempts");

        assert!(matches!(
            result.final_state,
            ReconnectState::Failed { attempts: 3, .. }
        ));
    }
}
