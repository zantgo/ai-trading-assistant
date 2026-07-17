# Data Infrastructure Engine — End-to-End Flow

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Purpose:** This document is the **single integrated narrative** for the Data Infrastructure Engine. It traces one trade tick from the exchange WebSocket to the dashboard broadcast and SQLite persistence in one diagram + walkthrough. Each layer's detailed contract lives in its dedicated doc; this document only cross-references them.

> **Reading order.** New operators should read this doc first, then drill into the layer-specific docs as needed. Layer docs (03-01-01..05) describe their layer in isolation; this doc shows how the layers compose.

---

## 1. Sequence — one trade, end-to-end

```
Exchange WS (Hyperliquid/Bitget)
    │  raw frame (JSON)
    ▼
[L1 Raw Data Layer]                  crates/network-adapters/src/adapters/{hyperliquid,bitget}.rs
    │  parse → NormalizedEvent::Trade
    │  send to bounded mpsc (capacity 10,000)
    ▼
[L1 → L2 channel]                    crates/network-adapters/src/registry/pipelines.rs::run_event_router
    │  fan-out to 4× TimeframePipeline receivers
    ▼
[L2 Market Data Layer]               crates/market-analyzer/src/candle_generator.rs
    │  process_trade → (Option<completed>, live) on the base (micro) tier
    │  on completion: build MarketSnapshot
    │  record_open_interest / record_funding_rate / record_mark_price / record_prev_day_px updates (if event)
    ▼
[L3 Data Quality Layer]              inline in market-analyzer/mod.rs
    │  assert_validity on the completed candle
    │  median filter on incoming ticks (warm-up then evaluate)
    │  late-tick drop counter
    │  CandleQualityEnvelope attached to the snapshot
    ▼
[L4 Data Distribution Layer]         crates/market-analyzer/src/analyzer/mod.rs broadcast channels
    │  broadcast_tx.send(snapshot) on the (symbol, timeframe) channel
    ▼
┌─────────────────┬─────────────────┬─────────────────┐
│ WS broadcast    │ Telemetry       │ MME analyzer    │
│ /ws clients     │ logger          │ (per-tier)      │
│                 │                 │                 │
│ crates/api-     │ crates/database-│ crates/market-  │
│ gateway/src/    │ storage/src/    │ analyzer/src/   │
│ ws.rs           │ logger.rs       │ analyzer/mod.rs │
└────────┬────────┴────────┬────────┴────────┬────────┘
         │                 │                 │
         ▼                 ▼                 ▼
   Svelte 5          SQLite WAL         Indicator pipeline
   runes store       market_snapshots   (50 indicators)
                     + connection_
                     quality_samples
                     + liquidation_
                     events
```

---

## 2. Layer walkthrough — what happens to one trade

| Step | Layer | Component | Action |
|------|-------|-----------|--------|
| 1 | L1 | Adapter (`hyperliquid::run_for_symbol` / `bitget::run_for_symbol`) | Open WebSocket. Subscribe to `trades` channel for assigned symbols. |
| 2 | L1 | Adapter | Parse trade JSON → `NormalizedTrade { exchange, symbol, price, size, side, timestamp_ms, trade_id }`. |
| 3 | L1 | Adapter | Wrap as `NormalizedEvent::Trade(...)`; send on bounded mpsc. |
| 4 | L2 | `run_event_router` | Fan out the event to the 4 timeframe receivers. |
| 5 | L2 | `CandleGenerator::process_trade` (per-tier) | Update or close the in-progress candle. Return `(Option<completed>, live)`. |
| 6 | L2 | MME analyzer pipeline | On `Some(completed)`, the MME analyzer pipeline builds the `MarketSnapshot` (candle, indicator buffers, alignment, etc.); DIE L2 emits only the `NormalizedCandle`. |
| 7 | L3 | (inline) | Run `assert_validity` on the candle. Attach `CandleQualityEnvelope` with `quality_score`. Increment `out_of_order_dropped` if applicable. |
| 8 | L4 | `broadcast_tx.send(snapshot)` | Publish on the `(symbol, timeframe)` channel. |
| 9 | L4 | Telemetry logger | On `is_completed = true`, send `TelemetryMsg::InsertSnapshot` to the DB-write task. |
| 10 | L4 | WS handler | Forward the snapshot to all subscribers on `/ws?symbol=…&timeframe_secs=…`. |
| 11 | L4 | MME analyzer | Subscribe via in-process broadcast; run indicators; produce Alignment/Analysis/Risk/Advisory. |
| 12 | DB | `logger.rs::run_telemetry_logger` | INSERT into `market_snapshots` (WAL mode). |

