# DIE Layer 3 — Data Quality Layer

**Version:** 6.4.1 (2026-07-18) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Layer:** 3 of 4
**Output Contract:** CandleQualityEnvelope + PipelineReliabilityMetrics (validated, gap-filled candle sets + reliability metrics)
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

Warm-up and gap recovery use the local-DB-first strategy implemented in `crates/portfolio-supervisor/src/registry/bootstrap.rs::collect_candles()`:

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
| **Sub-minute-aware** | Both bootstrap and live gap-fill paths handle sub-minute timeframes distinctly from ≥1m ladders. The two paths are documented separately below. |

#### 2.1.1 Startup bootstrap (sub-minute path)

The startup cascade (invoked from `crates/portfolio-supervisor/src/registry/bootstrap.rs::fetch_and_warm_bootstrap`) tries the following in order:

1. Query the local DB for the most recent `analysis_limit` candles of `(symbol, secs)`. The DB may already contain sub-minute candles persisted from a previous session, which is the most reliable warm seed.
2. With no local data, paginate REST fetches (venue page cap, e.g. `limit=200`) until the full `secs × analysis_limit` lookback is retrieved; on venues with hard caps, proceed best-effort — `min_warmup_bars` (50) remains the gate (venues without sub-minute history may return an empty array).
3. With no REST coverage, begin live tick ingestion from the moment of subscription; the EMA/linear reconstructor fills the pre-subscription window as it goes.

Returning an empty array for sub-minute timeframes would starve the EMA reconstruction on startup — see [08-04-candle-reconstruction.md §EMA Synthesis](../../operations-and-compliance/08-04-candle-reconstruction.md) — and force the platform into linear interpolation (or no reconstruction at all if history < 2) exactly when a network disconnect is most likely to need EMA-quality fills. The reconstructor's documented threshold (`≥ 50 history points` for EMA, `≥ 2` for linear) is still respected — a sub-50 seed uses linear projection for the first few reconstructions until the buffer fills.

#### 2.1.2 Live gap-fill (sub-minute path)

After startup, the `GapDetector` (see [08-04-candle-reconstruction.md §Gap Detection](../../operations-and-compliance/08-04-candle-reconstruction.md)) decides whether reconstruction is required for a runtime gap. Sub-minute runtime gaps follow the same EMA-preferred / linear-fallback ladder as startup, but:

- The trigger is `GapDetector.detect_gap(last_persisted_ts_ms, now_ms, gap_threshold_secs)` rather than a DB-empty check.
- The reconstructed candles are tagged with `ReconstructionMethod` and forwarded through the aggregator (per [08-04 §Forwarding](../../operations-and-compliance/08-04-candle-reconstruction.md)).
- New ticks arriving during reconstruction are buffered; indicator state is not updated until reconstruction completes (see [08-04 §Reconnect sequencing](../../operations-and-compliance/08-04-candle-reconstruction.md)).

The two paths share the EMA/linear thresholds (≥50 / ≥2) but differ in entry condition and ownership: §2.1.1 is invoked once at instance creation; §2.1.2 is invoked per detected runtime gap.

---

## 3. Sequence Auditing

The layer verifies candle-set ordering and continuity:

| Check | Action on Violation |
|-------|---------------------|
| **Chronological order** | Candles sorted oldest-first before use. |
| **Duplicate detection** | Candles with identical `start_time_ms` deduplicated (local preferred over REST). |
| **Missing bar** | A hole between consecutive `start_time_ms` values ≠ `duration_ms` flags a gap for REST recovery. |
| **Late-trade arrival** | A trade whose `timestamp_ms` falls inside a previously-closed interval is **dropped** at L3 and counted in the `out_of_order_dropped` reliability metric. The completed candle is immutable per the L4 Distribution Layer invariant (see [03-01-05-die-layer4-data-distribution.md §5](./03-01-05-die-layer4-data-distribution.md)). Retroactive reordering is forbidden: a broadcast `MarketSnapshot` may never be mutated after the **MME L1 producer** emits it (see [03-02-02-mme-layer1-metrics.md §8](../market-monitoring-engine/03-02-02-mme-layer1-metrics.md)). |

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

