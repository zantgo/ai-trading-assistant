# Distribution Matrix Specification

**Version:** 2.0
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Producing Layer:** Layer 4 — Data Distribution Layer
**Purpose:** This document defines the physical schema of the **Distribution Matrix** — the high-throughput, pub/sub event routing system that delivers validated market data to all downstream engine consumers.

---

## 1. Conceptual Definition

The Distribution Layer is the DIE's **output boundary**. It receives validated candles from the Data Quality Layer and routes them to all subscribed downstream engines (MME, real-time dashboards, historical storage) via bounded asynchronous channels.

```
[Data Quality Matrix] ──► DISTRIBUTION LAYER (L4) ──► [MME] · [DB Persistence] · [WebSocket to Frontend]
```

---

## 2. Physical Schema

The Distribution Matrix itself is not a single data structure but a **multiplexed channel topology**:

| Component | Description |
|-----------|-------------|
| **Channel per symbol** | Each symbol's validated candles are published on a dedicated bounded MPSC channel. |
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
| Observation loop (Raw → Distribution) | < 25 ms |
| Event channel capacity | 10,000 buffered events |
| Reconnect backoff | 2 s → 60 s (exponential) |
| Permanent disable threshold | 5 consecutive failures |

---

## 4. Distribution Contract

Each distributed frame is an envelope containing the validated candle and its quality metadata:

```json
{
  "exchange": "Hyperliquid",
  "symbol": "BTC-USDT",
  "timeframe_secs": 60,
  "timestamp": 1752192000000,
  "candle": {
    "open": "63890.0",
    "high": "64120.0",
    "low": "63850.0",
    "close": "64012.5",
    "volume": "182.4"
  },
  "quality": {
    "is_gap_filled": false,
    "quality_score": 98.0,
    "sequence_integrity": "VALID"
  }
}
```

---

## 5. Backpressure & Fault Tolerance

- **Bounded channels** prevent unbounded memory growth on consumer slowdown.
- **Broadcast lag signalling**: if a subscriber's channel approaches capacity, an internal backpressure warning is raised to the orchestrator (a structured event visible in `/api/system/status`).
- **Dropped frame policy**: frames are never silently dropped; broadcast lag is logged and surfaced in observability.

---

## 6. Cross-References

- [DIE Overview](../engines/data-infrastructure-engine/03-01-01-die-overview-spec.md) — Engine boundaries and performance targets.
- [DIE Layer 4 — Data Distribution](../engines/data-infrastructure-engine/03-01-05-die-layer4-data-distribution.md) — Producing-layer specification.
- [Data Quality Matrix](02-03-data-quality-matrix.md) — Upstream input.
- [Metrics Matrix](02-07-metrics-matrix.md) — Primary consumer (MME).
- [API Gateway Contract](../integration-and-api/06-01-api-gateway-contract.md) — WebSocket distribution to frontend.
