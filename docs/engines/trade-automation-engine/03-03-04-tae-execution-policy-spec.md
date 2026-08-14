# TAE — Execution Policy Specification

**Version:** 6.10 (2026-08-14) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Specified — **WIP**; the policy schema and evaluation engine are implemented; the dashboard surface that lets an operator author / inspect policies is a placeholder. See [`docs/ROADMAP.md`](../../ROADMAP.md) §3 Phase A.
**Engine:** Trade Automation Engine (TAE)
**Purpose:** This document defines the formal syntax, semantics, and lifecycle of **Execution Policies** — the user-configured conditional rules that govern automated order dispatch. Execution policies are the bridge between passive MME intelligence and active TAE execution.

---

## 1. Conceptual Definition

An Execution Policy is a deterministic, user-defined rule that evaluates incoming market intelligence (from the MME Decision Matrix) against programmatic constraints to decide whether to trigger an automated order.

Per the [Ontology](../../conceptual-foundations/01-01-ontology.md) §3.18:

> An Execution Policy is a user-defined, conditional rule managed by the Trade Automation Engine. Execution policies evaluate incoming decision-support parameters against programmatic constraints to determine whether to trigger an automated order.

```
[Decision Matrix] ──► EXECUTION POLICY ──(conditions met?)──► [Policy Matrix trigger] ──► [Execution Layer]
                                  │
                                  └──(conditions not met)──► (no action)
```

---

## 2. Policy Structure

