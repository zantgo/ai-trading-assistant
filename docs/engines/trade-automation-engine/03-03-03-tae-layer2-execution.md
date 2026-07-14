# TAE Layer 2 — Execution Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Trade Automation Engine (TAE)
**Layer:** 2 of 2
**Input Contract:** Policy Matrix (L1), [Capital Matrix](../portfolio-management-engine/03-04-04-pme-layer3-capital.md) (PME), [Decision Matrix](../../matrices/02-04-decision-matrix.md) (MME)
**Output Contract:** Execution Matrix (order lifecycle)
**Purpose:** This document specifies the Execution Layer — the transaction engine that translates validated Policy directives into exchange orders, manages order state machines, executes the Position Sizing Protocol, and maintains full transaction auditability.

---

## 1. Purpose

The Execution Layer is the platform's **order routing authority**. It receives triggered directives from the Policy Layer (L1), queries the PME Capital Matrix for available margin, computes position sizes, constructs order packets, dispatches them to exchanges (live or simulated), and manages the complete order lifecycle.

```
[Policy Matrix] ──► EXECUTION LAYER (L2) ──► [Exchange / Paper Engine]
                            │
                            └──(reads Capital Matrix)──► [PME]
```

---

## 2. Position Sizing Protocol

The defining computation of the Execution Layer is the **Position Sizing Protocol**:

$$S = \frac{E \times R}{D_{sl} / 100}$$

| Symbol | Meaning | Source | Units |
|--------|---------|--------|-------|
| $S$ | Position size (notional, quote currency) | Computed | — |
| $E$ | Available margin (quote currency) | PME Capital Matrix (`available_margin`) | — |
| $R$ | Risk-per-trade as a decimal fraction | User configuration (`risk_per_trade_pct / 100`) | fraction (0.01 = 1%) |
| $D_{sl}$ | Stop-loss distance percentage | MME Decision Matrix (`stop_loss_distance_pct`) | raw percent (1.5 = 1.5%) |

**Unit convention:** $R$ is a decimal fraction (the config field `risk_per_trade_pct = 1.0` is divided by 100 to yield $R = 0.01$). $D_{sl}$ is consumed as a **raw percentage float** (`stop_loss_distance_pct = 1.5` means 1.5%), so it is divided by 100 inside the formula to convert to a fraction before dividing.

**Example:** $E = \$10{,}000$, $R = 0.01$ (1%), $D_{sl} = 1.5$ (1.5%) → $S = \frac{10000 \times 0.01}{1.5 / 100} = \frac{100}{0.015} = \$6{,}666.67$ notional. This risks $\$100$ (1% of margin) on a 1.5% stop.

The size is then converted to base-asset units using the current mid-price and rounded to the exchange's minimum order size increment.

### 2.1 Type-Boundary Cast (f64 → Decimal)

> **Target Architecture (Not Yet Implemented).** The Execution Layer is the **cold path** entry point: it reads `stop_loss_distance_pct` ($D_{sl}$) from the Decision Matrix as an `f64`, pulls available margin ($E$) from the PME Capital Matrix as a `Decimal`, and performs a safe, rounded cast so the sizing runs entirely in fixed-point:
>
> ```rust
> // stop_loss_distance_pct is f64 (e.g. 1.5); available_margin & risk_pct are Decimal
> let d_sl = Decimal::from_f64_retain(stop_loss_distance_pct / 100.0)?; // 1.5 → 0.015
> let size_usd = (available_margin * risk_pct) / d_sl;                  // Decimal math
> ```
>
> *Current implementation:* the sizing math in `crates/engine/src/risk_calculator.rs` runs in `f64` end-to-end (`capital`, `max_risk_pct`, `position_notional` are all `f64`); the `Decimal` boundary cast is a planned migration, not the present behaviour.

---

## 3. Order Construction

### 3.1 Order Types

| Order Type | Usage |
|------------|-------|
| `Market` | Immediate execution at best available price; used when fill speed is critical. |
| `Limit` | Executes at specified price or better; used for entries with defined levels. |
| `Stop` | Triggers a market order when a price threshold is breached; used for stop-loss exits. |

> **`reduce_only` is an order *attribute*, not an order type.** Any of the three order types above may carry the `reduce_only = true` flag (§3.2), which guarantees the order can only decrease net exposure. This is distinct from the `CloseOnly` policy **stance** (L1) — see §3.3 for the stance→flag handoff.

### 3.2 Order Packet Fields

