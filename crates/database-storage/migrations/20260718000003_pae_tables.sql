-- Migration: PAE output tables — strategy_analytics_history, performance_matrix_snapshots
-- Per docs/engines/performance-analytics-engine/03-05-03 (L2) and 03-05-05 (L4)

CREATE TABLE IF NOT EXISTS strategy_analytics_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s','now') * 1000),
    policy_id TEXT NOT NULL,
    total_trades INTEGER NOT NULL,
    win_count INTEGER NOT NULL,
    loss_count INTEGER NOT NULL,
    win_rate REAL NOT NULL,
    gross_profit REAL NOT NULL,
    gross_loss REAL NOT NULL,
    profit_factor REAL,
    average_win REAL NOT NULL,
    average_loss REAL NOT NULL,
    avg_win_loss_ratio REAL NOT NULL,
    expectancy REAL NOT NULL,
    slippage_overhead REAL NOT NULL,
    t_statistic REAL NOT NULL,
    p_value REAL NOT NULL,
    p_mc REAL NOT NULL,
    monte_carlo_runs INTEGER NOT NULL,
    is_significant INTEGER NOT NULL,
    classification TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_strategy_analytics_policy
    ON strategy_analytics_history (policy_id, timestamp DESC);

CREATE TABLE IF NOT EXISTS performance_matrix_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s','now') * 1000),
    policy_id TEXT NOT NULL,
    regime TEXT NOT NULL,
    trade_count INTEGER NOT NULL,
    win_rate REAL NOT NULL,
    profit_factor REAL,
    avg_r_multiple REAL NOT NULL,
    total_pnl REAL NOT NULL,
    compatibility_label TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_performance_matrix_policy
    ON performance_matrix_snapshots (policy_id, timestamp DESC);
