# Connection Quality

**Status:** Implemented
**Backend:** `crates/engine/src/connection_quality.rs`
**Frontend:** `crates/frontend/src/components/ConnectionQualityPanel.svelte`
**Spec version:** 1.0

## Purpose

Aggregates raw WebSocket events (connect, disconnect, reconnect, heartbeat) into a composite quality score tracked over rolling time windows. Exposes the data via REST API for the dashboard and stores historical samples in SQLite for trend analysis.

## Three Rolling Windows

| Window | Duration | Use case |
|--------|----------|----------|
| `ONE_HOUR` | 3600 s | Real-time operational awareness |
| `SIX_HOUR` | 6 × 3600 s | Trading session quality review |
| `TWENTY_FOUR_HOUR` | 24 × 3600 s | Daily SLO / uptime reporting |

All three windows are computed and persisted in parallel. The dashboard can switch between them via tabs in the `ConnectionQualityPanel` component.

## Data Model

```rust
pub struct ConnectionQualityReport {
    pub window: QualityWindow,         // ONE_HOUR | SIX_HOUR | TWENTY_FOUR_HOUR
    pub window_start_ms: u64,
    pub window_end_ms: u64,
    pub uptime_pct: f64,                // 0..100
    pub disconnect_count: u32,
    pub avg_reconnect_ms: f64,
    pub total_data_loss_secs: u64,
    pub reconstructed_candles: u32,
    pub score: f64,                    // 0..100 composite
}
```

## Composite Score Formula

```
score = clamp(0..100,
    0.5  × uptime_pct
  + 30   × (1 − min(disconnect_count / 10, 1))
  + 20   × (1 − min(avg_reconnect_ms / 5000, 1))
)
```

Interpretation:
- **uptime_pct** contributes 0..50 points (the dominant signal)
- **disconnect_count** contributes 0..30 points (penalized linearly up to 10 disconnects)
- **avg_reconnect_ms** contributes 0..20 points (saturates at 5s reconnect time)

A perfect session (100% uptime, 0 disconnects, 0ms reconnect) scores 100. A session with 5% downtime, 8 disconnects, and 2s avg reconnect scores ~67.5.

## Event Sources

| Event | Source | Effect |
|-------|--------|--------|
| `Connected` | `Resilience::ReconnectState::Connected` | Reset connection-time accumulator |
| `Disconnected` | `Resilience::ReconnectState::Reconnecting` | Start data-loss timer |
| `ReconnectCompleted` | `Resilience::on_resume` callback | Stop data-loss timer, log duration |
| `Heartbeat` | WS adapter periodic tick | Detect silent drops |
| `ReconstructedCandle` | `reconstruction.rs` | Increment counter |

## Persistence

Samples are written to the `connection_quality_samples` SQLite table every 60 seconds by a background task:

```sql
CREATE TABLE IF NOT EXISTS connection_quality_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_ms INTEGER NOT NULL,
    window TEXT NOT NULL,
    uptime_pct REAL NOT NULL,
    disconnect_count INTEGER NOT NULL,
    avg_reconnect_ms REAL NOT NULL,
    total_data_loss_secs INTEGER NOT NULL,
    reconstructed_candles INTEGER NOT NULL,
    score REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_cq_window_time ON connection_quality_samples(window, timestamp_ms);
```

## REST API

```
GET /api/connection-quality?window=one_hour|six_hour|twenty_four_hour
```

Default window: `one_hour`. Response: `ConnectionQualityReport` JSON.

Example:
```json
{
  "window": "ONE_HOUR",
  "window_start_ms": 1784131217285,
  "window_end_ms": 1784134817285,
  "uptime_pct": 100.0,
  "disconnect_count": 0,
  "avg_reconnect_ms": 0.0,
  "total_data_loss_secs": 0,
  "reconstructed_candles": 0,
  "score": 100.0
}
```

## Frontend Panel

`ConnectionQualityPanel.svelte` (with companion `.module.css` and `.test.ts`) is wired into the dashboard between the Risks and Analysis workspace tabs. It:
- Polls `/api/connection-quality` every 30 seconds
- Switches between 1h / 6h / 24h tabs
- Color-codes the score (≥90 green, ≥75 lime, ≥50 amber, <50 red)
- Color-codes uptime (≥99 green, ≥95 lime, <95 red)
- Shows: score, uptime, disconnect count, avg reconnect ms, total data loss, reconstructed candle count

## Testing

Backend (`connection_quality.rs`):
- 7 unit tests covering: connect/disconnect recording, uptime computation, disconnect counting, avg reconnect, score monotonicity with disconnects, reconstructed candle counting, window filtering of old events.

Frontend (`ConnectionQualityPanel.test.ts`):
- 5 tests covering: loading state, post-fetch render, tab switching, error state, score-class application.

## Cross-References

- [Connection Resilience](08-03-connection-resilience.md) — source of `ReconnectState` events
- [Candle Reconstruction](08-04-candle-reconstruction.md) — source of `reconstructed_candles` count
- [AGENTS.md §Runtime](../../AGENTS.md) — operational overview
