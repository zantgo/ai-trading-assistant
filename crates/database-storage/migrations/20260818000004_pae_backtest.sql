-- PAE v7: record MME decision matrices on market_snapshots (backtest replay
-- source) + persist backtest runs.
ALTER TABLE market_snapshots ADD COLUMN market_regime TEXT;
ALTER TABLE market_snapshots ADD COLUMN opportunity_json TEXT;
ALTER TABLE market_snapshots ADD COLUMN decision_context_json TEXT;
ALTER TABLE market_snapshots ADD COLUMN analysis_json TEXT;
ALTER TABLE market_snapshots ADD COLUMN advisory_json TEXT;

CREATE TABLE IF NOT EXISTS backtest_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    params_json TEXT NOT NULL,
    summary_json TEXT NOT NULL,
    stats_json TEXT NOT NULL,
    trades_json TEXT NOT NULL,
    equity_curve_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
