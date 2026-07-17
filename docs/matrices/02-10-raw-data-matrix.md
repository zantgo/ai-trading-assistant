# Raw Data Matrix Specification

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
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
| `OrderBook` | `bids: Vec<[Decimal; 2]>`, `asks: Vec<[Decimal; 2]>`, `timestamp_ms: u64` | L2 order book snapshot or delta. Each entry is a `[price, size]` tuple, ordered best-to-worst (bids descending, asks ascending). |
| `AssetContext` | prev_day_px | Prior-day reference price. |
| `OpenInterest` | symbol, oi_value, timestamp | Current open interest. |
| `FundingRate` | symbol, rate, timestamp | Current perpetual funding rate. |
| `Status` | exchange, status, message | Connection lifecycle event (Connected, Disconnected, Reconnecting). |

---

## 3. JSON Wire Examples (per variant)

The `NormalizedEvent` enum serializes as a flat JSON object (no `event_type` discriminator field — variant identity is preserved by serde-tagging at the `NormalizedEvent` enum boundary, not on the wire). Each variant produces a flat object with the fields listed in §2.

### 3.1 `Trade`
```json
{
  "exchange": "Hyperliquid",
  "symbol": "BTC-USDT",
  "timestamp_ms": 1752192000000,
  "price": "64012.5",
  "size": "0.15",
  "side": "Buy",
  "trade_id": "123456"
}
```

### 3.2 `OrderBook`
```json
{
  "exchange": "Hyperliquid",
  "symbol": "BTC-USDT",
  "timestamp_ms": 1752192000000,
  "bids": [["64012.0", "0.5"], ["64011.5", "1.2"], ["64010.0", "0.8"]],
  "asks": [["64012.5", "0.4"], ["64013.0", "1.0"], ["64014.0", "0.6"]]
}
```

### 3.3 `AssetContext`
```json
{
  "exchange": "Hyperliquid",
  "symbol": "BTC-USDT",
  "prev_day_px": "63500.0"
}
```

### 3.4 `OpenInterest`
```json
{
  "exchange": "Hyperliquid",
  "symbol": "BTC-USDT",
  "oi_value": "125000.0",
  "timestamp_ms": 1752192000000
}
```

### 3.5 `FundingRate`
```json
{
  "exchange": "Hyperliquid",
  "symbol": "BTC-USDT",
  "rate": "0.000125",
  "timestamp_ms": 1752192000000
}
```

### 3.6 `Status`
```json
{
  "exchange": "Hyperliquid",
  "status": "Connected",
  "message": ""
}
```

---

## 4. Cross-References

- [DIE Overview](../engines/data-infrastructure-engine/03-01-01-die-overview-spec.md) — Engine boundaries and adapter model.
- [DIE Layer 1 — Raw Data](../engines/data-infrastructure-engine/03-01-02-die-layer1-raw-data.md) — Producing-layer specification.
- [Market Data Matrix](02-06-market-data-matrix.md) — Downstream consumer.
