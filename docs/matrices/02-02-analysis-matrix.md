# Analysis Matrix Specification

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 3 — Analysis Layer
**Purpose:** This document defines the physical schema, classification vocabulary, and derivation contract of the **Analysis Matrix** — the market interpretation object. It transforms multi-timeframe agreement into a complete diagnosis: categorical bias, a continuous `market_bias_score`, regime classification, and seven qualitative assessments.

---

## 1. Conceptual Definition

The Analysis Matrix represents the transition **from observation to understanding**. It consumes the [Alignment Matrix](02-01-alignment-matrix.md) and produces a structured interpretation of the asset's technical environment.

Its two headline outputs are:

1. **Categorical bias** — a five-state directional classification (`MarketBias`).
2. **Continuous market bias score** — a normalized value in `[-1.0, +1.0]` (surfaced from the alignment overall score scaled to `[-100, 100]`), where `-1.0` is absolute bearish and `+1.0` is absolute bullish.

```
[Alignment Matrix] ──► ANALYSIS LAYER (L3) ──► [Analysis Matrix]
                          derive_analysis()        (bias + regime + 7 assessments)
```

Implemented as `AnalysisMatrix` (`crates/shared/src/analysis.rs`), produced by `derive_analysis()`.

---

## 2. Physical Schema

### 2.1 AnalysisMatrix Fields

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | `string` | Entity under analysis. |
| `bias` | `MarketBias` | Categorical directional bias (§3.1). |
| `confidence` | `f64` | Interpretation confidence in `[0.0, 1.0]`. |
| `market_regime` | `MarketRegime` | Structural regime (§3.2). |
| `trend_assessment` | `TrendAssessment` | Trend-quality classification (§3.3). |
| `momentum_assessment` | `MomentumAssessment` | Momentum-state classification (§3.4). |
| `structure_assessment` | `StructureAssessment` | Structural-integrity classification (§3.5). |
| `volatility_assessment` | `VolatilityAssessment` | Volatility-state classification (§3.6). |
| `volume_assessment` | `VolumeAssessment` | Participation classification (§3.7). |
| `opportunity_analysis` | `OpportunityType` | Setup classification (§3.8). |
| `market_quality` | `QualityLevel` | Aggregate environment quality (§3.9). |
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

| Variant | `mtf_overall_score` band | Meaning |
|---------|--------------------------|---------|
| `STRONG_BULLISH` | `> 40` | Dominant bullish conviction. |
| `BULLISH` | `20 … 40` | Moderate bullish lean. |
| `NEUTRAL` | `-20 … 20` | No directional edge. |
| `BEARISH` | `-40 … -20` | Moderate bearish lean. |
| `STRONG_BEARISH` | `< -40` | Dominant bearish conviction. |

### 3.2 MarketRegime

`TRENDING_BULL`, `TRENDING_BEAR`, `RANGE`, `ACCUMULATION`, `DISTRIBUTION`, `EXPANSION`, `CONTRACTION`, `TRANSITION`.

**Canonical decision tree** (priority 1 → 6; first match wins). Uses `score = mtf_overall_score ∈ [-100, 100]`, `adx = Alignment Matrix dimension 0 score ∈ [0, 100]`, `bbwp = Context.volatility-derived BBWP score ∈ [0, 100]`, and `regime_one_bar_ago = prior Assessment Layer regime`.

| Priority | Condition | Regime |
|----------|-----------|--------|
| 1 | `bbwp ≥ 85` | `EXPANSION` |
| 1 | `bbwp ≤ 10` | `CONTRACTION` |
| 2 | `adx ≥ 25` AND `score > +20` | `TRENDING_BULL` |
| 2 | `adx ≥ 25` AND `score < -20` | `TRENDING_BEAR` |
| 3 | Rising score (positive 3-bar slope) AND `score ≥ 0` AND not in priority 1 | `ACCUMULATION` |
| 4 | Falling score (negative 3-bar slope) AND `score ≤ 0` AND not in priority 1 | `DISTRIBUTION` |
| 5 | `adx < 25` AND `bbwp ∈ (10, 85)` AND `regime_one_bar_ago ≠ current_priority_resolution` | `TRANSITION` |
| 6 | default (none of the above) | `RANGE` |

The decision tree deterministically produces all 8 variants. Empty/initial state defaults to `TRANSITION` (§6).

### 3.3 TrendAssessment
`WEAK`, `DEVELOPING`, `HEALTHY`, `STRONG`, `EXHAUSTED` — derived from alignment dimension 0 (trend).

### 3.4 MomentumAssessment
`INCREASING`, `STABLE`, `WEAKENING`, `REVERSING` — derived from alignment dimension 1 (momentum).

### 3.5 StructureAssessment
`STRONG`, `HEALTHY`, `WEAK`, `BROKEN`, `UNCLEAR` — derived from alignment dimension 4 (structure).

### 3.6 VolatilityAssessment
`COMPRESSED`, `NORMAL`, `EXPANDING`, `EXTREME`, `UNSTABLE` — derived from alignment dimension 3 (volatility).

### 3.7 VolumeAssessment
`WEAK`, `NORMAL`, `STRONG`, `EXCEPTIONAL` — derived from alignment dimension 2 (volume).

