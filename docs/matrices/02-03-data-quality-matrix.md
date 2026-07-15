# Data Quality Matrix Specification

**Version:** 2.0
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Producing Layer:** Layer 3 — Data Quality Layer
**Purpose:** This document defines the physical schema of the **Data Quality Matrix** — the validated, gap-filled, and integrity-checked candle stream with attached reliability metadata.

---

## 1. Conceptual Definition

The Data Quality Layer audits the Market Data Matrix output for integrity before it reaches downstream consumers. It detects missing candles, stale ticks, out-of-order sequences, and price spikes, producing sanitized data paired with reliability metrics.

```
[Market Data Matrix] ──► DATA QUALITY LAYER (L3) ──► [Data Quality Matrix] ──► [Distribution Layer (L4)]
```

---

## 2. Physical Schema

| Field | Type | Description |
|-------|------|-------------|
| `candle` | `NormalizedCandle` | The validated candle (from Market Data Matrix). |
| `is_gap_filled` | `bool` | `true` if this candle was synthetically filled (no data for this interval). |
| `is_stale` | `bool` | `true` if the candle's last trade timestamp exceeds the staleness threshold. |
| `spike_detected` | `bool` | `true` if a price spike was filtered from this candle. |
| `sequence_integrity` | `SequenceIntegrity` | `Valid` / `OutOfOrder` / `Duplicate`. |
| `quality_score` | `f64` | Overall reliability metric in `[0, 100]`. |
| `gap_since_last` | `u64` | Seconds since the last valid candle (0 = continuous). |
| `validated_at` | `u64` | Unix epoch of quality validation. |

---

## 3. JSON Serialization Contract

```json
{
  "candle": {
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
  },
  "is_gap_filled": false,
  "is_stale": false,
  "spike_detected": false,
  "sequence_integrity": "VALID",
  "quality_score": 98.0,
  "gap_since_last": 60,
  "validated_at": 1752192001
}
```

---

## 4. Quality Score Computation

The `quality_score` is derived from:

$$\text{quality\_score} = 100 - 20 \cdot (\text{is\_gap\_filled} ? 1 : 0) - 10 \cdot (\text{is\_stale} ? 1 : 0) - 30 \cdot (\text{spike\_detected} ? 1 : 0) - 15 \cdot (\text{sequence\_integrity} \neq \text{Valid} ? 1 : 0)$$

A score below 50 triggers a warning. Scores below 30 may cause the candle to be suppressed from downstream consumers.

---

## 5. Cross-References

- [DIE Overview](../engines/data-infrastructure-engine/03-01-01-die-overview-spec.md) — Engine boundaries and quality layer description.
- [DIE Layer 3 — Data Quality](../engines/data-infrastructure-engine/03-01-04-die-layer3-data-quality.md) — Producing-layer specification.
- [Market Data Matrix](02-06-market-data-matrix.md) — Upstream input.
- [Distribution Matrix](02-05-distribution-matrix.md) — Downstream consumer.
