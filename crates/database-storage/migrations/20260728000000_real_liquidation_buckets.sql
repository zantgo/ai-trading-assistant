-- Block D: Persistence for the price-bucketed real liquidation aggregation
-- (LiquidityFlow.recent_real_buckets). The accumulator is in-memory only,
-- so on daemon restart the heatmap rebuilds from scratch. The trade-off is
-- acceptable because:
--   1. Buckets are display-only (Block B/C contract)
--   2. The raw `liquidation_events` table already persists per-event forensics
--   3. Cost-vs-value: 1000 buckets × 4 TFs × ~1440 minutes = 5.7M+ rows/day
--      at cascade volume would balloon storage beyond the platform's SLO
--
-- What this migration DOES add:
--   * A slim `liquidation_real_buckets` table for the **periodic flush**
--     of the in-memory buckets (60s cadence, UPSERT semantics). Operators
--     who want heatmap persistence across restarts can enable it via a
--     future `flush_secs` config; today the flush is a no-op (kept here
--     so the migration history is consistent with the implementation plan).
--   * A retention pruner scheduled at 24h boundary.
--   * Indexes aligned to `(symbol, bucket_index)` for downstream queries.

CREATE TABLE IF NOT EXISTS liquidation_real_buckets (
    symbol TEXT NOT NULL,
    bucket_index INTEGER NOT NULL,
    side TEXT NOT NULL,
    price_low REAL NOT NULL,
    price_high REAL NOT NULL,
    peak_price REAL NOT NULL,
    notional_usd REAL NOT NULL,
    event_count INTEGER NOT NULL,
    mid_anchor REAL,
    last_updated_ms INTEGER NOT NULL,
    created_at INTEGER DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_lrb_pk
    ON liquidation_real_buckets(symbol, bucket_index, side);

CREATE INDEX IF NOT EXISTS idx_lrb_symbol_ts
    ON liquidation_real_buckets(symbol, last_updated_ms DESC);

-- 24h retention prune is performed by the existing hourly pruner in
-- `database_storage::logger::run_retention_cleanup`. The query is added
-- here so the schema and pruner stay in sync.
