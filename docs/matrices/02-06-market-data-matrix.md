# Market Data Matrix Specification

**Version:** 2.0
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Producing Layer:** Layer 2 — Market Data Layer
**Purpose:** This document defines the physical schema of the **Market Data Matrix** — the uniform, multi-timeframe OHLCV candle stream that is the primary input to the Market Monitoring Engine.

---

## 1. Conceptual Definition

The Market Data Matrix transforms the raw `NormalizedEvent` stream into standardized temporal boundaries. Trade events are aggregated into OHLCV (Open, High, Low, Close, Volume) candles across configurable timeframes. This is the **sole data contract** between the DIE and the MME.

```
[NormalizedEvent stream] ──► MARKET DATA LAYER (L2) ──► [Market Data Matrix] ──► [MME Metrics Layer (L1)]
```

---

## 2. Physical Schema

A single `NormalizedCandle` represents one completed candle for one symbol at one timeframe:

| Field | Type | Description |
|-------|------|-------------|
| `exchange` | `string` | Originating venue. |
| `symbol` | `string` | Unified internal symbol (e.g., `BTC-USDT`). |
| `timeframe_secs` | `u64` | Candle duration in seconds (60 / 180 / 300 / 900). |
| `timestamp` | `u64` | Candle close time (Unix epoch, milliseconds). |
| `open` | `Decimal` | First trade price in the interval. |
| `high` | `Decimal` | Highest trade price in the interval. |
| `low` | `Decimal` | Lowest trade price in the interval. |
| `close` | `Decimal` | Last trade price in the interval. |
| `volume` | `Decimal` | Total base-asset volume traded. |
| `trades_count` | `u64` | Number of trades aggregated. |
| `reconstructed` | `Option<ReconstructionMethod>` | Provenance flag — `Some(ExchangeHistorical)` / `Some(ExponentialMovingAverage)` / `Some(LinearInterpolation)` for candles filled by the reconstruction engine (see [08-04-candle-reconstruction.md](../operations-and-compliance/08-04-candle-reconstruction.md)); `None` (omitted on the wire) for live candles. The flag is forwarded through aggregation chains so a macro candle is marked `reconstructed` if any constituent sub-candle is reconstructed. |

---

## 3. JSON Serialization Contract

```json
{
  "exchange": "Hyperliquid",
  "symbol": "BTC-USDT",
  "timeframe_secs": 60,
  "timestamp": 1752192000000,
  "open": "63890.0",
  "high": "64120.0",
  "low": "63850.0",
  "close": "64012.5",
  "volume": "182.4",
  "trades_count": 345
}
```

---

## 4. Multi-Timeframe Output

The Market Data Layer produces candles at up to 4 timeframes simultaneously (micro, fast, slow, macro). Each timeframe has its own independent aggregation buffer per symbol.

---

## 5. Cross-References

- [DIE Overview](../engines/data-infrastructure-engine/03-01-01-die-overview-spec.md) — Engine boundaries and performance targets.
- [DIE Layer 2 — Market Data](../engines/data-infrastructure-engine/03-01-03-die-layer2-market-data.md) — Producing-layer specification.
- [Raw Data Matrix](02-10-raw-data-matrix.md) — Upstream input.
- [Metrics Matrix](02-07-metrics-matrix.md) — Downstream consumer (MME Layer 1).
- [Timeframe Model](../conceptual-foundations/01-04-timeframe-model.md) — Configurable 4-tier durations.
