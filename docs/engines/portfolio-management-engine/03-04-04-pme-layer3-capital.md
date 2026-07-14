# PME Layer 3 — Capital Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Portfolio Management Engine (PME)
**Layer:** 3 of 4
**Input Contract:** Position Matrix (L1), Exposure Matrix (L2), exchange balance events
**Output Contract:** Capital Matrix (high-frequency balance sheet of active and available capital)
**Purpose:** This document specifies the Capital Layer — the account solvency monitor that tracks balances, margin usage, leverage ratios, and equity curves.

---

## 1. Purpose

The Capital Layer is the PME's **balance-sheet authority**. It holds the definitive ledger of the trading account's financial state: available balances, committed margin, realized PnL, and equity curves. It is the sole source queried by the TAE [Execution Layer](../trade-automation-engine/03-03-03-tae-layer2-execution.md) for the Position Sizing Protocol.

```
[Position Matrix] ─┐
                   ├──► CAPITAL LAYER (L3) ──► [Capital Matrix] ──► [TAE sizing]
[Exposure Matrix] ─┘                                               └──► [Portfolio Layer (L4)]
```

---

## 2. Capital Matrix Schema

> **Target Architecture (Not Yet Implemented) / Cold-Path Invariant.** All capital, margin, balance, fee, and equity fields are maintained strictly as 128-bit arbitrary-precision decimals (`rust_decimal::Decimal`) to prevent rounding errors and guarantee penny-perfect cross-asset accounting. This is the OOP/Domain-Driven **cold path**; no `f64` intermediary is permitted in the ledger. The base capital supplied to the TAE Position Sizing Protocol is **`available_margin`** (free buying power), never `current_equity` (which includes unrealized PnL). *Note:* the schema fields below already use `Decimal`; the outstanding gap is that the downstream TAE sizing calculator still consumes these values as `f64` (see [TAE Layer 2 §2.1](../trade-automation-engine/03-03-03-tae-layer2-execution.md)).

### 2.1 Core Balance Fields

| Field | Type | Description |
|-------|------|-------------|
| `initial_balance` | `Decimal` | Starting capital at session initiation. |
| `current_equity` | `Decimal` | `initial_balance + realized_pnl + unrealized_pnl`. |
| `available_margin` | `Decimal` | Liquid capital available for new position initiation. |
| `committed_margin` | `Decimal` | Total margin locked by active positions. |
| `realized_pnl` | `Decimal` | Cumulative PnL from closed trades. |
| `unrealized_pnl` | `Decimal` | Aggregate unrealized PnL from all active positions. |

### 2.2 Risk Metrics

| Field | Type | Description |
|-------|------|-------------|
| `margin_usage_ratio` | `Decimal` | Percentage of total equity committed to maintenance/initial margin. |
| `leverage_ratio` | `Decimal` | `gross_exposure / current_equity`. |
| `max_daily_drawdown_pct` | `Decimal` | Maximum peak-to-trough equity decline in the current session. |
| `current_daily_pnl` | `Decimal` | Equity change since session start. |

---

## 3. Equity Tracking

The Capital Layer maintains a continuous equity time-series:

$$\text{current\_equity} = \text{initial\_balance} + \sum \text{realized\_pnl} + \sum \text{unrealized\_pnl} - \sum \text{fees}$$

Equity snapshots are persisted every 60 seconds to `portfolio_equity_history` for drawdown analysis and performance metrics (see [PAE Layer 3 — Risk Analytics](../performance-analytics-engine/03-05-04-pae-layer3-risk-analytics.md)).

---

## 4. Margin Model

### 4.1 Cross Margin (Default)

All positions share a single margin pool. The engine uses **cross leverage** (default: 20×, configurable via `config.json` `leverage.cross_leverage`). All quantities in the margin model are `rust_decimal::Decimal`.

$$\text{margin\_required} = \frac{\text{position\_notional}}{\text{cross\_leverage}}$$

$$\text{margin\_usage\_ratio} = \frac{\text{committed\_margin}}{\text{current\_equity}}$$

### 4.2 Available Margin

$$\text{available\_margin} = \text{current\_equity} - \text{committed\_margin}$$

This `available_margin` value is the `E` (available margin) supplied to the TAE Position Sizing Protocol — not `current_equity`, which includes unrealized PnL and is not spendable buying power.

---

## 5. Fee Tracking

| Fee Type | Tracking |
|----------|----------|
| **Trading fees** | Deducted from realized PnL on each fill (maker/taker). |
| **Funding payments** | Accrued every 8 hours based on `funding_rate_8h` and position notional. |
| **Spread cost** | Implicit cost captured by comparing fill price to mid-price; logged to `execution_slippage`. |

All fees are configurable via `config.json` `fees`.

---

## 6. Liquidation Risk Monitoring

The Capital Layer continuously monitors `margin_usage_ratio`:

| Threshold | Action |
|-----------|--------|
| `margin_usage_ratio > 80%` | Warning published to Portfolio Layer (L4). |
| `margin_usage_ratio > 95%` | Alert: automatic `CLOSE_ONLY` stance for all symbols. |
| `margin_usage_ratio ≥ 100%` | Potential liquidation — emergency position reduction. |

---

## 7. Interaction with TAE

The Capital Layer is the **single source of truth** for the TAE Position Sizing Protocol. On every sizing event:

1. TAE Execution Layer sends a synchronous request: `query_available_margin(symbol)`.
2. Capital Layer responds with `available_margin` (after reserving margin for the pending order).
3. TAE computes $S = \frac{E \times R}{D_{sl} / 100}$ and dispatches the order.
4. On fill confirmation, Capital Layer updates `committed_margin` and `available_margin`.

---

## 8. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Single source of truth** | No other engine computes or stores equity or margin values. |
| **Atomic balance updates** | Equity changes are applied atomically; partial updates are impossible. |
| **Immutable history** | Equity snapshots are append-only and never mutated. |

---

## 9. Cross-References

- [PME Overview](../portfolio-management-engine/03-04-01-pme-overview-spec.md) — Engine boundaries and ledger model.
- [PME Layer 1 — Position](03-04-02-pme-layer1-position.md) — Upstream valuation source.
- [PME Layer 2 — Exposure](03-04-03-pme-layer2-exposure.md) — Leverage and exposure aggregation.
- [PME Layer 4 — Portfolio](03-04-05-pme-layer4-portfolio.md) — Veto and drawdown enforcement.
- [TAE Layer 2 — Execution](../trade-automation-engine/03-03-03-tae-layer2-execution.md) — Position Sizing Protocol consumer.
- [Database Schema](../../integration-and-api/06-02-database-schema-spec.md) — `paper_balances`, `portfolio_equity_history`.
