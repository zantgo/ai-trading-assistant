-- Migration: Add Stochastic and ChandeMO column structures to market_snapshots
-- Dedicated normalized/state columns for high-speed ML feature indexing. The
-- authoritative full indicator map is still round-tripped via the
-- auxiliary_normalized_data JSON blob; these columns mirror it for querying.
ALTER TABLE market_snapshots ADD COLUMN stoch_k_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN stoch_k_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN stoch_d_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN stoch_d_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN chandemo_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN chandemo_state_label TEXT;

CREATE INDEX IF NOT EXISTS idx_snap_stoch_k ON market_snapshots (stoch_k_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_chandemo ON market_snapshots (chandemo_normalized);
