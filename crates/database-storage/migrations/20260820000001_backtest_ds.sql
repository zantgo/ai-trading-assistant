-- Migration: BTE data-science persistence schema.
--
-- Backtest results are stored in normalized tables (not JSON blobs) so
-- operators can run data-science queries later:
--   backtest_runs     — run metadata (+ legacy JSON summary columns kept)
--   backtest_trades   — one row per simulated close
--   backtest_equity   — downsampled equity curve points
--   backtest_portfolio— per-tick capital/exposure/drawdown samples
--   backtest_signals  — per-tick synthesized decision snapshots
--   backtest_metrics  — key/value summary + NHST metrics
--   backtest_input_bars — the exact input candles (reproducibility)

ALTER TABLE backtest_runs ADD COLUMN instance_id TEXT;
ALTER TABLE backtest_runs ADD COLUMN mode TEXT;
ALTER TABLE backtest_runs ADD COLUMN config_snapshot_json TEXT;

CREATE TABLE IF NOT EXISTS backtest_trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL,
    seq INTEGER NOT NULL,
    ts_close_secs INTEGER NOT NULL,
    direction TEXT NOT NULL,
    entry_price REAL NOT NULL,
    exit_price REAL NOT NULL,
    size REAL NOT NULL,
    pnl REAL NOT NULL,
    exit_reason TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_backtest_trades_run ON backtest_trades (run_id);

CREATE TABLE IF NOT EXISTS backtest_equity (
    run_id INTEGER NOT NULL,
    ts_secs INTEGER NOT NULL,
    equity REAL NOT NULL,
    PRIMARY KEY (run_id, ts_secs)
);

CREATE TABLE IF NOT EXISTS backtest_portfolio (
    run_id INTEGER NOT NULL,
    ts_secs INTEGER NOT NULL,
    equity REAL NOT NULL,
    cash REAL NOT NULL,
    margin_used REAL NOT NULL,
    exposure_pct REAL NOT NULL,
    drawdown_pct REAL NOT NULL,
    positions_open INTEGER NOT NULL,
    PRIMARY KEY (run_id, ts_secs)
);

CREATE TABLE IF NOT EXISTS backtest_signals (
    run_id INTEGER NOT NULL,
    ts_secs INTEGER NOT NULL,
    timeframe_secs INTEGER NOT NULL,
    label TEXT NOT NULL,
    kind TEXT NOT NULL,
    value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_backtest_signals_run ON backtest_signals (run_id, ts_secs);

CREATE TABLE IF NOT EXISTS backtest_metrics (
    run_id INTEGER NOT NULL,
    metric_key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (run_id, metric_key)
);

CREATE TABLE IF NOT EXISTS backtest_input_bars (
    run_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    timeframe_secs INTEGER NOT NULL,
    ts_secs INTEGER NOT NULL,
    open TEXT NOT NULL,
    high TEXT NOT NULL,
    low TEXT NOT NULL,
    close TEXT NOT NULL,
    volume TEXT NOT NULL,
    PRIMARY KEY (run_id, symbol, timeframe_secs, ts_secs)
);
