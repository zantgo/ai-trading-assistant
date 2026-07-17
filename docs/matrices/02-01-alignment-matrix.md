# Alignment Matrix Specification

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 2 — Alignment Layer
**Purpose:** This document defines the physical schema, computation contract, and 10-dimensional agreement model of the **Alignment Matrix** — the multi-timeframe consensus object. It measures whether independent Metrics Matrices for the same symbol, computed at different temporal resolutions, describe the *same* market condition.

---

## 1. Conceptual Definition

The Alignment Matrix answers a single question: **Do multiple timeframes of the same entity agree?**

Where the [Metrics Matrix](02-07-metrics-matrix.md) measures *local confluence* (agreement among indicators within one timeframe), the Alignment Matrix measures *cross-timeframe agreement* (agreement among timeframes). It consumes a set of Metrics Matrices — one per timeframe for a single symbol — and produces a unified consensus object.

```
[Metrics Matrix: micro 60s ]─┐
[Metrics Matrix: fast  180s]─┤
[Metrics Matrix: slow  300s]─┼──► ALIGNMENT LAYER (L2) ──► [Alignment Matrix]
[Metrics Matrix: macro 900s]─┘         (10 dimensions)
```

The Alignment Matrix is implemented as `AlignmentMatrix` (`crates/core-domain/src/alignment.rs`), produced by `compute_alignment()`.

---

## 2. Physical Schema

### 2.1 AlignmentMatrix Fields

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | `string` | The entity under analysis. |
| `timeframes_present` | `u8` | Count of timeframes contributing (1–4). |
| `dimensions` | `AlignmentDimension[10]` | The 10 alignment dimensions (ordered — see §3). |
| `mtf_trend_alignment` | `f64` | Weighted signed trend consensus `[-1, 1]`. |
| `mtf_momentum_alignment` | `f64` | Weighted signed momentum consensus `[-1, 1]`. |
| `mtf_volume_alignment` | `f64` | Weighted signed volume consensus `[-1, 1]`. |
| `mtf_volatility_alignment` | `f64` | Weighted signed volatility consensus `[-1, 1]`. |
| `mtf_overall_score` | `f64` | Blended MTF score in `[-100, 100]`. |
| `mtf_overall_label` | `string` | `STRONG_BULL_MTF` / `WEAK_BULL_MTF` / `NEUTRAL_MTF` / `WEAK_BEAR_MTF` / `STRONG_BEAR_MTF` / `NO_DATA`. |
| `timeframe_alignments` | `TfAlignmentInfo[]` | Per-timeframe breakdown (see §4). |
| `signal_cross_tf_count` | `u32` | Number of distinct signal keys (indicator × SignalKind) with an active signal on ≥2 timeframes in the current snapshot. |
| `trend_agreement_pct` | `f64` | Percentage of timeframes agreeing on direction `[0, 100]`. |

### 2.2 AlignmentDimension

| Field | Type | Range | Description |
|-------|------|-------|-------------|
| `score` | `f64` | `[0, 100]` | Alignment strength (higher = stronger agreement). |
| `state` | `AlignState` | — | `BULLISH` / `BEARISH` / `NEUTRAL` / `MIXED` / `ALIGNED` / `PARTIAL` / `DIVERGENT`. |
| `confidence` | `f64` | `[0, 100]` | Measurement confidence. |

### 2.3 TfAlignmentInfo (per-timeframe breakdown)

| Field | Type | Description |
|-------|------|-------------|
| `timeframe` | `string` | Label, e.g. `fast180`. |
| `timeframe_secs` | `u64` | Duration in seconds. |
| `trend_score` | `f64` | Local trend score `[-1, 1]`. |
| `momentum_score` | `f64` | Local momentum score `[-1, 1]`. |
| `overall_score` | `i32` | Local overall conviction `[-100, 100]`. |
| `regime` | `string` | Local regime classification. |
| `active_signals` | `u32` | Number of signals active on this timeframe. |
| `price` | `f64` | Reference price at this timeframe. |

---

## 3. The 10 Alignment Dimensions

The `dimensions` array is ordered. Each index maps to a specific agreement axis:

