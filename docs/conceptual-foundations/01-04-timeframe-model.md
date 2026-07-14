# Timeframe Model Specification

**Version:** 2.0
**Status:** Approved
**Purpose:** This document defines the configurable 4-tier timeframe model used by the Market Monitoring Engine. Every Market Instance runs 4 independent timeframe pipelines — micro, fast, slow, and macro — producing per-timeframe Metrics Matrices that feed the multi-timeframe Alignment layer.

---

## 1. The 4-Tier Model

The engine uses a fixed 4-tier structure, but each tier's duration is **configurable per session** via `config.json`. The tiers are always ordered fastest to slowest:

| Tier | Name | Default Duration | Default Label | Config Key |
|------|------|-----------------|---------------|------------|
| 1 | **Micro** | 60 s (1 minute) | `micro60` | `candles.duration_seconds` |
| 2 | **Fast** | 180 s (3 minutes) | `fast180` | `fast_timeframe.duration_seconds` |
| 3 | **Slow** | 300 s (5 minutes) | `slow300` | `slow_timeframe.duration_seconds` |
| 4 | **Macro** | 900 s (15 minutes) | `macro900` | `macro_timeframe.duration_seconds` |

The `fast_timeframe`, `slow_timeframe`, and `macro_timeframe` objects each have an `enabled` toggle (`true`/`false`) and a `duration_seconds` parameter. The micro timeframe is always active (it is the base candle duration from `candles`).

---

## 2. Configuration

All four tiers are configured in `config.json`:

```json
{
  "candles": {
    "duration_seconds": 60
  },
  "fast_timeframe": {
    "enabled": true,
    "duration_seconds": 180
  },
  "slow_timeframe": {
    "enabled": true,
    "duration_seconds": 300
  },
  "macro_timeframe": {
    "enabled": true,
    "duration_seconds": 900
  }
}
```

The user may change any duration to suit their trading style (e.g., 15min / 1h / 4h / 1d for swing traders). The engine respects the 4-tier structure regardless of the numeric values — the semantics are always "micro < fast < slow < macro."

---

## 3. Pipeline Architecture

Each Market Instance spawns up to 4 concurrent `TimeframePipeline` workers, one per enabled tier:

```
Market Instance: BTC-USDT
├── TimeframePipeline: micro (60 s)
├── TimeframePipeline: fast  (180 s)
├── TimeframePipeline: slow  (300 s)
└── TimeframePipeline: macro (900 s)
```

Per the [MME Concurrency Strategy](../engines/market-monitoring-engine/03-02-01-mme-overview-spec.md#3-concurrency-strategy):

- Each pipeline is isolated — no shared mutable state between timeframe workers.
- Each pipeline runs the full indicator computation chain on every completed candle.
- The output is a per-timeframe Metrics Matrix.
- All pipelines feed the cross-TF synthesis stage (L2–L6) which produces the unified Alignment, Analysis, Opportunity, Risk, and Decision matrices.

### 3.1 UTC Clock Alignment and Aggregation Rules

To maintain perfect synchronization with external exchange servers and prevent index drift, every timeframe pipeline enforces strict UTC clock boundaries.

- **Aggregator Triggering:** The `CandleAggregator` closes and emits completed candles at the exact millisecond of the UTC clock rollover for that timeframe.

**Boundary Map (epoch-duration multiples of UTC):**

Each candle closes at the next exact UTC epoch-duration multiple:

- `micro60` closes at the start of the next minute (`:00.000`).
- `fast180` closes at the top of every third minute (`:03:00.000`, `:06:00.000`, `:09:00.000`, …).
- `slow300` closes at the top of every fifth minute (`:05:00.000`, `:10:00.000`, `:15:00.000`, …).
- `macro900` closes at the top of every fifteenth minute (`:00:00.000`, `:15:00.000`, `:30:00.000`, `:45:00.000`).

The aggregator formula — `interval_start = ⌊timestamp_ms / duration_ms⌋ × duration_ms` — deterministically produces these boundaries, so candles close on the integer epoch multiple, never at `:59.999`.

- **Late-Trade Recovery:** Any late-arriving trade whose exchange-server timestamp belongs to a prior time boundary is processed as a retroactive update to the historical buffer. It must never cause the active candle boundary to shift or delay its close.
- **Clock Drift:** Local server system clocks execute continuous NTP polling to keep local system time drift under $\le 50 \text{ microseconds}$ of UTC, ensuring local indicator values align exactly with exchange historical benchmarks. See [Global Architecture §2.1](01-02-global-architecture.md).

---

## 4. Cross-Timeframe Weighting

Higher timeframes carry more weight in the Alignment layer's consensus calculations:

$$w_{tf} = \text{clamp}\left(\frac{\text{duration\_seconds}}{\text{macro\_duration\_seconds}},\ 0.2,\ 1.0\right)$$

The divisor is the session's **active Macro timeframe duration** (`macro_timeframe.duration_seconds`), not a fixed constant. This keeps the hierarchy intact for any configured session — the Macro tier always weights `1.0` and shorter tiers scale down proportionally, preserving the semantic ordering micro ≤ fast ≤ slow ≤ macro. With default durations (macro = 900 s) this yields:

| Tier | Duration | Weight |
|------|----------|--------|
| micro | 60 s | 0.20 |
| fast | 180 s | 0.20 |
| slow | 300 s | 0.33 |
| macro | 900 s | 1.00 |

---

## 5. Warm-Up & History

Each timeframe pipeline bootstraps from historical candle data before subscribing to live broadcasts. The `analysis_limit` parameter (`[candles] analysis_limit`) controls the lookback depth (default: 500 bars).

---

## 6. Performance Targets

| Metric | Target |
|--------|--------|
| Per-pipeline indicator computation (50 indicators) | < 10 ms |
| Cross-TF synthesis (L2–L6) | < 5 ms |
| Full 7-layer cascade per candle | < 25 ms |

---

## 7. Cross-References

- [Global Architecture](01-02-global-architecture.md) — Engine positioning and 2D framework.
- [MME Overview](../engines/market-monitoring-engine/03-02-01-mme-overview-spec.md) — Instance lifecycle and pipeline model.
- [Alignment Matrix](../matrices/02-01-alignment-matrix.md) — Cross-timeframe agreement (weighted by tier).
- [Systemic Data Flow](01-03-systemic-data-flow.md) — Observation loop sequence.
