# Distribution Matrix Specification

**Version:** 6.4.1 (2026-07-18) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Producing Layer:** Layer 4 — Data Distribution Layer
**Purpose:** This document defines the physical schema of the **Distribution Matrix** — the high-throughput, pub/sub event routing system that delivers validated market data to all downstream engine consumers.

---

## 1. Conceptual Definition

The Distribution Layer is the DIE's **output boundary**. It receives validated candles from the Data Quality Layer and routes them to all subscribed downstream engines (MME, real-time dashboards, historical storage) via bounded asynchronous channels.

```
[CandleQualityEnvelope] ──► DISTRIBUTION LAYER (L4) ──► [MME] · [DB Persistence] · [WebSocket to Frontend]
```

---

## 2. Physical Schema

The Distribution Matrix itself is not a single data structure but a **multiplexed channel topology**:

| Component | Description |
|-----------|-------------|
| **Channel per `(symbol, timeframe)` pipeline** | Each `(symbol, timeframe_secs)` combination owns a dedicated broadcast channel. A 4-tier ladder with one symbol thus yields four channels; a workspace of `N` symbols yields `4 × N`. |
| **MME subscriber** | The Market Monitoring Engine subscribes to the candle channel for indicator computation. |
| **DB subscriber** | The telemetry logger subscribes to persist candles to `market_snapshots`. |
| **WS subscriber** | The WebSocket broadcaster subscribes to emit real-time updates to the frontend. |
| **Channel capacity** | 10,000 buffered events per channel. |

---

## 3. Performance Targets

| Metric | Target |
|--------|--------|
| Raw frame → `NormalizedEvent` | < 1 ms |
| Trade → live candle update | < 2 ms |
| Observation loop (Raw → Market Data Matrix) | < 25 ms |
| Event channel capacity | 10,000 buffered events |
| Reconnect backoff | 1 s → 30 s (exponential, ±20 % jitter) |
| Permanent disable threshold | 5 consecutive failures |

The observation-loop latency budget decomposes as: **DIE contribution ≤ 10 ms; MME contribution ≤ 15 ms**.

---

## 4. Distribution Contract

Each distributed frame is a **`CandleDistributionFrame`** — the wire envelope containing the validated candle and its quality metadata. The frame is a documented **projection** of the upstream products, not a verbatim copy:

| Frame field | Projected from |
|-------------|----------------|
| `exchange`, `symbol`, `timeframe_secs`, `timestamp` | `NormalizedCandle` ([02-06-market-data-matrix.md](02-06-market-data-matrix.md)) — hoisted out of the candle to the envelope top level. |
| `candle.open` / `high` / `low` / `close` / `volume` | `NormalizedCandle` (02-06). |
| `quality.is_gap_filled`, `quality.quality_score`, `quality.sequence_integrity` | `CandleQualityEnvelope` ([02-03-data-quality-matrix.md](02-03-data-quality-matrix.md)) — the 3-field `CandleQualitySummary` subset. |

`NormalizedCandle.trades_count` and the four remaining `CandleQualityEnvelope` quality fields (`is_stale`, `spike_detected`, `gap_since_last`, `validated_at`) are **intentionally excluded** from the wire frame.

```json
{
  "exchange": "Hyperliquid",
  "symbol": "BTC-USDT",
  "timeframe_secs": 60,
  "timestamp": 1752192000000,
  "candle": {
    "open": 63890.0,
    "high": 64120.0,
    "low": 63850.0,
    "close": 64012.5,
    "volume": 182.4
  },
  "quality": {
    "is_gap_filled": false,
    "quality_score": 100.0,
    "sequence_integrity": "VALID"
  }
}
```

---

## 5. Backpressure & Fault Tolerance

- **Bounded channels** prevent unbounded memory growth on consumer slowdown.
- **Broadcast lag signalling**: if a subscriber's channel approaches capacity, an internal backpressure warning is raised to the orchestrator (logged via `tracing::warn` and surfaced through `/api/system/observability`).
- **Dropped frame policy**: frames are never silently dropped; broadcast lag is logged and surfaced in observability.

---

## 6. Cross-References

- [DIE Overview](../engines/data-infrastructure-engine/03-01-01-die-overview-spec.md) — Engine boundaries and performance targets.
- [DIE Layer 4 — Data Distribution](../engines/data-infrastructure-engine/03-01-05-die-layer4-data-distribution.md) — Producing-layer specification.
- [Candle Quality Envelope](02-03-data-quality-matrix.md) — Upstream input.
- [Metrics Matrix](02-07-metrics-matrix.md) — Primary consumer (MME).
- [API Gateway Contract](../integration-and-api/06-01-api-gateway-contract.md) — WebSocket distribution to frontend.
