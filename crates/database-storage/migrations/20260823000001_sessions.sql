-- v10: session identity — every live/paper run gets a monotonic,
-- persisted, never-reused session number. All telemetry rows join on it.

CREATE TABLE IF NOT EXISTS sessions (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    mode                   TEXT NOT NULL,
    exchange               TEXT,
    currency               TEXT,
    portfolio_capital_usd  REAL,
    started_at_ms          INTEGER NOT NULL,
    ended_at_ms            INTEGER,
    status                 TEXT NOT NULL DEFAULT 'active',
    config_snapshot_json   TEXT
);

ALTER TABLE market_snapshots ADD COLUMN session_id INTEGER;
ALTER TABLE trade_telemetry_history ADD COLUMN session_id INTEGER;
ALTER TABLE paper_trades ADD COLUMN session_id INTEGER;
ALTER TABLE portfolio_equity_history ADD COLUMN session_id INTEGER;
ALTER TABLE automation_activity ADD COLUMN session_id INTEGER;
ALTER TABLE risk_control_events ADD COLUMN session_id INTEGER;
ALTER TABLE backtest_runs ADD COLUMN session_id INTEGER;
