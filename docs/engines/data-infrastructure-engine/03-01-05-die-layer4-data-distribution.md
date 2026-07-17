# DIE Layer 4 — Data Distribution Layer

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Layer:** 4 of 4
**Output Contract:** Distribution Matrix (high-throughput real-time channels)
**Purpose:** This document specifies the Data Distribution Layer — the tier that routes validated data to downstream consumers via asynchronous broadcast channels, with low-overhead serialization and a subscription registry.

---

## 1. Purpose

The Data Distribution Layer is the DIE's egress. It publishes the validated Market Data Matrix to all interested consumers (chiefly the Market Monitoring Engine and the WebSocket-connected frontend) with minimal latency and zero coupling between producer and consumers.

```
[validated candle / snapshot]
       │
       ▼
┌─────────────────────────────────┐
│   DATA DISTRIBUTION LAYER (L4)   │
│  per-timeframe broadcast channels│
│  subscription registry           │
│  low-overhead serialization      │
└─────────────────────────────────┘
       │           │            │
       ▼           ▼            ▼
   [MME]      [WS clients]  [telemetry logger]
```

---

## 2. Broadcast Channel Model

Distribution uses Tokio `broadcast` channels — a single producer, many independent consumers, each with its own cursor.

| Property | Value |
|----------|-------|
| Topology | One broadcast channel per `(symbol, timeframe)` pipeline. |
| Fan-out | Unlimited subscribers; each receives every message. |
| Lag handling | Slow consumers receive `RecvError::Lagged(n)`; they resynchronize rather than blocking the producer. |
| Payload | Completed and shadow `MarketSnapshot` frames. |

### 2.1 Producer / Consumer Decoupling

The producer (analyzer pipeline) never awaits a specific consumer. A crashed or slow subscriber cannot stall ingestion — it simply misses frames and re-subscribes at the current head. This satisfies the platform's **Decoupled Producer/Consumer** principle.

**Shared-state caveat.** The "zero shared state" framing is aspirational; in practice, state shared across the engine boundary is held in `Arc<…>` containers owned by `RegistryContext` (see [01-06-crate-layout-and-cycles.md §3.2](../../conceptual-foundations/01-06-crate-layout-and-cycles.md)). The actual invariant is *no mutable shared state without synchronisation*: shared containers are read-only after construction, or guarded by Tokio primitives (`RwLock`, `Mutex`, atomic counters). The decoupled-API guarantee is that a slow consumer cannot block the producer; it is not that no state is shared.

> **Target Architecture.** See [01-07 §1](../../conceptual-foundations/01-07-target-architecture-roadmap.md) — "Zero-copy MME distribution". The target design splits distribution into two explicitly different formats: internal distribution (to MME) bypasses JSON serialization, routing raw zero-copy binary memory slices; external distribution (to Frontend / DB) uses the canonical JSON-RPC 2.0 schemas (§4). *Current implementation:* the internal MME path broadcasts cloned `MarketSnapshot` structs; the external path uses JSON-RPC 2.0 as documented in §4.

---

## 3. Subscription Registry

Consumers subscribe by `(symbol, timeframe_secs)`. The L4 layer maintains **two distinct broadcast channels** per `(symbol, timeframe)` pipeline:

1. A `NormalizedCandle` broadcast channel carrying raw OHLCV candles (and `ReconstructionMethod` provenance). This is the DIE L4 transport channel, consumed by the Candle Aggregator for higher-timeframe rollup.
2. A `MarketSnapshot` broadcast channel carrying the full analytical envelope (indicators, alignment, risk, advisory, etc., per [02-07-metrics-matrix.md](../../matrices/02-07-metrics-matrix.md)). This is an MME L1 artifact (not produced by DIE L4), consumed by the MME analyzer, the telemetry logger, and the WebSocket clients.

The two channels are independent: lag in one does not affect the other. Both are Tokio `broadcast` with non-blocking semantics (lagged consumers surface `RecvError::Lagged(n)` but do not stall the producer).

| Consumer | Subscription | Transport |
|----------|-------------|-----------|
| MME analyzer | All configured timeframes for an instance. | `MarketSnapshot` broadcast receiver. |
| Frontend | 4 parallel connections (micro/fast/slow/macro). | WebSocket `/ws?symbol=&timeframe_secs=` → `MarketSnapshot` channel. |
| Telemetry logger | Completed snapshots. | `MarketSnapshot` broadcast receiver → SQLite. |
| Candle aggregator | Base timeframe (micro) closes. | Dedicated `NormalizedCandle` broadcast receiver. |

