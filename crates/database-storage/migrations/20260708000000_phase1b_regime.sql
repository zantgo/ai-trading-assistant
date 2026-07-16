-- Migration: Add Phase 1B market-regime indicator columns to market_snapshots
-- (Aroon, Choppiness Index, Linear Regression Slope, Z-Score).
ALTER TABLE market_snapshots ADD COLUMN aroon_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN aroon_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN choppiness_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN choppiness_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN linreg_slope_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN linreg_slope_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN zscore_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN zscore_state_label TEXT;

CREATE INDEX IF NOT EXISTS idx_snap_aroon ON market_snapshots (aroon_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_zscore ON market_snapshots (zscore_normalized);
