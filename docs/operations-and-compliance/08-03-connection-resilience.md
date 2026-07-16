# Connection Resilience

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Implemented

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

Jitter is applied **before** capping so the effective delay range at attempt `n` is `[delay_n × (1 − jitter_pct), min(delay_n × (1 + jitter_pct), max_backoff)]`. The hard maximum actual delay is `max_backoff × (1 + jitter_pct)` only briefly during the geometric ramp-up; once `delay_n ≥ max_backoff`, the cap dominates and the actual delay range is `[max_backoff × 0.8, max_backoff × 1.2]` regardless of `n`:

```
base_delay_n = min(initial × 2^n, max_backoff)
delay_n      = base_delay_n × (1 + uniform(-jitter_pct, +jitter_pct))
             = min(base_delay_n × (1 + jitter_pct), max_backoff × (1 + jitter_pct))
```

Sequence with default policy (1s initial, 30s max, ±20% jitter):
- Attempt 1 → ~1s (range `[0.8s, 1.2s]`)
- Attempt 2 → ~2s (range `[1.6s, 2.4s]`)
- Attempt 3 → ~4s (range `[3.2s, 4.8s]`)
- Attempt 4 → ~8s (range `[6.4s, 9.6s]`)
- Attempt 5 → ~16s (range `[12.8s, 19.2s]`)
- Attempt 6 → cap range `[24s, 30s]` (jitter bounded by `max_backoff`)
- Attempt 7+ → `[24s, 30s]`

## Retry Budgets

The platform has three distinct retry budgets that operate independently. They are not interchangeable; conflating them produces either silent stalls (when `max_attempts = None` is treated as a fixed cap) or premature abandonments (when the supervisor's 5-cycle disable is treated as `max_attempts`):

| Layer | Scope | Default | Effect on exhaustion |
|---|---|---|---|
| Adapter reconnect loop | One WebSocket cycle (sequence of `max_attempts` retries against the same exchange) | `max_attempts: None` (infinite) | The adapter keeps retrying indefinitely within a single cycle. The supervisor (next layer) terminates the cycle. |
| Engine supervisor | Number of cycles before permanent adapter disable | 5 cycles | After 5 failed cycles the adapter is permanently disabled for that pair; operator must restart or re-enable manually. See [08-01-user-manual.md §8](../operations-and-compliance/08-01-user-manual.md) for the operator-facing behaviour ("permanent disable after 5 consecutive failures"). |
| REST client retry budget | REST endpoint from `crates/network-adapters/src/adapters/*_rest.rs (Hyperliquid: hyperliquid_rest.rs; Bitget: bitget_rest.rs)` and the Svelte frontend wrapper | 30 attempts | The REST client retries up to 30 times before surfacing the failure to the caller. Independent of the adapter or supervisor budgets. |

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

Used by `crates/network-adapters/src/adapters/hyperliquid.rs` and `crates/network-adapters/src/adapters/bitget.rs` to wrap their WebSocket loops. The same policy is applied to both exchanges.

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
