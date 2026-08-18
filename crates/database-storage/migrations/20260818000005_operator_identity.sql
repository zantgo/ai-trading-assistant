-- E3 (2026-08-18): single-operator identity on the safety audit trail.
-- Every row is stamped `operator_id = 'local'` (single-operator local
-- deployment — AUDIT-V4-076 cancelled by design).
ALTER TABLE risk_control_events ADD COLUMN operator_id TEXT NOT NULL DEFAULT 'local';

CREATE INDEX IF NOT EXISTS idx_rce_operator_time
    ON risk_control_events (operator_id, timestamp_ms DESC);
