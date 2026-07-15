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

This is a **strategy-agnostic, direction-neutral** contract: it describes only the shape, quality, and precondition satisfaction of the opportunity. Directional bias, entry price, position sizing, and execution authorization are *not* the Opportunity Matrix's responsibility; those belong to the [Decision Matrix](02-04-decision-matrix.md) and the Trade Automation Engine.

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
| `forecast_confidence` | `f64` | Confidence in the profiling `[0, 1]`. *(Renamed from `confidence` in the institutional redesign; see [02-00b-confidence-hierarchy.md](02-00b-confidence-hierarchy.md).)* |
| `contributing_signals` | `string[]` | Signal labels supporting the primary opportunity. |
| `invalidation_note` | `string` | Condition that would nullify the opportunity. |
| `entry_zone` | `PriceRange` | Recommended entry band. *(Added in the institutional redesign — institutional quant field.)* |
| `target_zone` | `PriceRange` | Expected target band. *(Added in the institutional redesign.)* |
| `invalid_level` | `Decimal` | Structural invalidation price. *(Added in the institutional redesign.)* |
| `expected_rr` | `f64` | Expected reward/risk ratio for this setup. *(Added in the institutional redesign.)* |
| `time_horizon` | `TimeHorizon` | Expected holding period: `INTRADAY` / `SWING` / `POSITION`. *(Added in the institutional redesign.)* |

#### 2.1.1 PriceRange

| Field | Type | Description |
|-------|------|-------------|
| `low` | `Decimal` | Lower price bound. |
| `high` | `Decimal` | Upper price bound. |

#### 2.1.2 TimeHorizon

`INTRADAY` (held for minutes to hours) / `SWING` (held for days) / `POSITION` (held for weeks). Drives the cadence at which the Decision Layer's `exit_guidance` is updated.

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

The `OpportunityType` enum is the **canonical home** of the setup selector (in the institutional redesign, this enum was removed from the Analysis Matrix and moved here, where it belongs as a forecast field). Six values:

| OpportunityType | Precondition Signature |
|-----------------|------------------------|
| `TrendContinuation` | Strong/healthy trend (dim ≥ 75) + directional bias + momentum not exhausted. |
| `Breakout` | Volatility expansion (dim ≥ 70) + healthy structure (dim ≥ 60) + compression release or level breach. |
| `Pullback` | Established trend (dim ≥ 60) + weakening momentum + price retracing toward a dynamic level. |
| `MeanReversion` | Volatility compression (dim ≤ 30) + range regime + oscillator extreme. |
| `Reversal` | Confirmed divergence + structure break + momentum reversing. |
| `NoClearOpportunity` | Opportunity dimension < 30 or conflicting evidence. |

---

## 4. Setup-Selection Rule

The Opportunity Layer applies the following decision tree (priority 1 → 6, first match wins) to derive `primary_opportunity`. This rule was formerly located in [02-02-analysis-matrix.md §4.3](02-02-analysis-matrix.md); it has been moved here as part of the institutional redesign because setup selection is a forecast, not a state interpretation.

```
# Priority order (first match wins):
1. trend ≥ 75 AND bias bullish                                     → TREND_CONTINUATION
2. volatility ≥ 70 AND structure ≥ 60                              → BREAKOUT
3. confirmed_divergence AND structure_broken AND momentum_exhausted → REVERSAL
4. trend ≥ 60 AND momentum weakening                              → PULLBACK
5. volatility ≤ 30                                                → MEAN_REVERSION
6. opportunity_dim < 30                                           → NO_CLEAR_OPPORTUNITY
7. otherwise (default)                                             → TREND_CONTINUATION
```

Where `confirmed_divergence` is true when at least one `Divergence` indicator signal has reached `status = CONFIRMED` ([Metrics Matrix §4.2](02-07-metrics-matrix.md)), `structure_broken` is true when Alignment Matrix dimension 4 (`Structure`) score is below 40, and `momentum_exhausted` is true when Alignment Matrix dimension 1 (`Momentum`) score is below 25. All six values of `OpportunityType` are reachable via the explicit branches; the `ELSE` (priority 7) is a defensive default that may also resolve to `TREND_CONTINUATION`.

Each profile records a `preconditions_met / preconditions_total` fraction, providing an explainable basis for its score.

