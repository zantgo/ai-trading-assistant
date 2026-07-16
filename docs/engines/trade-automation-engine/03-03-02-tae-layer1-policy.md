# TAE Layer 1 — Policy Layer

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
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
2. **Step 2a — Hard Exit Path (only for `AVOID` triggers).** For any active positions on the affected symbol, the Policy Layer dispatches a **liquidation directive** (not a cancellation) to the Execution Layer **before** the stance change commits (Step 2c). The Execution Layer converts this directive into a `market order` whose `size` is sourced from the **Position Matrix** (bypassing the Position Sizing Protocol — see [03-03-03-tae-layer2-execution.md §3.5](../trade-automation-engine/03-03-03-tae-layer2-execution.md)). The order carries `reduce_only = true` and `is_emergency_liquidation = true` (bypasses Gate 1 stance check per [08-02-pre-trade-risk-controls.md §3](../operations-and-compliance/08-02-pre-trade-risk-controls.md)). The Execution Layer awaits exchange acknowledgement with bounded timeout `hard_exit_ack_timeout_ms` (default `2000 ms`). For `CLOSE_ONLY` triggers, **Step 2a is skipped entirely** — existing positions are managed by their protective stops and policy exits; no forced liquidation.
3. **Step 2b — Discard pending triggers.** Any pending trigger payloads for the affected symbol are discarded.
4. **Step 2c — Commit stance change.** Policy Layer changes affected symbol stances to `AVOID` or `CLOSE_ONLY`. **For `AVOID` triggers, this step fires only after Step 2a's Hard Exit acknowledgement (or timeout)** — if the stance is committed before the Hard Exit dispatches, Gate 1 in the Pre-Trade Risk Controls would block the liquidation order, leaving the platform's own emergency exit silently rejected.
5. **Step 2d — Cancel remaining orders.** After Hard Exit acknowledgement (or timeout), the Execution Layer issues batch cancellation orders for any remaining outstanding limit/stop orders on the exchange. If acknowledgement exceeds the timeout, the cancellation batch still proceeds — protective stops are cancelled unconditionally to prevent zombie orders, and the liquidation is flagged as `unconfirmed_exit` in the audit trail.
6. **Step 2e — Nullify entry triggers.** Execution Layer nullifies pending entry triggers for the affected symbol.
7. **Step 2f — Audit.** The veto is logged with timestamp, trigger, target stance, and rationale.

> **Veto loophole correction + ordering invariant (v2.1).** A previous version of this section sequenced the steps as *(1) PME override → (2) stance change → (3) discard triggers → (4) Hard Exit dispatch → (5) cancellations → (6) log*. That ordering **inverts** the documented invariant in [03-04-05-pme-layer4-portfolio.md §4.2](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) ("Hard Exit fires *before* stance transitions to `AVOID`") and would cause Gate 1 to block the Hard Exit order. The corrected sequence above — `Step 2a (Hard Exit) → Step 2b (discard) → Step 2c (commit stance) → Step 2d (cancel remaining)` — is non-negotiable. The original loophole concern (cancellations alone leave positions unprotected) is addressed by Step 2a's forced liquidation, which closes open exposure on venue at the time the veto asserts. The 2a-before-2c ordering is also asserted by [01-03-systemic-data-flow.md Sequence D §4–§6](../../conceptual-foundations/01-03-systemic-data-flow.md).

---

## 8. Cross-References

- [TAE Overview](../trade-automation-engine/03-03-01-tae-overview-spec.md) — Engine boundaries and trigger modes.
- [TAE Layer 2 — Execution](03-03-03-tae-layer2-execution.md) — Order construction and routing.
- [TAE Execution Policy Specification](../trade-automation-engine/03-03-04-tae-execution-policy-spec.md) — Formal policy syntax and examples.
- [Decision Matrix](../../matrices/02-04-decision-matrix.md) — Primary input contract.
- [PME Layer 4 — Portfolio](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — Veto authority.
- [Ontology — Execution Policy](../../conceptual-foundations/01-01-ontology.md) — Conceptual definitions.
