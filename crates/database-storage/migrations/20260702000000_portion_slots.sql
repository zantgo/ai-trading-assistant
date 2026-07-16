-- Migration: 4-Portion Dynamic Margin State Machine
-- Replaces active_position_portions with slot-level position_slots.
-- Adds lifecycle capital tracking to active_positions.
-- Adds per-position equity history table.

-- Step 1: Add new columns to active_positions for cycle capital tracking
ALTER TABLE active_positions ADD COLUMN initial_allocated_margin REAL NOT NULL DEFAULT 0.0;
ALTER TABLE active_positions ADD COLUMN realized_pnl_accumulator REAL NOT NULL DEFAULT 0.0;

-- Step 2: Create the new position_slots table
CREATE TABLE IF NOT EXISTS position_slots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('LONG', 'SHORT')),
    slot_index INTEGER NOT NULL CHECK (slot_index BETWEEN 0 AND 3),
    is_active INTEGER NOT NULL DEFAULT 0,
    entry_price REAL NOT NULL DEFAULT 0.0,
    size REAL NOT NULL DEFAULT 0.0,
    allocated_usd REAL NOT NULL DEFAULT 0.0,
    realized_pnl REAL DEFAULT 0.0,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (position_id) REFERENCES active_positions(id) ON DELETE CASCADE
);

-- Unique index: at most one active slot per index per position
CREATE UNIQUE INDEX IF NOT EXISTS idx_position_slots_active
ON position_slots (position_id, slot_index) WHERE is_active = 1;

-- Regular index for symbol-based lookups
CREATE INDEX IF NOT EXISTS idx_position_slots_symbol
ON position_slots (symbol, is_active);

-- Step 3: Preserve existing portions as slot records
INSERT OR IGNORE INTO position_slots (position_id, symbol, direction, slot_index, is_active, entry_price, size, allocated_usd, realized_pnl, timestamp)
SELECT
    position_id,
    symbol,
    direction,
    portion_number AS slot_index,
    1 AS is_active,
    entry_price,
    size,
    allocated_usd,
    0.0 AS realized_pnl,
    timestamp
FROM active_position_portions
WHERE portion_number < 4;

-- Update active_positions.initial_allocated_margin from existing position data
UPDATE active_positions SET initial_allocated_margin = allocated_usd WHERE initial_allocated_margin = 0.0;

-- Step 4: Drop the deprecated table
DROP TABLE IF EXISTS active_position_portions;

-- Step 5: Create position-level equity history table for performance charts
CREATE TABLE IF NOT EXISTS position_equity_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    equity_value REAL NOT NULL,
    cash_balance REAL NOT NULL,
    unrealized_pnl REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pos_equity_ts
ON position_equity_snapshots (symbol, timestamp ASC);
