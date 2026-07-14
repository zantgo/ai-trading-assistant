# MME Layer 6 — Decision Support Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Layer:** 6 of 7
**Output Contract:** [Decision Matrix](../../matrices/02-04-decision-matrix.md)
**Purpose:** This document specifies the Decision Support Layer — the process that synthesizes bias, opportunity, and risk into structured tactical guidance: trade-readiness states, directional guidance, and dynamic protection/target boundaries.

---

## 1. Purpose

The Decision Support Layer transforms market intelligence into **actionable guidance without executing trades**. It consumes the Analysis, Opportunity, and Risk matrices and produces the [Decision Matrix](../../matrices/02-04-decision-matrix.md) (`AdvisoryMatrix` + `DecisionContext` structs).

```
[Analysis Matrix] ─┐
[Opportunity Mat.] ─┼──► DECISION SUPPORT (L6) ──► [Decision Matrix]
[Risk Matrix     ] ─┘     compute_advisory()
```

Implementation: `crates/shared/src/advisory.rs::compute_advisory()`, `decision_context.rs`.

---

## 2. Trade-Readiness State Management

Trade readiness is derived from directional guidance, confidence, and stance. The canonical vocabulary is [Decision Matrix §4](../../matrices/02-04-decision-matrix.md):

| Readiness | Condition |
|-----------|-----------|
| `READY` | Non-neutral guidance + `confidence_assessment ≥ 60` + stance ∈ {`AGGRESSIVE`, `CONSTRUCTIVE`}. |
| `FORMING` | Directional guidance present, `confidence_assessment` 40–60, or entry = `WAIT_FOR_CONFIRMATION`. |
| `WATCH` | Neutral guidance or `confidence_assessment` 20–40. |
| `STAND_ASIDE` | Stance = `AVOID` or `confidence_assessment < 20`. |

Confidence itself is risk-discounted:

$$\text{confidence} = \text{clamp}\Big(\text{analysis.confidence} \times \big(1 - \tfrac{\text{overall\_risk}}{100}\big) \times 100,\ 0,\ 100\Big)$$

---

## 3. Directional & Stance Guidance

`DirectionalGuidance` is derived from `bias × overall_risk`; `MarketStance` from `market_quality × overall_risk`. Full derivation tables: [Decision Matrix §3](../../matrices/02-04-decision-matrix.md).

---

## 4. Dynamic Protection Boundaries

The layer recommends **how** to place protective stops and targets — not the price directly, but the method:

### 4.1 Protection Strategy (Stops)
```
volatility compressed → StructureBased
volatility_risk > 60  → VolatilityBased
otherwise             → ATRBased
```

### 4.2 Target Strategy
```
structure strong/healthy → ResistanceBased
reward_risk < 40         → RRBased
otherwise                → VolatilityBased
```

### 4.3 Stop-Loss Distance Handoff (Type Boundary)

Layer 6 is the platform's **type-boundary handoff**: it receives the fast, raw `f64` analytics from Layers 1–5, resolves them into trade readiness, and emits the **Decision Matrix** carrying the required stop-loss distance (`stop_loss_distance_pct`) as a standard `f64` (e.g. `1.5`, representing 1.5%).

The recommended protection method resolves to a concrete **stop-loss distance percentage (`D_sl`)** — computed from ATR or structural levels. This `f64` value is the critical input to the TAE Position Sizing Protocol, where it is cast to `Decimal` at the execution boundary (see [Global Architecture §6.3](../../conceptual-foundations/01-02-global-architecture.md) and [TAE Layer 2 — Execution](../trade-automation-engine/03-03-03-tae-layer2-execution.md)):

$$S = \frac{E \times R}{D_{sl} / 100}$$

`D_sl` is a raw percentage float (e.g. `1.5` = 1.5%), divided by 100 in the formula; `E` is available margin.

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
