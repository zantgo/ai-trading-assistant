# DIE Layer 2 — Market Data Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Data Infrastructure Engine (DIE)
**Layer:** 2 of 4
**Output Contract:** Market Data Matrix (multi-timeframe `NormalizedCandle`s)
**Purpose:** This document specifies the Market Data Layer — the tier that transforms the raw event stream into structured, uniform temporal boundaries (OHLCV candles) across multiple timeframes, and audits time-sync latency.

---

## 1. Purpose

The Market Data Layer converts irregular, event-based ticks into regular, time-boxed OHLCV bars. It answers *"what did the market do in each fixed interval?"* and produces the multi-timeframe candle set that is the foundation of all downstream analysis.

```
[NormalizedEvent: Trade] ──► MARKET DATA LAYER (L2) ──► [NormalizedCandle × N timeframes]
                               CandleGenerator (per-tick)
                               CandleAggregator (rollup)
```

---

## 2. Output Contract: NormalizedCandle

```rust
struct NormalizedCandle {
    symbol: String,
    start_time_ms: u64,
    duration_ms: u64,
    open: Decimal, high: Decimal, low: Decimal, close: Decimal,
    volume: Decimal,
    trades_count: u64,
}
```

Every candle carries `assert_validity()` invariants (enforced downstream): `high ≥ low`, `open`/`close ∈ [low, high]`, `volume ≥ 0`.

> **Target Architecture (Not Yet Implemented).** The current candle history is an **Array of Structures (AoS)** — a collection of `NormalizedCandle` structs with `Decimal` OHLCV fields. The target hot-path model replaces this with a **Structure of Arrays (SoA)** so that all historical values of a field reside contiguously in the CPU cache for downstream indicator loops:
>
> ```rust
> pub struct ContiguousCandleHistory {
>     pub opens:   [f64; 1000],
>     pub highs:   [f64; 1000],
>     pub lows:    [f64; 1000],
>     pub closes:  [f64; 1000],
>     pub volumes: [f64; 1000],
>     pub write_ptr: usize,
> }
> ```
>
> This lets the compiler load each column into vector registers and auto-vectorize (SIMD) the indicator math. It is a target design; the AoS `NormalizedCandle` contract above remains authoritative for the current implementation.

---

## 3. Real-Time Candle Generation

The `CandleGenerator` (`crates/shared/src/normalized/candle_generator.rs`) builds candles tick-by-tick. On each trade it returns a tuple `(Option<completed>, live)`:

| Situation | Behaviour |
|-----------|-----------|
| First trade | Initializes the candle; returns `(None, live)`. |
| Trade within current interval | Updates high/low/close/volume/count; returns `(None, live)`. |
| Trade crosses interval boundary | Emits the completed candle; opens a new one; returns `(Some(completed), live)`. |

### 3.1 Interval Alignment

Candles align to epoch buckets:

$$\text{interval\_start} = \left\lfloor \frac{\text{timestamp\_ms}}{\text{duration\_ms}} \right\rfloor \times \text{duration\_ms}$$

For example, a 60 s candle for a trade at `123456 ms` aligns to `120000 ms`.

**UTC boundary map** (closing instant of each candle, by tier): candle boundaries are *exact epoch-duration multiples of UTC*. `micro60` closes at `:00.000` of every minute (the next minute's start); `fast180` closes at `:03:00.000`, `:06:00.000`, `:09:00.000`, … (top of every third minute); `slow300` closes at `:05:00.000`, `:10:00.000`, `:15:00.000`, … (top of every fifth minute); `macro900` closes at `:00:00.000`, `:15:00.000`, `:30:00.000`, `:45:00.000` of every hour. The aggregator formula `interval_start = ⌊timestamp_ms / duration_ms⌋ × duration_ms` deterministically produces these boundaries, so candles always close on the integer epoch multiple, never at a `.999` sub-second offset.

**Clock-drift budget:** Local server system clocks execute continuous NTP polling to keep local time drift under $\le 50 \text{ microseconds}$ of UTC, ensuring locally computed indicator values match exchange historical benchmarks to the millisecond. See [Global Architecture §2.1](../../conceptual-foundations/01-02-global-architecture.md) and [Timeframe Model §3.1](../../conceptual-foundations/01-04-timeframe-model.md).

### 3.2 Live "Shadow" Candles

The `live` candle returned on every tick is the **shadow** value — the real-time flickering state of the in-progress candle. It powers live dashboard updates but does **not** feed downstream matrices; only the `completed` candle (`is_completed = true`) triggers the analytical cascade.

---

## 4. Multi-Timeframe Aggregation

The platform monitors four timeframes per instance. The base (micro) timeframe is generated directly from ticks; higher timeframes are rolled up.

### 4.1 Standard Timeframe Ladder

| Tier | Default Duration | Source |
|------|------------------|--------|
| Micro | 60 s | Direct from ticks (`CandleGenerator`). |
| Fast | 180 s | Rollup / dedicated generator. |
| Slow | 300 s | Config `[slow_timeframe]`. |
| Macro | 900 s (configurable, e.g. 3600 s / 86400 s) | Config `[macro_timeframe]` + `CandleAggregator`. |

### 4.2 Higher-Timeframe Aggregation

The `CandleAggregator` (`crates/engine/src/candle_aggregator.rs`) rolls the base micro candle stream into the configured `fast`, `slow`, and `macro` timeframe buckets. The duration of each tier is configured in `config.json` (`fast_timeframe.duration_seconds`, `slow_timeframe.duration_seconds`, `macro_timeframe.duration_seconds`); the default ladder is micro 60 s / fast 180 s / slow 300 s / macro 900 s, but other ladders (e.g. 4h, 1d) are produced the same way when configured.

```
1m close ──► process_1m_candle() ──► (Option<fast>, Option<slow>, Option<macro>)
             │
             ├─ update pending_<tf>: high=max, low=min, close=latest, volume+=, count+=
             └─ on interval rollover: emit completed <tf> candle, reset pending
```

Aggregation preserves OHLCV integrity: `high = max(highs)`, `low = min(lows)`, `close = last close`, `volume = Σ volumes`, `trades_count = Σ counts`.

---

## 5. Time-Sync Latency Audit

Because venues timestamp differently and network jitter varies, the Market Data Layer tracks:

| Audit | Purpose |
|-------|---------|
| Ingest-vs-event skew | Difference between local receipt time and `timestamp_ms`. |
| Interval boundary drift | Ensures candles close on aligned epoch boundaries regardless of tick arrival jitter. |
| Cross-venue offset | When a symbol can be sourced from multiple venues, timestamp offsets are reconciled to the unified clock. |

Latency measurements surface through the system status endpoint (`latency_ms`).

---

## 6. Guarantees

| Property | Guarantee |
|----------|-----------|
| **Uniform intervals** | Every completed candle spans exactly `duration_ms` aligned to epoch buckets. |
| **OHLCV integrity** | Aggregation and generation preserve open/high/low/close/volume semantics. |
| **Shadow separation** | Live shadow candles never contaminate the completed-candle analytical path. |
| **Determinism** | Identical trade sequences yield identical candle sets. |

---

## 7. Cross-References

- [DIE Layer 1 — Raw Data](03-01-02-die-layer1-raw-data.md) — Input.
- [DIE Layer 3 — Data Quality](03-01-04-die-layer3-data-quality.md) — Validation & gap-fill.
- [Metrics Matrix](../../matrices/02-07-metrics-matrix.md) — Ultimate consumer of candles.
