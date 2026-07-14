# MME Layer 3 — Analysis Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Layer:** 3 of 7
**Output Contract:** [Analysis Matrix](../../matrices/02-02-analysis-matrix.md)
**Purpose:** This document specifies the Analysis Layer — the process that transforms multi-timeframe agreement into a complete market interpretation: categorical bias, the continuous `market_bias_score`, regime classification, and real-time regime detection.

---

## 1. Purpose

The Analysis Layer is the transition from *observation* to *understanding*. It consumes the [Alignment Matrix](../../matrices/02-01-alignment-matrix.md) and produces the [Analysis Matrix](../../matrices/02-02-analysis-matrix.md).

```
[Alignment Matrix] ──► ANALYSIS LAYER (L3) ──► [Analysis Matrix]
                        derive_analysis()        bias + regime + 7 assessments
```

Implementation: `crates/shared/src/analysis.rs::derive_analysis()`.

---

## 2. Continuous Bias Calculation

The headline `market_bias_score` is the alignment `mtf_overall_score` (range `[-100, 100]`), interpreted as `[-1.0, +1.0]` after scaling. The categorical `MarketBias` buckets it:

| `mtf_overall_score` | MarketBias |
|---------------------|-----------|
| `> 40` | `STRONG_BULLISH` |
| `20 … 40` | `BULLISH` |
| `-20 … 20` | `NEUTRAL` |
| `-40 … -20` | `BEARISH` |
| `< -40` | `STRONG_BEARISH` |

### 2.1 Confidence Model

```
base = |mtf_overall_score| / 100
+0.15 if trend_agreement_pct ≥ 75
cap 0.5 if trend_agreement_pct < 50
+0.10 if signal_cross_tf_count ≥ 3
cap 0.5 if timeframes_present ≤ 1
confidence = clamp(base, 0, 1)
```

---

## 3. Real-Time Regime Detection

The layer classifies the structural regime from the alignment score and per-timeframe context using the canonical decision tree in [Analysis Matrix §3.2](../../matrices/02-02-analysis-matrix.md):

| Regime | Trigger |
|--------|---------|
| `EXPANSION` | `bbwp ≥ 85` (priority 1) |
| `CONTRACTION` | `bbwp ≤ 10` (priority 1) |
| `TRENDING_BULL` | `adx ≥ 25` AND `score > +20` (priority 2) |
| `TRENDING_BEAR` | `adx ≥ 25` AND `score < -20` (priority 2) |
| `ACCUMULATION` | bullish slope (score rising over 3 bars) AND `score ≥ 0` AND no expansion (priority 3) |
| `DISTRIBUTION` | bearish slope (score falling over 3 bars) AND `score ≤ 0` AND no expansion (priority 4) |
| `TRANSITION` | `adx < 25` AND `bbwp` in `(10, 85)` AND regime shifted within last 3 bars (priority 5) |
| `RANGE` | default — none of the above (priority 6) |

The full decision tree with detailed conditions lives in the canonical Analysis Matrix spec; this layer is a thin executor of that tree. Regime detection is continuous — it re-evaluates on every completed candle, enabling downstream layers to adapt (e.g. the Decision Layer's strategy environment).

---

## 4. Seven Qualitative Assessments

Each is derived from a specific alignment dimension score (see [Analysis Matrix §4.2](../../matrices/02-02-analysis-matrix.md)):

| Assessment | Source dim | Vocabulary |
|-----------|-----------|-----------|
| Trend | 0 | `WEAK` / `DEVELOPING` / `HEALTHY` / `STRONG` / `EXHAUSTED` |
| Momentum | 1 | `INCREASING` / `STABLE` / `WEAKENING` / `REVERSING` |
| Volume | 2 | `WEAK` / `NORMAL` / `STRONG` / `EXCEPTIONAL` |
| Volatility | 3 | `COMPRESSED` / `NORMAL` / `EXPANDING` / `EXTREME` / `UNSTABLE` |
| Structure | 4 | `STRONG` / `HEALTHY` / `WEAK` / `BROKEN` / `UNCLEAR` |
| Opportunity | 9 | (selects `OpportunityType`) |
| Quality | mean(0,1,2,4) | `POOR` / `WEAK` / `AVERAGE` / `GOOD` / `EXCELLENT` |

---

## 5. Explainability Trace

The layer emits:

- `market_interpretation` — natural-language summary of regime + assessments.
- `rationale` — the numeric derivation (score, agreement %, cross-TF signals).
- `supporting_signals` / `contradicting_signals` — per-timeframe evidence split by whether it agrees with the derived bias.

This satisfies the ontology's Explainability principle: the categorical bias is always traceable to its numeric and per-timeframe evidence.

---

## 6. Guarantees

| Property | Guarantee |
|----------|-----------|
| **Single interpretation** | Exactly one bias, regime, and set of assessments per symbol. |
| **Risk-free** | The Analysis Layer never evaluates danger (that is Layer 5). |
| **Deterministic** | A given Alignment Matrix always yields the same Analysis Matrix. |
| **Empty safety** | Zero timeframes → neutral empty analysis. |

---

## 7. Cross-References

- [Alignment Matrix](../../matrices/02-01-alignment-matrix.md) — Input.
- [Analysis Matrix](../../matrices/02-02-analysis-matrix.md) — Output contract.
- [MME Layer 4 — Opportunity](03-02-05-mme-layer4-opportunity.md) · [MME Layer 5 — Risk](03-02-06-mme-layer5-risk.md) — Consumers.
