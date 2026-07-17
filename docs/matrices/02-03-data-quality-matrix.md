# Candle Quality Envelope Specification

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE) (per-candle); Market Monitoring Engine (computation)
**Producing Layer:** Layer 3 — Data Quality Layer (DIE); layer name in code: `CandleQualityEnvelope`
**Purpose:** This document defines the physical schema of the **per-candle quality envelope** — the integrity-checked candle with attached validity metadata that rides the `MarketSnapshot` payload. (v6.0 renamed the document from "Data Quality Matrix Specification" to "Candle Quality Envelope Specification" to disambiguate it from the per-instance `PipelineReliabilityMetrics` documented in [03-01-04 §5](../engines/data-infrastructure-engine/03-01-04-die-layer3-data-quality.md).)

---

## 1. Conceptual Definition

The Data Quality Layer audits the Market Data Matrix output for integrity before it reaches downstream consumers. It detects missing candles, stale ticks, out-of-order sequences, and price spikes, producing sanitized data paired with per-candle validity metadata (this document) and per-instance reliability metrics (see [03-01-04 §5](../engines/data-infrastructure-engine/03-01-04-die-layer3-data-quality.md) for `PipelineReliabilityMetrics`).

```
[Market Data Matrix] ──► DATA QUALITY LAYER (L3) ──► [Candle Quality Envelope + Pipeline Reliability Metrics] ──► [Distribution Layer (L4)]
```

---

## 2. Physical Schema

| Field | Type | Description |
|-------|------|-------------|
| `candle` | `NormalizedCandle` | The validated candle (from Market Data Matrix). |
| `is_gap_filled` | `bool` | `true` if this candle was synthetically filled (no data for this interval). |
| `is_stale` | `bool` | `true` if the candle's last trade timestamp exceeds the staleness threshold. |
| `spike_detected` | `bool` | `true` if a price spike was filtered from this candle. |
| `sequence_integrity` | `SequenceIntegrity` | `VALID` / `OUT_OF_ORDER` / `DUPLICATE`. |
| `quality_score` | `f64` | Overall reliability metric in `[0, 100]`. |
| `gap_since_last` | `u64` | Seconds since the last valid candle (0 = continuous). |
| `validated_at` | `u64` | Unix epoch of quality validation, in **milliseconds** (consistent with the canonical timestamp unit defined in [02-06-market-data-matrix.md §2](02-06-market-data-matrix.md)). |

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
  "quality_score": 100.0,
  "gap_since_last": 60,
  "validated_at": 1752192001000
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
