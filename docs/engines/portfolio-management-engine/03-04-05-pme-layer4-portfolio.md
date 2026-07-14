# PME Layer 4 — Portfolio Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Portfolio Management Engine (PME)
**Layer:** 4 of 4
**Input Contract:** Position Matrix (L1), Exposure Matrix (L2), Capital Matrix (L3), [Overview Matrix](../../matrices/02-09-overview-matrix.md) (MME L7)
**Output Contract:** Portfolio Matrix (unified master ledger + safety veto)
**Purpose:** This document specifies the Portfolio Layer — the top-level consolidation layer that synthesizes all child matrices into a unified account health vector and enforces the Ontological Priority Veto over automated trading.

---

## 1. Purpose

The Portfolio Layer is the PME's **command authority**. It synthesizes Position, Exposure, and Capital matrices into a single unified ledger, continuously evaluates systemic safety thresholds, and — when a threshold is breached — asserts **Ontological Priority (Veto Power)** to override TAE stances, blocking new entries and canceling pending orders.

```
[Position Matrix ] ─┐
[Exposure Matrix ] ─┼──► PORTFOLIO LAYER (L4) ──► [Portfolio Matrix]
[Capital Matrix  ] ─┘                                  │
[Overview Matrix ] ─┘                                  │
                                                       └──(veto)──► [TAE stances]
```

---

## 2. Portfolio Matrix Schema

| Field | Type | Description |
|-------|------|-------------|
| `current_equity` | `Decimal` | Total account equity. |
| `realized_pnl` | `Decimal` | Cumulative realized profit/loss. |
| `unrealized_pnl` | `Decimal` | Aggregate unrealized PnL. |
| `gross_exposure` | `Decimal` | Total notional exposure. |
| `net_exposure` | `Decimal` | Directional net exposure. |
| `margin_usage_ratio` | `Decimal` | Percentage of equity committed to margin. |
| `leverage_ratio` | `Decimal` | Effective leverage (`gross_exposure / equity`). |
| `daily_pnl` | `Decimal` | PnL in current session. |
| `max_daily_drawdown_pct` | `Decimal` | Cumulative PnL decline within the trading session. Distinct from `drawdown_limit_pct`; see §4 below. |
| `drawdown_limit_pct` | `Decimal` | Equity peak-to-trough decline threshold. Default 30 %. This is the **hard veto** stop-loss denominator (see §4). |
| `safety_state` | `SafetyState` | `NORMAL` / `CAUTIOUS` / `SUSPENDED` / `DRAWDOWN_STOP`. |
| `systemic_risk_score` | `f64` | MME Overview Matrix Systemic Risk Score. |
| `active_stances` | `map<string, Stance>` | Per-symbol authorization: `ACTIVE` / `CLOSE_ONLY` / `AVOID`. |
| `position_count` | `u32` | Number of active positions. |

---

## 3. Safety Circuit Breakers

The `SafetyManager` (`crates/engine/src/safety.rs`) tracks four escalating safety states:

```
NORMAL ──(consecutive losses ≥ caution)──► CAUTIOUS
       ──(consecutive losses ≥ dropout)──► SUSPENDED (timed cooldown)
       ──(equity drawdown ≥ limit)──────► DRAWDOWN_STOP
```

| State | Trigger | Effect |
|-------|---------|--------|
| `NORMAL` | Default | Full trading permitted. |
| `CAUTIOUS` | ≥ 3 consecutive losses | Warning only; no stance changes yet. |
| `SUSPENDED` | ≥ 5 consecutive losses | All stances → `CLOSE_ONLY`; 8-hour cooldown. A win resets the counter. |
| `DRAWDOWN_STOP` | Equity drawdown ≥ `drawdown_limit_pct` (default 30 %) | All stances → `AVOID`; immediate veto. |

Defaults are configurable via `config.json` `safety`.

---

## 4. Ontological Priority Veto

The Portfolio Layer holds **veto authority** over the TAE. This is the platform's ultimate safety mechanism.

### 4.1 Veto Triggers

| Trigger | Action |
|---------|--------|
| **Drawdown breach** | `current_equity / peak_equity < (1 − drawdown_limit_pct)` |
| **Margin ceiling** | `margin_usage_ratio ≥ 95%` |
| **Systemic risk** | MME Overview `systemic_risk_score` exceeds safety tolerance |
| **Manual override** | Operator-initiated stance change |

### 4.2 Veto Execution

When a veto triggers:

1. Portfolio Layer publishes a high-priority `VetoMessage` to TAE.
2. TAE Policy Layer sets affected symbol stances to `AVOID` or `CLOSE_ONLY`.
3. TAE Execution Layer nullifies pending entry triggers.
4. TAE issues batch cancellation for outstanding orders.
5. The veto is logged with timestamp and rationale for audit.

### 4.3 Veto Release

When the condition clears (e.g., equity recovers above drawdown limit):

1. Safety state transitions back to `NORMAL` (or `CAUTIOUS` if recent losses).
2. Stances are restored to user-configured defaults.
3. Operator must manually re-enable automated trading (one-time confirmation).

---

## 5. Systemic Risk Integration

The Portfolio Layer reads the MME [Overview Matrix](../../matrices/02-09-overview-matrix.md) `systemic_risk_score` on every update:

- The Systemic Risk Score combines market-wide danger factors (see Overview Matrix §4).
- When elevated, the Portfolio Layer may pre-emptively shift stances to `CAUTIOUS` even before a drawdown occurs.
- The operator may configure a `systemic_risk_threshold` to gate fully automated trading.

---

## 6. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Veto priority** | Veto messages take absolute priority over all other TAE operations. |
| **Auditability** | Every veto trigger, state transition, and stance change is logged. |
| **Deterministic thresholds** | Safety states are computed deterministically from equity and loss counters. |
| **Operator override** | Manual stance changes are always permitted, even during veto. |

---

## 7. Cross-References

- [PME Overview](../portfolio-management-engine/03-04-01-pme-overview-spec.md) — Engine boundaries and safety architecture.
- [PME Layer 1 — Position](03-04-02-pme-layer1-position.md) — Position data source.
- [PME Layer 2 — Exposure](03-04-03-pme-layer2-exposure.md) — Exposure and correlation data.
- [PME Layer 3 — Capital](03-04-04-pme-layer3-capital.md) — Equity and margin data.
- [Overview Matrix](../../matrices/02-09-overview-matrix.md) — Systemic Risk Score source.
- [TAE Layer 1 — Policy](../trade-automation-engine/03-03-02-tae-layer1-policy.md) — Veto consumer.
- [Systemic Data Flow — Sequence D](../../conceptual-foundations/01-03-systemic-data-flow.md) — Veto loop sequence.
- [Ontology — Portfolio Management](../../conceptual-foundations/01-01-ontology.md) — Conceptual definitions.
