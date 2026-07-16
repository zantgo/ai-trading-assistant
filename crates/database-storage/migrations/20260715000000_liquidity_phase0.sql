-- Phase 0: Activate dormant derivatives telemetry.
--
-- Adds columns for mark price, index price, and the mark-vs-index spread
-- percent to the `market_snapshots` table. These fields have been declared
-- on `MarketSnapshot` (shared::models) since v0 but never persisted. With
-- Phase 0 active WS + REST subscriptions, the values now arrive live and
-- need to land in the central telemetry store for backtesting.
--
-- Also creates the `liquidation_events` table for raw exchange-published
-- forced closes (Phase 1). Retention: 90 days, enforced by a periodic
-- cleanup pass in the telemetry logger.
--
-- The `liquidation_buckets` per-candle aggregation table is added in a
-- later migration when Phase 1 introduces the flow aggregator.

ALTER TABLE market_snapshots ADD COLUMN mark_price REAL;
ALTER TABLE market_snapshots ADD COLUMN index_price REAL;
ALTER TABLE market_snapshots ADD COLUMN mark_index_spread_pct REAL;

CREATE INDEX IF NOT EXISTS idx_snapshots_mark_price
    ON market_snapshots(symbol, timeframe_secs, timestamp DESC)
    WHERE mark_price IS NOT NULL;

CREATE TABLE IF NOT EXISTS liquidation_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange TEXT NOT NULL,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,                 -- 'LONG' or 'SHORT' (the side that got liquidated)
    price REAL NOT NULL,
    size_usd REAL NOT NULL,
    timestamp INTEGER NOT NULL,         -- ms since epoch
    venue_order_id TEXT,
    created_at INTEGER DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
);

CREATE INDEX IF NOT EXISTS idx_liq_events_lookup
    ON liquidation_events(symbol, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_liq_events_exchange
    ON liquidation_events(exchange, timestamp DESC);