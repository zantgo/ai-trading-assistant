# PAE Layer 5 — Backtest

**Version:** 7.0 (2026-08-18) — delivered with the v7 PAE release.
**Status:** Specified — implemented.
**Engine:** Performance Analytics Engine (PAE)
**Layer:** 5 of 5
**Input Contract:** Recorded completed `market_snapshots` (with decision matrices), `[workspace.minimal_tae]` executor config, fee/leverage config
**Output Contract:** `BacktestResult` (params, summary, NHST stats, trades, equity curve) persisted to `backtest_runs`
**Purpose:** This document specifies the backtest layer — a deterministic replay of recorded MME decisions through the unchanged TAE setup executor and unified paper engine, with the full statistical treatment (t-test, Monte Carlo, α = 0.05, edge classification) applied to the simulated trades.

---

## 1. Design principle: record today, replay tomorrow

Every completed candle snapshot already embeds the **full MTF-synthesized decision** (opportunity profiles with entry/SL/TP zones, decision context, analysis bias, advisory). Since the v7 migration, these matrices are **persisted** to `market_snapshots` (`opportunity_json`, `decision_context_json`, `analysis_json`, `advisory_json`, `market_regime`) by the WAL snapshot logger.

The backtest therefore replays **exactly what the MME recommended at the time**, through **exactly the same code the live executor runs**:

```
recorded snapshots (ascending ts) ──► [ExecutionEngine (fresh, seeded capital)]
                                          │  mark_to_market(mid)
                                          │  evaluate_order_fills(mid)
                                          ▼
                                    [SetupExecutor.tick(snapshot, mid)]
                                          │  (unchanged live logic)
                                          ▼
                                    simulated closes ──► stats + NHST + equity curve
```

**Why this is honest:** the executor consumes the same `MarketSnapshot` wire format live and in backtest; the only difference is the source (DB replay vs. live buffers) and the destination (statistics vs. persisted telemetry). No decision logic is re-implemented.

## 2. Replay contract

- **Source:** `market_snapshots` rows for `(symbol, timeframe_secs)` with `timestamp ∈ [from_ms, to_ms]`, `is_completed = 1`, ordered ascending. Single timeframe per run — each recorded snapshot already carries the MTF decision.
- **Bound:** the window is capped (≤ 50,000 snapshots per run) to keep runs synchronous and fast.
- **Engine:** a fresh `ExecutionEngine` with the configured fee/slippage/leverage config, seeded with `initial_capital`. Paper fills only — the same `PaperSimulation` backend as live paper trading.
- **Executor:** the same `SetupExecutor` (same `extract_top_setup`, same invalidation, same gates) driven once per snapshot with the snapshot's `mid_price`; the daemon-equivalent loop body is `mark_to_market → evaluate_order_fills → tick`.
- **Determinism:** identical inputs ⇒ identical results (no wall clock, no randomness in the executor path).
- **Isolation:** backtest runs never write trade telemetry, activity logs, or open state; they only write the `backtest_runs` result row.

## 3. Result shape (`BacktestResult`)

| Block | Fields |
|-------|--------|
| `params` | symbol, timeframe_secs, from_ms, to_ms, initial_capital |
| `summary` | total trades, win count, loss count, win rate, gross profit, gross loss, profit factor, expectancy, max drawdown %, avg R:R |
| `stats` (NHST — same machinery as L2) | `t_statistic`, `p_value`, `p_mc`, `monte_carlo_runs` (10,000), `alpha` (0.05), `is_significant`, `classification` (edge verdict; `InsufficientData` under 30 simulated trades) |
| `trades` | simulated closes: timestamp, side, entry, exit, size, PnL, fees, exit reason (tp/sl/invalidated_signal/manual/stop_flatten) |
| `equity_curve` | points `{ ts, equity }` sampled per close (and per fill for open positions) |

## 4. API

| Endpoint | Behavior |
|----------|----------|
| `POST /api/backtest/run` | Body `{ symbol, timeframe_secs, from_ms, to_ms, initial_capital }`. Runs synchronously, persists the `BacktestResult` to `backtest_runs`, returns it with `backtest_id`. |
| `GET /api/backtest/:id` | Returns the persisted run (or 404). |

## 5. Trader-facing interpretation

The backtest answers two questions at once:

1. **"Would the setup executor have been profitable over this window?"** — win rate, profit factor, expectancy, max drawdown, equity curve, trade-by-trade log with exit reasons.
2. **"Is that result real or luck?"** — the NHST verdict: if `is_significant` (both p-values < α = 0.05 over ≥ 30 simulated trades), the edge is statistically distinguishable from random; otherwise the result may be noise. The dashboard renders this as an **edge verdict card** (e.g. "Significant at α = 0.05 — t-test p = 0.003, Monte Carlo p = 0.001, 10,000 runs").

## 6. Cross-References

- [PAE Overview](03-05-01-pae-overview-spec.md) — layer map + statistical contract.
- [PAE Layer 2 — Strategy Analytics](03-05-03-pae-layer2-strategy-analytics.md) — the shared NHST machinery.
- [TAE Overview](../trade-automation-engine/03-03-01-tae-overview-spec.md) — the replayed executor + engine.
- [Database Schema](../../integration-and-api/06-02-database-schema-spec.md) — `market_snapshots` matrix columns, `backtest_runs`.
