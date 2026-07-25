# MME Layer 6 — Decision Support Layer

**Version:** 6.5 (2026-07-24) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Layer:** 6 of 7
**Output Contract:** [Decision Matrix](../../matrices/02-04-decision-matrix.md)
**Purpose:** This document specifies the Decision Support Layer — the process that synthesizes bias, opportunity, and risk into structured tactical guidance: trade-readiness states, directional guidance, and dynamic protection/target boundaries.

---

## 1. Purpose

The Decision Support Layer transforms market intelligence into **actionable guidance without executing trades**. It consumes the Analysis (L3), Opportunity (L4), and Risk (L5) matrices and produces the [Decision Matrix](../../matrices/02-04-decision-matrix.md) (`AdvisoryMatrix` + `DecisionContext` structs).

> **Three inputs, not two.** L6 reads directly from **L3** as well as from L4 and L5. L3 supplies `bias`, `state_confidence`, `market_quality`, `market_regime`, and the **five** qualitative assessments (Trend, Momentum, Volume, Volatility, Structure — `market_quality` is a separate categorical field, not counted among these five), which feed `directional_guidance`, `strategy_environment`, and `confidence_assessment`. L4 and L5 supply opportunity and risk vectors. L6 is the **only** synthesis point in the pipeline.

```
[Analysis Matrix] ─┐
[Opportunity Mat.] ─┼──► DECISION SUPPORT (L6) ──► [Decision Matrix]
[Risk Matrix     ] ─┘     compute_advisory()
```

Implementation: `crates/core-domain/src/advisory.rs::compute_advisory()`, `decision_context.rs`.

---

## 2. Trade-Readiness State Management

Trade readiness is derived from directional guidance, confidence, and `market_stance`. The canonical rule table is the ordered, first-match `TradeReadiness` ruleset — with a tiling proof that the rules partition the full input space — in [Decision Matrix §4](../../matrices/02-04-decision-matrix.md) (`READY` / `FORMING` / `WATCH` / `STAND_ASIDE`); this layer executes that ruleset and does not re-define it.

> **Stance-vs-`market_stance` disambiguation.** The readiness rules reference `market_stance` (the L6 `MarketStance` 5-state enum: `AGGRESSIVE` / `CONSTRUCTIVE` / `NEUTRAL` / `CAUTIOUS` / `AVOID`, derived from L3 `market_quality` × L5 `overall_risk`) — the L6 Decision field. They do **not** reference the per-symbol **stance** (the `Stance` 3-state enum: `ACTIVE` / `CLOSE_ONLY` / `AVOID`), which is the TAE/PME execution-authorization concept managed by the PME safety veto. Although both enums share an `AVOID` variant, they are independent and serve different purposes.

Confidence itself is risk-discounted into the terminal `confidence_assessment` output (which lives on `advisory` and is distinct from the four-level pipeline-level confidence flow; see [`02-00b-confidence-hierarchy.md`](../../matrices/02-00b-confidence-hierarchy.md)):

$$\text{confidence\_assessment} = \text{clamp}\Big(\text{analysis.state\_confidence} \times \big(1 - \tfrac{\text{overall\_risk}}{100}\big) \times 100,\ 0,\ 100\Big)$$

**L6 Confluence Score** (composite, distinct from `confidence_assessment`, in `[0, 100]`):

```
confluence_score = clamp(
    0.50 × alignment.tradability_dim
  + 0.30 × analysis.market_quality_score
  + 0.20 × opportunity.opportunity_score,
  0, 100)
```

The three components are read from L2 (Alignment `tradability_dim`), L3 (Analysis `market_quality_score` — the raw numeric mean in `[0, 100]`, distinct from the categorical `market_quality` `QualityLevel` enum), and L4 (Opportunity `opportunity_score`) respectively and weighted by their predictive power for entry timing. `confluence_score` is the **composite L6 output** distinct from the risk-discounted `confidence_assessment` (the latter is the safety-aware confidence, the former is the raw setup strength).

