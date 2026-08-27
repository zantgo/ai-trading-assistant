-- v7 TAE: open-state persistence for restart recovery.
-- One row per instance: the tracked setup, pending/bracket orders, the open
-- position, and the equity ledger at the last graceful shutdown. On boot the
-- executor restores the row (subject to staleness) instead of losing the
-- trader's account and positions.
CREATE TABLE IF NOT EXISTS tae_open_state (
    instance_id TEXT PRIMARY KEY,
    symbol TEXT NOT NULL,
    saved_at_ms INTEGER NOT NULL,
    setup_fingerprint TEXT NOT NULL DEFAULT '',
    tracked_setup_json TEXT NOT NULL DEFAULT '',
    entry_order_json TEXT NOT NULL DEFAULT '',
    bracket_tp_json TEXT NOT NULL DEFAULT '',
    bracket_sl_json TEXT NOT NULL DEFAULT '',
    position_json TEXT NOT NULL DEFAULT '',
    equity TEXT NOT NULL DEFAULT '',
    realized_pnl TEXT NOT NULL DEFAULT ''
);
