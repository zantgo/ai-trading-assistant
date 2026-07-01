-- Migration: Break-Even Trail toggle
-- Adds a per-pair toggle to enable/disable automatic break-even trailing after TP fills.
-- Default: OFF (0)

ALTER TABLE paper_balances ADD COLUMN break_even_trail_enabled INTEGER NOT NULL DEFAULT 0;
