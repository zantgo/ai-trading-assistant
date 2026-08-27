-- Production hardening (audit C5/DB): indices for session_id joins,
-- unique constraint for backtest_trades run_id+seq, and prune helpers.
-- FK enforcement is enabled per-connection via PRAGMA foreign_keys=ON in
-- init_db; these indices make the session_id joins efficient and the
-- UNIQUE prevents duplicate seq on reruns.

CREATE INDEX IF NOT EXISTS idx_market_snapshots_session_id
    ON market_snapshots (session_id);
CREATE INDEX IF NOT EXISTS idx_trade_telemetry_history_session_id
    ON trade_telemetry_history (session_id);
CREATE INDEX IF NOT EXISTS idx_paper_trades_session_id
    ON paper_trades (session_id);
CREATE INDEX IF NOT EXISTS idx_portfolio_equity_history_session_id
    ON portfolio_equity_history (session_id);
CREATE INDEX IF NOT EXISTS idx_automation_activity_session_id
    ON automation_activity (session_id);
CREATE INDEX IF NOT EXISTS idx_risk_control_events_session_id
    ON risk_control_events (session_id);
CREATE INDEX IF NOT EXISTS idx_backtest_runs_session_id
    ON backtest_runs (session_id);

-- Backtest trades: prevent duplicate seq per run (rerun dedup was INSERT OR IGNORE)
CREATE UNIQUE INDEX IF NOT EXISTS idx_backtest_trades_run_seq
    ON backtest_trades (run_id, seq);

-- Archive prune helper: ts_secs is filtered in prune_candle_archive
CREATE INDEX IF NOT EXISTS idx_candle_archive_ts_secs
    ON candle_archive (ts_secs);

-- Connection quality prune helper
CREATE INDEX IF NOT EXISTS idx_connection_quality_samples_timestamp
    ON connection_quality_samples (timestamp_ms);
