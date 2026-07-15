# DIE Layer 1 — Raw Data Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Layer:** 1 of 4
**Output Contract:** Raw Data Matrix (`NormalizedEvent` stream)
**Purpose:** This document specifies the Raw Data Layer — the lowest-level connection tier responsible for establishing and maintaining WebSocket and REST connections to external venues, managing heartbeats and rate limits, and standardizing raw network frames into a uniform event envelope.

---

## 1. Purpose

The Raw Data Layer converts venue-specific network chaos into a single, uniform, venue-agnostic event stream. It is the only tier that touches raw sockets, exchange-specific JSON schemas, and rate-limit headers.

```
[Exchange WS/REST]
       │  raw frames
       ▼
┌───────────────────────────┐
│   RAW DATA LAYER (L1)      │
│  connection pool           │
│  heartbeat / ping-pong     │
│  frame parse & normalize   │
│  rate-limit accounting     │
└───────────────────────────┘
       │  NormalizedEvent
       ▼
[Market Data Layer (L2)]
```

---

## 2. Output Contract: NormalizedEvent

The Raw Data Matrix is the `NormalizedEvent` enum (`crates/shared/src/normalized/mod.rs`):

| Variant | Fields | Description |
|---------|--------|-------------|
| `Trade(NormalizedTrade)` | exchange, symbol, price, size, side, timestamp_ms, trade_id | A single executed trade. |
| `OrderBook(NormalizedOrderBook)` | exchange, symbol, bids[], asks[], timestamp_ms | L2 depth snapshot. |
| `AssetContext(AssetContext)` | symbol, prev_day_px | Reference context. |
| `OpenInterest(OpenInterestEvent)` | symbol, oi | Derivatives open interest. |
| `FundingRate(FundingRateEvent)` | symbol, rate | Perpetual funding rate. |
| `Status { .. }` | exchange, status, message | Connection lifecycle. |

All numeric fields use `Decimal` to preserve exchange precision. `timestamp_ms` is Unix epoch in milliseconds.

> **Target Architecture (Not Yet Implemented).** The target hot-path ingress parses raw WebSocket JSON directly into **pre-allocated, flat arena memory buffers**, reclaiming memory via object pools to avoid heap fragmentation and allocator stalls, and streams the Raw Data Matrix downstream as a **zero-copy `f64` primitive array** rather than an enum of `Decimal`-bearing structs. *Current implementation:* raw frames are parsed into the `NormalizedEvent` enum (numeric fields as `Decimal`) and delivered over a bounded `mpsc` channel.

---

## 3. Connection Pool

Each registered adapter runs in an independent Tokio task spawned by the `MarketDataOrchestrator`. The pool has these properties:

| Property | Value |
|----------|-------|
| Isolation | One task per venue; a crash in one never affects another. |
| Channel | Bounded `mpsc` channel, capacity 10,000 `NormalizedEvent`s. |
| Backpressure | When the channel is full, the adapter awaits; no events are dropped silently. |
| Symbol scoping | Each adapter subscribes only to its assigned normalized symbols (via `SymbolMapper`). |

---

## 4. Heartbeats & Keep-Alive

Each venue adapter maintains liveness according to its protocol:

| Venue | Keep-Alive Mechanism |
|-------|----------------------|
| Hyperliquid | Periodic ping frames; subscription acknowledgements monitored. |
| Bitget | Client `ping` → server `pong` on a fixed interval; missed pongs trigger reconnect. |

A stalled stream (no frames within the venue's expected cadence) is treated as a disconnect and surfaced as a `Status { status: Disconnected }` event, triggering the supervisor's backoff loop.

---

## 5. REST Fallback

When live streaming cannot supply required history (cold start, gap recovery), the Raw Data Layer falls back to REST:

| Venue | REST Module | Primary Function |
|-------|-------------|------------------|
| Hyperliquid | `adapters/hyperliquid_rest.rs` | `fetch_historical_candles()` |
| Bitget | `adapters/bitget_rest.rs` | `fetch_historical_candles()`, `symbol_exists()` |

REST is used for **bootstrap warm-up** and **gap-filling** (see [Layer 3](03-01-04-die-layer3-data-quality.md)), never as the primary real-time feed.

---

## 6. Rate-Limiting

The Raw Data Layer respects venue rate limits through:

- **Request pacing:** REST historical fetches are chunked and spaced to stay within venue quotas.
- **Subscription batching:** WebSocket subscriptions for multiple symbols are batched into minimal frames.
- **Backoff coupling:** The supervisor's exponential backoff (1 s → 30 s, ±20 % jitter — see [08-03-connection-resilience.md §3](../operations-and-compliance/08-03-connection-resilience.md)) doubles as a rate-limit relief valve after `429`/rejection responses.

---

## 7. Reconnection Protocol

Reconnection is owned by the `MarketDataOrchestrator` supervisor loop (see [Overview §4](03-01-01-die-overview-spec.md)):

1. Adapter `start()` returns (clean exit or error).
2. Supervisor emits a `Status` event describing the outcome.
3. On error: increment consecutive-failure counter (reset if > 300 s since last failure).
4. If failures ≥ 5 → permanent disable.
5. Otherwise sleep `retry_cooldown`, then loop; `retry_cooldown = min(retry_cooldown × 2, 60)`.

---

## 8. Guarantees

| Property | Guarantee |
|----------|-----------|
| **Venue-agnostic output** | Downstream layers never see exchange-specific schemas. |
| **Precision preservation** | All prices/sizes carried as `Decimal`. |
| **No silent loss** | Bounded channel applies backpressure rather than dropping. |
| **Self-healing** | Automatic reconnect with capped exponential backoff. |

---

## 9. Cross-References

- [DIE Overview](03-01-01-die-overview-spec.md)
- [DIE Layer 2 — Market Data](03-01-03-die-layer2-market-data.md) — Direct consumer.
- [Systemic Data Flow — Sequence A](../../conceptual-foundations/01-03-systemic-data-flow.md)