| # | Dimension | Measures | Computation Basis |
|---|-----------|----------|-------------------|
| 0 | **Trend** | Directional trend agreement | Weighted mean of per-TF `MarketContext.trend.score`, mapped signed→`[0,100]`. |
| 1 | **Momentum** | Momentum-vector agreement | Weighted mean of per-TF momentum scores. |
| 2 | **Volume** | Participation agreement | Weighted mean of per-TF volume scores. |
| 3 | **Volatility** | Volatility-regime agreement | Weighted mean of per-TF volatility scores. |
| 4 | **Structure** | S/R role agreement | % of TFs whose support/resistance label agrees. |
| 5 | **Signal** | Cross-TF signal confluence | % of signals appearing in ≥2 TFs. |
| 6 | **Regime** | Regime-classification agreement | % of TFs sharing the dominant regime. |
| 7 | **Confidence** | Confidence consistency | `100 − sample_stddev(per_tf_confidence_scores)` (Bessel-corrected sample standard deviation, `N − 1` denominator). For N ≤ 1 timeframe, sample stddev is undefined and confidence defaults to the mean per-TF confidence score. |
| 8 | **Liquidity** | RVOL consistency | `(1 − coefficient_of_variation)` of RVOL across TFs. |
| 9 | **Tradability** | Cross-timeframe tradability agreement | % of TFs with non-neutral bias and non-compressed regime. *(Renamed from "Opportunity" in the institutional redesign — the L4 Opportunity Matrix is the canonical owner of opportunity concepts; this dimension measures TFs agreeing on whether conditions are tradable.)* |

### 3.1 AlignState Derivation

The `AlignState` enum grows from four to seven values: `BULLISH` | `BEARISH` | `NEUTRAL` | `MIXED` | `ALIGNED` | `PARTIAL` | `DIVERGENT`.

**Signed dimensions** (Trend, Momentum, Volume, Volatility, Confidence):

- Inputs: signed mean `m ∈ [-1, 1]`; sign-agreement `a` = fraction of timeframes sharing the majority sign.
- If `a < 0.6` → `MIXED`.
- Else if `m > +0.3` → `BULLISH`.
- Else if `m < -0.3` → `BEARISH`.
- Else → `NEUTRAL`.

**Unsigned dimensions** (Structure, Signal, Regime, Liquidity, Tradability):

- `score ≥ 60` → `ALIGNED`.
- `30 ≤ score < 60` → `PARTIAL`.
- `score < 30` → `DIVERGENT`.

---

## 4. Computation Contract

### 4.1 Timeframe Weighting

Each contributing timeframe is weighted by its duration, favouring higher timeframes:

$$w_{tf} = \text{clamp}\left(\frac{\text{duration\_seconds}}{\text{divisor}},\ 0.2,\ 1.0\right)$$

The divisor is the session's **slowest enabled** tier's duration (see [Timeframe Model §4](../conceptual-foundations/01-04-timeframe-model.md)). The slowest tier always weights `1.0`; shorter tiers scale down proportionally. This is dynamic rather than the fixed `900 s` constant so custom sessions (e.g. macro = 1 d, or `macro_timeframe.enabled = false`) retain a proper hierarchy instead of clamping the slowest active tier to `1.0` (or, with the bug-fixed rule, leaving an inactive macro as the divisor with no active tier above the clamp ceiling).

**Divisor rule:** `divisor = max({duration_seconds for tier in enabled_tiers})`. With default durations (micro=60, fast=180, slow=300, macro=900, all enabled): `divisor = 900 s`.

The weighted consensus for a dimension is:

$$\text{mtf\_alignment} = \text{clamp}\left(\frac{\sum_{tf} \text{score}_{tf} \cdot w_{tf}}{\sum_{tf} w_{tf}},\ -1,\ 1\right)$$

### 4.2 Overall Blend

The four signed consensus scores are blended with fixed weights:

$$\text{mtf\_overall\_score} = \text{clamp}\big((0.5\,T + 0.3\,M + 0.1\,V_{t} + 0.1\,V_{m}) \times 100,\ -100,\ 100\big)$$

