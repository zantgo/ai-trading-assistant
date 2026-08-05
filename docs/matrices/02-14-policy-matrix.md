# Policy Matrix Specification

**Version:** 6.9 (2026-08-04) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Trade Automation Engine (TAE)
**Producing Layer:** Layer 1 — Policy Layer
**Purpose:** This document defines the physical schema of the **Policy Matrix** — the set of validated execution directives produced by the TAE Policy Layer after evaluating user-configured execution policies against MME decision intelligence. It is the TAE input handoff between Policy evaluation and Execution routing.

---

## 1. Conceptual Definition

Per the [Ontology](../conceptual-foundations/01-01-ontology.md) §3.14, the **Policy Layer** bridges passive market intelligence and active execution. It consumes symbol-specific Decision Matrices and global Overview Matrices from the MME, evaluates them against active user-configured execution policies, and produces a **Policy Matrix** containing validated entry, exit, and management directives.

The Policy Matrix is a **transient in-memory structure** — it is produced per trigger cycle and consumed immediately by the Execution Layer (L2). It is not independently persisted beyond the associated `risk_control_events` and `open_orders` audit trails.

```
[Decision Matrix] ─┐
                   ├──► POLICY LAYER (L1) ──► [Policy Matrix] ──► [Execution Layer (L2)]
[Overview Matrix] ─┘     evaluate_policies()
```

---

## 2. Schema

| Field | Type | Description |
|-------|------|-------------|
| `policy_id` | `string` | Source policy identifier. |
| `symbol` | `string` | Target instrument. |
| `direction` | `Direction` | `Long` / `Short`. |
| `trigger_timestamp` | `u64` | Unix epoch when triggered. |
| `decision_context` | `DecisionContext` | The MME decision snapshot that triggered this policy. |
| `stance` | `Stance` | Current per-symbol execution-authorization state (`ACTIVE` / `CLOSE_ONLY` / `AVOID`). Managed by PME Veto (see [PME Layer 4 §4](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)). Persisted as `active_stance` in the DB schema ([06-02 §3.5](../integration-and-api/06-02-database-schema-spec.md)). |
| `risk_parameters` | `RiskParams` | Risk-per-trade %, max position size. |

---

## 3. Production Rules

The Policy Matrix is produced by `evaluate_policies()` (see [TAE Layer 1 — Policy Layer](../engines/trade-automation-engine/03-03-02-tae-layer1-policy.md)):

1. Receive Decision Matrix + Overview Matrix (MME push).
2. Filter policies to active-stance policies for matching symbols.
3. For each policy, evaluate conditions sequentially (short-circuit on first failing AND clause).
4. If all conditions pass → policy is TRIGGERED.
5. Emit validated directive to the Execution Layer (L2).

The matrix is produced only when an active policy is triggered. There is no periodic publishing cadence — it is event-driven.

---

## 4. Consumption

| Consumer | Module | Usage |
|----------|--------|-------|
| **TAE Execution Layer (L2)** | `03-03-03-tae-layer2-execution.md` | Translates directives into order packets, runs Position Sizing Protocol, dispatches to exchange. |
| **PME Veto Override** | `03-03-02-tae-layer1-policy.md §7` | Hard Exit path writes emergency liquidation directives into the matrix before stance changes. |

---

## 5. Cross-References

- [TAE Layer 1 — Policy Layer](../engines/trade-automation-engine/03-03-02-tae-layer1-policy.md) — Producing-layer specification.
- [TAE Layer 2 — Execution Layer](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md) — Primary consumer.
- [Decision Matrix](02-04-decision-matrix.md) · [Overview Matrix](02-09-overview-matrix.md) — Input contracts.
- [Matrix Field Ownership](02-00-matrix-field-ownership.md) — Canonical per-field ownership mapping.
