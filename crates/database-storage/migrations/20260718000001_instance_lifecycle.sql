CREATE TABLE IF NOT EXISTS instance_lifecycle (
  instance_id         TEXT PRIMARY KEY,
  lifecycle_state     TEXT NOT NULL DEFAULT 'STOPPED'
                      CHECK (lifecycle_state IN ('RUNNING','PAUSED','STOPPING','STOPPED')),
  automation_json     TEXT CHECK (automation_json IS NULL OR json_valid(automation_json)),
  entered_state_at_ms INTEGER NOT NULL,
  deleted_at_ms       INTEGER,
  updated_at_ms       INTEGER NOT NULL
);
