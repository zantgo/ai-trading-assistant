# Analysis Matrix Specification

**Version:** 6.10 (2026-08-05) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 3 — Analysis Layer
**Purpose:** This document defines the physical schema, classification vocabulary, and derivation contract of the **Analysis Matrix** — the market interpretation object. It transforms multi-timeframe agreement into a complete diagnosis: categorical bias, a continuous `market_bias_score`, regime classification, and **six qualitative assessments**.

---

## 1. Conceptual Definition

The Analysis Matrix represents the transition **from observation to understanding**. It consumes the [Alignment Matrix](02-01-alignment-matrix.md) and produces a structured interpretation of the asset's technical environment.

Its two headline outputs are:

1. **Categorical bias** — a five-state directional classification (`MarketBias`).
2. **Continuous market bias score** — a normalized value in `[-1.0, +1.0]` (surfaced from the alignment overall score scaled to `[-100, 100]`), where `-1.0` is absolute bearish and `+1.0` is absolute bullish.

```
[Alignment Matrix] ──► ANALYSIS LAYER (L3) ──► [Analysis Matrix]
                          derive_analysis()        (bias + regime + 6 assessments)
```

Implemented as `AnalysisMatrix` (`crates/core-domain/src/analysis.rs`), produced by `derive_analysis()`.

---

## 2. Physical Schema

### 2.1 AnalysisMatrix Fields

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | `string` | Entity under analysis. |
| `bias` | `MarketBias` | Categorical directional bias (§3.1). |
| `state_confidence` | `f64` | Interpretation confidence in `[0.0, 1.0]`. *(Renamed from `confidence` in the institutional redesign; see [02-00b-confidence-hierarchy.md](02-00b-confidence-hierarchy.md).)* |
| `market_regime` | `MarketRegime` | Structural regime (§3.2). |
| `trend_assessment` | `TrendAssessment` | Trend-quality classification (§3.3). |
| `momentum_assessment` | `MomentumAssessment` | Momentum-state classification (§3.4). |
| `structure_assessment` | `StructureAssessment` | Structural-integrity classification (§3.5). |
| `volatility_assessment` | `VolatilityAssessment` | Volatility-state classification (§3.6). |
| `volume_assessment` | `VolumeAssessment` | Participation classification (§3.7). |
| `market_quality` | `QualityLevel` | Aggregate environment quality. Categorical enum (`POOR / WEAK / AVERAGE / GOOD / EXCELLENT`) used by Decision Matrix `MarketStance` derivation and the GUI. |
| `market_quality_score` | `f64` | Raw numeric mean of the per-dimension scores (trend, momentum, structure, volume) in `[0, 100]`. The numeric companion to `market_quality`, consumed by the Layer 6 `confluence_score` formula and other downstream numeric aggregations. When unavailable at the L3 boundary, callers must map `QualityLevel → f64` via the §3.8 numeric bands. |
| `market_phase` | `MarketPhase` | Wyckoff-style market-cycle phase: 4 phases (`ACCUMULATION` / `MARKUP` / `DISTRIBUTION` / `MARKDOWN`) + `UNKNOWN` empty-state sentinel (§3.9). |
| `market_interpretation` | `string` | Human-readable natural-language summary. |
| `rationale` | `string` | Explainability trace of the derivation. |
| `supporting_signals` | `string[]` | Per-TF observations agreeing with `bias`. |
| `contradicting_signals` | `string[]` | Per-TF observations opposing `bias`. |
| `timeframes_considered` | `u8` | Timeframe count inherited from alignment. |

### 2.2 The Continuous Market Bias Score

The `market_bias_score ∈ [-1.0, 1.0]` referenced throughout the platform is the Alignment Matrix's `mtf_overall_score` (range `[-100, 100]`) divided by 100. The categorical `bias` is a bucketing of this same underlying score.

---

## 3. Classification Vocabularies

### 3.1 MarketBias

