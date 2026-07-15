# Pre-Trade Risk Controls

**Version:** 1.0
**Status:** Approved
**Category:** Operations & Compliance

---

## 1. Purpose

Every order dispatched by the Trade Automation Engine (TAE) must pass a deterministic sequence of pre-trade gates between the Policy Layer's trigger and the Execution Layer's `ExchangeAdapter.start()` call. This document enumerates those gates, the order in which they run, the configuration sources for their thresholds, and the operator override path when a gate blocks a trade.

The gates are designed to fail closed — any failure short-circuits the dispatch and emits a logged rejection. No order reaches the exchange without passing every gate.

---

## 2. Control Catalogue

The following gates run between a Policy trigger (L1) and the Exchange dispatch (L2). Each gate references its authoritative specification.

| # | Gate | Source | Configuration |
|---|------|--------|---------------|
| 1 | **Symbol stance** | The current `Stance` for the symbol must be `ACTIVE`. `CLOSE_ONLY` permits only `reduce_only = true` orders; `AVOID` blocks all dispatches. | Set manually by operator or automatically by the [PME Veto](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md#4-ontological-priority-veto). |
| 2 | **Decision guard (trade readiness)** | The Decision Matrix's `trade_readiness` field must be `READY` or `FORMING`. `WATCH` is a soft warning; `STAND_ASIDE` blocks the dispatch. | Computed by the MME Decision Layer from `directional_guidance × confidence_assessment × market_stance`. See [Decision Matrix §4](../matrices/02-04-decision-matrix.md). |
| 3 | **Capital query — available margin** | The TAE issues a synchronous request to the PME Capital Matrix for `available_margin`. The query returns 0 if the order would push `margin_usage_ratio` ≥ 95 %. | Live, computed from [PME Layer 3](../engines/portfolio-management-engine/03-04-04-pme-layer3-capital.md). |
| 4 | **Position sizing** | The Position Sizing Protocol computes $S = E \times R / (D_{sl} / 100)$. If the result exceeds `risk_parameters.max_position_size_usd`, sizing is clipped. If `risk_parameters.max_leverage` is exceeded, the order is rejected. | Per-policy in `config.json` `execution_policies.*`. |
| 5 | **Slippage ceiling** | The Execution Layer queries the live order book and computes estimated slippage. If estimated slippage ≥ the configured ceiling (default 0.5 % of position size), the order is held for manual review. | `config.json` `execution.slippage_ceiling_pct`. |
| 6 | **Exposure concentration** | The Exposure Layer rejects new positions that would breach the single-pair concentration limit (default 20 %), the portfolio exposure limit (default 50 %), or the correlation limit (default 0.8). | [PME Layer 2 §3](../engines/portfolio-management-engine/03-04-03-pme-layer2-exposure.md). |
| 7 | **PME safety veto** | Even if all previous gates pass, the PME Portfolio Layer can force a stance change to `AVOID` or `CLOSE_ONLY` when systemic thresholds are breached (drawdown ≥ `drawdown_limit_pct`, margin ceiling, or MME `systemic_risk_score`). | [PME Layer 4 §4](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md). |

---

## 3. Evaluation Order & Short-Circuiting

The gates run in the order listed in §2, first-match-wins short-circuit. The first failure aborts the dispatch and emits a rejection log entry:

```
Gate 1 stance → if not ACTIVE → block (STAND_ASIDE override required)
Gate 2 readiness → if STAND_ASIDE → block (operator override required)
Gate 3 capital → if 0 → block (insufficient margin)
Gate 4 sizing → if max_position_size_usd exceeded → clip, continue
             → if max_leverage exceeded → block
Gate 5 slippage → if over ceiling → hold for manual review
Gate 6 concentration → if breach → block (reduce position size or close existing)
Gate 7 PME veto → if stance forced AVOID → block (only manual re-enable)
```

Hard-stops (block) vs hold-for-review (suspend):

| Disposition | Gates |
|---|---|
| **Hard-stop** (no order ever reaches exchange) | Gate 1, Gate 2 (`STAND_ASIDE`), Gate 3, Gate 4 (max leverage), Gate 6, Gate 7 |
| **Hold-for-review** (logged, suspended, operator must manually approve) | Gate 5 (slippage ceiling), Gate 4 (size cap — clipped, not blocked) |

---

## 4. Parameters & Configuration

| Parameter | Source | Default | Override Path |
|-----------|--------|---------|---------------|
| `risk_per_trade_pct` | per-policy `risk` block | 1.0 | Edit `config.json` `execution_policies.*.risk.risk_per_trade_pct` |
| `max_position_size_usd` | per-policy | unlimited | Set in policy |
| `max_leverage` | per-policy | 20 | Set in policy |
| `execution.slippage_ceiling_pct` | global | 0.5 | Set in `config.json` |
| `leverage.cross_leverage` | global | 20 | Set in `config.json` |
| `safety.drawdown_limit_pct` | global | 30 | Set in `config.json` `safety.*` |
| `safety.caution_threshold` | global | 3 losses | Set in `config.json` `safety.*` |
| `safety.dropout_threshold` | global | 5 losses | Set in `config.json` `safety.*` |
| `risk_profiles.*.max_risk_pct` | per risk-profile | 2 | Edit via `POST /api/risk-profiles` |
| `risk_profiles.*.leverage` | per risk-profile | 20 | Edit via `POST /api/risk-profiles` |

---

## 5. Rejections & Operator Overrides

Every gate failure produces:
1. A rejection log entry in the Execution Matrix with the gate ID and reason.
2. An observability event surfaced via `GET /api/system/observability?symbol=…`.
3. No exchange order placement.

**Operator override paths:**

- **Slippage ceiling (Gate 5).** The order sits in `OPEN` with status `HELD_FOR_REVIEW`. The operator can either: (a) wait for better liquidity, (b) cancel via `DELETE /api/instances/by-pair/:pair_key`, or (c) manually execute via `POST /api/instances/:id/manual/open`.
- **Concentration breach (Gate 6).** To override, close an existing position in the affected sector first, then re-trigger.
- **PME veto (Gate 7).** Veto is sticky. Per [PME Layer 4 §4.3](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md#43-veto-release), the operator must clear the underlying condition (e.g. equity must recover above `drawdown_limit_pct`) and then issue a one-time manual stance re-enable.
- **Stance `AVOID` (Gate 1).** The operator can manually re-enable via `POST /api/instances/:id/safety/reset`.

All overrides are logged with operator ID, timestamp, and prior state for audit.

---

## 6. Cross-References

- [TAE Layer 2 — Execution](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md) — Position Sizing Protocol & order dispatch.
- [PME Layer 3 — Capital](../engines/portfolio-management-engine/03-04-04-pme-layer3-capital.md) — Available margin source.
- [PME Layer 4 — Portfolio](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — Veto authority.
- [TAE Execution Policy Spec](../engines/trade-automation-engine/03-03-04-tae-execution-policy-spec.md) — Per-policy risk parameters.
- [User Manual](08-01-user-manual.md) — Operational context.
