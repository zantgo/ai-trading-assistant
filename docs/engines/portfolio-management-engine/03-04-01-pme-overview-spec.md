# Portfolio Management Engine — Overview Specification

**Version:** 6.10 (2026-08-13) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Specified — **WIP**; backend code is implemented (safety manager, position/exposure/capital/portfolio layers, registry, veto loop, all wired in `execution-daemon`), but the dedicated `PortfolioDashboard` is a hardcoded placeholder. Full wiring and dashboard wiring land in [`docs/ROADMAP.md`](../../ROADMAP.md) §3 Phase A + C.
**Engine:** Portfolio Management Engine (PME)
**Purpose:** This document specifies the boundaries, ledger model, margin/leverage restrictions, and safety architecture of the Portfolio Management Engine — the engine responsible for capital preservation, position tracking, exposure control, and the systemic safety veto.

---

## 1. Mission & Boundaries

The PME is the platform's **capital custodian and safety authority**. It tracks active positions, aggregates exposure, manages capital and margin, and enforces account-level safety through the Ontological Priority Veto. It performs **no market interpretation** and **no order construction** (it supplies capital data to the TAE and receives fills back).

```
[Fills & Fails] ──► PME ──► [Capital Matrix] ──► [TAE sizing]
                     │
                     └──(veto)──► [TAE stances]
```

### 1.1 Layer Structure

| Layer | Name | Output |
|-------|------|--------|
| L1 | [Position Layer](03-04-02-pme-layer1-position.md) | Position Matrix |
| L2 | [Exposure Layer](03-04-03-pme-layer2-exposure.md) | Exposure Matrix |
| L3 | [Capital Layer](03-04-04-pme-layer3-capital.md) | Capital Matrix |
| L4 | [Portfolio Layer](03-04-05-pme-layer4-portfolio.md) | Portfolio Matrix + Veto |

---

## 2. Ledger Model

The PME maintains the authoritative financial state:

| Concept | Storage |
|---------|---------|
| Active positions | `active_positions` (one per symbol). |
| Position scaling | `position_slots` (4-slot dynamic margin). |
| Capital config | `paper_balances` (initial/current/allocation/leverage). |
| Equity history | `portfolio_equity_history`, `position_equity_snapshots`. |
| Closed trades | `paper_trades`, `trade_telemetry_history`. |

Equity snapshots are logged periodically (60 s cadence, `portfolio_equity.rs`) with a 30-day retention purge.

---

## 3. Margin & Leverage Restrictions

| Restriction | Default | Source |
|-------------|---------|--------|
| Cross leverage | 20× | `config.toml` `leverage.cross_leverage` |
| Max single-pair exposure | 20% of capital | `PortfolioRiskState` |
| Max portfolio exposure | 50% of capital | `PortfolioRiskState` |
| Max correlation | 0.8 | `PortfolioRiskState` |
| `max_daily_drawdown_pct` (configuration limit; live metric is `daily_drawdown_pct = -daily_pnl / starting_session_equity`) | 5% (= 0.05) | `PortfolioRiskState` |
| `drawdown_limit_pct` (equity peak-to-trough; the **hard veto** threshold) | 30% (= 0.30) | `PortfolioRiskState` |

These two drawdown metrics are distinct and are not synonyms. See §4.1 below and [PME Layer 4](03-04-05-pme-layer4-portfolio.md) §3–§4 for which trigger activates which veto.

---

## 4. Safety Circuit Breakers

The `SafetyManager` (`crates/portfolio-supervisor/src/safety.rs`) tracks **five** escalating states:

```
NORMAL ──(daily_drawdown_pct ≥ max_daily_drawdown_pct)──► WARN
       ──(consecutive_losses[sym] ≥ caution_threshold)──► CAUTIOUS
       ──(consecutive_losses[sym] ≥ dropout_threshold)──► SUSPENDED (timed)
       ──(current_equity / peak_equity < 1 − drawdown_limit_pct)──► DRAWDOWN_STOP
```

Systemic-risk breach is enforced by the veto loop ([03-04-05 §4.1](03-04-05-pme-layer4-portfolio.md)) and pre-trade Gate 7 ([08-02](../../operations-and-compliance/08-02-pre-trade-risk-controls.md)), not by safety states.

Defaults (`config.toml` `safety`): WARN at 5 % daily drawdown (no stance change — pre-veto alert), caution at 3 consecutive losses, dropout at 5 (8 h suspension), capital drawdown stop at 30 % (`drawdown_limit_pct`). A win resets the consecutive-loss counter. See README "Key Conventions" for the distinction between `max_daily_drawdown_pct` (5 % early-warning, session PnL) and `drawdown_limit_pct` (30 % equity peak-to-trough, hard veto).

---

## 5. The Ontological Priority Veto

The PME holds **veto power** over the TAE: when a systemic threshold is breached (daily drawdown, aggregate margin, or the MME Systemic Risk Score), the Portfolio Layer forces affected symbol stances to `Avoid` / `Close Only` at the execution boundary. See [PME Layer 4](03-04-05-pme-layer4-portfolio.md).

---

## 6. Cross-References

- [PME Layer 1 — Position](03-04-02-pme-layer1-position.md)
- [PME Layer 2 — Exposure](03-04-03-pme-layer2-exposure.md)
- [PME Layer 3 — Capital](03-04-04-pme-layer3-capital.md)
- [PME Layer 4 — Portfolio](03-04-05-pme-layer4-portfolio.md)
- [Systemic Data Flow — Sequences C & D](../../conceptual-foundations/01-03-systemic-data-flow.md)
- [TAE Overview](../trade-automation-engine/03-03-01-tae-overview-spec.md)
