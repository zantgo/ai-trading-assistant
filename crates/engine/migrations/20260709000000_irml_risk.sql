-- Migration: Institutional Risk Management Layer (IRML)
-- Adds persistent risk state (streaks/drawdown/permission) and the
-- per-pair adaptive Reward/Risk calibration ledger.
-- See docs/institutional-risk-management-layer.md Section 19.2.

-- Per-evaluation risk state snapshot (survives restarts).
CREATE TABLE IF NOT EXISTS risk_events (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key       TEXT NOT NULL,
    timestamp      INTEGER NOT NULL,
    overall_risk   REAL NOT NULL,
    overall_level  TEXT NOT NULL,
    drawdown_state TEXT NOT NULL,
    permission     TEXT NOT NULL,
    losing_streak  INTEGER NOT NULL,
    winning_streak INTEGER NOT NULL,
    explanation    TEXT
);

CREATE INDEX IF NOT EXISTS idx_risk_events_pair ON risk_events (pair_key, timestamp);

-- Per-block adaptive Reward/Risk calibration ledger (Section 12.5).
-- One row is appended each time a pair completes `rr_block_size` trades.
CREATE TABLE IF NOT EXISTS rr_calibration (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key          TEXT NOT NULL,
    block_index       INTEGER NOT NULL,
    wins              INTEGER NOT NULL,
    losses            INTEGER NOT NULL,
    win_rate_estimate REAL NOT NULL,
    breakeven_ratio   REAL NOT NULL,
    recommended_ratio REAL NOT NULL,
    confidence        REAL NOT NULL,
    net_block_pnl     REAL NOT NULL,
    timestamp         INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rr_calibration_pair ON rr_calibration (pair_key, block_index);