| Field | Type | Description |
|-------|------|-------------|
| `client_order_id` | `string` | Idempotency key — prevents duplicate submission on retry. |
| `symbol` | `string` | Instrument. |
| `side` | `Side` | `Buy` / `Sell`. |
| `order_type` | `OrderType` | `Market` / `Limit` / `Stop`. |
| `price` | `Decimal` | Limit/stop trigger price (null for market). |
| `size` | `Decimal` | Base-asset quantity. |
| `reduce_only` | `bool` | Whether the order carries the reduce-only flag (a per-order attribute, NOT an order type). Mirrors the exchange-native concept (Hyperliquid `reduceOnly`, Bitget/Binance `reduceOnly`). Independent of — but deterministically populated by — the Policy Layer's `CloseOnly` stance; see §3.3. |
| `associated_position_id` | `u64` | Position this order relates to (for exits/modifications). |

---

## 4. Transaction State Machine

Every order transitions through a logged lifecycle:

```
         ┌──────────┐   size+route   ┌──────────┐   ack    ┌──────────┐
         │  PENDING │───────────────►│ SUBMITTED│─────────►│  OPEN    │
         └──────────┘                └──────────┘          └──────────┘
              │                            │                    │
              │ reject                     │ cancel             │ fill / stop / target
              ▼                            ▼                    ▼
         ┌──────────┐                ┌──────────┐          ┌──────────┐
         │ REJECTED │                │ CANCELLED│          │  CLOSED  │
         └──────────┘                └──────────┘          └──────────┘
```

Every transition is written to the Execution Matrix with a high-resolution timestamp, guaranteeing full auditability. Partial fills are tracked against the associated position.

---

## 5. Slippage Control

| Mechanism | Description |
|-----------|-------------|
| **Limit-offset filter** | Limit orders are placed with a configurable offset from the reference price to protect against adverse fills. |
| **Book-depth check** | Immediately before dispatch, the Execution Layer queries the real-time order book depth to verify sufficient liquidity at the target price level. |
| **Slippage ceiling** | If estimated slippage exceeds a user-configured percentage of the position size, the order is held for manual review. |

---

## 6. API Rate Limiting & Idempotency

| Concern | Policy |
|---------|--------|
| **Rate limiting** | Order dispatch throttled to venue limits; batched cancellations on veto. |
| **Idempotency** | `client_order_id` prevents duplicate submission on retry — the exchange ignores duplicate IDs. |
| **Capital query** | Synchronous read-only pull from PME Capital Matrix at sizing time. |

---

## 7. Output: Execution Matrix

The Execution Matrix is a persistent log of all order states:

| Field | Type | Description |
|-------|------|-------------|
| `order_id` | `string` | Exchange-assigned order ID. |
| `client_order_id` | `string` | Idempotency key. |
| `symbol` | `string` | Instrument. |
| `order_type` | `string` | `MARKET` / `LIMIT` / `STOP`. |
| `direction` | `string` | `BUY` / `SELL`. |
| `price` | `Decimal` | Order price (limit/stop). |
| `trigger_price` | `Decimal` | Trigger price (stop orders). |
| `size` | `Decimal` | Base-asset quantity. |
| `filled_size` | `Decimal` | Cumulative filled quantity. |
| `status` | `string` | `PENDING` / `SUBMITTED` / `OPEN` / `PARTIALLY_FILLED` / `CLOSED` / `REJECTED` / `CANCELLED`. |
| `is_reduce_only` | `bool` | Reduce-only flag. |
| `associated_position_id` | `u64` | Linked position. |
| `created_at` | `u64` | Unix epoch timestamp. |
| `updated_at` | `u64` | Last state transition timestamp. |
| `slippage_bps` | `f64` | Fill slippage in basis points. |

---

## 8. Paper Trading Engine

When operating in paper/simulated mode (see [TAE Paper Trading](03-03-05-tae-paper-trading-spec.md)), the Execution Layer routes orders to the internal matching engine instead of a live exchange. The same state machine, sizing protocol, and audit logging apply — only the order destination changes.

---

## 9. Cross-References

- [TAE Overview](../trade-automation-engine/03-03-01-tae-overview-spec.md) — Engine boundaries and operational modes.
- [TAE Layer 1 — Policy](03-03-02-tae-layer1-policy.md) — Upstream trigger source.
- [TAE Paper Trading](03-03-05-tae-paper-trading-spec.md) — Simulated execution engine.
- [Decision Matrix](../../matrices/02-04-decision-matrix.md) — Stop-loss distance source.
- [PME Layer 3 — Capital](../portfolio-management-engine/03-04-04-pme-layer3-capital.md) — Equity source.
- [Ontology — Trade Execution](../../conceptual-foundations/01-01-ontology.md) — Conceptual definition.
