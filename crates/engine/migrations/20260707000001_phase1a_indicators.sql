-- Migration: Add Phase 1A indicator columns to market_snapshots
-- (Supertrend, Keltner, Donchian, OBV, CMF, MFI, Historical Volatility).
-- Dedicated normalized/state columns for ML indexing; the authoritative full
-- indicator map (incl. sub-values + signals) is round-tripped via the
-- auxiliary_normalized_data JSON blob.
ALTER TABLE market_snapshots ADD COLUMN supertrend_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN supertrend_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN keltner_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN keltner_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN donchian_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN donchian_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN obv_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN obv_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN cmf_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN cmf_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN mfi_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN mfi_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN hv_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN hv_state_label TEXT;

CREATE INDEX IF NOT EXISTS idx_snap_supertrend ON market_snapshots (supertrend_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_obv ON market_snapshots (obv_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_mfi ON market_snapshots (mfi_normalized);
