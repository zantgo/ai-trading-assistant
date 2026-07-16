# Timeframe Model Specification

**Version:** 5.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Approved
**Purpose:** This document defines the configurable 4-tier timeframe model used by the Market Monitoring Engine. Every Market Instance runs 4 independent timeframe pipelines — micro, fast, slow, and macro — producing per-timeframe Metrics Matrices that feed the multi-timeframe Alignment layer.

---

## 1. The 4-Tier Model

The engine uses a fixed 4-tier structure, but each tier's duration is **configurable per session** via `config.toml`. The tiers are always ordered fastest to slowest:

| Tier | Name | Default Duration | Default Label | Config Key |
|------|------|-----------------|---------------|------------|
| 1 | **Micro** | 60 s (1 minute) | `micro60` | `candles.duration_seconds` |
| 2 | **Fast** | 180 s (3 minutes) | `fast180` | `fast_timeframe.duration_seconds` |
| 3 | **Slow** | 300 s (5 minutes) | `slow300` | `slow_timeframe.duration_seconds` |
| 4 | **Macro** | 900 s (15 minutes) | `macro900` | `macro_timeframe.duration_seconds` |

The `fast_timeframe`, `slow_timeframe`, and `macro_timeframe` objects each have an `enabled` toggle (`true`/`false`) and a `duration_seconds` parameter. The micro timeframe is always active (it is the base candle duration from `candles`).

> **Sub-minute durations (v2.1).** The 4-tier model supports any positive integer duration via `config.toml`. The micro tier default is 60 s, but operators may configure sub-minute durations (e.g. 15 s, 30 s) for high-frequency strategies by setting `[candles.duration_seconds]` to the desired value. Sub-minute timeframes are not documented in the standard 4-tier ladder because most institutional strategies operate at 1m+ resolution; they are supported by the underlying pipeline (and by the reconstruction engine — see [08-04-candle-reconstruction.md](../operations-and-compliance/08-04-candle-reconstruction.md)) but require explicit configuration.

---

## 2. Configuration

All four tiers are configured in `config.toml`:

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

The aggregator formula — `interval_start = ⌊timestamp_ms / duration_ms⌋ × duration_ms` — deterministically produces the **start of the candle interval** as the integer epoch multiple. The **closing instant** of the candle is `interval_start + duration_ms` (i.e. the start of the next interval). Candles close on the integer epoch multiple (e.g. a `micro60` candle closing at the start of the next minute), never at `:59.999`.

- **Late-Trade Recovery:** Any late-arriving trade whose exchange-server timestamp belongs to a prior time boundary is processed as a retroactive update to the historical buffer. It must never cause the active candle boundary to shift or delay its close.
- **Clock Drift:** Local server system clocks execute continuous NTP polling to keep local system time drift under $\le 50 \text{ microseconds}$ of UTC, ensuring local indicator values align exactly with exchange historical benchmarks. Drift is enforced at runtime by `crates/network-adapters/src/clock_monitor.rs` (spawned from `main.rs`, configured via the `"clock_monitor"` block of `config.toml`). See [Global Architecture §2.1](01-02-global-architecture.md).

---

## 4. Cross-Timeframe Weighting

Higher timeframes carry more weight in the Alignment layer's consensus calculations:

$$w_{tf} = \text{clamp}\left(\frac{\text{duration\_seconds}}{\text{divisor}},\ 0.2,\ 1.0\right)$$

The divisor is the session's **slowest enabled tier's duration** (the slowest tier with `enabled = true`). This keeps the hierarchy intact for any configured session — the slowest active tier always weights `1.0` and shorter tiers scale down proportionally, preserving the semantic ordering micro ≤ fast ≤ slow ≤ macro.

**Divisor selection rule.** The denominator is determined by the **slowest active** tier, not unconditionally `macro_duration_seconds`:

```
divisor = max({duration_seconds for tier in enabled_tiers})  // slowest active tier wins
```

- Default config (micro=60, fast=180, slow=300, macro=900, all enabled): divisor = 900 s. Equivalent to the original definition.
- If `macro_timeframe.enabled = false` (a swing trader running `micro / fast / slow / off`): divisor = 300 s. Slow tier still weights `1.0`, micro/fast scale below it.
- If only `micro` is enabled (a single-tier config): divisor = 60 s. The lone micro tier still weights `1.0` (the clamp `min(w, 1.0)`); all other tiers are absent from the consensus.

> **Why dynamic.** A previous version of this section used `macro_duration_seconds` as the divisor unconditionally. When the macro tier is disabled, the divisor stays at its default value (e.g. 900 s for a default session) even though no tier uses it. The slowest *actual* tier gets clamped to its minimum (0.20) and the weighting carries no information about the real consensus structure. The dynamic divisor preserves the semantic ordering for any active subset of tiers.

With default durations (macro = 900 s, all enabled) this yields:

| Tier | Duration | Weight |
|------|----------|--------|
| micro | 60 s | 0.20 |
| fast | 180 s | 0.20 |
| slow | 300 s | 0.33 |
| macro | 900 s | 1.00 |

This rule is shared by [Alignment Matrix §4.1](../matrices/02-01-alignment-matrix.md) and [MME Layer 2 §3](../../engines/market-monitoring-engine/03-02-03-mme-layer2-alignment.md) — the formula and divisor rule are identical at all three locations.

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