| Variant | `mtf_overall_score ∈ [-100, 100]` band | Meaning |
|---------|--------------------------|---------|
| `STRONG_BULLISH` | `> 40` | Dominant bullish conviction. |
| `BULLISH` | `> 20 AND ≤ 40` | Moderate bullish lean. |
| `NEUTRAL` | `≥ -20 AND ≤ 20` | No directional edge. |
| `BEARISH` | `≥ -40 AND < -20` | Moderate bearish lean. |
| `STRONG_BEARISH` | `< -40` | Dominant bearish conviction. |

> **Half-open intervals.** The bands are pinned to half-open intervals to keep `score = 20.0`, `40.0`, etc. from double-mapping: `STRONG_BULLISH = (40, 100]`, `BULLISH = (20, 40]`, `NEUTRAL = [-20, 20]`, `BEARISH = [-40, -20)`, `STRONG_BEARISH = [-100, -40)`. The same score never maps to two bands.

### 3.2 MarketRegime

`TRENDING_BULL`, `TRENDING_BEAR`, `RANGE`, `ACCUMULATION`, `DISTRIBUTION`, `EXPANSION`, `CONTRACTION`, `TRANSITION`.

> **Enum disambiguation.** `MarketRegime.ACCUMULATION/DISTRIBUTION` and `MarketPhase.ACCUMULATION/DISTRIBUTION` (§3.9) are different enums with different derivations; context determines which is meant. `MarketRegime` is a structural-regime classifier derived from ADX, BBWP, and score direction; `MarketPhase` is a Wyckoff-style market-cycle phase derived from volume trend, price trend, and structure slope.

**Canonical decision tree** (priority 1 → 6; first match wins). Uses `score = mtf_overall_score ∈ [-100, 100]`, `adx` = the ADX indicator value on the instance's **macro timeframe** (L1 Metrics, `[0, 100]`), `bbwp` = the BBWP indicator's raw percentile output on the macro timeframe (L1 Metrics, `[0, 100]`), and `regime_one_bar_ago = prior Assessment Layer regime`.

| Priority | Condition | Regime |
|----------|-----------|--------|
| 1 | `bbwp ≥ 85` | `EXPANSION` |
| 1 | `bbwp ≤ 10` | `CONTRACTION` |
| 2 | `adx ≥ 25` AND `score > +20` | `TRENDING_BULL` |
| 2 | `adx ≥ 25` AND `score < -20` | `TRENDING_BEAR` |
| 3 | Rising score (positive 3-bar slope) AND `score ≥ 0` AND not in priority 1 | `ACCUMULATION` |
| 4 | Falling score (negative 3-bar slope) AND `score ≤ 0` AND not in priority 1 | `DISTRIBUTION` |
| 5 | `adx < 25` AND `bbwp ∈ (10, 85)` AND regime shifted within last **3** bars | `TRANSITION` |
| 6 | default (none of the above) | `RANGE` |

The decision tree deterministically produces all 8 variants. Empty/initial state defaults to `TRANSITION` (§6).

> **Layer-specific BBWP thresholds (intentional divergence).** This L3 decision tree uses `bbwp ≤ 10` for `CONTRACTION` and `bbwp ≥ 85` for `EXPANSION`. The Layer 1 local 4-state regime in [03-02-02-mme-layer1-metrics.md §6](../engines/market-monitoring-engine/03-02-02-mme-layer1-metrics.md) uses looser thresholds (`bbwp ≤ 15` for `COMPRESSION`, `bbwp ≥ 85` for `EXPANSION`). A value in `[10, 15]` therefore classifies as `COMPRESSION` at Layer 1 but as `RANGE` (or `TRANSITION`) at Layer 3 — both states are valid for their respective layers. This document is authoritative for the L3 classifier; Layer 1's looser threshold is documented in the layer 1 spec.

### 3.3 TrendAssessment
`WEAK`, `DEVELOPING`, `HEALTHY`, `STRONG`, `EXHAUSTED`, `UNKNOWN` — derived from alignment dimension 0 (trend).