---

## 5. Setup-Quality Classification

The categorical `setup_quality` buckets the `opportunity_score`:

| SetupQuality | `opportunity_score` | Interpretation |
|--------------|---------------------|----------------|
| `Prime` | `≥ 85` | High-conviction configuration, all key preconditions met. |
| `Strong` | `70 … 85` | Robust setup with minor gaps. |
| `Moderate` | `50 … 70` | Tradable but requires confirmation. |
| `Marginal` | `30 … 50` | Weak edge; confluence-only. |
| `None` | `< 30` | No actionable opportunity. |

---

## 6. Scoring Model

The `opportunity_score` for a candidate setup blends four factors, each normalized to `[0, 100]`:

$$\text{score} = 0.35\,Q_{ctx} + 0.30\,S_{sig} + 0.20\,A_{mtf} + 0.15\,F_{fresh}$$

| Factor | Symbol | Source |
|--------|--------|--------|
| Context quality | `Q_ctx` | Analysis `market_quality` + relevant assessment dimension. |
| Signal support | `S_sig` | Strength and confirmation status of contributing Metrics-Matrix signals. |
| MTF agreement | `A_mtf` | Alignment `trend_agreement_pct` for directional setups. |
| Freshness | `F_fresh` | Inverse of the youngest contributing signal's `age_bars`. |

The primary opportunity is the profile with the highest score; ties resolve toward the highest-precondition-satisfaction profile.

---

## 7. JSON Serialization Contract

A representative Opportunity Matrix frame. The example illustrates the JSON shape; the canonical scoring formula is in §6.

```json
{
  "symbol": "BTC-USDT",
  "primary_opportunity": "BREAKOUT",
  "opportunity_score": 85.0,
  "setup_quality": "PRIME",
  "forecast_confidence": 0.81,
  "profiles": [
    { "opportunity_type": "BREAKOUT", "score": 85.0,
      "preconditions_met": 3, "preconditions_total": 3,
      "notes": "Volatility expanding, structure healthy, compression released." },
    { "opportunity_type": "TREND_CONTINUATION", "score": 62.0,
      "preconditions_met": 2, "preconditions_total": 3,
      "notes": "Trend healthy but momentum stabilizing." }
  ],
  "contributing_signals": ["squeeze:COMPRESSION_RELEASE", "donchian:BREAKOUT_UP"],
  "invalidation_note": "Close back inside the prior Donchian channel invalidates the breakout.",
  "entry_zone":  { "low": "64000.0", "high": "64200.0" },
  "target_zone": { "low": "65500.0", "high": "66000.0" },
  "invalid_level": "63850.0",
  "expected_rr": 2.5,
  "time_horizon": "SWING"
}
```

Enum values serialize as `SCREAMING_SNAKE_CASE`.

---

## 8. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Direction-neutral scoring** | The score reflects setup *viability*, not profit expectation. |
| **Strategy-agnostic** | No strategy assumptions (scalping, swing, arbitrage) leak into the profiling. |
| **Explainability** | Every score decomposes into its four weighted factors and precondition fractions. |
| **Bounded** | `opportunity_score` and all profile scores clamp to `[0, 100]`. |
| **Canonical OpportunityType** | This matrix is the **only** producer of `OpportunityType`. The Analysis Matrix's former `opportunity_analysis` field has been removed (see [02-00-matrix-field-ownership.md](02-00-matrix-field-ownership.md)). |

---

## 9. Cross-References

- [Analysis Matrix](02-02-analysis-matrix.md) — Context input (`bias`, `market_quality`, `state_confidence`, qualitative assessments).
- [Risk Matrix](02-11-risk-matrix.md) — Parallel directional-neutral counterpart (danger).
- [Decision Matrix](02-04-decision-matrix.md) — Only synthesis point: combines opportunity + risk + state into trade readiness.
- [02-00-matrix-field-ownership.md](02-00-matrix-field-ownership.md) — Canonical producer-layer mapping.
- [MME Layer 4 — Opportunity](../engines/market-monitoring-engine/03-02-05-mme-layer4-opportunity.md) — Producing-layer specification.
- [Ontology — Opportunity](../conceptual-foundations/01-01-ontology.md) — Conceptual definition.
