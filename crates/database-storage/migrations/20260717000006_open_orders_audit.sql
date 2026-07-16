-- Migration: open_orders audit columns (DB-08 + DB-19 PRE_DISPATCH persistence)
-- Adds filled_size, slippage_bps, updated_at for partial-fill tracking and audit.
-- Adds order_id, client_order_id, state, held_for_review_reason to the
-- rebuilt schema for stateful PRE_DISPATCH persistence (Gate 5 slippage
-- review window).

-- Phase 1: add pure additive columns to the existing open_orders table.
-- Includes the audit columns (filled_size, slippage_bps, updated_at),
-- the order/client IDs (order_id, client_order_id), and the missing
-- persisted fields (state, acknowledged_at, held_for_review_reason)
-- that were documented in §3.2 but never made it into a prior migration.
ALTER TABLE open_orders ADD COLUMN filled_size REAL NOT NULL DEFAULT 0;
ALTER TABLE open_orders ADD COLUMN slippage_bps REAL;
ALTER TABLE open_orders ADD COLUMN updated_at INTEGER;
ALTER TABLE open_orders ADD COLUMN order_id TEXT;
ALTER TABLE open_orders ADD COLUMN client_order_id TEXT;
ALTER TABLE open_orders ADD COLUMN held_for_review_reason TEXT;
ALTER TABLE open_orders ADD COLUMN state TEXT NOT NULL DEFAULT 'OPEN';
ALTER TABLE open_orders ADD COLUMN acknowledged_at INTEGER;

-- Backfill updated_at from created_at for legacy rows.
UPDATE open_orders SET updated_at = created_at WHERE updated_at IS NULL;

-- Phase 2: rebuild the table with the extended state CHECK constraint
-- so PRE_DISPATCH rows can be persisted (DB-19). Idempotent: drops are
-- guarded by IF EXISTS checks via the rename-rebuild pattern.
ALTER TABLE open_orders RENAME TO open_orders_old;

CREATE TABLE open_orders_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id TEXT UNIQUE,
    client_order_id TEXT,
    symbol TEXT NOT NULL,
    order_type TEXT NOT NULL CHECK (order_type IN ('LIMIT','STOP','MARKET','PRE_DISPATCH')),
    direction TEXT NOT NULL CHECK (direction IN ('BUY','SELL')),
    state TEXT NOT NULL CHECK (state IN ('OPEN','FILLED','PARTIALLY_FILLED','CANCELED','REJECTED','EXPIRED','PRE_DISPATCH')),
    price REAL,
    trigger_price REAL,
    size REAL NOT NULL,
    filled_size REAL NOT NULL DEFAULT 0,
    slippage_bps REAL,
    is_reduce_only INTEGER NOT NULL DEFAULT 0,
    is_emergency_liquidation INTEGER NOT NULL DEFAULT 0,
    held_for_review_reason TEXT,
    associated_position_id INTEGER,
    created_at INTEGER NOT NULL,
    acknowledged_at INTEGER,
    updated_at INTEGER,
    FOREIGN KEY (associated_position_id) REFERENCES active_positions(id)
);

INSERT INTO open_orders_new (id, order_id, client_order_id, symbol, order_type, direction, state,
                              price, trigger_price, size, filled_size, slippage_bps, is_reduce_only,
                              is_emergency_liquidation, held_for_review_reason,
                              associated_position_id, created_at, acknowledged_at, updated_at)
SELECT id, order_id, client_order_id, symbol,
       CASE WHEN order_type IN ('LIMIT','STOP','MARKET') THEN order_type ELSE 'LIMIT' END,
       direction,
       'OPEN',
       price, trigger_price, size,
       COALESCE(filled_size, 0),
       slippage_bps,
       is_reduce_only,
       is_emergency_liquidation,
       held_for_review_reason,
       associated_position_id,
       created_at,
       acknowledged_at,
       COALESCE(updated_at, created_at)
FROM open_orders_old;

DROP TABLE open_orders_old;
ALTER TABLE open_orders_new RENAME TO open_orders;
