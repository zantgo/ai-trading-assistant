# Trade Automation Engine — Overview Specification

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Approved
**Engine:** Trade Automation Engine (TAE)
**Purpose:** This document specifies the boundaries, API limits, transaction state-transition model, and order-management architecture of the Trade Automation Engine — the engine that evaluates user-defined execution policies against MME decision support and routes orders to live or simulated venues.

---

## 1. Mission & Boundaries

The TAE is the platform's **execution authority**. It consumes the [Decision Matrix](../../matrices/02-04-decision-matrix.md) from the MME, evaluates it against user-configured execution policies, sizes positions, and transmits orders. It performs **no market interpretation** and holds **no capital ledger** (that is the PME's domain).

```
[Decision Matrix] ──► TAE ──► [Order Packets] ──► [Exchange / Paper Engine]
       │                │
       │                └──(reads Capital Matrix)──► [PME]
```

### 1.1 Layer Structure

| Layer | Name | Output |
|-------|------|--------|
| L1 | [Policy Layer](03-03-02-tae-layer1-policy.md) | Policy Matrix (validated directives) |
| L2 | [Execution Layer](03-03-03-tae-layer2-execution.md) | Execution Matrix (order lifecycle) |

Plus the [Paper Trading](03-03-05-tae-paper-trading-spec.md) simulated matching engine.

---

## 2. Operational Modes

The TAE operates in one of several configured modes (`OperationalMode`):

| Mode | Behaviour |
|------|-----------|
| `ManualOnly` | No automated order dispatch; operator triggers manually. |
| `DeterministicHeuristics` | Rule-based policy evaluation drives automated triggers. |

Trigger cadence is governed by `TriggerMode`:

| TriggerMode | Fires On |
|-------------|----------|
| `Interval { seconds }` | Fixed time interval. |
| `CandleClose { timeframe, count }` | Every N completed candles of a timeframe. |
| `EventDriven { events }` | Named MME events (squeeze release, S/R flip, etc.). |

> **`OperationalMode` vs `TriggerMode`.** `OperationalMode` answers *"is trading automated at all?"* — `ManualOnly` suspends all automated dispatch; `DeterministicHeuristics` enables policy-driven evaluation. `TriggerMode` answers *"when does an enabled policy evaluate?"* — a `ManualOnly` operational mode has no `TriggerMode` because no automated evaluation runs, while a `DeterministicHeuristics` mode pairs with one or more `TriggerMode`s. The two are orthogonal.

---

## 3. Transaction State Machine

Every order transitions through a logged lifecycle:

```
        ┌──────────┐   size+route   ┌──────────┐   ack    ┌──────────┐
        │  PENDING │───────────────►│ SUBMITTED│─────────►│  OPEN    │
        └──────────┘                └──────────┘          └──────────┘
             │                            │                    │  partial fill
             │ reject                     │ cancel             ▼
             ▼                            ▼              ┌─────────────────┐
        ┌──────────┐                ┌──────────┐          │ PARTIALLY_FILLED │
        │ REJECTED │                │ CANCELLED│          └─────────────────┘
        └──────────┘                └──────────┘             │            │
                                                             │ more fill  │ cancel
                                                             ▼            ▼
                                                        ┌──────────┐  ┌──────────┐
                                                        │  CLOSED  │  │ CANCELLED│
                                                        └──────────┘  └──────────┘
```

Every transition is written to the Execution Matrix with a high-resolution timestamp, guaranteeing full auditability.

---

## 4. Order Management

| Concern | Design |
|---------|--------|
| Order types | Market, Limit, Stop, Reduce-Only. |
| Order registry | `open_orders` table tracks outstanding orders (`order_type`, `direction`, `price`, `trigger_price`, `size`, `is_reduce_only`, `associated_position_id`). |
| Partial fills | Tracked against the associated position. |
| Reduce-only | Enforced during veto / de-risking. |

---

## 5. API Limits

| Concern | Policy |
|---------|--------|
| Rate limiting | Order dispatch throttled to venue limits; batched cancellations on veto. |
| Idempotency | Client order IDs prevent duplicate submission on retry. |
| Capital query | Synchronous read-only pull from PME Capital Matrix at sizing time. |

---

## 6. Position Sizing Protocol

The TAE's defining computation is the Position Sizing Protocol (see [Execution Layer](03-03-03-tae-layer2-execution.md)):

$$S = \frac{E \times R}{D_{sl} / 100}$$

where `E` = available margin (from PME), `R` = risk-per-trade as a decimal fraction (`risk_per_trade_pct / 100`), `D_sl` = stop-loss distance as a raw percentage float (divided by 100 in the formula). Implemented in `crates/portfolio-supervisor/src/risk_calculator.rs`.

---

## 7. Cross-References

- [TAE Layer 1 — Policy](03-03-02-tae-layer1-policy.md)
- [TAE Layer 2 — Execution](03-03-03-tae-layer2-execution.md)
- [TAE Paper Trading](03-03-05-tae-paper-trading-spec.md)
- [Decision Matrix](../../matrices/02-04-decision-matrix.md) — Input contract.
- [Systemic Data Flow — Sequence B](../../conceptual-foundations/01-03-systemic-data-flow.md) — Entry loop.
- [PME Layer 3 — Capital](../portfolio-management-engine/03-04-04-pme-layer3-capital.md) — Equity source.
