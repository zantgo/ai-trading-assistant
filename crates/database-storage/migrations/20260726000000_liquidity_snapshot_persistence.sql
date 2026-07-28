-- Phase 0-4: persist Liquidity Intelligence per-bar state.
--
-- Before this migration, every completed `MarketSnapshot` carried its
-- `liquidity` (LiquidityFlow) and `cluster` (LiquidationClusterMatrix)
-- payloads only in memory and on the live WebSocket broadcast. After a
-- daemon restart the `/api/history` endpoint had no historical flow /
-- cluster series to return, so charts that rely on the history
-- bootstrap (cold start, before the first per-TF refresh tick fires)
-- rendered as fully empty until the live stream caught up.
--
-- We add eight scalar columns summarising the per-bar flow + cluster
-- snapshot, leaving the heavy `LiquidationClusterMatrix` JSON to the
-- existing `auxiliary_normalized_data` blob (where the indicator map
-- already lives). The columns are NULL-tolerant so legacy rows from
-- pre-Phase-4 daemons continue to work unchanged.
--
-- Retention: the same 7-day window already applied to
-- `market_snapshots` continues to cover these columns; no separate
-- cleanup pass is required.

ALTER TABLE market_snapshots ADD COLUMN liquidity_long_usd REAL;
ALTER TABLE market_snapshots ADD COLUMN liquidity_short_usd REAL;
ALTER TABLE market_snapshots ADD COLUMN liquidity_net_usd REAL;
ALTER TABLE market_snapshots ADD COLUMN liquidity_events INTEGER;
ALTER TABLE market_snapshots ADD COLUMN liquidity_cascade_state TEXT;
ALTER TABLE market_snapshots ADD COLUMN liquidity_cascade_intensity REAL;
ALTER TABLE market_snapshots ADD COLUMN cluster_long_count INTEGER;
ALTER TABLE market_snapshots ADD COLUMN cluster_short_count INTEGER;
ALTER TABLE market_snapshots ADD COLUMN cluster_total_notional_usd REAL;
ALTER TABLE market_snapshots ADD COLUMN cluster_estimation_confidence REAL;
-- Full payloads for round-trip (history endpoint, chart bootstrap after
-- a daemon restart). Heavy JSON blobs that we don't want to lose.
ALTER TABLE market_snapshots ADD COLUMN liquidity_json TEXT;
ALTER TABLE market_snapshots ADD COLUMN cluster_json TEXT;

CREATE INDEX IF NOT EXISTS idx_snapshots_liquidity_cascade
    ON market_snapshots(symbol, timeframe_secs, timestamp DESC)
    WHERE liquidity_cascade_state IS NOT NULL;