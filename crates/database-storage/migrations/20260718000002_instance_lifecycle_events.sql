CREATE TABLE IF NOT EXISTS instance_lifecycle_events (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  instance_id  TEXT NOT NULL,
  from_state   TEXT CHECK (from_state IS NULL OR from_state IN ('RUNNING','PAUSED','STOPPING','STOPPED')),
  to_state     TEXT NOT NULL CHECK (to_state IN ('RUNNING','PAUSED','STOPPING','STOPPED','DELETED')),
  actor        TEXT NOT NULL CHECK (actor IN ('operator','automation','system')),
  reason_json  TEXT CHECK (reason_json IS NULL OR json_valid(reason_json)),
  timestamp_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_lifecycle_events_instance_time
  ON instance_lifecycle_events(instance_id, timestamp_ms DESC);
