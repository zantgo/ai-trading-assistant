-- 08-05-connection-quality.md: samples are tracked per (pair_key, timeframe_secs).
-- Legacy process-wide rows keep the 'GLOBAL' / 0 defaults.
ALTER TABLE connection_quality_samples ADD COLUMN pair_key TEXT NOT NULL DEFAULT 'GLOBAL';
ALTER TABLE connection_quality_samples ADD COLUMN timeframe_secs INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_cq_scope
    ON connection_quality_samples(pair_key, timeframe_secs, window, timestamp_ms);
