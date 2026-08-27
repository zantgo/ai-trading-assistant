-- M10 (production audit, 2026-08-17): the `risk_control_events` audit log.
--
-- The execution-daemon veto loop and the TAE gate chain have INSERTed
-- into this table since v4.0, but NO migration ever created it — every
-- veto event and gate decision was silently dropped (SQLite "no such
-- table" errors swallowed by `let _ =`). This migration creates the
-- table + the index the queries already assume.

CREATE TABLE IF NOT EXISTS risk_control_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    instance_id TEXT NOT NULL,
    symbol TEXT,
    gate_id INTEGER,
    decision TEXT,
    reason TEXT,
    timestamp_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rce_instance_gate_time
    ON risk_control_events (instance_id, gate_id, timestamp_ms DESC);
CREATE INDEX IF NOT EXISTS idx_rce_symbol_time
    ON risk_control_events (symbol, timestamp_ms DESC);
