-- Migration: Add creator_name to saved_edges for identicon/deterministic avatar attribution
-- This migration is purely additive and does not alter any existing data.

ALTER TABLE saved_edges ADD COLUMN creator_name TEXT;
