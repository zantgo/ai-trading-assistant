-- Migration: Fractional Normalized Indicator States (v2.0)
-- Adds dual-representation persistence to market_snapshots using a hybrid
-- model: dedicated REAL/TEXT columns for the 8 primary scored indicators
-- (fast ML feature-vector querying), dedicated resting-level columns for the
-- Fibonacci boundaries, and a single auxiliary JSON blob for the remaining
-- indicator metadata (multi-line raw series + low-priority indicators).

-- ── Primary 8 scored indicators: normalized [-1.0,1.0] + level state label ──
ALTER TABLE market_snapshots ADD COLUMN rsi_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN rsi_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN macd_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN macd_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN squeeze_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN squeeze_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN adx_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN adx_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN bbwp_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN bbwp_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN rvol_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN rvol_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN ema_stack_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN ema_stack_state_label TEXT;
ALTER TABLE market_snapshots ADD COLUMN vwap_normalized REAL;
ALTER TABLE market_snapshots ADD COLUMN vwap_state_label TEXT;

-- ── Resting horizontal level columns (Fibonacci structure) ──
ALTER TABLE market_snapshots ADD COLUMN fib_GP_top REAL;
ALTER TABLE market_snapshots ADD COLUMN fib_GP_bottom REAL;
ALTER TABLE market_snapshots ADD COLUMN fib_ext_1618 REAL;
ALTER TABLE market_snapshots ADD COLUMN fib_ext_2618 REAL;

-- ── Auxiliary catch-all JSON (remaining indicators + multi-line raw series) ──
ALTER TABLE market_snapshots ADD COLUMN auxiliary_normalized_data TEXT;

-- ── Indexes for high-speed ML feature selection on the primary 8 ──
CREATE INDEX IF NOT EXISTS idx_snap_rsi_norm ON market_snapshots (rsi_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_rsi_label ON market_snapshots (rsi_state_label);
CREATE INDEX IF NOT EXISTS idx_snap_macd_norm ON market_snapshots (macd_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_macd_label ON market_snapshots (macd_state_label);
CREATE INDEX IF NOT EXISTS idx_snap_squeeze_norm ON market_snapshots (squeeze_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_squeeze_label ON market_snapshots (squeeze_state_label);
CREATE INDEX IF NOT EXISTS idx_snap_adx_norm ON market_snapshots (adx_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_adx_label ON market_snapshots (adx_state_label);
CREATE INDEX IF NOT EXISTS idx_snap_bbwp_norm ON market_snapshots (bbwp_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_bbwp_label ON market_snapshots (bbwp_state_label);
CREATE INDEX IF NOT EXISTS idx_snap_rvol_norm ON market_snapshots (rvol_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_rvol_label ON market_snapshots (rvol_state_label);
CREATE INDEX IF NOT EXISTS idx_snap_ema_stack_norm ON market_snapshots (ema_stack_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_ema_stack_label ON market_snapshots (ema_stack_state_label);
CREATE INDEX IF NOT EXISTS idx_snap_vwap_norm ON market_snapshots (vwap_normalized);
CREATE INDEX IF NOT EXISTS idx_snap_vwap_label ON market_snapshots (vwap_state_label);