### 2.1 Full Policy Schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `policy_id` | `string` | Yes | Unique policy identifier. |
| `policy_name` | `string` | Yes | Human-readable label. |
| `description` | `string` | No | Free-text description of the policy intent. |
| `symbol` | `string` | Yes | Target trading pair (e.g., `BTC-USDT`). |
| `direction` | `Direction` | Yes | `Long` / `Short`. |
| `conditions` | `ConditionGroup` | Yes | Boolean expression tree of conditions. |
| `trigger_mode` | `TriggerMode` | Yes | When the policy is evaluated (§4). |
| `risk` | `RiskParams` | Yes | Position sizing parameters (§6). |
| `enabled` | `bool` | Yes | Master on/off switch. |
| `cooldown_seconds` | `u64` | No | Minimum interval between consecutive triggers (prevents overtrading). |
| `reduce_only_on_close_only` | `bool` | No (default `true`) | **Deprecated.** This option was an early design knob for toggling the `CLOSE_ONLY`-stance-to-`reduce_only` flag pipeline. The Execution Layer now unconditionally forces `reduce_only = true` for any order dispatched under a `CLOSE_ONLY` stance (see [TAE Layer 2 §3.3.1](03-03-03-tae-layer2-execution.md#331-safety-invariant-unconditional-force)), making this option a no-op for behavior. It is preserved in the schema for forward-compatibility reads; new policies should leave it at the default `true`. Operators who set it to `false` get the safe `true` behavior at load time. |

> **Stance is not a policy field.** Execution authorization is read from the per-symbol stance ([03-04-05 §2](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)) at dispatch; the stance lifecycle is specified in §5.

### 2.2 Condition Structure

```
ConditionGroup ::= AND(Condition*) | OR(Condition*)
Condition      ::= { field: string, operator: Operator, value: Value }
Operator       ::= EQ | GT | LT | GTE | LTE | IN | BETWEEN | NOT_EQ
Value          ::= number | string | [number] | [string]
```

### 2.3 Available Condition Fields

| Field Path | Type | Source | Example Values |
|-----------|------|--------|---------------|
| `decision.bias` | `string` | Decision Matrix | `"BULLISH"`, `"STRONG_BULLISH"` |
| `decision.confidence_assessment` | `number` | Decision Matrix | `72.0` (0–100) |
| `decision.market_stance` | `string` | Decision Matrix | `"AGGRESSIVE"`, `"CONSTRUCTIVE"` |
| `decision.directional_guidance` | `string` | Decision Matrix | `"STRONG_LONG"`, `"LONG"` |
| `decision.strategy_environment` | `string` | Decision Matrix | `"TREND_FOLLOWING"`, `"BREAKOUT"` |
| `decision.entry_guidance` | `string` | Decision Matrix | `"IMMEDIATE"`, `"PULLBACK"` |
| `analysis.market_regime` | `string` | Analysis Matrix | `"TRENDING_BULL"`, `"RANGE"` |
| `analysis.market_quality` | `string` | Analysis Matrix | `"GOOD"`, `"EXCELLENT"` |
| `opportunity.primary_opportunity` | `string` | Opportunity Matrix (L4) | `"BREAKOUT"`, `"TREND_CONTINUATION"`, `"LIQUIDITY_SQUEEZE"`, `"SCALP"`, … — see [02-08-opportunity-matrix.md §3](../../matrices/02-08-opportunity-matrix.md) for the canonical eight-variant precondition table (canonical producer — replaces the removed `decision.opportunity_type` field per the v2.1 institutional redesign; see [02-00-matrix-field-ownership.md §3](../../matrices/02-00-matrix-field-ownership.md) for the migration map) |
| `opportunity.opportunity_score` | `number` | Opportunity Matrix | `85.0` (0–100) |
| `risk.market_risk.score` | `number` | Risk Matrix | `35.0` (0–100) |
| `risk.volatility_risk.score` | `number` | Risk Matrix | `45.0` (0–100) |
| `risk.execution_liquidity_risk.score` | `number` | Risk Matrix | `15.0` (0–100) |
| `risk.structure_risk.score` | `number` | Risk Matrix | `25.0` (0–100) |
| `risk.momentum_risk.score` | `number` | Risk Matrix | `20.0` (0–100) |
| `risk.signal_risk.score` | `number` | Risk Matrix | `30.0` (0–100) |
| `risk.execution_risk.score` | `number` | Risk Matrix | `25.0` (0–100) |
| `risk.cascade_risk.score` | `number` | Risk Matrix | `30.0` (0–100) |
| `risk.overall_risk.score` | `number` | Risk Matrix | `28.3` (0–100) |

---

## 3. Example Policies

### 3.1 Trend Following — Long

```
AND(
  decision.bias IN ["BULLISH", "STRONG_BULLISH"],
  decision.confidence_assessment GTE 60,
  decision.market_stance IN ["AGGRESSIVE", "CONSTRUCTIVE"],
  opportunity.primary_opportunity IN ["TREND_CONTINUATION", "BREAKOUT"],
  risk.overall_risk.score LT 40
)
→ Trigger LONG
```

### 3.2 Breakout — Long

```
AND(
  decision.directional_guidance IN ["STRONG_LONG", "LONG"],
  opportunity.primary_opportunity EQ "BREAKOUT",
  opportunity.opportunity_score GTE 75,
  risk.volatility_risk.score LT 50,
  risk.overall_risk.score LT 30
)
→ Trigger LONG
```

### 3.3 Mean Reversion — Short

```
AND(
  analysis.market_regime IN ["RANGE", "TRANSITION"],
  decision.bias IN ["BEARISH", "STRONG_BEARISH"],
  decision.entry_guidance IN ["IMMEDIATE", "WAIT_FOR_CONFIRMATION"],
  risk.overall_risk.score LT 50
)
→ Trigger SHORT
```

---

## 4. Trigger Modes

| TriggerMode | Type | Fires On | Use Case |
|-------------|------|----------|----------|
| `Interval { seconds }` | Periodic | Every N seconds. | High-frequency scalping. |
| `CandleClose { timeframe, count }` | Event | Every N completed candles. | Swing / position trading. |
| `EventDriven { events }` | Event | Named MME signal events. | Precision entries on squeeze release, S/R flip, etc. |

**EventDriven event names:** `SQUEEZE_RELEASE`, `DIVERGENCE_CONFIRMED`, `BREAKOUT_CONFIRMED`, `TREND_FLIP`, `SR_FLIP`, `PATTERN_CONFIRMED`, `VOLUME_CLIMAX`.

Multiple trigger modes may be active simultaneously on a single policy.

---

## 5. Stance Lifecycle

| Stance | Behaviour |
|--------|-----------|
| `ACTIVE` | Policy evaluates normally; may trigger orders. |
| `CLOSE_ONLY` | Only exit/protection operations; new entries blocked. Forces `reduce_only = true` on all dispatched orders (see [TAE Layer 2 §3.3](03-03-03-tae-layer2-execution.md#33-closeonly-stance--reduce_only-flag-handoff)). |
| `AVOID` | Policy suspended; all triggers ignored. |

### 5.1 Stance Transitions

| From | To | Driver |
|------|----|--------|
| `ACTIVE` | `CLOSE_ONLY` | PME veto (margin ceiling, loss streak, exposure limit) or operator |
| `CLOSE_ONLY` | `ACTIVE` | Operator, after veto release per [03-04-05 §4.3](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) |
| `ACTIVE` | `AVOID` | PME veto (drawdown breach, margin exhaustion, systemic risk) or operator disable |
| `AVOID` | `ACTIVE` | Operator re-enable, after veto release per [03-04-05 §4.3](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) |
| `CLOSE_ONLY` | `AVOID` | PME veto escalates |
| `AVOID` | `CLOSE_ONLY` | Operator (a veto downgrade is not automatic) |

Transitions to `CLOSE_ONLY` or `AVOID` from PME veto are **irreversible by the policy itself** — only manual operator confirmation can restore `ACTIVE`. All transitions are audit-logged.

> **Policy `AUTO_PAUSED` (CA-10/QA-3, scope = policy).** Distinct from the instance-scope lifecycle `PAUSED` defined in [03-03-06-tae-instance-lifecycle-spec.md](../trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md). When a policy-level `AUTO_PAUSED` fires (consecutive losses, operator override), the affected **policy** stops evaluating while the instance continues trading under its other policies; this never moves the instance's `LifecycleState`. Conversely, an instance-level `PAUSED` (lifecycle Gate 0) blocks entries for **every** policy on that instance but leaves policy state machines intact. The two axes are independent — see [03-03-06 §6 Interaction matrix](../trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md).

---

## 6. Position Sizing Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `risk_per_trade_pct` | `number` | 1.0 | Risk-per-trade as a percentage of margin. Converted to the formula's decimal fraction $R = \text{risk\_per\_trade\_pct} / 100$. |
| `max_position_size_usd` | `number` | (unlimited) | Hard cap on position notional. |
| `max_leverage` | `number` | 20 | Maximum leverage for this policy. |
| `use_dynamic_stops` | `bool` | true | Whether to use MME-recommended stop distances. |
| `fixed_stop_loss_pct` | `number` | (optional) | Fixed stop distance; overrides MME dynamic stops if set. |
| `target_rr_ratio` | `number` | 2.5 | Desired reward-to-risk ratio. |

The position size $S$ is computed via the [Position Sizing Protocol](03-03-03-tae-layer2-execution.md#2-position-sizing-protocol):

$$S = \frac{E \times R}{D_{sl} / 100}$$

where $E$ = available margin, $R$ = `risk_per_trade_pct / 100` (decimal fraction), and $D_{sl}$ = `stop_loss_distance_pct` (raw percentage float, divided by 100 in the formula).

---

## 7. Policy Evaluation Engine

### 7.1 Evaluation Flow

```
1. MME Decision Matrix arrives (push).
2. Filter: only Active-stance, enabled policies targeting this symbol.
3. Check cooldown: skip if triggered within cooldown_seconds.
4. Evaluate condition tree (short-circuit AND, first-match OR).
5. If all conditions satisfied → policy TRIGGERED.
6. Emit trigger → TAE Execution Layer → Position Sizing → Order dispatch.
7. Record trigger in Policy Matrix with timestamp and decision snapshot.
```

### 7.2 Conflict Resolution

If multiple policies trigger for the same symbol simultaneously:
1. Policies specifying the same direction → merge (use the larger confidence).
2. Policies specifying opposite directions → **block both**; log conflict for operator review.

---

## 8. Observability

Every policy trigger is logged with:

| Field | Description |
|-------|-------------|
| `policy_id` | Which policy triggered. |
| `trigger_timestamp` | When. |
| `decision_snapshot` | Complete Decision Matrix at trigger time (for audit). |
| `conditions_evaluated` | Which conditions passed/failed. |
| `result` | `TRIGGERED` / `BLOCKED_COOLDOWN` / `BLOCKED_CONFLICT` / `SKIPPED_STANCE`. |

This data is available via `GET /api/system/observability` (see [API Gateway Contract](../../integration-and-api/06-01-api-gateway-contract.md)).

---

## 9. Cross-References

- [TAE Overview](03-03-01-tae-overview-spec.md) — Engine boundaries and operational modes.
- [TAE Layer 1 — Policy](03-03-02-tae-layer1-policy.md) — Policy evaluation engine specification.
- [TAE Layer 2 — Execution](03-03-03-tae-layer2-execution.md) — Position Sizing Protocol and order dispatch.
- [TAE Instance Lifecycle & Programmable State Control](03-03-06-tae-instance-lifecycle-spec.md) — Lifecycle × policy orthogonality, scoped-enum rule.
- [Decision Matrix](../../matrices/02-04-decision-matrix.md) — Primary intelligence source.
- [PME Layer 4 — Portfolio](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — Veto authority over stances.
- [Ontology — Execution Policy](../../conceptual-foundations/01-01-ontology.md) — Conceptual definitions.
