-- Migration: Active position protection prices (DB-03)
-- Per-fill model requires stop_loss_price and take_profit_price to be persisted
-- so an engine restart does not open the user to unhedged exposure when the engine
-- crashes while a SL/TP bracket is active.

ALTER TABLE active_positions ADD COLUMN stop_loss_price TEXT;
ALTER TABLE active_positions ADD COLUMN take_profit_price TEXT;
