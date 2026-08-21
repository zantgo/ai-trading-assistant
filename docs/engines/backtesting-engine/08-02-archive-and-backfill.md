# BTE Layer 1 — Candle Archive & Backfill

**Version:** 8.0 (2026-08-20)
**Engine:** Backtesting Engine
**Tables:** `candle_archive`, `backfill_jobs`
**Code:** `crates/database-storage/src/queries/archive.rs`,
`crates/backtesting-engine/src/backfill.rs`

## 1. The candle archive

`candle_archive` is a lightweight OHLCV store (Unix seconds, consistent
with `market_snapshots.timestamp`) — the deep-history input of the
historical runner:

| Column | Notes |
|--------|-------|
| exchange, symbol, timeframe_secs, ts_secs | UNIQUE — one row per (venue, pair, TF, candle) |
| open/high/low/close/volume | TEXT decimals |
| source | `live` · `reconstructed` (live pipeline) · `backfill` (on-demand job) |

Two write paths:

1. **Live path (every session mode)** — `insert_snapshot_internal`
   upserts the completed snapshot's OHLCV in the same worker, so the
   archive stays warm as long as the daemon runs (source `live`, or
   `reconstructed` for gap-filled candles).
2. **Backfill path** — the on-demand job below (source `backfill`).

Retention: `prune_candle_archive` deletes rows older than
`[workspace.backtest].archive_depth_days` (1..=365) hourly.

## 2. Exchange limits (what bounds the depth)

| Exchange | Endpoint | Limit behaviour |
|----------|----------|-----------------|
| Hyperliquid | `candleSnapshot` (POST `/info`) | No `limit` param — window-bounded; the platform pages conservatively at `page_cap = 1000` candles/request |
| Bitget | GET `/api/v2/mix/market/candles` | `limit` 1..1000; the platform pages at `page_cap = 200`/request |

Neither exchange documents a hard history depth — pagination can reach
listing day. The practical bound is the **request budget**
(`max_pages_per_run` × page cap × TF duration) plus the configured
archive depth:

```
theoretical_max_lookback(tf) = min(archive_depth_days × 86400,
                                   page_cap × max_pages_per_run × tf)
```

The coverage endpoint (`GET /api/backtest/coverage`) reports the actual
archived span and this theoretical ceiling per (symbol, TF).

## 3. The backfill job

`POST /api/backtest/archive/backfill` `{ instance_id, depth_days? }`:

- Validates the bound instance (exists + running) and the depth
  (1..=365); rejects a second active job for the instance (409).
- Pages the instance's exchange backward from `now` to
  `now − depth_days` for every **≥ 1-minute** TF in the instance ladder
  (sub-minute TFs bypass exchange history — HFP-03; their archive
  coverage comes from the live path only).
- **Resumable** — the cursor starts just below the earliest archived
  candle, so covered spans cost zero requests.
- **Rate-limited** — config-driven per-page delay; hard page ceiling.
- Progress updates per page into the in-memory registry
  (`BacktestRegistry`) and persists to `backfill_jobs` every 10 pages;
  `GET /api/backtest/archive/progress/:id` serves live progress;
  `POST /api/backtest/archive/cancel/:id` cancels.

## 4. Coverage surface

`GET /api/backtest/coverage?instance_id=` returns:

- `archive_depth_days` — the configured depth ceiling;
- `snapshots[]` — recorded-snapshot coverage (the recorded mode source);
- `archive[]` — per (symbol, TF): `candle_count`, `earliest_secs`,
  `latest_secs`, `covered_span_secs`, `max_lookback_secs`, `coverage_pct`;
- `backfill_jobs[]` — recent job rows.

## 5. Cross-references

- [BTE overview](08-01-bte-overview.md)
- [Historical runner](08-03-historical-runner.md)
- [Database schema](../../integration-and-api/06-02-database-schema-spec.md)
