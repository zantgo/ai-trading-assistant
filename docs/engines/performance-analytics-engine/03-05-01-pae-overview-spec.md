# Performance Analytics Engine — Overview Specification

**Version:** 6.4 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Performance Analytics Engine (PAE)
**Purpose:** This document specifies the boundaries, performance database, scheduled tasks, and report templates of the Performance Analytics Engine — the engine that evaluates historical trading records to isolate strategy efficacy and identify system drag.

---

## 1. Mission & Boundaries

The PAE is the platform's **retrospective analyst**. It consumes closed-trade ledgers from the PME, reconstructs trades, computes statistics and significance tests, and maps strategy performance to market regimes. It is **read-only with respect to live trading** — it never influences active positions or market interpretation.

```
[Closed Trade Ledgers] ──► PAE ──► [Performance Matrix] ──► [GUI / optimization feedback]
```

### 1.1 Layer Structure

| Layer | Name | Output |
|-------|------|--------|
| L1 | [Trade Analytics](03-05-02-pae-layer1-trade-analytics.md) | Trade Analytics Matrix |
| L2 | [Strategy Analytics](03-05-03-pae-layer2-strategy-analytics.md) | Strategy Analytics Matrix (NHST) |
| L3 | [Risk Analytics](03-05-04-pae-layer3-risk-analytics.md) | Risk Analytics Matrix |
| L4 | [Performance](03-05-05-pae-layer4-performance.md) | Performance Matrix (regime compatibility) |

---

## 2. Performance Database

The PAE reads from the shared telemetry store (see [Database Schema](../../integration-and-api/06-02-database-schema-spec.md)):

| Table | Role |
|-------|------|
| `paper_trades` | Closed paper-trade records. |
| `trade_telemetry_history` | Automated trade telemetry (entry/exit, fees, PnL, ROI). |
| `trade_learning_journal` | Human-annotated trade journal. |
| `portfolio_equity_history` | Equity time-series for drawdown/Sharpe. |
| `market_snapshots` | Regime context at trade time. |
| `performance_matrix_snapshots` | **Written by PAE** — Performance Matrix snapshots at scheduled cadence (default 300 s). |
| `strategy_analytics_history` | **Written by PAE** — Statistical-significance history per execution policy. |

---

## 3. Scheduled Tasks

| Task | Cadence | Module |
|------|---------|--------|
| Dashboard stat compilation | On demand (`/api/dashboard/stats`) | `stats_compiler.rs` |
| Performance evaluation | 300 s | `performance_evaluator.rs` |
| Strategy optimization | 3600 s | `strategy_optimizer.rs` |

---

## 4. Report Templates

The PAE produces:

- **DashboardStats** — 20+ metric categories (equity curve, win rates, calendar, streaks, direction breakdown, trader style, commissions, monthly summaries).
- **OptimizationReport** — per-regime performance + recommendations.
- **Trade journal exports** — CSV/JSON downloads of the annotated ledger.

---

## 5. Dual-Mode Analytics

Per [Global Architecture §4.4](../../conceptual-foundations/01-02-global-architecture.md), the PAE enables **retroactive analysis** of headless CLI runs: trades persisted during cloud execution are re-analyzed when the operator later boots the GUI, running full trade reconstruction, significance tests, and regime mapping against the historical record.

---

## 6. Cross-References

- [PAE Layer 1 — Trade Analytics](03-05-02-pae-layer1-trade-analytics.md)
- [PAE Layer 2 — Strategy Analytics](03-05-03-pae-layer2-strategy-analytics.md)
- [PAE Layer 3 — Risk Analytics](03-05-04-pae-layer3-risk-analytics.md)
- [PAE Layer 4 — Performance](03-05-05-pae-layer4-performance.md)
- [Systemic Data Flow — Sequence E](../../conceptual-foundations/01-03-systemic-data-flow.md) — Analytics loop.
- [UI Dashboard Layout](../../ui-ux/07-02-ui-dashboard-layout.md) — Report rendering.