The WebSocket handler (`server/ws.rs`) resolves the requested `(symbol, timeframe_secs)` to the correct `MarketSnapshot` channel and streams frames to the client.

---

## 4. Serialization

### 4.1 Wire Format

Frames are serialized as **JSON-RPC 2.0 notifications** for the WebSocket transport:

```json
{
  "jsonrpc": "2.0",
  "method": "broadcast.market_snapshot",
  "params": {
    "symbol": "BTC-USDT",
    "timeframe_secs": 60,
    "snapshot": { /* MarketSnapshot */ }
  }
}
```

Notifications carry no `id` (no response expected). See the [API Gateway Contract](../../integration-and-api/06-01-api-gateway-contract.md) for the full protocol.

### 4.2 Low-Overhead Rules

| Rule | Effect |
|------|--------|
| `skip_serializing_if = "Option::is_none"` | Absent optional fields are omitted, shrinking frames. |
| Empty collections omitted | Empty `signals` / null `values` maps dropped. |
| Decimal-as-string | Prices serialize as strings — no float precision loss. |
| Shadow streaming | Live shadow frames (`is_completed = false`) stream at tick cadence; completed frames (`is_completed = true`) on candle close. The platform does not rate-limit shadow frames at the L4 layer — any local throttling is the consumer's responsibility (the WebSocket handler, the MME analyzer, etc.). |

---

## 5. Distribution Guarantees

| Property | Guarantee |
|----------|-----------|
| **Non-blocking** | A slow/failed subscriber never stalls the producer. |
| **At-most-once per cursor** | Each subscriber sees each frame at most once; lag is signalled explicitly. |
| **Ordering** | Frames within a channel are delivered in production order. |
| **Immutability** | Once broadcast, a completed snapshot is never mutated (see [Metrics Matrix §7](../../matrices/02-07-metrics-matrix.md)). |

### 5.1 Operational Acceptance Criteria

The L4 (Data Distribution Layer) layer meets these criteria when run with default configuration under nominal load:

| ID | Criterion | Verification |
|----|-----------|--------------|
| `AC-L4-1` | Per-frame serialization p95 < 1 ms under nominal load (50 indicators, 4 sub-matrix envelopes). | `crates/market-analyzer/tests/perf_serialize.rs` (Phase 1) |
| `AC-L4-2` | End-to-end observation loop (raw frame → completed snapshot on the WS broadcast) p95 < 25 ms. (Mirrors `AC-DIE-3`.) | `crates/api-gateway/tests/observation_loop.rs` (Phase 1) |
| `AC-L4-3` | Broadcast fan-out is `O(subscribers)` and non-blocking; a 1 s sleep on one subscriber does not delay other subscribers or the producer. | `crates/market-analyzer/tests/broadcast_fanout.rs` (Phase 1) |
| `AC-L4-4` | Lagged consumer receives `RecvError::Lagged(n)` within 1 frame of falling behind; the consumer can resubscribe at the current head. | `crates/market-analyzer/tests/broadcast_lag.rs` (existing) |
| `AC-L4-5` | Two broadcast channels per `(symbol, timeframe)` (`NormalizedCandle` + `MarketSnapshot`) operate independently; lag in one does not affect the other. | `crates/market-analyzer/tests/dual_channel.rs` (Phase 1) |

---

## 6. Performance Targets

| Metric | Target |
|--------|--------|
| Serialization per frame | < 1 ms |
| Broadcast dispatch | O(subscribers), non-blocking |
| End-to-end observation loop | < 25 ms |

---

## 7. Cross-References

- [DIE Layer 3 — Data Quality](03-01-04-die-layer3-data-quality.md) — Input.
- [MME Layer 1 — Metrics](../market-monitoring-engine/03-02-02-mme-layer1-metrics.md) — Primary consumer.
- [API Gateway Contract](../../integration-and-api/06-01-api-gateway-contract.md) — WebSocket protocol.
- [UI Overview](../../ui-ux/07-01-ui-overview-spec.md) — Frontend subscription model.
