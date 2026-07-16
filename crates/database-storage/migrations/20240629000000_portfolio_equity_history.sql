CREATE TABLE IF NOT EXISTS portfolio_equity_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    total_value REAL NOT NULL,
    cash_balance REAL NOT NULL,
    unrealized_pnl REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_equity_history_timestamp_desc
    ON portfolio_equity_history (timestamp DESC);
