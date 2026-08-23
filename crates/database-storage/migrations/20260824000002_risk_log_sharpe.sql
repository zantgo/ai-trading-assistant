-- v10.1: log-return Sharpe persisted alongside the simple-return family.

ALTER TABLE risk_analytics_history ADD COLUMN sharpe_ratio_log REAL;
