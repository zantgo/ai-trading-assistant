# Opportunity Matrix Specification

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 4 — Opportunity Layer
**Purpose:** This document defines the physical schema, scoring model, and setup-quality classification of the **Opportunity Matrix** — the strategy-agnostic profiling object. It identifies and scores favourable market configurations (breakout, continuation, pullback, mean-reversion, reversal) on a 0–100 scale, independent of any execution parameters.

---

## 1. Conceptual Definition

Per the [Ontology](../conceptual-foundations/01-01-ontology.md) §3.14, **Opportunity** represents the positive market potential present in the current conditions. The Opportunity Matrix evaluates whether favourable setups exist and scores their statistical viability — **without** committing to a direction of exposure, position size, or entry price. Those belong to the [Decision Matrix](02-04-decision-matrix.md) and the Trade Automation Engine.

The Opportunity Matrix consumes the [Analysis Matrix](02-02-analysis-matrix.md) (context) and the underlying [Metrics Matrix](02-07-metrics-matrix.md) signals (evidence), and emits one profiled opportunity per candidate setup type.

```
[Analysis Matrix] ─┐
                   ├──► OPPORTUNITY LAYER (L4) ──► [Opportunity Matrix]
[Metrics Matrix ]  ┘        (profile + score 0-100)
```

This is a **strategy-agnostic** contract: it describes the shape and quality of the opportunity, leaving the decision of whether to act to external consumers.

---

## 2. Physical Schema

### 2.1 OpportunityMatrix Fields

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | `string` | Entity under analysis. |
| `primary_opportunity` | `OpportunityType` | The dominant setup classification. |
| `opportunity_score` | `f64` | Overall setup viability in `[0, 100]`. |
| `setup_quality` | `SetupQuality` | Categorical quality band (§4). |
| `profiles` | `OpportunityProfile[]` | Per-setup-type scored profiles (§3). |
| `direction` | `OpportunityDirection` | `Long` / `Short` / `Neutral` bias of the opportunity. |
| `confidence` | `f64` | Confidence in the profiling `[0, 1]`. |
| `contributing_signals` | `string[]` | Signal labels supporting the primary opportunity. |
| `invalidation_note` | `string` | Condition that would nullify the opportunity. |

### 2.2 OpportunityProfile

| Field | Type | Description |
|-------|------|-------------|
| `opportunity_type` | `OpportunityType` | Setup being profiled. |
| `score` | `f64` | Viability `[0, 100]` for this specific setup. |
| `preconditions_met` | `u32` | Count of satisfied preconditions. |
| `preconditions_total` | `u32` | Total preconditions evaluated. |
| `notes` | `string` | Human-readable profiling rationale. |

---

## 3. Opportunity Types & Preconditions

The `OpportunityType` vocabulary matches the Analysis Matrix (`crates/shared/src/analysis.rs`):

| OpportunityType | Precondition Signature |
|-----------------|------------------------|
| `TrendContinuation` | Strong/healthy trend (dim ≥ 75) + directional bias + momentum not exhausted. |
| `Breakout` | Volatility expansion (dim ≥ 70) + healthy structure (dim ≥ 60) + compression release or level breach. |
| `Pullback` | Established trend (dim ≥ 60) + weakening momentum + price retracing toward a dynamic level. |
| `MeanReversion` | Volatility compression (dim ≤ 30) + range regime + oscillator extreme. |
| `Reversal` | Confirmed divergence + structure break + momentum reversing. |
| `NoClearOpportunity` | Opportunity dimension < 30 or conflicting evidence. |

Each profile records a `preconditions_met / preconditions_total` fraction, providing an explainable basis for its score.

---

## 4. Setup-Quality Classification

The categorical `setup_quality` buckets the `opportunity_score`:

| SetupQuality | `opportunity_score` | Interpretation |
|--------------|---------------------|----------------|
| `Prime` | `≥ 85` | High-conviction configuration, all key preconditions met. |
| `Strong` | `70 … 85` | Robust setup with minor gaps. |
| `Moderate` | `50 … 70` | Tradable but requires confirmation. |
| `Marginal` | `30 … 50` | Weak edge; confluence-only. |
| `None` | `< 30` | No actionable opportunity. |

---

## 5. Scoring Model

The `opportunity_score` for a candidate setup blends four factors, each normalized to `[0, 100]`:

$$\text{score} = 0.35\,Q_{ctx} + 0.30\,S_{sig} + 0.20\,A_{mtf} + 0.15\,F_{fresh}$$

| Factor | Symbol | Source |
|--------|--------|--------|
| Context quality | `Q_ctx` | Analysis `market_quality` + relevant assessment dimension. |
| Signal support | `S_sig` | Strength and confirmation status of contributing Metrics-Matrix signals. |
| MTF agreement | `A_mtf` | Alignment `trend_agreement_pct` for directional setups. |
| Freshness | `F_fresh` | Inverse of the youngest contributing signal's `age_bars`. |

The primary opportunity is the profile with the highest score; ties resolve toward the Analysis Matrix's `opportunity_analysis`.

---

## 6. JSON Serialization Contract

```json
{
  "symbol": "BTC-USDT",
  "primary_opportunity": "BREAKOUT",
  "opportunity_score": 85.0,
  "setup_quality": "PRIME",
  "direction": "LONG",
  "confidence": 0.81,
  "profiles": [
    { "opportunity_type": "BREAKOUT", "score": 85.0,
      "preconditions_met": 3, "preconditions_total": 3,
      "notes": "Volatility expanding, structure healthy, compression released." },
    { "opportunity_type": "TREND_CONTINUATION", "score": 62.0,
      "preconditions_met": 2, "preconditions_total": 3,
      "notes": "Trend healthy but momentum stabilizing." }
  ],
  "contributing_signals": ["squeeze:COMPRESSION_RELEASE", "donchian:BREAKOUT_UP"],
  "invalidation_note": "Close back inside the prior Donchian channel invalidates the breakout."
}
```

Enum values serialize as `SCREAMING_SNAKE_CASE`.

---

## 7. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Direction-neutral scoring** | The score reflects setup *viability*, not profit expectation. |
| **Strategy-agnostic** | No strategy assumptions (scalping, swing, arbitrage) leak into the profiling. |
| **Explainability** | Every score decomposes into its four weighted factors and precondition fractions. |
| **Bounded** | `opportunity_score` and all profile scores clamp to `[0, 100]`. |

---

## 8. Cross-References

- [Analysis Matrix](02-02-analysis-matrix.md) — Context input (`opportunity_analysis`).
- [Risk Matrix](02-11-risk-matrix.md) — Paired directional-neutral counterpart (danger vs opportunity).
- [Decision Matrix](02-04-decision-matrix.md) — Combines opportunity + risk into trade readiness.
- [MME Layer 4 — Opportunity](../engines/market-monitoring-engine/03-02-05-mme-layer4-opportunity.md) — Producing-layer specification.
- [Ontology — Opportunity](../conceptual-foundations/01-01-ontology.md) — Conceptual definition.
