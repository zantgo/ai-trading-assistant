# DIE Layer 4 — Data Distribution Layer

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
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

The producer (analyzer pipeline) never awaits a specific consumer. A crashed or slow subscriber cannot stall ingestion — it simply misses frames and re-subscribes at the current head. This satisfies the platform's **Zero Shared State** and **Decoupled API** principles.

> **Target Architecture (Not Yet Implemented).** The target design splits distribution into two explicitly different formats:
>
> - **Internal distribution (to MME):** bypasses JSON serialization entirely, routing raw, zero-copy binary memory slices directly over Tokio broadcast channels into MME Layer 1.
> - **External distribution (to Frontend / DB):** serializes completed and shadow metrics into the canonical JSON-RPC 2.0 schemas (§4).
>
> *Current implementation:* the internal MME path broadcasts **cloned `MarketSnapshot` structs** (not zero-copy binary slices) over Tokio `broadcast` channels; the external path already uses JSON-RPC 2.0 as documented in §4.

---

## 3. Subscription Registry

Consumers subscribe by `(symbol, timeframe_secs)`:

| Consumer | Subscription | Transport |
|----------|-------------|-----------|
| MME analyzer | All configured timeframes for an instance. | In-process broadcast receiver. |
| Frontend | 4 parallel connections (micro/fast/slow/macro). | WebSocket `/ws?symbol=&timeframe_secs=`. |
| Telemetry logger | Completed snapshots. | In-process receiver → SQLite. |
| Candle aggregator | 1-minute closes. | Dedicated `broadcast::Receiver<NormalizedCandle>`. |

The WebSocket handler (`server/ws.rs`) resolves the requested symbol/timeframe to the correct broadcast channel and streams frames to the client.

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
| Shadow throttling | Live shadow frames stream at tick cadence; completed frames on candle close. |

---

## 5. Distribution Guarantees

| Property | Guarantee |
|----------|-----------|
| **Non-blocking** | A slow/failed subscriber never stalls the producer. |
| **At-most-once per cursor** | Each subscriber sees each frame at most once; lag is signalled explicitly. |
| **Ordering** | Frames within a channel are delivered in production order. |
| **Immutability** | Once broadcast, a completed snapshot is never mutated (see [Metrics Matrix §7](../../matrices/02-07-metrics-matrix.md)). |

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
