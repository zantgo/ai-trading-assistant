# PME Layer 1 — Position Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Portfolio Management Engine (PME)
**Layer:** 1 of 4
**Input Contract:** Exchange execution events and order fill confirmations (from TAE)
**Output Contract:** Position Matrix (active position directory with live valuation metrics)
**Purpose:** This document specifies the Position Layer — the active exposure tracking system that monitors individual positions, updates mark-to-market valuations, and coordinates dynamic stop adjustments.

---

## 1. Purpose

The Position Layer is the PME's **active position tracker**. It receives execution fill events from the TAE and exchange, initializes position records, continuously updates mark-to-market valuations from live DIE price feeds, computes unrealized PnL and ROI, and manages dynamic stop-loss and take-profit levels.

```
[Exchange Fills] ──► POSITION LAYER (L1) ──► [Position Matrix] ──► [Exposure Layer (L2)]
```

---

## 2. Position Lifecycle

```
        fill confirmed
             │
             ▼
       ┌─────────────┐   mark-price updates    ┌─────────────┐
       │  OPEN        │───────────────────────►│  MANAGING    │
       └─────────────┘   (continuous)          └─────────────┘
             │                                        │
             │ fill cancelled / rejected              │ exit trigger (stop / target / signal)
             ▼                                        ▼
       ┌─────────────┐                          ┌─────────────┐
       │  REJECTED    │                          │  CLOSED     │
       └─────────────┘                          └─────────────┘
```

---

## 3. Position Matrix Schema

### 3.1 Core Position Fields

| Field | Type | Description |
|-------|------|-------------|
| `position_id` | `u64` | Unique position identifier. |
| `symbol` | `string` | Instrument (e.g., `BTC-USDT`). |
| `direction` | `Direction` | `Long` / `Short`. |
| `entry_price` | `Decimal` | Initial fill price. |
| `average_entry_price` | `Decimal` | Volume-weighted average entry (adjusts on scaled entries). |
| `size` | `Decimal` | Current base-asset quantity. |
| `allocated_usd` | `Decimal` | Notional value at entry. |
| `entry_timestamp` | `u64` | Unix epoch of first fill. |

### 3.2 Live Valuation Fields

| Field | Type | Description |
|-------|------|-------------|
| `current_price` | `Decimal` | Latest DIE mid-price for this symbol. |
| `unrealized_pnl` | `Decimal` | Live dollar profit/loss (before fees). |
| `roi_pct` | `Decimal` | Live percentage return. |
| `unrealized_pnl_after_fees` | `Decimal` | Live PnL with estimated exit fees deducted. |

### 3.3 Protection Fields

| Field | Type | Description |
|-------|------|-------------|
| `stop_loss_price` | `Decimal` | Current stop-loss trigger level. |
| `take_profit_price` | `Decimal` | Current take-profit target level. |
| `final_invalidation_level` | `Decimal` | Structural level whose breach nullifies the thesis. |
| `target_profit_ratio` | `Decimal` | Desired reward-to-risk ratio for this position. |

### 3.4 Scaled Entry Fields

| Field | Type | Description |
|-------|------|-------------|
| `current_portions` | `u32` | Number of filled position slots (1–4). |
| `initial_allocated_margin` | `Decimal` | Margin committed at first entry. |
| `realized_pnl_accumulator` | `Decimal` | PnL from partially closed portions. |

---

## 4. Dynamic Stop Management

The Position Layer reads updated invalidation levels from the MME [Decision Matrix](../../matrices/02-04-decision-matrix.md) as market structure evolves:

1. MME Decision Layer publishes updated `stop_loss_distance_pct` and `invalidation_levels`.
2. Position Layer recomputes `stop_loss_price` from the new distance.
3. If the new stop improves the position (closer to entry for profitable trades, further from entry for losing trades), the stop is tightened.
4. Updated stop is routed to the exchange via the TAE Execution Layer.

---

## 5. Scaled Entry Model (Position Slots)

Positions support up to 4 scaling slots via `position_slots`:

| Slot | Behaviour |
|------|-----------|
| Slot 1 | Initial entry — always filled first. |
| Slot 2–4 | Conditional scaling — triggered when price moves favourably by a configurable percentage. |
| Allocation curve | `Stepped` (equal portions), `Linear` (linearly increasing), or `Exponential` (aggressive scaling). |

Each slot's entry updates `average_entry_price` (volume-weighted) and `allocated_usd`.

---

## 6. Interaction with Capital & Exposure

- The Position Layer reports `allocated_usd` and `unrealized_pnl` to the [Capital Layer](03-04-04-pme-layer3-capital.md) for margin tracking.
- The Position Layer reports `direction × size` to the [Exposure Layer](03-04-03-pme-layer2-exposure.md) for concentration checks.

---

## 7. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Deterministic valuation** | Unrealized PnL is computed identically from `(current_price − entry_price) × size`. |
| **Stop improvement only** | Dynamic stops only tighten; they never widen automatically. |
| **Partial-close tracking** | Realized PnL from partially closed portions is accumulated separately. |

---

## 8. Cross-References

- [PME Overview](../portfolio-management-engine/03-04-01-pme-overview-spec.md) — Engine boundaries and ledger model.
- [PME Layer 2 — Exposure](03-04-03-pme-layer2-exposure.md) — Concentration and net exposure.
- [PME Layer 3 — Capital](03-04-04-pme-layer3-capital.md) — Margin and equity tracking.
- [Decision Matrix](../../matrices/02-04-decision-matrix.md) — Invalidation level source.
- [Database Schema](../../integration-and-api/06-02-database-schema-spec.md) — `active_positions`, `position_slots`, `position_equity_snapshots`.
- [Ontology — Position](../../conceptual-foundations/01-01-ontology.md) — Conceptual definition.
