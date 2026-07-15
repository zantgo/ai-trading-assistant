# Connection Resilience

**Status:** Implemented
**Module:** `crates/engine/src/adapters/resilience.rs`
**Spec version:** 1.0

## Purpose

Defines the platform's WebSocket reconnection policy. Ensures the engine survives network crashes, exchange rate limits, and transient transport failures without manual intervention, while providing visibility into disconnections for monitoring and analysis.

## Public API

```rust
pub struct ReconnectPolicy {
    pub initial_backoff: Duration,    // default: 1s
    pub max_backoff: Duration,        // default: 30s
    pub jitter_pct: f64,              // default: 0.2 (±20%)
    pub max_attempts: Option<u32>,    // default: None (infinite) — per-cycle retry cap within the adapter's reconnect loop. The *supervisor* (engine supervisor, separate component) disables the adapter after 5 failed *cycles* (each cycle is a sequence of `max_attempts` retries). The two layers do not conflict: the adapter retries indefinitely within each cycle; the supervisor terminates the adapter after 5 cycles. See [08-01-user-manual.md §8](../operations-and-compliance/08-01-user-manual.md) for the operator-facing behavior ("permanent disable after 5 consecutive failures").
}

pub enum ReconnectState {
    Connected { since: Instant },
    Reconnecting { attempt: u32, next_retry: Instant },
    Failed { last_error: String, attempts: u32 },
}

pub async fn run_with_reconnect<F, R, S>(
    url: &str,
    policy: ReconnectPolicy,
    on_message: F,
    on_resume: R,
    state_callback: S,
    cancel: CancellationToken,
) -> ReconnectResult
```

## Backoff Formula

```
delay_n = min(initial × 2^n, max_backoff) × (1 + uniform(-jitter_pct, +jitter_pct))
```

Sequence with default policy (1s initial, 30s max, ±20% jitter):
- Attempt 1 → ~1s
- Attempt 2 → ~2s
- Attempt 3 → ~4s
- Attempt 4 → ~8s
- Attempt 5 → ~16s
- Attempt 6+ → ~30s (capped)

## State Transitions

```
                    ┌─────────────────┐
                    │   Connected     │ ◄─── on connect / on resume
                    └────────┬────────┘
                             │ transport error
                             ▼
                    ┌─────────────────┐
            ┌──────│  Reconnecting   │──────┐
            │      └─────────────────┘      │
   backoff   │            ▲                 │ max_attempts reached
   elapsed   │            │ sleep           ▼
            │      ┌─────────────────┐
            └─────►│   Reconnecting  │
                   │   (attempt n+1) │ ─► (loop)
                   └─────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │     Failed      │ (only on max_attempts or cancel)
                    └─────────────────┘
```

## Resume Callback

The `on_resume` callback fires **once per reconnect**, **before** any new messages are forwarded to `on_message`. This gives downstream consumers a hook to:
- Re-subscribe to channels lost during disconnect
- Reset rolling buffers that may have stale data
- Synchronize with the exchange's current state

## Logging

Every state transition is logged:
- `info!` on first connect and on each successful resume
- `warn!` on each retry attempt (with attempt count + delay)
- `error!` on max-attempts exhaustion

## Integration

Used by `crates/engine/src/adapters/hyperliquid.rs` and `crates/engine/src/adapters/bitget.rs` to wrap their WebSocket loops. The same policy is applied to both exchanges.

## Testing

- 4 unit tests in `resilience.rs`:
  - `backoff_progression_is_exponential`
  - `jitter_within_bounds`
  - `cancel_stops_loop`
  - `max_attempts_returns_failed`

## Cross-References

- [Connection Quality](08-05-connection-quality.md) — the metrics derived from reconnect events
- [Candle Reconstruction](08-04-candle-reconstruction.md) — the consumer of reconnect events for gap-filling
- [Clock Monitor](08-06-clock-monitor.md) — unrelated to WS but co-resident in the resilience family
