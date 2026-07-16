-- Migration: Persistent safety & equity state (DB-04, DB-07, DB-18)
-- Adds persistent columns to paper_balances so the engine does not lose:
--   * active_stance          (PME veto path)
--   * starting_session_equity (used for max_daily_drawdown_pct early warning)
--   * peak_equity           (used for the 30% Hard Exit threshold)
--   * cooldown_start_ms     (SUSPENDED state 8-hour timer)
-- across an engine restart.

ALTER TABLE paper_balances ADD COLUMN active_stance TEXT NOT NULL DEFAULT 'ACTIVE';
ALTER TABLE paper_balances ADD COLUMN starting_session_equity TEXT NOT NULL DEFAULT '0';
ALTER TABLE paper_balances ADD COLUMN peak_equity TEXT NOT NULL DEFAULT '0';
ALTER TABLE paper_balances ADD COLUMN cooldown_start_ms INTEGER;

-- Backfill peak_equity / starting_session_equity from the most recent portfolio snapshot.
UPDATE paper_balances
SET starting_session_equity = COALESCE(
        (SELECT total_value FROM portfolio_equity_history
         ORDER BY timestamp ASC LIMIT 1), '0'),
    peak_equity            = COALESCE(
        (SELECT MAX(total_value) FROM portfolio_equity_history), '0')
WHERE starting_session_equity = '0' OR peak_equity = '0';
