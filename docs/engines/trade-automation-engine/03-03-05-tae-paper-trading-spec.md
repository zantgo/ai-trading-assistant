# TAE — Paper Trading Specification

**Version:** 5.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Approved
**Engine:** Trade Automation Engine (TAE)
**Purpose:** This document specifies the internal paper trading engine — a simulated matching engine that mirrors live exchange order lifecycles for strategy development, backtesting, and zero-risk validation without external API dependencies.

---

## 1. Purpose

The Paper Trading Engine is a **simulated execution venue** embedded within the TAE. It intercepts orders routed by the [Execution Layer](03-03-03-tae-layer2-execution.md) and processes them against real-time market data (prices from the DIE) to produce synthetic fills, rejections, and cancellations — all logged to the same Execution Matrix as live trades.

---

## 2. Operational Principle

```
[Execution Layer] ──► [Paper Trading Engine] ──► [Execution Matrix] ──► [PME]
                          │
                          └── consumes live mid-price from DIE
```

The engine applies the same Position Sizing Protocol, order state machine, and audit logging. The difference is the fill engine:

| Aspect | Live Exchange | Paper Engine |
|--------|--------------|--------------|
| Fill price source | Exchange order book | DIE mid-price + spread simulation |
| Slippage model | Actual exchange slippage | Configurable simulated slippage |
| Rejection sources | Exchange API errors | Pre-flight validation only |
| Latency | Real network latency | Near-zero (local) |

---

## 3. Fill Simulation

### 3.1 Market Orders
Filled immediately at the current DIE mid-price, adjusted by the simulated spread:
- `fill_price = mid_price + (spread / 2)` for buys
- `fill_price = mid_price - (spread / 2)` for sells

### 3.2 Limit Orders
Filled when the DIE mid-price crosses the limit price:
- Buy limit: filled when `mid_price ≤ limit_price`
- Sell limit: filled when `mid_price ≥ limit_price`

### 3.3 Stop Orders
Triggered when the DIE mid-price crosses the stop level:
- Stop (sell): triggers when `mid_price ≤ stop_price`
- Stop (buy): triggers when `mid_price ≥ stop_price`
Once triggered, the order becomes a market order and fills per §3.1.

---

## 4. Simulated Costs

| Cost | Default | Config Source |
|------|---------|---------------|
| Maker fee | 0.02% | `config.toml` `fees.maker_fee_pct` |
| Taker fee | 0.06% | `config.toml` `fees.taker_fee_pct` |
| Funding rate (8h) | 0.01% | `config.toml` `fees.funding_rate_8h` |
| Simulated spread | 0.01% | `config.toml` `fees` or per-instance |

Fees are deducted from the realized PnL on each fill.

---

## 5. Persistence

Paper trades are written to the same database tables as live trades:

| Table | Content |
|-------|---------|
| `paper_trades` | Closed paper-trade records with PnL. |
| `trade_telemetry_history` | Automated trade telemetry (entry/exit, fees, PnL, ROI). |
| `paper_balances` | Per-symbol capital tracking. |
| `portfolio_equity_history` | Equity time-series for drawdown/Sharpe computation. |

See [Database Schema](../../integration-and-api/06-02-database-schema-spec.md).

---

## 6. Replay & Reproducibility

The paper engine supports **deterministic replay**: feeding a historical sequence of market data through the same policy set and sizing protocol reproduces identical trades. This is essential for:
- Backtesting strategy modifications
- Validating policy changes against historical data
- Auditing execution logic without live market risk

---

## 7. Cross-References

- [TAE Overview](03-03-01-tae-overview-spec.md) — Operational modes and boundaries.
- [TAE Layer 2 — Execution](03-03-03-tae-layer2-execution.md) — Order construction and sizing protocol.
- [TAE Layer 1 — Policy](03-03-02-tae-layer1-policy.md) — Trigger source.
- [Database Schema](../../integration-and-api/06-02-database-schema-spec.md) — Persistent state.
- [Systemic Data Flow — Sequence B](../../conceptual-foundations/01-03-systemic-data-flow.md) — Entry loop.
