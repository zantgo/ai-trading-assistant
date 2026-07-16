-- Migration: Destructive rename `final_invalidation_level` → `invalidation_level` (DB-15)
-- The Opportunity / Decision / Position Matrix schema was renamed in v2.1;
-- this migration finishes the rename by also updating the SQLite column.

ALTER TABLE active_positions RENAME COLUMN final_invalidation_level TO invalidation_level;
