-- Add slot_index column to open_orders for per-slot bracket association.
ALTER TABLE open_orders ADD COLUMN slot_index INTEGER;