### 3.8 OpportunityType
`TREND_CONTINUATION`, `BREAKOUT`, `PULLBACK`, `MEAN_REVERSION`, `REVERSAL`, `NO_CLEAR_OPPORTUNITY`.

### 3.9 QualityLevel
`POOR`, `WEAK`, `AVERAGE`, `GOOD`, `EXCELLENT` — computed as the mean of the trend, momentum, structure, and volume dimension scores.

---

## 4. Derivation Contract

### 4.1 Confidence Model

```
base_confidence = |mtf_overall_score| / 100
IF trend_agreement_pct ≥ 75  → confidence += 0.15
IF trend_agreement_pct < 50  → confidence  = min(confidence, 0.5)
IF signal_cross_tf_count ≥ 3 → confidence += 0.10
IF timeframes_present ≤ 1    → confidence  = min(confidence, 0.5)
confidence = clamp(confidence, 0, 1)
```

### 4.2 Assessment Thresholds

| Assessment | Source dim | Bands |
|-----------|-----------|-------|
| Trend | dim 0 | `≥90` `STRONG` · `≥75` `HEALTHY` · `≥50` `DEVELOPING` · `≥25` `WEAK` · else `EXHAUSTED` |
| Momentum | dim 1 | `≥80` `INCREASING` · `≥60` `STABLE` · `≥40` `WEAKENING` · else `REVERSING` |
| Structure | dim 4 | `≥80` `STRONG` · `≥60` `HEALTHY` · `≥40` `WEAK` · `≥20` `BROKEN` · else `UNCLEAR` |
| Volatility | dim 3 | `≥90` `EXTREME` · `≥70` `EXPANDING` · `≥40` `NORMAL` · `≥20` `COMPRESSED` · else `UNSTABLE` |
| Volume | dim 2 | `≥90` `EXCEPTIONAL` · `≥70` `STRONG` · `≥40` `NORMAL` · else `WEAK` |

### 4.3 Opportunity Selection

```
IF trend ≥ 75 AND bias bullish                              → TREND_CONTINUATION
ELIF volatility ≥ 70 AND structure ≥ 60                     → BREAKOUT
ELIF confirmed_divergence AND structure_broken AND momentum_exhausted → REVERSAL
ELIF trend ≥ 60 AND momentum weakening                       → PULLBACK
ELIF volatility ≤ 30                                         → MEAN_REVERSION
ELIF opportunity_dim < 30                                    → NO_CLEAR_OPPORTUNITY
ELSE                                                          → TREND_CONTINUATION
```

Where `confirmed_divergence` is true when at least one `Divergence` indicator signal has reached `status = CONFIRMED` (§4.2 of [Metrics Matrix](02-07-metrics-matrix.md)), `structure_broken` is true when Alignment Matrix dimension 4 (`Structure`) score is below 40 (per §4.2), and `momentum_exhausted` is true when Alignment Matrix dimension 1 (`Momentum`) score is below 25. All six values of `OpportunityType` are reachable from this rule.

### 4.4 Explainability

The `rationale` and `market_interpretation` strings are generated deterministically from the numeric derivation, satisfying the ontology's **Explainability** principle: every categorical output traces back to the alignment scores and per-timeframe evidence recorded in `supporting_signals` / `contradicting_signals`.

---

## 5. JSON Serialization Contract

```json
{
  "symbol": "BTC-USDT",
  "bias": "BULLISH",
  "confidence": 0.82,
  "market_regime": "TRENDING_BULL",
  "trend_assessment": "HEALTHY",
  "momentum_assessment": "STABLE",
  "structure_assessment": "HEALTHY",
  "volatility_assessment": "NORMAL",
  "volume_assessment": "STRONG",
  "opportunity_analysis": "TREND_CONTINUATION",
  "market_quality": "GOOD",
  "market_interpretation": "Bullish trending market with healthy trend, stable momentum, healthy structure, normal volatility, and strong volume participation. Favors trend continuation.",
  "rationale": "MTF overall score 41/100 → BULLISH. Majority of 4 timeframes agree (75%). 3 signals across multiple timeframes.",
  "supporting_signals": ["fast180 (bullish): score +42, TRENDING regime, 3 signals"],
  "contradicting_signals": [],
  "timeframes_considered": 4
}
```

Enum values serialize as `SCREAMING_SNAKE_CASE`.

---

## 6. Empty State

When `timeframes_present == 0`, `derive_analysis` returns `AnalysisMatrix::empty()`: `bias = NEUTRAL`, `confidence = 0.0`, `market_regime = TRANSITION`, `market_quality = POOR`, and the interpretation `"No data available — no candles have been completed."`.

---

## 7. Cross-References

- [Alignment Matrix](02-01-alignment-matrix.md) — Sole input.
- [Opportunity Matrix](02-08-opportunity-matrix.md) — Consumes `opportunity_analysis`.
- [Risk Matrix](02-11-risk-matrix.md) — Consumes the full Analysis Matrix.
- [Decision Matrix](02-04-decision-matrix.md) — Downstream synthesis.
- [MME Layer 3 — Analysis](../engines/market-monitoring-engine/03-02-04-mme-layer3-analysis.md) — Producing-layer specification.