### 3.4 MomentumAssessment
`INCREASING`, `STABLE`, `WEAKENING`, `REVERSING`, `UNKNOWN` — derived from alignment dimension 1 (momentum).

### 3.5 StructureAssessment
`STRONG`, `HEALTHY`, `WEAK`, `BROKEN`, `UNKNOWN` — derived from alignment dimension 4 (structure). *(Renamed from `UNCLEAR` — the canonical empty-state sentinel for every assessment/phase enum is `UNKNOWN`.)*

### 3.6 VolatilityAssessment
`COMPRESSED`, `NORMAL`, `EXPANDING`, `EXTREME`, `UNSTABLE`, `UNKNOWN` — derived from alignment dimension 3 (volatility).

### 3.7 VolumeAssessment
`WEAK`, `NORMAL`, `STRONG`, `EXCEPTIONAL`, `UNKNOWN` — derived from alignment dimension 2 (volume).

> **`UNKNOWN` sentinel.** Every assessment enum admits `UNKNOWN` as its empty-state value (§6). For Structure, `UNKNOWN` is also the §4.2 fall-through band (score `< 20`); for the other four enums it is reachable only via the empty state. Enum values serialize as `SCREAMING_SNAKE_CASE`.

### 3.8 QualityLevel
`POOR`, `WEAK`, `AVERAGE`, `GOOD`, `EXCELLENT` — computed as the mean of the trend, momentum, structure, and volume dimension scores. Numeric bands:

| QualityLevel | `mean(0,1,2,4)` Score |
|--------------|------------------------|
| `POOR` | **< 30** |
| `WEAK` | **≥ 30 AND < 50** |
| `AVERAGE` | **≥ 50 AND < 70** |
| `GOOD` | **≥ 70 AND < 85** |
| `EXCELLENT` | **≥ 85** |

### 3.9 MarketPhase
Four phases — `ACCUMULATION`, `MARKUP`, `DISTRIBUTION`, `MARKDOWN` — plus the `UNKNOWN` empty-state sentinel (§6). Wyckoff-style market-cycle phase. Derived from volume trend + price trend + structure slope:

> **Enum disambiguation.** `MarketPhase.ACCUMULATION/DISTRIBUTION` and `MarketRegime.ACCUMULATION/DISTRIBUTION` (§3.2) are different enums with different derivations; context determines which is meant. `MarketPhase` is a Wyckoff-style market-cycle phase derived from volume trend, price trend, and structure slope; `MarketRegime` is a structural-regime classifier derived from ADX, BBWP, and score direction.

| Phase | Condition |
|-------|-----------|
| `ACCUMULATION` | price ranging (low volatility) + rising volume_assessment (WEAK → STRONG) + structure healthy |
| `MARKUP` | price trending up + volume STRONG/EXCEPTIONAL + bias BULLISH/STRONG_BULLISH |
| `DISTRIBUTION` | price ranging (low volatility) + falling volume_assessment (STRONG → WEAK) + structure weakening |
| `MARKDOWN` | price trending down + volume STRONG/EXCEPTIONAL + bias BEARISH/STRONG_BEARISH |

---

## 4. Derivation Contract

### 4.1 Confidence Model

```
base = |mtf_overall_score| / 100
state_confidence = base
IF trend_agreement_pct ≥ 75  → state_confidence += 0.15
IF trend_agreement_pct < 50  → state_confidence  = min(state_confidence, 0.5)
IF signal_cross_tf_count ≥ 3 → state_confidence += 0.10
IF timeframes_present ≤ 1    → state_confidence  = min(state_confidence, 0.5)
state_confidence = clamp(state_confidence, 0, 1)
```

### 4.2 Assessment Thresholds

