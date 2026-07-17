# Connection Quality

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Implemented
**Spec version:** 1.0

## Purpose

Aggregates raw WebSocket events (connect, disconnect, reconnect, heartbeat) into a composite quality score tracked over rolling time windows. Exposes the data via REST API for the dashboard and stores historical samples in SQLite for trend analysis.

## Three Rolling Windows

| Window | Duration | Use case |
|--------|----------|----------|
| `ONE_HOUR` | 3600 s | Real-time operational awareness |
| `SIX_HOUR` | 6 × 3600 s | Trading session quality review |
| `TWENTY_FOUR_HOUR` | 24 × 3600 s | Daily SLO / uptime reporting |

All three windows are computed **within a single 60-second tick** and persisted as three independent rows per tick (one per window, sharing the same `timestamp_ms`). The dashboard switches between them via tabs in the `ConnectionQualityPanel` component; each tab switch triggers a fresh REST request for the chosen window. The REST API returns a single `ConnectionQualityReport` per request, not an aggregate of all three.

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
  50 × (uptime_pct / 100)
  + 30 × (1 − min(disconnect_count / 10, 1))
  + 20 × (1 − min(avg_reconnect_ms / 5000, 1))
  − 5 × min(total_data_loss_s / 600, 1)
  − 5 × min(reconstructed_candles_pct / 100, 1)
)
```

Interpretation:

| Term | Weight | Saturates at | Notes |
|------|--------|--------------|-------|
| `uptime_pct` | 0..50 points | 100% uptime | The dominant signal. |
| `disconnect_count` | 0..30 points | 10 disconnects | Penalised linearly up to 10, then floor. |
| `avg_reconnect_ms` | 0..20 points | 5000 ms | Saturates at 5s reconnect time. |
| `total_data_loss_secs` | 0..5 points subtracted | 600 s of data loss | Reflects sustained outage beyond what `uptime_pct` alone captures. |
| `reconstructed_candles` | 0..5 points subtracted | 100 reconstructed candles | Penalises reconstruction-heavy windows (a proxy for venue instability). |

**Worked example** (uptime=95, dc=8, rc_ms=2000, data_loss=300s, reconstructed=50%):
```
50 × (95 / 100)     = 47.50
30 × (1 − min(8 / 10, 1)) = 6.00
20 × (1 − min(2000 / 5000, 1)) = 12.00
− 5 × min(300 / 600, 1) = −5 × 0.5 = −2.50
− 5 × min(50 / 100, 1)  = −5 × 0.5 = −2.50

total = 47.5 + 6 + 12 − 2.5 − 2.5 = 60.5
```

A perfect session (100% uptime, 0 disconnects, 0 ms reconnect, 0 data loss, 0 reconstructed candles) scores 100.

**Saturation rationale.** The 5 s reconnect ceiling and 10-disconnect ceiling match the [08-03-connection-resilience.md §State Transitions](../operations-and-compliance/08-03-connection-resilience.md) "anything worse than this is the supervisor's problem, not the tracker's" boundary. The 600 s data-loss ceiling matches the 5-minute operational SLO in [`08-01-user-manual.md §9`](../operations-and-compliance/08-01-user-manual.md). The 100-reconstructed-candle ceiling reflects one full micro-tier recovery window.

## Event Sources

| Event | Source | Effect |
|-------|--------|--------|
| `Connected` | `Resilience::ReconnectState::Connected` | Reset connection-time accumulator |
| `Disconnected` | `Resilience::ReconnectState::Reconnecting` | Start data-loss timer |
| `ReconnectCompleted` | `Resilience::on_resume` callback | Stop data-loss timer, log duration |
| `Heartbeat` | WS adapter periodic tick | Detect silent drops |
| `ReconstructedCandle` | `reconstruction.rs` | Increment counter |

## Persistence

Samples are written to the `connection_quality_samples` SQLite table every 60 seconds by a background task. The table is **per-`(pair_key, timeframe_secs)`** (one Market Instance × timeframe pipeline owns one series); a process-wide aggregate is not retained.

```sql
CREATE TABLE IF NOT EXISTS connection_quality_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key TEXT NOT NULL,
    timeframe_secs INTEGER NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    window TEXT NOT NULL,
    uptime_pct REAL NOT NULL,
    disconnect_count INTEGER NOT NULL,
    avg_reconnect_ms REAL NOT NULL,
    total_data_loss_secs INTEGER NOT NULL,
    reconstructed_candles INTEGER NOT NULL,
    score REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_cq_pair_tf_window_time
    ON connection_quality_samples(pair_key, timeframe_secs, window, timestamp_ms DESC);
```

The `pair_key` and `timeframe_secs` columns scope every sample to its owning `(instance, pipeline)`. v4.0 introduced this per-instance shape to back the per-instance dashboard panel; the earlier process-wide eight-column form is no longer used. See [`06-02-database-schema-spec.md §3.9`](../integration-and-api/06-02-database-schema-spec.md) for the authoritative DDL.

## REST API

```
GET /api/connection-quality?instance_id=…&timeframe_secs=…&window=one_hour|six_hour|twenty_four_hour

The `instance_id` and `timeframe_secs` query parameters are **required** as of v4.0. Connection-quality is reported per Market Instance × timeframe (one WebSocket connection per `TimeframePipeline`); the API does not return a process-wide aggregate. See `06-01-api-gateway-contract.md §2.3`.
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

`ConnectionQualityPanel.svelte` (with companion `.module.css` and `.test.ts`) is wired into the dashboard under the **Data Infrastructure → Overview → Connectivity** sub-tab (see [`07-02-ui-dashboard-layout.md §5.3`](../ui-ux/07-02-ui-dashboard-layout.md)). It:
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
