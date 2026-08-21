# Backtesting Engine — Overview

**Version:** 8.0 (2026-08-20)
**Status:** Implemented
**Engine:** Backtesting Engine (BTE) — the sixth logical engine
**Crate:** `crates/backtesting-engine`
**Docs:** this directory

## 1. Position in the architecture

The BTE is the research engine of the platform. It simulates the **entire
trading stack** — DIE data, MME analysis, TAE execution, PME capital, PAE
statistics — over historical data, using the **same code paths** the live
session runs. It exists so an operator can validate a configuration and
know that a paper session run with the same config will behave the same.

| Session | Engines on the left panel |
|---------|---------------------------|
| Observe | Data Infrastructure · Market Monitor · **Backtesting** · Profile |
| Paper   | Data Infrastructure · Market Monitor · Trade Automation · Portfolio Management · Performance Analytics · Profile |
| Live    | Data Infrastructure · Market Monitor · Trade Automation · Portfolio Management · Performance Analytics · Profile |

The BTE is **observe-only in the UI** (research happens before capital is
deployed); its backend endpoints work for any running instance.

## 2. Instance binding

A backtest binds to **one running instance** — the instance the operator
selected in the right-side Instances panel or the Market Monitor Workspace
tab. The instance provides, read-only:

- the exchange (Hyperliquid / Bitget) and the raw symbol,
- the base/quote currency pair,
- the TF ladder (micro / fast / slow / macro durations),
- every config the run consumes (`[workspace.*]`, fees, leverage, risk).

Rules:

- **No running instance → no backtest.** The BTE navbar collapses to
  Overview + History + Settings and shows the shared no-instance state.
- **One backtest at a time** (global run lock → 409 on concurrent runs).
- **One backfill per instance at a time** (409 while running).

## 3. The two replay modes

| Mode | Source | Pipeline |
|------|--------|----------|
| `recorded` | Completed `market_snapshots` (recorded MME decisions, ≤ 7-day retention) | Replay through the unchanged setup executor |
| `historical` | `candle_archive` OHLCV (live-warm + backfilled, up to 365 days) | Full MME pipeline over archived candles → MTF synthesis → executor |

Both modes feed the **same** `run_tick` session body as the live daemon —
see [08-04 parity contract](08-04-parity-contract.md).

## 4. Layers (docs map)

| Doc | Layer |
|-----|-------|
| `08-01-bte-overview.md` | this document — boundaries, instance binding, modes |
| `08-02-archive-and-backfill.md` | the candle archive + the on-demand backfill job |
| `08-03-historical-runner.md` | the full-pipeline historical replay |
| `08-04-parity-contract.md` | why backtest = paper, and what live adds |
| `08-05-study-persistence.md` | the data-science persistence schema |

## 5. UI surface

`BacktestingDashboard` (`ui/src/components/backtesting/`):

- Navbar = one tab per simulated engine (DIE · MME · TAE · PME · PAE),
  plus the Study Report, History, and Settings — tab order = layer order.
- No-instance state → simplified navbar (Overview + History + Settings)
  with the shared `NoInstanceState` look; the navbar re-charges
  reactively when an instance is selected.
- The run form is **depth-driven (v8.1)**: the archive depth control
  (slider + typed input, 1..=365) is the only window control — Start/End
  dates are removed. The backtest window derives from it
  (`[now − days + burn_in, now]`; the first ~3.1 days warm the pipeline).
- **Automatic data preparation**: pressing Run Backtest checks the
  four-timeframe archive coverage (micro · fast · slow · macro, burn-in
  included) and, when any TF is short, automatically starts the backfill
  and shows live progress — the run fires the moment coverage is
  sufficient. A per-TF readiness strip shows all four timeframes.
- The **Study Report** presents the finished analysis: KPI strip, equity
  curve, drawdown, rolling win-rate, trade P&L histogram, exit-reason
  table, and the NHST edge verdict.

## 6. Config surface

```toml
[workspace.backtest]
archive_depth_days = 180     # 1..=365 (M8-validated) — archive retention + backfill depth
warmup_bars = 300            # burn-in before the first valid MTF decision
store_input_bars = true      # persist the exact input candles per run
max_equity_points = 2000     # equity-curve downsampling cap
max_snapshots = 50000        # recorded-replay tick cap

[workspace.backtest.hyperliquid]
page_cap = 1000              # window-bounded candleSnapshot; conservative page cap
rate_limit_delay_ms = 1000
max_pages_per_run = 2000

[workspace.backtest.bitget]
page_cap = 200               # endpoint accepts limit 1..1000
rate_limit_delay_ms = 100
max_pages_per_run = 6000
```

## 7. Cross-references

- [Parity contract](08-04-parity-contract.md)
- [API gateway contract](../../integration-and-api/06-01-api-gateway-contract.md)
- [Database schema](../../integration-and-api/06-02-database-schema-spec.md)
- [Engine dashboard vocabulary](../../ui-ux/07-07-engine-dashboard-vocabulary.md)
