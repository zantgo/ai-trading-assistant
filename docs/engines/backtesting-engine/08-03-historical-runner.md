# BTE Layer 2 — Historical Runner (deep-history simulation)

**Version:** 8.0 (2026-08-20)
**Engine:** Backtesting Engine
**Code:** `crates/backtesting-engine/src/historical.rs`

## 1. What it replays

The historical runner reproduces the **entire MME pipeline** over
archived candles — no re-implementation:

```text
candle_archive (per TF windows, burn-in inclusive)
  → warm_indicators_for_timeframe     (the SAME warm path the live daemon
                                       boots through — full per-candle
                                       snapshots: 52 indicators, normalized
                                       maps, market context)
  → synthesize_cross_tf               (the SAME pure MTF synthesizer the
                                       live L4/L5 assembly calls)
     + DecisionContext::compute       (the same decision assembly)
  → run_tick                          (the SAME per-tick session body as
                                       paper/live — fills, sizing, safety)
```

## 2. Pipeline details

1. **Window loading** — per ladder TF, `query_archive_window` loads
   `[from − burn_in, to]` where `burn_in = warmup_bars × max(tf_secs)`
   (default 300 bars × 900 s ≈ 3.1 days). The burn-in only warms
   indicator windows; it produces no decisions.

2. **Exact MME parameters (v8.1 fidelity)** — the run builds the per-slot
   `TimeframeConfig`s from the instance entry (`micro_term` / `fast_term`
   / `slow_term` / `macro_term`, with the same registry fallbacks) and
   the `ActiveSet` from the global + per-instance `[activation]` union
   exactly like `registry::add_instance` / `registry/pipelines.rs` — so
   the replay uses the same indicator periods, weights and activation
   toggles the live MME uses.

3. **Chunked warm replay** — the warm path is CPU-bound (~50–60 ms per
   candle); the platform replays in chunks of 800 candles with a
   300-candle overlap (every indicator lookback in this platform is
   ≤ 300 bars, so chunk tails are mathematically identical to a
   continuous replay). Per-TF warms run in parallel on the tokio
   blocking pool.

4. **MTF synthesis** — per entry-TF candle, the latest aligned snapshot
   from every ladder TF feeds `synthesize_cross_tf` (same state
   carry-over of score/regime/volume-dim/bias as live). The unsigned
   3-factor confluence blend and `DecisionContext::compute` mirror the
   live assembly exactly.

5. **Replay** — the synthesized snapshot (opportunity, decision context,
   analysis, advisory) drives `run_tick` with a virtual `candle_ts`.
   Simulated closes, equity samples, decision snapshots and portfolio
   samples are captured exactly like the recorded runner.

## 3. Honest limitations

- **No derivatives** — the archive stores no order-book / OI / funding /
  liquidation data; `synthesize_cross_tf` runs with `liquidity_flow =
  None`, `cluster = None`, `signals = []`. The synthesized decision is
  candle-based; live decisions additionally see the derivatives layers.
- **Determinism** — no wall clock, no randomness in the executor path;
  Monte Carlo is seeded (42). Identical inputs ⇒ identical outputs.
- **Cost** — a 7-day window at the default ladder is minutes of CPU
  (parallel warms). The synchronous endpoint holds the global run lock
  for the duration.

## 4. Coverage validation

`POST /api/backtest/run` (mode `historical`) fails with
`400 not_enough_data` when any ladder TF lacks archive rows in
`[from − burn_in, to]` — the response carries the per-TF coverage detail
so the operator can shrink the window or run a backfill first.

## 5. Cross-references

- [BTE overview](08-01-bte-overview.md)
- [Archive & backfill](08-02-archive-and-backfill.md)
- [Parity contract](08-04-parity-contract.md)