> **Bootstrap behaviour (v2.1 — clarification).** The rolling window is initialised lazily. For the first `N = median_window_size` ticks (default `20`, configurable via `config.toml` `[quality.median_window_size]`), every tick is accepted unfiltered — the warm-up mode allows the window to fill before the filter evaluates normally. From tick `N + 1` onward the median filter evaluates against the prior `N` ticks. The current tick is appended to the window **after** the filter check (not before), so the filter cannot reject its own input. When the median is exactly zero (rare but possible on a venue reset), the filter is bypassed for that tick and a debug-level log entry is emitted. The window is monotonically expanded during warm-up; ticks observed during warm-up are still written to the candle and propagated downstream — only their filter rejection is deferred.
>
> **Filter parameters (canonical defaults).**
>
> | Parameter | Default | Config key | Type |
> |-----------|---------|------------|------|
> | `median_window_size` | 20 | `[quality.median_window_size]` | `usize` |
> | `outlier_tolerance` | 0.05 (5% deviation from rolling median) | `[quality.outlier_tolerance]` | `f64` (raw decimal fraction; 0.05 = 5%) |
> | `bypass_on_zero_median` | true | `[quality.bypass_on_zero_median]` | `bool` |
>
> A tick is rejected when `|p − median| / median > outlier_tolerance`. The bypass-on-zero-median prevents division-by-zero on a venue reset (rare but observed); bypassed ticks are logged at debug level and counted under `outliers_bypassed` in `PipelineReliabilityMetrics`.
>
> **Target Architecture.** See [01-07 §1](../../conceptual-foundations/01-07-target-architecture-roadmap.md) — "DOD hot-path" row. The rolling median price filter and standard-deviation outlier calculations would execute over the contiguous `f64` price arrays resident in the CPU cache. *Current implementation:* these checks run over `Decimal`/`VecDeque`-style windows.

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

## 5. Pipeline Reliability Metrics

These are **per-instance** operational metrics (not the per-candle `CandleQualityEnvelope` envelope defined in [02-03-data-quality-matrix.md](../../matrices/02-03-data-quality-matrix.md)). They are served via `GET /api/data-quality` (`crates/api-gateway/src/handlers/data_quality.rs`; see [06-01-api-gateway-contract.md §2.11](../../integration-and-api/06-01-api-gateway-contract.md)) and roll up to the dashboard's Data Quality panel.

| Metric | Meaning |
|--------|---------|
| Coverage | Fraction of expected bars present after gap-fill. |
| Gap count | Number of holes detected and repaired. |
| Outliers rejected | Count of ticks filtered this session. |
| Source mix | Ratio of DB-warm vs REST-gap vs live candles. |
| `out_of_order_dropped` | Trades dropped because their `timestamp_ms` fell inside a previously-completed interval (§3). Persisted in-memory; surfaced through `GET /api/data-quality`; lost on restart. |

**Naming disambiguation.** The per-candle envelope documented in [02-03-data-quality-matrix.md](../../matrices/02-03-data-quality-matrix.md) is `CandleQualityEnvelope` (formerly called the Data Quality Matrix). It rides the `MarketSnapshot` and contains `quality_score: f64 ∈ [0, 100]`. The metrics in this section are `PipelineReliabilityMetrics` — a per-instance roll-up, not a per-candle annotation. The two are complementary: `CandleQualityEnvelope` evaluates one candle's validity; `PipelineReliabilityMetrics` measures the sanitization pipeline's health.

---

## 6. Bootstrap Warm-Up

At instance startup, `fetch_and_warm_bootstrap()` (`crates/portfolio-supervisor/src/registry/bootstrap.rs`) feeds the sanitized historical candle set into the MME warm-up pipeline (`analyzer/warm.rs`), producing a `WarmedPipelineState` so the pipeline emits fully-formed Metrics Matrices from the first live candle rather than after a cold warm-up. The DIE does not compute indicators itself (see [03-01-01-die-overview-spec.md §1](./03-01-01-die-overview-spec.md) "Mission & Boundaries"); it produces validated candle histories and hands them to the MME for indicator warm-up.

### 6.1 `WarmedPipelineState`

`WarmedPipelineState` is the per-`(symbol, timeframe)` handoff record produced at the end of a successful bootstrap and consumed by the live pipeline on the first tick. One instance is built **per configured tier** by `warm_indicators_for_timeframe(candles, tf_config, fib_config, symbol, timeframe_secs)` — there is no single cross-timeframe struct; the per-tier fan-out lives in the caller (`fetch_and_warm_bootstrap()` invokes the warm-up once per configured timeframe).

