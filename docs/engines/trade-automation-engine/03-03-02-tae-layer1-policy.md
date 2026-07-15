# TAE Layer 1 — Policy Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Trade Automation Engine (TAE)
**Layer:** 1 of 2
**Input Contract:** [Decision Matrix](../../matrices/02-04-decision-matrix.md), [Overview Matrix](../../matrices/02-09-overview-matrix.md) (MME)
**Output Contract:** Policy Matrix (validated execution directives)
**Purpose:** This document specifies the Policy Layer — the rule evaluation engine that maps incoming MME decision intelligence against user-configured execution policies and produces validated, triggerable directives.

---

## 1. Purpose

The Policy Layer bridges passive market intelligence and active execution. It consumes symbol-specific Decision Matrices and global Overview Matrices from the MME, evaluates them against active user-configured execution policies, and produces a **Policy Matrix** containing validated entry, exit, and management directives.

```
[Decision Matrix] ─┐
                   ├──► POLICY LAYER (L1) ──► [Policy Matrix] ──► [Execution Layer (L2)]
[Overview Matrix] ─┘     evaluate_policies()
```

It performs **no order construction** and holds **no capital state** — those responsibilities belong to the Execution Layer (L2) and the PME.

---

## 2. Execution Policy Structure

An **Execution Policy** is a deterministic, user-configured conditional rule. Each policy maps market intelligence fields to programmatic trigger boundaries.

### 2.1 Policy Schema

| Field | Type | Description |
|-------|------|-------------|
| `policy_id` | `string` | Unique identifier. |
| `symbol` | `string` | Target instrument (e.g., `BTC-USDT`). |
| `direction` | `Direction` | `Long` / `Short`. |
| `conditions` | `Condition[]` | Ordered set of AND/OR clauses. |
| `trigger_mode` | `TriggerMode` | When the policy is evaluated. |
| `stance` | `Stance` | `ACTIVE` / `CLOSE_ONLY` / `AVOID`. |
| `risk_parameters` | `RiskParams` | Risk-per-trade, max position size, etc. |

### 2.2 Condition Structure

Each condition targets a field from the Decision Matrix:

```
IF (Decision.bias ∈ {"BULLISH", "STRONG_BULLISH"})
   AND (Decision.confidence_assessment > 60)
   AND (Decision.market_stance ∈ {"AGGRESSIVE", "CONSTRUCTIVE"})
   AND (Decision.directional_guidance ∈ {"STRONG_LONG", "LONG"})
THEN Trigger LONG
```

Conditions support operators: `==`, `>`, `<`, `>=`, `<=`, `∈` (in set), `BETWEEN`, `NOT_EQ` (the formal grammar lists all eight in [03-03-04 §2.2](03-03-04-tae-execution-policy-spec.md#22-condition-structure)).

---

## 3. Trigger Modes

The `TriggerMode` controls evaluation cadence per the [TAE Overview](../trade-automation-engine/03-03-01-tae-overview-spec.md#2-operational-modes):

| TriggerMode | Fires On |
|-------------|----------|
| `Interval { seconds }` | Fixed time interval (e.g., every 300 s). |
| `CandleClose { timeframe, count }` | Every N completed candles of a timeframe. |
| `EventDriven { events }` | Named MME events (squeeze release, S/R flip, etc.). |

---

## 4. Symbol Stances

Stances control per-symbol execution authorization:

| Stance | Behaviour |
|--------|-----------|
| `ACTIVE` | Full automated trading permitted — policies evaluate and may trigger orders. |
| `CLOSE_ONLY` | Only exit or protection-tightening operations allowed; new entries blocked. Programmatically forces `reduce_only = true` on every order the Execution Layer generates (see [TAE Layer 2 §3.3](03-03-03-tae-layer2-execution.md#33-closeonly-stance--reduce_only-flag-handoff)). |
| `AVOID` | All execution triggers blocked for this symbol. |

Stances may be changed manually by the operator or automatically by the **PME Veto** (see [PME Layer 4 — Portfolio](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)).

> **Stance vs. order flag:** The `CLOSE_ONLY` *stance* is a Policy-Layer scope restriction (evaluate exit rules only). It is distinct from — but deterministically maps onto — the Execution-Layer `reduce_only` order *attribute* (a per-order flag guaranteeing exposure only decreases). A `CLOSE_ONLY` stance ⇒ exit-only evaluation ⇒ `reduce_only = true` on all resulting order packets.

---

## 5. Policy Evaluation Flow

```
1. Receive Decision Matrix + Overview Matrix (MME push).
2. Filter policies: only Active-stance policies for matching symbols.
3. For each policy, evaluate conditions sequentially (short-circuit on first failing AND clause).
4. If all conditions pass → policy is TRIGGERED.
5. Emit validated directive to Execution Layer (L2):
   { policy_id, symbol, direction, trigger_timestamp, decision_context }
```

---

## 6. Output: Policy Matrix

The Policy Matrix is the set of all validated directives:

| Field | Type | Description |
|-------|------|-------------|
| `policy_id` | `string` | Source policy. |
| `symbol` | `string` | Instrument. |
| `direction` | `Direction` | `Long` / `Short`. |
| `trigger_timestamp` | `u64` | Unix epoch when triggered. |
| `decision_context` | `DecisionContext` | The MME decision snapshot that triggered this policy. |
| `stance` | `Stance` | Current authorization state. |
| `risk_parameters` | `RiskParams` | Risk-per-trade %, max size. |

---

## 7. Veto Override

Per [Systemic Data Flow — Sequence D](../../conceptual-foundations/01-03-systemic-data-flow.md#sequence-d-systemic-safety-veto-the-circuit-breaker-loop), if the PME asserts Ontological Priority:

1. PME publishes a high-priority override message to TAE.
2. Policy Layer processes the override: changes affected symbol stances to `AVOID` or `CLOSE_ONLY`.
3. Any pending trigger payloads for the affected symbol are discarded.
4. Existing open orders are routed to the Execution Layer for cancellation.

---

## 8. Cross-References

- [TAE Overview](../trade-automation-engine/03-03-01-tae-overview-spec.md) — Engine boundaries and trigger modes.
- [TAE Layer 2 — Execution](03-03-03-tae-layer2-execution.md) — Order construction and routing.
- [TAE Execution Policy Specification](../trade-automation-engine/03-03-04-tae-execution-policy-spec.md) — Formal policy syntax and examples.
- [Decision Matrix](../../matrices/02-04-decision-matrix.md) — Primary input contract.
- [PME Layer 4 — Portfolio](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — Veto authority.
- [Ontology — Execution Policy](../../conceptual-foundations/01-01-ontology.md) — Conceptual definitions.
