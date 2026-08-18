-- v7 TAE: automation activity log (audit trail for the setup executor).
CREATE TABLE IF NOT EXISTS automation_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    instance_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    ts_ms INTEGER NOT NULL,
    event TEXT NOT NULL,
    detail TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_automation_activity_instance_ts
    ON automation_activity (instance_id, ts_ms DESC);
