-- Migration: paper_balances Decimal-precision columns (DB-02)
-- Mirrors 20260715200000_risk_profiles_decimal.sql for paper_balances.
-- The cold-path Decimal precision invariant requires that all monetary ledger
-- columns round-trip through rust_decimal::Decimal without intermediate f64.
--
-- The rebuild pattern preserves all data:
--   * Rename paper_balances → paper_balances_old
--   * Create paper_balances_new with the Decimal-precision column types
--   * Copy data with explicit CAST(REAL AS TEXT) (cleanest decimal representation)
--   * Drop old, rename new

ALTER TABLE paper_balances RENAME TO paper_balances_old;

CREATE TABLE paper_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    initial_usd TEXT NOT NULL DEFAULT '10000',
    current_cash TEXT NOT NULL DEFAULT '10000',
    allocation_pct REAL NOT NULL DEFAULT 10.0,
    auto_execute INTEGER NOT NULL DEFAULT 0,
    max_risk_pct TEXT NOT NULL DEFAULT '2',
    leverage INTEGER NOT NULL DEFAULT 20,
    auto_execute_intervals INTEGER NOT NULL DEFAULT 15,
    lookback_trades INTEGER NOT NULL DEFAULT 10,
    break_even_trail_enabled INTEGER NOT NULL DEFAULT 0,
    active_stance TEXT NOT NULL DEFAULT 'ACTIVE',
    starting_session_equity TEXT NOT NULL DEFAULT '0',
    peak_equity TEXT NOT NULL DEFAULT '0',
    cooldown_start_ms INTEGER
);

INSERT INTO paper_balances (id, symbol, initial_usd, current_cash, allocation_pct,
                            auto_execute, max_risk_pct, leverage, auto_execute_intervals,
                            lookback_trades, break_even_trail_enabled, active_stance,
                            starting_session_equity, peak_equity, cooldown_start_ms)
SELECT id, symbol,
       CAST(initial_usd AS TEXT),
       CAST(current_cash AS TEXT),
       allocation_pct,
       auto_execute,
       CAST(max_risk_pct AS TEXT),
       leverage,
       auto_execute_intervals,
       lookback_trades,
       break_even_trail_enabled,
       active_stance,
       starting_session_equity,
       peak_equity,
       cooldown_start_ms
FROM paper_balances_old;

DROP TABLE paper_balances_old;
