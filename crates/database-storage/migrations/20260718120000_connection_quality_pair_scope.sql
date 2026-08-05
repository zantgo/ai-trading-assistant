-- 08-05-connection-quality.md: samples are tracked per (pair_key, timeframe_secs).
-- Legacy process-wide rows keep the 'GLOBAL' / 0 defaults.
--
-- AUDIT-V9 B13: the cross-scope process-wide aggregate is computed
-- on-demand at query time by `ConnectionQualityRegistry::aggregate_report`,
-- NOT persisted. The DEFAULT 'GLOBAL' / 0 is retained only for
-- backwards-compat with rows persisted by older daemons; the current
-- persistence loop (`crates/network-adapters/src/connection_quality_tracker.rs::run_persistence_loop`)
-- writes one row per `(pair_key, timeframe_secs, window)` and never
-- writes a row with `pair_key='GLOBAL'`. Operators querying for
-- "workspace-wide" should call `aggregate_report` directly.
ALTER TABLE connection_quality_samples ADD COLUMN pair_key TEXT NOT NULL DEFAULT 'GLOBAL';
ALTER TABLE connection_quality_samples ADD COLUMN timeframe_secs INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_cq_scope
    ON connection_quality_samples(pair_key, timeframe_secs, window, timestamp_ms);
