CREATE TABLE IF NOT EXISTS connection_quality_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_ms INTEGER NOT NULL,
    window TEXT NOT NULL,
    uptime_pct REAL NOT NULL,
    disconnect_count INTEGER NOT NULL,
    avg_reconnect_ms REAL NOT NULL,
    total_data_loss_secs INTEGER NOT NULL,
    reconstructed_candles INTEGER NOT NULL,
    score REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cq_window_time
    ON connection_quality_samples(window, timestamp_ms);
