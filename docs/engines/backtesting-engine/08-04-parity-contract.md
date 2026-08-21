# BTE Parity Contract — why backtest = paper

**Version:** 8.0 (2026-08-20)
**Engine:** Backtesting Engine
**Code:** `crates/portfolio-supervisor/src/execution/session_tick.rs`

## 1. The mechanism: one session body

Every tick of the TAE loop — live session **and** backtest — runs the
**same function** through the **same code**:

```rust
portfolio_supervisor::execution::session_tick::run_tick(
    engine, executor, instance_id, symbol, snaps, mid, ctx,
    live_fills,           // Some(fills) only for live-mode engines
    capture_last_close,   // true only for the backtest trade log
)
```

| Session | tick source | dispatch | live_fills |
|---------|-------------|----------|------------|
| Observe (daemon) | live WS snapshots | false | None |
| Paper (daemon) | live WS snapshots | true | None (paper evaluates) |
| Live (daemon) | live WS snapshots | true | Some(broker fills) |
| Backtest recorded | `market_snapshots` replay | true | None |
| Backtest historical | archived candles → full MME pipeline | true | None |

Fills, sizing, safety ladder and fees are the same functions on the same
config structs (`FeesConfig`, `MinimalTaeConfig`, leverage) — there is no
per-mode config fork. A backtest result therefore equals the paper result
**by construction**, not by convention.

## 2. What this guarantees

- Identical setup extraction (`extract_top_setup`), identical invalidation
  semantics, identical risk sizing.
- Identical `PaperSimulation` fill math (deterministic in mid).
- Identical NHST treatment (seeded Monte Carlo — deterministic).

## 3. What live adds (documented honestly)

Live mode runs the same logic against a real venue, so it differs in ways
a backtest cannot model:

- venue fills, partial fills, and latency;
- real funding timestamps and venue fees;
- connection gaps and reconstruction.

Paper↔backtest parity is exact; live is "same logic, real venue".

## 4. Enforced in tests

- `session_tick` unit tests: deterministic, dispatch-gated ticks.
- Recorded + historical runners: deterministic reruns (byte-identical
  trades/equity), burn-in respected.
- The daemon loop body is the same `run_tick` call — the existing
  failover/liquidation/engine suites are the regression net for the
  refactor.

## 5. Cross-references

- [BTE overview](08-01-bte-overview.md)
- [Historical runner](08-03-historical-runner.md)
- [TAE overview](../trade-automation-engine/03-03-01-tae-overview-spec.md)
