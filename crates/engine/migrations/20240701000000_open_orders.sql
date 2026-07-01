-- Unified open_orders table: handles Limit, Stop, TP, and SL brackets
CREATE TABLE IF NOT EXISTS open_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    order_type TEXT NOT NULL CHECK (order_type IN ('LIMIT', 'STOP')),
    direction TEXT NOT NULL CHECK (direction IN ('BUY', 'SELL')),
    price REAL,
    trigger_price REAL,
    size REAL NOT NULL,
    is_reduce_only INTEGER NOT NULL DEFAULT 0,
    associated_position_id INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (associated_position_id) REFERENCES active_positions(id)
);

-- Port uncompleted take-profit targets to open_orders:
--   LONG positions  →  LIMIT SELL with is_reduce_only = 1
--   SHORT positions →  LIMIT BUY  with is_reduce_only = 1
INSERT INTO open_orders (symbol, order_type, direction, price, trigger_price, size, is_reduce_only, associated_position_id, created_at)
SELECT
    t.symbol,
    'LIMIT',
    CASE WHEN p.direction = 'LONG' THEN 'SELL' ELSE 'BUY' END,
    t.target_price,
    NULL,
    t.size_fraction * 100.0,
    1,
    t.position_id,
    COALESCE(t.timestamp, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
FROM position_take_profit_targets t
JOIN active_positions p ON p.id = t.position_id
WHERE t.is_hit = 0;

-- Update existing paper balances: minimum allocation_pct is now 25%
UPDATE paper_balances SET allocation_pct = 25 WHERE allocation_pct < 25;

-- Drop legacy table
DROP TABLE IF EXISTS position_take_profit_targets;
