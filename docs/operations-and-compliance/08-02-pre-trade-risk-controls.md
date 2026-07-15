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
| 1 | **Symbol stance** | The current `Stance` for the symbol must be `ACTIVE`, or the order must be a `reduce_only = true` exit under `CLOSE_ONLY`. `AVOID` blocks all dispatches except those tagged `emergency_liquidation = true` (Hard Exit path, see Gate 7 below and [PME Layer 4 §4.2](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)). | Set manually by operator or automatically by the [PME Veto](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md#4-ontological-priority-veto). |
| 2 | **Decision guard (trade readiness)** | The Decision Matrix's `trade_readiness` field must be `READY` or `FORMING`. `WATCH` is a soft warning; `STAND_ASIDE` blocks the dispatch. | Computed by the MME Decision Layer from `directional_guidance × confidence_assessment × market_stance`. See [Decision Matrix §4](../matrices/02-04-decision-matrix.md). |
| 3 | **Capital query — available margin** | The TAE issues a synchronous request to the PME Capital Matrix for `available_margin`. The query returns 0 if the order would push `margin_usage_ratio` ≥ 0.95. | Live, computed from [PME Layer 3](../engines/portfolio-management-engine/03-04-04-pme-layer3-capital.md). |
| 4 | **Position sizing** | The Position Sizing Protocol computes $S = E \times R / (D_{sl} / 100)$. If the result exceeds `risk_parameters.max_position_size_usd`, sizing is clipped. If `risk_parameters.max_leverage` is exceeded, the order is rejected. **Bypass:** orders with `reduce_only = true` skip Gate 4 (sizing) — size is copied verbatim from the Position Matrix. | Per-policy in `config.json` `execution_policies.*`. |
| 5 | **Slippage ceiling** | The Execution Layer queries the live order book and computes estimated slippage. If estimated slippage ≥ the configured ceiling (default 0.5 % of position size), the order is held for manual review. | `config.json` `execution.slippage_ceiling_pct`. |
| 6 | **Exposure concentration** | The Exposure Layer rejects new positions that would breach the single-pair concentration limit (default 0.20), the portfolio exposure limit (default 0.50), or the correlation limit (default 0.8). **Bypass:** orders with `reduce_only = true` skip Gate 6 (concentration) — exit orders must be permitted even if the portfolio is overconcentrated. | [PME Layer 2 §3](../engines/portfolio-management-engine/03-04-03-pme-layer2-exposure.md). |
| 7 | **PME safety veto** | Even if all previous gates pass, the PME Portfolio Layer can force a stance change to `AVOID` or `CLOSE_ONLY` when systemic thresholds are breached (drawdown ≥ `drawdown_limit_pct`, margin ceiling, loss streak ≥ dropout_threshold, or MME `systemic_risk_score ≥ systemic_risk_threshold`). **Bypass:** orders tagged `emergency_liquidation = true` (Hard Exit path from `AVOID` triggers) bypass Gate 7 so the liquidation is dispatched even when the stance is `AVOID`. | [PME Layer 4 §4](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md). |

---

## 3. Evaluation Order & Short-Circuiting

The gates run in the order listed in §2, first-match-wins short-circuit. The first failure aborts the dispatch and emits a rejection log entry:

```
Gate 1 stance → if (stance == AVOID AND not emergency_liquidation) → block
              → if (stance == CLOSE_ONLY AND not reduce_only) → block
Gate 2 readiness → if STAND_ASIDE → block (operator override required)
Gate 3 capital → if 0 → block (insufficient margin)
Gate 4 sizing → if max_position_size_usd exceeded → clip, continue
              → if max_leverage exceeded → block
              → reduce_only orders skip Gate 4 entirely
Gate 5 slippage → if over ceiling → hold for manual review
Gate 6 concentration → if breach → block (reduce position size or close existing)
                     → reduce_only orders skip Gate 6 entirely
Gate 7 PME veto → if (stance == AVOID AND not emergency_liquidation) → block
```

Hard-stops (block) vs hold-for-review (suspend):

| Disposition | Gates |
|---|---|
| **Hard-stop** (no order ever reaches exchange) | Gate 1 (AVOID without emergency_liquidation, or CLOSE_ONLY without reduce_only), Gate 2 (`STAND_ASIDE`), Gate 3, Gate 4 (max leverage), Gate 6 (for non-reduce_only orders), Gate 7 (AVOID without emergency_liquidation) |
| **Hold-for-review** (logged, suspended, operator must manually approve) | Gate 5 (slippage ceiling), Gate 4 (size cap — clipped, not blocked) |

### 3.1 Exit / Reduce-Only Bypass Summary

| Order characteristic | Gates bypassed | Gates still applied |
|---|---|---|
| `reduce_only = true` | Gate 4 (sizing), Gate 6 (concentration) | Gate 1 (stance — must be ACTIVE or CLOSE_ONLY), Gate 2 (readiness), Gate 3 (capital), Gate 5 (slippage), Gate 7 (PME veto — bypassed only with `emergency_liquidation = true`) |
| `emergency_liquidation = true` (Hard Exit) | Gate 1 (stance — even AVOID), Gate 2, Gate 4 (sizing), Gate 5 (slippage), Gate 6, Gate 7 | Gate 3 (capital — check margin available for the closing side) |
| Neither flag | — | All gates apply |

### 3.2 Order State Pre-Dispatch (`PRE_DISPATCH`)

Orders held by Gate 5 (slippage ceiling) or pending manual review sit in the **`PRE_DISPATCH`** state with status `HELD_FOR_REVIEW` (not in `OPEN` — the `OPEN` state is only valid after exchange acknowledgement). The order can be cancelled via `DELETE /api/instances/by-pair/:pair_key` or manually executed via `POST /api/instances/:id/manual/open`. `PRE_DISPATCH` orders do not consume committed margin and do not appear in `open_orders`.

---

## 4. Parameters & Configuration

| Parameter | Source | Default | Override Path |
|-----------|--------|---------|---------------|
| `risk_per_trade_pct` | per-policy `risk` block | 1.0 | Edit `config.json` `execution_policies.*.risk.risk_per_trade_pct` |
| `max_position_size_usd` | per-policy | unlimited | Set in policy |
| `max_leverage` | per-policy | 20 | Set in policy |
| `execution.slippage_ceiling_pct` | global | 0.5 | Set in `config.json` |
| `leverage.cross_leverage` | global | 20 | Set in `config.json` |
| `safety.drawdown_limit_pct` | global | 0.30 (= 30 %) | Set in `config.json` `safety.*` |
| `safety.max_daily_drawdown_pct` | global | 0.05 (= 5 %) | Set in `config.json` `safety.*` (drives the `WARN` state — see [03-04-05 §3](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)) |
| `safety.caution_threshold` | global | 3 losses | Set in `config.json` `safety.*` |
| `safety.dropout_threshold` | global | 5 losses | Set in `config.json` `safety.*` |
| `safety.systemic_risk_threshold` | global | **0.80** (`systemic_risk_score ≥ 0.80` triggers the systemic-risk veto branch of Gate 7) | Set in `config.json` `safety.systemic_risk_threshold` |
| `risk_profiles.*.max_risk_pct` | per risk-profile | 2 (= 2 %) | Edit via `POST /api/risk-profiles` |
| `risk_profiles.*.leverage` | per risk-profile | 20 | Edit via `POST /api/risk-profiles` |
| `fees.maker_fee_pct` | global | 0.02 (= 0.02 %) | Set in `config.json` `fees.*` |
| `fees.taker_fee_pct` | global | 0.06 (= 0.06 %) | Set in `config.json` `fees.*` |
| `fees.funding_rate_8h` | global | 0.01 (= 0.01 %) | Set in `config.json` `fees.*` (8-hour funding rate; see [03-03-05 §4](../engines/trade-automation-engine/03-03-05-tae-paper-trading-spec.md)) |

---

## 5. Rejections & Operator Overrides

Every gate failure produces:
1. A rejection log entry in the Execution Matrix with the gate ID and reason.
2. An observability event surfaced via `GET /api/system/observability?symbol=…`.
3. No exchange order placement.

**Operator override paths:**

- **Slippage ceiling (Gate 5).** The order sits in `PRE_DISPATCH` with status `HELD_FOR_REVIEW` (not in `OPEN` — see §3.2). The operator can either: (a) wait for better liquidity, (b) cancel via `DELETE /api/instances/by-pair/:pair_key`, or (c) manually execute via `POST /api/instances/:id/manual/open`.
- **Concentration breach (Gate 6).** To override, close an existing position in the affected sector first, then re-trigger.
- **PME veto (Gate 7).** Veto is sticky. Per [PME Layer 4 §4.3](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md#43-veto-release), the operator must clear the underlying condition (e.g. equity must recover above `drawdown_limit_pct`) **and** call the dedicated veto-release endpoint **`POST /api/instances/:id/safety/release-veto`** (Issue 4.O). The endpoint returns `400` if the veto condition is still active. Resetting the consecutive-loss counter alone (the older `/safety/reset` endpoint) is **insufficient** to release a drawdown- or systemic-risk-based veto.
- **Stance `AVOID` (Gate 1, after Hard Exit has completed).** Once the Hard Exit fires and the stance transitions to `AVOID`, the operator must use `POST /api/instances/:id/safety/release-veto` (not `/safety/reset`) to restore the default stance. The `/safety/reset` endpoint only clears the per-symbol `consecutive_losses` counter; it does **not** release a drawdown- or systemic-risk-based veto.

All overrides are logged with operator ID, timestamp, and prior state for audit.

---

## 6. Cross-References

- [TAE Layer 2 — Execution](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md) — Position Sizing Protocol & order dispatch.
- [PME Layer 3 — Capital](../engines/portfolio-management-engine/03-04-04-pme-layer3-capital.md) — Available margin source.
- [PME Layer 4 — Portfolio](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — Veto authority.
- [TAE Execution Policy Spec](../engines/trade-automation-engine/03-03-04-tae-execution-policy-spec.md) — Per-policy risk parameters.
- [User Manual](08-01-user-manual.md) — Operational context.