where `T` = trend alignment, `M` = momentum alignment, `V_t` = volatility alignment, `V_m` = volume alignment.

### 4.3 Trend Agreement Percentage

$$\text{trend\_agreement\_pct} = \frac{\max(\text{positive\_tf\_count}, \text{negative\_tf\_count})}{\text{total\_tf}} \times 100$$

### 4.4 Cross-Timeframe Signal Count

When ≥2 timeframes are present:

`signal_cross_tf_count` = number of distinct signal keys (indicator × SignalKind) with an active signal on ≥2 timeframes in the current snapshot.

### 4.5 Overall Label

```
score ≥  60 → STRONG_BULL_MTF
score ≥  20 → WEAK_BULL_MTF
score ≤ -60 → STRONG_BEAR_MTF
score ≤ -20 → WEAK_BEAR_MTF
otherwise   → NEUTRAL_MTF
```

---

## 5. Empty / Degenerate States

| Condition | Result |
|-----------|--------|
| No timeframes supplied | `AlignmentMatrix::empty()` → `timeframes_present = 0`, `mtf_overall_label = "NO_DATA"`, 10 zero-score dimensions. |
| Single timeframe | Dimensions still computed but agreement percentages reflect a single data point; confidence dimensions default to 50. |
| Missing indicator (e.g. no S/R) | The affected dimension degrades to score `0` rather than failing. |

---

## 6. JSON Serialization Contract

```json
{
  "symbol": "BTC-USDT",
  "timeframes_present": 4,
  "dimensions": [
    { "score": 78.0, "state": "BULLISH", "confidence": 78.0 },
    { "score": 65.0, "state": "BULLISH", "confidence": 65.0 },
    { "score": 55.0, "state": "NEUTRAL", "confidence": 55.0 },
    { "score": 60.0, "state": "BULLISH", "confidence": 60.0 },
    { "score": 33.3, "state": "DIVERGENT", "confidence": 33.3 },
    { "score": 75.0, "state": "ALIGNED", "confidence": 75.0 },
    { "score": 100.0, "state": "ALIGNED", "confidence": 100.0 },
    { "score": 88.0, "state": "BULLISH", "confidence": 88.0 },
    { "score": 70.0, "state": "ALIGNED", "confidence": 70.0 },
    { "score": 100.0, "state": "ALIGNED", "confidence": 100.0 }
  ],
  "mtf_trend_alignment": 0.56,
  "mtf_momentum_alignment": 0.30,
  "mtf_volume_alignment": 0.10,
  "mtf_volatility_alignment": 0.20,
  "mtf_overall_score": 40.0,
  "mtf_overall_label": "WEAK_BULL_MTF",
  "timeframe_alignments": [
    { "timeframe": "micro60", "timeframe_secs": 60, "trend_score": 0.5,
      "momentum_score": 0.3, "overall_score": 42, "regime": "TRENDING",
      "active_signals": 3, "price": 64012.5 }
  ],
  "signal_cross_tf_count": 3,
  "trend_agreement_pct": 75.0
}
```

---

## 7. Lifecycle & Guarantees

| Property | Guarantee |
|----------|-----------|
| **Determinism** | Identical per-TF Metrics Matrices produce an identical Alignment Matrix. |
| **Dimensional invariant** | `dimensions.len()` is **always** 10, even in the empty state. |
| **Bounded scores** | All dimension scores clamp to `[0, 100]`; MTF blends clamp to `[-100, 100]`. |
| **Upstream isolation** | The Alignment Matrix reads only completed Metrics Matrices; it never triggers indicator recomputation. |

---

## 8. Cross-References

- [Metrics Matrix](02-07-metrics-matrix.md) — Per-timeframe input.
- [Analysis Matrix](02-02-analysis-matrix.md) — Direct downstream consumer (`derive_analysis`).
- [MME Layer 2 — Alignment](../engines/market-monitoring-engine/03-02-03-mme-layer2-alignment.md) — Producing-layer specification.
- [Ontology — Alignment](../conceptual-foundations/01-01-ontology.md) — Conceptual definition.
