-- Migration: BTE candle archive + backfill job tracking.
--
-- The deep-history backtest replays the full MME pipeline over archived
-- OHLCV candles. `candle_archive` is a lightweight OHLCV store (seconds,
-- consistent with `market_snapshots.timestamp`) written by two paths:
--   * live — every completed snapshot upserts its OHLCV (source 'live' /
--     'reconstructed' for gap-filled candles), and
--   * backfill — on-demand exchange pagination (source 'backfill').
-- `backfill_jobs` tracks the on-demand fetch jobs for progress/resume.

CREATE TABLE IF NOT EXISTS candle_archive (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange TEXT NOT NULL DEFAULT 'Hyperliquid',
    symbol TEXT NOT NULL,
    timeframe_secs INTEGER NOT NULL,
    ts_secs INTEGER NOT NULL,
    open TEXT,
    high TEXT,
    low TEXT,
    close TEXT,
    volume TEXT,
    trades_count INTEGER,
    source TEXT NOT NULL DEFAULT 'live',
    UNIQUE (exchange, symbol, timeframe_secs, ts_secs)
);

CREATE INDEX IF NOT EXISTS idx_candle_archive_lookup
    ON candle_archive (symbol, timeframe_secs, ts_secs);

CREATE TABLE IF NOT EXISTS backfill_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    instance_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    exchange TEXT NOT NULL DEFAULT 'Hyperliquid',
    depth_days INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    pages_fetched INTEGER NOT NULL DEFAULT 0,
    candles_stored INTEGER NOT NULL DEFAULT 0,
    earliest_ts_secs INTEGER,
    latest_ts_secs INTEGER,
    error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