Latency budget (per `AC-DIE-3`): p95 < 25 ms end-to-end.

---

## 3. Connection-quality parallel flow

The connection-quality tracking runs alongside the trade flow, not inline:

```
Adapter lifecycle event (Connected / Disconnected / ReconnectCompleted / Heartbeat)
    │
    ▼
ConnectionQualityTracker::record_*       crates/network-adapters/src/connection_quality_tracker.rs
    │  in-memory rolling windows (1h / 6h / 24h)
    ▼
every 60s: run_persistence_loop          crates/network-adapters/src/connection_quality_tracker.rs::run_persistence_loop
    │  INSERT 3 rows per tick into connection_quality_samples
    ▼
SQLite connection_quality_samples        per-(pair_key, timeframe_secs) rows
    │
    ▼
GET /api/connection-quality?instance_id=…&timeframe_secs=…&window=…
    │  api-gateway/src/handlers/connection_quality.rs
    ▼
Svelte runes store → ConnectionQualityPanel.svelte
```

Reconstructed candles (from [08-04-candle-reconstruction.md](../../operations-and-compliance/08-04-candle-reconstruction.md)) increment `reconstructed_candles` on the tracker. Disconnects / reconnects increment the matching counters; the 60s persistence loop collapses them into per-window samples.

---

## 4. Clock-monitor parallel flow

```
ClockMonitor::run_until_cancelled        crates/network-adapters/src/clock_monitor.rs
    │  every 30s (configurable): poll NTP servers
    │  compute offset + RTT
    │  classify: WithinThreshold | BreachThreshold | NetworkError
    ▼
on breach: log + (optional panic)         see 08-06 §Failure Mode
    │  breach counter exposed via /api/system/clock (Phase 3)
    ▼
(no direct coupling to candles; see Drift-Breach Consequence)
```

The clock monitor does not affect the trade path directly. Its drift budget (≤ 50 µs) is the contract that the L2 candle-alignment invariant relies on.

---

## 5. What does NOT flow through the DIE

The DIE does not compute indicators, bias, or risk. Those happen in the MME (consuming the L4 broadcast). The DIE's only outputs are:

- `NormalizedEvent` stream (L1 → L2)
- `NormalizedCandle` stream (L2 → L3)
- `MarketSnapshot` with `CandleQualityEnvelope` (L3 → L4)
- `PipelineReliabilityMetrics` (per-instance, exposed via `/api/data-quality`)
- `ConnectionQualityReport` (per-`(pair_key, timeframe_secs)`, exposed via `/api/connection-quality`)

Anything that requires interpretation of the data (indicators, signals, regime, risk, opportunity, decision) is MME's responsibility and is documented in the MME spec files under `docs/engines/market-monitoring-engine/`.

---

## 6. Cross-References

- Layer docs: [03-01-01..05](./03-01-01-die-overview-spec.md)
- Matrix specs: [02-03, 02-05, 02-06, 02-07, 02-10](../../matrices/02-00-matrix-field-ownership.md)
- Connection resilience: [08-03](../../operations-and-compliance/08-03-connection-resilience.md)
- Candle reconstruction: [08-04](../../operations-and-compliance/08-04-candle-reconstruction.md)
- Connection quality: [08-05](../../operations-and-compliance/08-05-connection-quality.md)
- Clock monitor: [08-06](../../operations-and-compliance/08-06-clock-monitor.md)
- API gateway: [06-01](../../integration-and-api/06-01-api-gateway-contract.md)
- Database schema: [06-02](../../integration-and-api/06-02-database-schema-spec.md)
- Crate layout: [01-06](../../conceptual-foundations/01-06-crate-layout-and-cycles.md)
- Target architecture roadmap: [01-07](../../conceptual-foundations/01-07-target-architecture-roadmap.md)