The authoritative Rust definition lives in `crates/market-analyzer/src/analyzer/warm.rs`. Its canonical shape (abridged — the full struct holds ~40 warmed indicator state machines):

```rust
pub struct WarmedPipelineState {
    // Fully-warmed indicator instances (EMA stack, RSI, MACD, ADX, Squeeze,
    // Bollinger, ATR, BBWP, Stochastic, ChandeMO, Supertrend, Keltner,
    // Donchian, OBV, CMF, MFI, HV, Aroon, Choppiness, LinReg, ZScore,
    // divergence detectors, PivotPoints, Candlestick, Ichimoku, CCI, PSAR,
    // Williams %R, HullMA, AO, ForceIndex, StdDevChannel, VolumeProfile,
    // SMC, AnchoredVwap, …), e.g.:
    pub ema_fast: Ema,
    pub rsi_14: Rsi,
    // …

    /// VWAP session accumulators.
    pub vwap_sum_tp_vol: Decimal,
    pub vwap_sum_vol: Decimal,
    /// Rolling volume baseline feeding the MME-side `average_volume`.
    pub volume_history: VecDeque<Decimal>,
    /// Indicator-ready candle history (capped at `HIST_BUFFER_MAX = 1000`).
    pub history: Vec<NormalizedCandle>,
    /// Support/resistance role-tracker state.
    pub sr_tracker: SrRoleTracker,
    /// Most recent snapshot + rolling snapshot history for warm replay.
    pub latest_snapshot: Option<MarketSnapshot>,
    pub snapshot_history: Vec<MarketSnapshot>,
    // …
}
```

The bootstrap path (physically hosted in `portfolio-supervisor`; see §2) collects the validated candle history and replays it through every indicator state machine in chronological order, so the first live candle lands on fully-warmed state. Warm-up sufficiency is governed by the cold-start minimums of [08-04-candle-reconstruction.md §Cold-start minimums](../../operations-and-compliance/08-04-candle-reconstruction.md): indicators emit `state_label = INSUFFICIENT_DATA` / `confidence = 0.0` until their per-indicator minimum buffers fill.

---

## 7. Guarantees

| Property | Guarantee |
|----------|-----------|
| **Completeness** | Gaps are filled before candles reach the analyzer. |
| **Ordering** | Candle sets are strictly chronological and deduplicated. |
| **Cleanliness** | Outlier ticks and invalid candles are rejected. |
| **Warm start** | Indicators are pre-warmed from validated history. |

### 7.1 Operational Acceptance Criteria

The L3 (Data Quality Layer) layer meets these criteria when run with default configuration under nominal load:

| ID | Criterion | Verification |
|----|-----------|--------------|
| `AC-L3-1` | Median warm-up accepts every tick for the first `median_window_size = 20` ticks; from tick 21 onward the filter evaluates against the prior 20. | `crates/network-adapters/tests/median_warmup.rs` (existing) |
| `AC-L3-2` | Tick rejection: a tick whose `|p − median| / median > 0.05` is dropped and counted in `outliers_rejected`. | `crates/network-adapters/tests/median_filter.rs` (Phase 1) |
| `AC-L3-3` | Late ticks (timestamp earlier than a previously-completed candle) are dropped at L3 and counted in `out_of_order_dropped`. | `crates/network-adapters/tests/late_tick_drop.rs` (Phase 1) |
| `AC-L3-4` | Median = 0 (venue reset) bypasses the filter for that tick and logs at debug level. | `crates/network-adapters/tests/median_zero_bypass.rs` (Phase 1) |
| `AC-L3-5` | Every completed candle passes `assert_validity()` before reaching L4; quarantine and re-fetch from REST on failure. | `crates/market-analyzer/tests/candle_validity.rs` (existing) |
| `AC-L3-6` | `CandleQualityEnvelope.quality_score` for a fully-valid candle (no gap, no spike, no staleness, valid integrity) equals 100. | `crates/market-analyzer/tests/quality_score.rs` (existing) |

---

## 8. Cross-References

- [DIE Layer 2 — Market Data](03-01-03-die-layer2-market-data.md) — Input.
- [DIE Layer 4 — Data Distribution](03-01-05-die-layer4-data-distribution.md) — Downstream.
- [MME Overview](../market-monitoring-engine/03-02-01-mme-overview-spec.md) — Warm-up consumer.
- [Database Schema](../../integration-and-api/06-02-database-schema-spec.md) — `market_snapshots` warm base.
