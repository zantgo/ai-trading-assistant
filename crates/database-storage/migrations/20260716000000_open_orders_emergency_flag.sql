-- Add Hard Exit / emergency liquidation support to open_orders.
-- This column carries the `is_emergency_liquidation` flag set by the PME Veto
-- Hard Exit path so the audit trail and the pre-trade gate post-mortem can
-- distinguish forced liquidation orders from regular exits.
--
-- The column defaults to 0; existing orders are unaffected.
ALTER TABLE open_orders ADD COLUMN is_emergency_liquidation INTEGER NOT NULL DEFAULT 0;