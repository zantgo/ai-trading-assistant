# Distribution Matrix Specification

**Version:** 6.10 (2026-08-16) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Producing Layer:** Layer 4 — Data Distribution Layer
**Purpose:** This document defines the physical schema of the **Distribution Matrix** — the transport contract for validated `NormalizedCandle` frames published by DIE L4 to the Candle Aggregator for higher-timeframe rollup.

The analytical `MarketSnapshot` broadcast channel (which carries indicators, matrices, and telemetry to the MME, UI, and DB) is an **MME L1** artifact. Its channel specification, subscriber table, and wire format are documented at `03-02-02-mme-layer1-metrics.md §8`.

---

## 1. Conceptual Definition

The Distribution Layer is the DIE's **output boundary**. It receives validated candles from the Data Quality Layer and publishes them as `NormalizedCandle` frames over a per-`(symbol, timeframe)` broadcast channel consumed exclusively by the Candle Aggregator for higher-timeframe rollup.

```
[CandleQualityEnvelope] ──► DISTRIBUTION LAYER (L4) ──► [Candle Aggregator] ──► higher-TF rollup
```

---

## 2. Physical Schema

The Distribution Matrix itself is not a single data structure but a **multiplexed channel topology** for DIE's raw OHLCV transport:

| Component | Description |
|-----------|-------------|
| **Channel per `(symbol, timeframe)` pipeline** | Each `(symbol, timeframe_secs)` combination owns a dedicated `NormalizedCandle` broadcast channel. A 4-tier ladder with one symbol thus yields four channels. |
| **Candle Aggregator subscriber** | The Candle Aggregator subscribes to the micro (base) timeframe `NormalizedCandle` channel for higher-timeframe rollup. |
| **Channel capacity** | 10,000 buffered events per channel. |

The `MarketSnapshot` broadcast channel (consumed by MME L2–L7, the frontend WebSocket, and the telemetry logger) is produced by MME L1 and documented at `03-02-02-mme-layer1-metrics.md §8`.

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

Each DIE L4 `NormalizedCandle` frame carries the OHLCV candle with its `ReconstructionMethod` provenance flag (see [02-06-market-data-matrix.md](02-06-market-data-matrix.md)). Transport is by in-memory `Arc<NormalizedCandle>` broadcast; no independent JSON serialization is performed at this layer. The candle is serialized downstream by MME L1 as part of the `MarketSnapshot` envelope (see `03-02-02 §8.3`).

The `CandleDistributionFrame` (the wire envelope carrying the validated candle and its quality metadata to external consumers) is projected from the `NormalizedCandle` and `CandleQualityEnvelope` at MME L1 serialization time, not at DIE L4:

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
- [MME Layer 1 §8 (MarketSnapshot channel)](../engines/market-monitoring-engine/03-02-02-mme-layer1-metrics.md) — The analytical broadcast channel consumed by MME L2–L7, UI, and DB.
- [Market Data Matrix](02-06-market-data-matrix.md) — The `NormalizedCandle` schema contract.
