# Data Infrastructure Engine — Overview Specification

**Version:** 2.0
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Purpose:** This document specifies the boundaries, responsibilities, layer structure, exchange adapters, performance targets, and connection-monitoring model of the Data Infrastructure Engine — the first engine in the platform's unidirectional cascade. The DIE ingests, normalizes, validates, and distributes exchange telemetry.

---

## 1. Mission & Boundaries

The DIE is the **sole ingress point** for external market data. It owns everything from raw network frames to the clean, uniform Market Data Matrix that the Market Monitoring Engine consumes. It performs **no market interpretation** — it does not compute indicators, bias, or risk.

> **Target Architecture (Not Yet Implemented).** The DIE is intended to be a **strict Data-Oriented Design (DOD)** engine sustaining continuous ingestion of ≥ 50,000 events/sec, processing data in hardware-native `f64` primitive slices rather than heap-allocated structures. *Current implementation:* events flow as `NormalizedEvent` / `NormalizedCandle` structs (with `Decimal` price fields) over Tokio channels.

```
[Exchange APIs] ──► DIE ──► [Market Data Matrix] ──► [MME]
```

### 1.1 Responsibilities

| In Scope | Out of Scope |
|----------|--------------|
| WebSocket connection management | Indicator computation |
| REST historical fetching / gap-filling | Bias / regime interpretation |
| Symbol normalization across venues | Order execution |
| OHLCV candle aggregation | Portfolio state |
| Data quality validation | Strategy logic |
| Broadcast distribution | Persistence beyond the telemetry store |

### 1.2 Layer Structure

| Layer | Name | Output |
|-------|------|--------|
| L1 | [Raw Data Layer](03-01-02-die-layer1-raw-data.md) | Standardized `NormalizedEvent` stream |
| L2 | [Market Data Layer](03-01-03-die-layer2-market-data.md) | Uniform multi-timeframe `NormalizedCandle`s |
| L3 | [Data Quality Layer](03-01-04-die-layer3-data-quality.md) | Gap-filled, validated candle sets |
| L4 | [Data Distribution Layer](03-01-05-die-layer4-data-distribution.md) | Broadcast channels to consumers |

---

## 2. Exchange Adapters

The DIE supports pluggable venue adapters conforming to the `ExchangeAdapter` trait (`crates/shared/src/normalized/mod.rs`):

```rust
#[async_trait]
pub trait ExchangeAdapter: Send + Sync {
    fn exchange(&self) -> Exchange;
    async fn start(
        &self,
        symbols: Vec<String>,
        event_tx: Sender<NormalizedEvent>,
        mapper: Arc<SymbolMapper>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
```

### 2.1 Supported Venues

| Exchange | WebSocket Endpoint | REST Fallback | Adapter |
|----------|--------------------|--------------|---------|
| **Hyperliquid** | `wss://api.hyperliquid.xyz/ws` | `hyperliquid_rest.rs` | `HyperliquidAdapter` |
| **Bitget** | `wss://ws.bitget.com/v2/ws/public` | `bitget_rest.rs` | `bitget::run_for_symbol` |

Endpoints are configured in `config.json` (`hyperliquid.ws_url`, `bitget.ws_url`).

### 2.2 Ingested Data Types

Each adapter emits a `NormalizedEvent` enum:

| Variant | Payload | Source |
|---------|---------|--------|
| `Trade` | price, size, side, timestamp, id | Trade stream |
| `OrderBook` | bids/asks depth ladders | L2 book stream |
| `AssetContext` | previous-day price | Asset context feed |
| `OpenInterest` | current OI | Derivatives feed |
| `FundingRate` | current funding rate | Derivatives feed |
| `Status` | connection lifecycle message | Adapter supervisor |

---

## 3. Performance Targets

| Metric | Target |
|--------|--------|
| Raw frame → NormalizedEvent | < 1 ms |
| Trade → live candle update | < 2 ms |
| Observation loop (Raw → Market Data Matrix) | < 25 ms |
| Event channel capacity | 10,000 buffered events |
| Reconnect backoff | 2 s → 60 s (exponential) |
| Permanent disable threshold | 5 consecutive failures |

> **Target Architecture (Not Yet Implemented).** The ≥ 50,000 events/sec sustained-ingestion target above assumes the DOD hot-path model: raw frames parsed into pre-allocated flat buffers and processed as contiguous `f64` slices, avoiding per-event heap allocation. The current millisecond targets are met by the struct-based pipeline.

---

## 4. Connection Monitoring & Fault Tolerance

The `MarketDataOrchestrator` (`crates/engine/src/orchestrator.rs`) supervises every adapter in an independent Tokio task with a self-healing loop:

```
       ┌──────────────────────────────────────────────┐
       │              Adapter Supervisor               │
       │                                               │
       │  [get active symbols] ──► empty? ──► dormant  │
       │        │ non-empty                            │
       │        ▼                                      │
       │  emit Connecting ──► adapter.start()          │
       │        │                                      │
       │   ┌────┴────┐                                 │
       │   ▼         ▼                                 │
       │  Ok(())   Err(e)                              │
       │   │         │ failures++                      │
       │   │         ▼                                 │
       │   │    failures ≥ 5 ──► permanently disable   │
       │   ▼                                           │
       │  emit Disconnected ──► backoff ──► retry      │
       └──────────────────────────────────────────────┘
```

### 4.1 Fault-Tolerance Rules

- **Exponential backoff:** `retry_cooldown = min(retry_cooldown × 2, 60s)`, starting at 2 s.
- **Failure window reset:** If > 300 s elapse since the last failure, the consecutive-failure counter resets to 0.
- **Permanent disable:** After 5 consecutive failures, the adapter emits a terminal `Disconnected` status and the supervisor loop breaks.
- **Dormant state:** With no configured symbols, the adapter idles (polling every 2 s) rather than failing.

### 4.2 ConnectionStatus Lifecycle

```
Connecting ──► Connected ──► (stream) ──► Disconnected ──► Reconnecting ──► Connecting
```

---

## 5. Symbol Normalization

The `SymbolMapper` (`crates/shared/src/normalized/symbol_mapper.rs`) maps exchange-native symbols (e.g. Hyperliquid `BTC`, Bitget `BTCUSDT`) to unified internal symbols (e.g. `BTC-USDT`). The configured `symbols` list uses `Exchange:Symbol` syntax (e.g. `Hyperliquid:BTC`) to bind each internal symbol to exactly one preferred venue. **Aggregation of parallel streams from multiple venues for the same symbol is not supported; cross-venue failover is not implemented.**

---

## 6. Configuration Surface

| Config Key | Purpose |
|------------|---------|
| `symbols` | Target instruments (`Exchange:Symbol` form). |
| `candles.duration_seconds` | Base (micro) candle duration. |
| `candles.analysis_limit` | Warm-up lookback depth. |
| `slow_timeframe` / `macro_timeframe` | Additional timeframe pipelines. |
| `hyperliquid.ws_url` / `bitget.ws_url` | Venue WebSocket endpoints. |

---

## 7. Cross-References

- [DIE Layer 1 — Raw Data](03-01-02-die-layer1-raw-data.md)
- [DIE Layer 2 — Market Data](03-01-03-die-layer2-market-data.md)
- [DIE Layer 3 — Data Quality](03-01-04-die-layer3-data-quality.md)
- [DIE Layer 4 — Data Distribution](03-01-05-die-layer4-data-distribution.md)
- [Global Architecture](../../conceptual-foundations/01-02-global-architecture.md) — Engine positioning.
- [Systemic Data Flow](../../conceptual-foundations/01-03-systemic-data-flow.md) — Observation loop.
