# DIE Layer 3 — Data Quality Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Layer:** 3 of 4
**Output Contract:** Data Quality Matrix (validated, gap-filled candle sets + reliability metrics)
**Purpose:** This document specifies the Data Quality Layer — the tier that enforces data integrity through REST gap-filling, sequence auditing, and median-based outlier filtering. It transforms raw candle streams into sanitized, trustworthy datasets.

---

## 1. Purpose

The Data Quality Layer guarantees that downstream analysis operates on **complete, ordered, clean** data. It reconciles the local telemetry store against exchange REST history, fills gaps, and rejects anomalous ticks.

```
[NormalizedCandle stream] ─┐
[Local telemetry store   ] ─┼──► DATA QUALITY LAYER (L3) ──► [validated candle set]
[Exchange REST history   ] ─┘        gap-fill · audit · filter
```

---

## 2. DB-First / REST-Gap-Fill Algorithm

Warm-up and gap recovery use the local-DB-first strategy implemented in `bootstrap.rs::collect_candles()`:

```
db_candles = query_recent_candles(symbol, secs, limit)   # local warm base (PRIMARY)

rest_start = db_candles.last().start_time_ms + secs·1000   # gap boundary
           = now - secs·limit·1000   (if no local data)

IF rest_start < now:
    interval = timeframe_secs_to_interval(secs)   # venue-specific granularity
    rest_candles = fetch_historical_candles(...)  # fetch ONLY the missing gap

merged = db_candles ++ dedup(rest_candles)       # chronological, oldest-first
```

The cascade is uniform across all timeframes — including sub-minute. Sub-minute REST history is generally unavailable from venue APIs (Hyperliquid and Bitget both return ≥1m candles), but the local DB may already contain sub-minute candles persisted from a previous session, and that local cache is the most reliable warm seed.

### 2.1 Strategy Properties

| Property | Rationale |
|----------|-----------|
| **DB-first** | Minimizes REST calls; the local store is authoritative for already-seen candles. |
| **Gap-only REST** | Only the window between the last local candle and `now` is fetched. |
| **Full-window fallback** | With no local data, the entire `secs × limit` lookback is fetched. |
| **Sub-minute cascade (v2.1 — correction)** | A previous version returned an empty array for sub-minute timeframes (`return []`), bypassing the local DB. That left the EMA reconstruction ([08-04-candle-reconstruction.md §EMA Synthesis](../operations-and-compliance/08-04-candle-reconstruction.md)) starved of history on startup, so a network disconnect within minutes of launch would force a fallback to linear interpolation (or no reconstruction at all if history < 2). The corrected cascade queries the local DB for sub-minute timeframes first (which may already contain history from a prior session), then falls back to a best-effort `limit=200` REST fetch (may return empty for venues without sub-minute history), then falls back to live ticks. The reconstructor's documented threshold (`≥ 50 history points` for EMA, `≥ 2` for linear) is still respected — a sub-50 seed will use linear projection for the first few reconstructions until the buffer fills. |

---

## 3. Sequence Auditing

The layer verifies candle-set ordering and continuity:

| Check | Action on Violation |
|-------|---------------------|
| **Chronological order** | Candles sorted oldest-first before use. |
| **Duplicate detection** | Candles with identical `start_time_ms` deduplicated (local preferred over REST). |
| **Missing bar** | A hole between consecutive `start_time_ms` values ≠ `duration_ms` flags a gap for REST recovery. |
| **Out-of-order arrival** | Late-arriving frames are reordered into the correct interval bucket. |

---

## 4. Outlier Filtering

Anomalous ticks (fat-finger prints, venue glitches) are rejected before they contaminate candles:

### 4.1 Median Price Filter

For an incoming tick price `p` against a rolling window of recent prices:

```
median = median(recent_window)
IF |p − median| / median  >  outlier_tolerance:
    reject tick   (do not update candle)
```

This suppresses single-print spikes while preserving genuine fast moves (which persist across multiple ticks and shift the median).

> **Bootstrap behaviour (v2.1 — clarification).** The rolling window is initialised lazily. For the first `N = median_window_size` ticks (default `20`, configurable via `config.json` `quality.median_window_size`), every tick is accepted unfiltered — the warm-up mode allows the window to fill before the filter evaluates normally. From tick `N + 1` onward the median filter evaluates against the prior `N` ticks. The current tick is appended to the window **after** the filter check (not before), so the filter cannot reject its own input. When the median is exactly zero (rare but possible on a venue reset), the filter is bypassed for that tick and a debug-level log entry is emitted. The window is monotonically expanded during warm-up; ticks observed during warm-up are still written to the candle and propagated downstream — only their filter rejection is deferred.
>
> **Target Architecture (Not Yet Implemented).** In the DOD hot-path model the rolling median price filter and standard-deviation outlier calculations execute over the contiguous `f64` price arrays resident in the CPU cache, achieving sub-millisecond execution without heap traversal. *Current implementation:* these checks run over `Decimal`/`VecDeque`-style windows.

### 4.2 Structural Validity

Every candle passes `NormalizedCandle::assert_validity()`:

```
high ≥ low
open  ∈ [low, high]
close ∈ [low, high]
volume ≥ 0
```

Candles failing validity are quarantined and refetched from REST.

---

## 5. Reliability Metrics

The Data Quality Matrix pairs each sanitized dataset with reliability metadata:

| Metric | Meaning |
|--------|---------|
| Coverage | Fraction of expected bars present after gap-fill. |
| Gap count | Number of holes detected and repaired. |
| Outliers rejected | Count of ticks filtered this session. |
| Source mix | Ratio of DB-warm vs REST-gap vs live candles. |

---

## 6. Bootstrap Warm-Up

At instance startup, `fetch_and_warm_bootstrap()` (`registry/bootstrap.rs`) feeds the sanitized historical candle set through all indicator calculators (`analyzer/warm.rs`), producing a `WarmedPipelineState` so the pipeline emits fully-formed Metrics Matrices from the first live candle rather than after a cold warm-up.

---

## 7. Guarantees

| Property | Guarantee |
|----------|-----------|
| **Completeness** | Gaps are filled before candles reach the analyzer. |
| **Ordering** | Candle sets are strictly chronological and deduplicated. |
| **Cleanliness** | Outlier ticks and invalid candles are rejected. |
| **Warm start** | Indicators are pre-warmed from validated history. |

---

## 8. Cross-References

- [DIE Layer 2 — Market Data](03-01-03-die-layer2-market-data.md) — Input.
- [DIE Layer 4 — Data Distribution](03-01-05-die-layer4-data-distribution.md) — Downstream.
- [MME Overview](../market-monitoring-engine/03-02-01-mme-overview-spec.md) — Warm-up consumer.
- [Database Schema](../../integration-and-api/06-02-database-schema-spec.md) — `market_snapshots` warm base.
