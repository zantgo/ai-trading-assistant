-- Migration: PAE extended tables — risk_analytics_history, performance_matrix_summaries, optimization_reports
-- Per docs/engines/performance-analytics-engine/03-05-04 (L3), 03-05-05 (L4), and optimizer

CREATE TABLE IF NOT EXISTS risk_analytics_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s','now') * 1000),
    maximum_drawdown_pct REAL NOT NULL,
    max_drawdown_duration_days REAL NOT NULL,
    average_drawdown_pct REAL NOT NULL,
    drawdown_count INTEGER NOT NULL,
    sharpe_ratio REAL,
    sortino_ratio REAL,
    ulcer_index REAL NOT NULL,
    calmar_ratio REAL,
    daily_volatility REAL NOT NULL,
    downside_deviation REAL NOT NULL,
    value_at_risk_95 REAL NOT NULL,
    expected_shortfall_95 REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_risk_analytics_ts
    ON risk_analytics_history (timestamp DESC);

CREATE TABLE IF NOT EXISTS performance_matrix_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s','now') * 1000),
    policy_id TEXT NOT NULL,
    total_trades INTEGER NOT NULL,
    overall_profit_factor REAL,
    overall_expectancy REAL NOT NULL,
    overall_sharpe REAL,
    overall_sortino REAL,
    max_drawdown_pct REAL NOT NULL,
    regime_strength_json TEXT NOT NULL DEFAULT '[]',
    recommendations_json TEXT NOT NULL DEFAULT '[]',
    overall_rating TEXT NOT NULL DEFAULT 'Unrated',
    last_evaluated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_perf_summary_policy
    ON performance_matrix_summaries (policy_id, timestamp DESC);

CREATE TABLE IF NOT EXISTS optimization_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    total_trades INTEGER NOT NULL,
    regime_reports_json TEXT NOT NULL DEFAULT '[]',
    recommendations_json TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_opt_reports_ts
    ON optimization_reports (timestamp DESC);
