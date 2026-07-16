# Raw Data Matrix Specification

**Version:** 5.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Producing Layer:** Layer 1 — Raw Data Layer
**Purpose:** This document defines the physical schema of the **Raw Data Matrix** — the standardized envelope for raw exchange events. It normalizes heterogeneous venue-specific wire formats into a single `NormalizedEvent` stream consumed by the Market Data Layer.

---

## 1. Conceptual Definition

The Raw Data Matrix is the DIE's first transformation: converting raw WebSocket frames and REST responses from multiple exchanges into a unified, typed event stream. It standardizes the transport format but does not aggregate, validate, or temporally align the data.

```
[Exchange WebSocket] ──► RAW DATA LAYER (L1) ──► [NormalizedEvent stream] ──► [Market Data Layer (L2)]
```

---

## 2. NormalizedEvent Variants

| Variant | Payload | Description |
|---------|---------|-------------|
| `Trade` | price, size, side, timestamp, trade_id | Single executed trade. |
| `OrderBook` | bids (price → size), asks (price → size), timestamp | L2 order book snapshot or delta. |
| `AssetContext` | prev_day_px | Prior-day reference price. |
| `OpenInterest` | symbol, oi_value, timestamp | Current open interest. |
| `FundingRate` | symbol, rate, timestamp | Current perpetual funding rate. |
| `Status` | exchange, state, message | Connection lifecycle event (Connected, Disconnected). |

---

## 3. JSON Serialization Contract

```json
{
  "exchange": "Hyperliquid",
  "event_type": "Trade",
  "symbol": "BTC-USDT",
  "timestamp": 1752192000000,
  "payload": {
    "price": "64012.5",
    "size": "0.15",
    "side": "Buy",
    "trade_id": "123456"
  }
}
```

---

## 4. Cross-References

- [DIE Overview](../engines/data-infrastructure-engine/03-01-01-die-overview-spec.md) — Engine boundaries and adapter model.
- [DIE Layer 1 — Raw Data](../engines/data-infrastructure-engine/03-01-02-die-layer1-raw-data.md) — Producing-layer specification.
- [Market Data Matrix](02-06-market-data-matrix.md) — Downstream consumer.