> **`market_quality_score` vs `market_quality` (v2.1 — type clarification).** The Analysis Matrix schema ([02-02-analysis-matrix.md §2.1](../../matrices/02-02-analysis-matrix.md)) defines two distinct fields: the categorical `market_quality: QualityLevel` enum (`POOR / WEAK / AVERAGE / GOOD / EXCELLENT`) and the numeric `market_quality_score: f64` carrying the raw mean in `[0, 100]`. The L6 formula uses the **numeric** `market_quality_score`, not the enum. If only the enum is available at runtime, an explicit `QualityLevel → f64` mapping (e.g. `POOR → 20.0, WEAK → 40.0, AVERAGE → 55.0, GOOD → 70.0, EXCELLENT → 100.0`) should be applied at the L3 → L6 boundary; the L3 Analyzer SHOULD populate `market_quality_score` directly when the per-dimension scores are available.

---

## 3. Directional & Stance Guidance

`DirectionalGuidance` is derived from `bias × overall_risk × market_stance` (priority order, first match wins): `market_stance = AVOID → AVOID_DIRECTIONAL_EXPOSURE`; otherwise the bias × risk grid. `MarketStance` is derived from `market_quality × overall_risk` with sticky AVOID/CAUTIOUS guards. Full derivation tables: [Decision Matrix §3](../../matrices/02-04-decision-matrix.md).

---

## 4. Dynamic Protection Boundaries

The layer recommends **how** to place protective stops and targets — not the price directly, but the method:

### 4.1 Protection Strategy (Stops)

The canonical ordered rules — including the `volatility_assessment ∈ {EXPANDING, EXTREME}` and `StructureAssessment ∈ {STRONG, HEALTHY}` conditions and the empty-state `NO_RECOMMENDATION` fallback — live in [Decision Matrix §3.6](../../matrices/02-04-decision-matrix.md); this layer executes that ruleset and does not re-define it.

### 4.2 Target Strategy

The canonical ordered rules — including the `entry_danger.level`-banded `TRAILING_METHOD` condition and the empty-state `NO_RECOMMENDATION` fallback — live in [Decision Matrix §3.7](../../matrices/02-04-decision-matrix.md); this layer executes that ruleset and does not re-define it.

### 4.3 Stop-Loss Distance Handoff (Type Boundary)

Layer 6 is the platform's **type-boundary handoff**: it receives the fast, raw `f64` analytics from Layers 1–5, resolves them into trade readiness, and emits the **Decision Matrix** carrying the required stop-loss distance (`stop_loss_distance_pct`) as a standard `f64` (e.g. `1.5`, representing 1.5%).

The recommended protection method resolves to a concrete **stop-loss distance percentage (`D_sl`)** — computed from ATR or structural levels. This `f64` value is the critical input to the TAE Position Sizing Protocol, where it is cast to `Decimal` at the execution boundary (see [Global Architecture §6.3](../../conceptual-foundations/01-02-global-architecture.md) and [TAE Layer 2 — Execution](../trade-automation-engine/03-03-03-tae-layer2-execution.md)):

$$S = \frac{E \times R}{D_{sl} / 100}$$

`D_sl` is a raw percentage float (e.g. `1.5` = 1.5%), divided by 100 in the formula; `E` is available margin. *(Units: `E` = available margin (Decimal, quote currency); `R = risk_per_trade_pct / 100` (unitless fraction in `[0, 1]`); `D_sl` = raw percent float in `[0, 100]` (divided by 100 in the formula).)*

---

## 5. Scenario Pathways

The Decision Matrix records the structural invalidation level and conditional bull/bear pathways in `final_recommendation`, giving the TAE Policy Layer and human operators an explainable map of what would change the thesis.

---

## 6. Guarantees

| Property | Guarantee |
|----------|-----------|
| **No autonomous execution** | Recommends only; never places orders. |
| **Risk-discounted** | Confidence always attenuated by overall risk. |
| **Explainable** | `final_recommendation` + `contributing_indicators` trace all guidance. |
| **Stable contract** | The TAE depends only on public Decision Matrix fields. |

---

## 7. Cross-References

- [Analysis](../../matrices/02-02-analysis-matrix.md) · [Opportunity](../../matrices/02-08-opportunity-matrix.md) · [Risk](../../matrices/02-11-risk-matrix.md) — Inputs.
- [Decision Matrix](../../matrices/02-04-decision-matrix.md) — Output contract.
- [MME Layer 7 — Overview](03-02-08-mme-layer7-overview.md) — Aggregates decision matrices.
- [TAE Layer 1 — Policy](../trade-automation-engine/03-03-02-tae-layer1-policy.md) — Primary consumer.