| Assessment | Source dim | Bands |
|-----------|-----------|-------|
| Trend | dim 0 | `≥90` `STRONG` · `≥75` `HEALTHY` · `≥50` `DEVELOPING` · `≥25` `WEAK` · else `EXHAUSTED` |
| Momentum | dim 1 | `≥80` `INCREASING` · `≥60` `STABLE` · `≥40` `WEAKENING` · else `REVERSING` |
| Structure | dim 4 | `≥80` `STRONG` · `≥60` `HEALTHY` · `≥40` `WEAK` · `≥20` `BROKEN` · else `UNKNOWN` |
| Volatility | dim 3 | `≥90` `EXTREME` · `≥70` `EXPANDING` · `≥40` `NORMAL` · `≥20` `COMPRESSED` · else `UNSTABLE` |
| Volume | dim 2 | `≥90` `EXCEPTIONAL` · `≥70` `STRONG` · `≥40` `NORMAL` · else `WEAK` |

### 4.3 *(Removed in the institutional redesign)*

The opportunity-selection decision tree has been **moved to the Opportunity Matrix** ([02-08-opportunity-matrix.md §4](02-08-opportunity-matrix.md#4-setup-selection-rule)) because the `OpportunityType` field is a forecast (L4-owned) and not a state interpretation (L3-owned). See [02-00-matrix-field-ownership.md](02-00-matrix-field-ownership.md) for the canonical mapping.

### 4.4 Explainability

The `rationale` and `market_interpretation` strings are generated deterministically from the numeric derivation, satisfying the ontology's **Explainability** principle: every categorical output traces back to the alignment scores and per-timeframe evidence recorded in `supporting_signals` / `contradicting_signals`.

---

## 5. JSON Serialization Contract

```json
{
  "symbol": "BTC-USDT",
  "bias": "BULLISH",
  "state_confidence": 0.65,
  "market_regime": "TRENDING_BULL",
  "trend_assessment": "HEALTHY",
  "momentum_assessment": "STABLE",
  "structure_assessment": "HEALTHY",
  "volatility_assessment": "EXPANDING",
  "volume_assessment": "STRONG",
  "market_quality": "GOOD",
  "market_interpretation": "Bullish trending market with healthy trend, stable momentum, healthy structure, expanding volatility, and strong volume participation. Favors trend continuation.",
  "rationale": "state_confidence = |40|/100 + 0.15 (agreement 75%) + 0.10 (3 cross-TF signals) = 0.65",
  "supporting_signals": ["fast180 (bullish): score +42, TRENDING regime, 3 signals"],
  "contradicting_signals": [],
  "timeframes_considered": 4
}
```

Enum values serialize as `SCREAMING_SNAKE_CASE`.

---

## 6. Empty State

When `timeframes_present == 0`, `derive_analysis` returns `AnalysisMatrix::empty()`. All defaults:

| Field | Empty-state value |
|-------|-------------------|
| `bias` | `NEUTRAL` |
| `state_confidence` | `0.0` |
| `market_regime` | `TRANSITION` |
| `market_quality` | `POOR` |
| `market_quality_score` | `0.0` |
| `market_phase` | `UNKNOWN` |
| `trend_assessment` | `UNKNOWN` |
| `momentum_assessment` | `UNKNOWN` |
| `structure_assessment` | `UNKNOWN` |
| `volatility_assessment` | `UNKNOWN` |
| `volume_assessment` | `UNKNOWN` |
| `market_interpretation` | `"No data available — no candles have been completed."` |
| `supporting_signals` | `[]` |
| `contradicting_signals` | `[]` |
| `timeframes_considered` | `0` |

---

## 7. Cross-References

- [Alignment Matrix](02-01-alignment-matrix.md) — Sole input.
- [Opportunity Matrix](02-08-opportunity-matrix.md) — Consumes Analysis Matrix state (`bias`, `market_quality`, `state_confidence`, qualitative assessments) and is the canonical producer of `OpportunityType` (formerly `opportunity_analysis`).
- [Risk Matrix](02-11-risk-matrix.md) — Consumes the full Analysis Matrix.
- [Decision Matrix](02-04-decision-matrix.md) — Downstream synthesis.
- [MME Layer 3 — Analysis](../engines/market-monitoring-engine/03-02-04-mme-layer3-analysis.md) — Producing-layer specification.
