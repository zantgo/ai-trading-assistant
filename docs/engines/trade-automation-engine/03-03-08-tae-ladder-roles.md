# TAE Ladder Roles — TF-Role Separation for Short-TF Execution

**Version:** 11.0 (2026-08-26) — execution model v11: TF-role separation.
**Status:** Implemented.
**Engine:** Trade Automation Engine (TAE) + Market Monitoring Engine (MME) synthesis.
**Depends on:** [03-02-01 MME Overview](../market-monitoring-engine/03-02-01-mme-overview-spec.md), [03-03-03 Execution](03-03-03-tae-layer2-execution.md), [01-04 Timeframe Model](../../conceptual-foundations/01-04-timeframe-model.md).

---

## 1. Problem

Running a swing-calibrated default strategy (trend≥75, stance Constructive) on a `1m/3m/5m/15m` ladder produces near-zero trades: the micro-TF (1m) reads pullback noise inside a macro bull trend as `bias Neutral`, `volatility` as danger, and `market_stance Cautious` — the L4/L6 gate chain vetoes everything while the 15m macro sees a clean trend (`EMA50>200` 81-100% bullish in the verified 7-day window).

## 2. Solution — Roles

One strategy, four TF slots, four roles. Roles map to TF slots; when `micro < 3600` (sub-hour) the roles diverge, otherwise they collapse to the legacy behavior (all roles = representative/micro).

| Role | Default Slot (`micro < 1h`) | What it feeds |
|------|-----------------------------|---------------|
| `decision_tf` | `macro` | L3 `bias`/`regime`/`market_quality`, L5 `overall_risk`/`market_stance`, `confidence_assessment` |
| `entry_tf` | `micro` | L4 opportunity zones + entry timing |
| `stop_tf` | `macro` | SL distance floor (`L6 stop_loss_distance_pct` or macro ATR) |
| `target_tf` | `micro` | TP zone |

Legacy (`micro ≥ 1h` or `ladder_roles.enabled = false`): all roles = micro (old behavior, no code fork).

## 3. Config

```toml
[workspace.strategies.default.ladder_roles]
enabled = true          # explicit — default OFF preserves legacy behavior
decision_tf = "macro"
entry_tf = "micro"
stop_tf = "macro"
target_tf = "micro"
```

Schema-driven: `StrategyForm` renders the four enum selects; validation enforces the slot names (`micro`/`fast`/`slow`/`macro`). Recommended for sub-hour ladders (`micro < 3600`); leave OFF for swing ladders (legacy first-TF-wins representative set).

## 4. Stop Floor & TP Reachability (quantity-first)

**Stop floor — floor, don't refuse.** `SetupPlan.effective()` computes `SL = max(zone invalidation, stop_floor)` where `stop_floor` is:

- `l6_formula` (default): `advisory.stop_loss_distance_pct` from the `stop_tf` snapshot (L6: `base_mult×2% + vol/100×10`, clamp [0.5,15]) — **data-proven**: ZEC T1 replay `0.72%` zone SL → floored to `2%` → TP hit (was a loss).
- `atr_mult`: `k × ATR(stop_tf)` (strategy `tae.risk.min_sl_atr`).

The old `min_sl_atr` **refuse** (`entry_blocked` when `distance < k×ATR`) is replaced by flooring.

**TP cap:** `TP = entry ± min(net_rr, max_tp_rr) × SL_distance` where `max_tp_rr` defaults `1.5`. Prevents the unreachable +2.4% micro-targets that never fill on 1m (observed T2 MFE +0.62% vs TP +2.43%).

Both are wired in `SetupPlan::effective()`; `arm_bracket` arms the bracket at the floored values. Fees/slippage/funding and `ExecutionBackend` stay mode-neutral.

## 5. Wiring & Parity

- `OpportunityParams`/`DecisionParams`/`RiskParams` carry `LadderRoles`.
- Live: `market-analyzer/src/analyzer/mod.rs:3671`, replay: `backtesting-engine/src/historical.rs:459` — both call `synthesize_cross_tf` with the same role-selected snapshots. **One producer, three sinks** (live/paper/backtest identical).
- TAE: `extract_top_setup` picks zones per `entry_tf`; `tick_idle` floors + caps per `stop_tf`/`target_tf`. `strategy_gates` use `decision_tf` bias/breadth.

## 6. Defaults

| Dial | Old → **New** (becomes the default) |
|------|--------------------------------------|
| `min_net_rr` | 1.0 → **0.5** |
| `max_tp_rr` | — → **1.5** |
| `readiness_ready_min` | 60 → **20** |
| `stance: constructive` | 30 → **55** (neutral 40→50, cautious 60→75, avoid 80→90, aggressive 20→45) |
| `entry_vol_no_entry` | 60 → **80**, breakout 20 → **40** |
| `exec-liquidity baseline` | 30 → **10**, `rvol_high` 2.0→**5.0** |
| `l4 quality_bands` | [85,70,50,30] → **[75,60,45,25]** |

These are now `StrategyConfig::default()` in `crates/config-models/src/strategy.rs` and mirrored in `core-domain`.

## 7. Verification

- Backtest `7d × BTC/ETH/SOL @ 1m/3m/5m/15m` with new defaults: **≥15 trades**, T1-style entries survive to TP, `symbol:""` bug fixed.
- Paper 1h smoke at `1m/3m/5m/15m` with `--tae-on`: fills now.
- New CLI `--backtest-gates <id>` table: `READY%`, stance histogram, stop-floor/tp-cap counts.
- Golden vectors unaffected (they don't use `StrategyConfig::default()` for this path).
