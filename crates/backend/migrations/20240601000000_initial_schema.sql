-- Migration 001: Initial schema with all tables and columns (final form).
-- Consolidates all CREATE TABLE IF NOT EXISTS and ALTER TABLE ADD COLUMN
-- from the previous ad-hoc migration approach into a single versioned migration.

-- market_snapshots (with all added columns)
CREATE TABLE IF NOT EXISTS market_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange TEXT NOT NULL DEFAULT 'Hyperliquid',
    symbol TEXT NOT NULL,
    timeframe_secs INTEGER NOT NULL DEFAULT 60,
    timestamp INTEGER NOT NULL,
    mid_price TEXT NOT NULL,
    bid_price TEXT NOT NULL,
    ask_price TEXT NOT NULL,
    open TEXT,
    high TEXT,
    low TEXT,
    close TEXT,
    volume TEXT,
    average_volume TEXT,
    bb_upper TEXT,
    bb_middle TEXT,
    bb_lower TEXT,
    atr_14 TEXT,
    vwap TEXT,
    ema_fast TEXT,
    ema_medium TEXT,
    ema_slow TEXT,
    ema_long TEXT,
    rsi_14 TEXT,
    macd_line TEXT,
    macd_signal TEXT,
    macd_hist TEXT,
    adx_14 TEXT,
    adx_plus TEXT,
    adx_minus TEXT,
    squeeze_on INTEGER,
    squeeze_momentum TEXT,
    bbwp TEXT,
    support_levels TEXT,
    resistance_levels TEXT
);

CREATE INDEX IF NOT EXISTS idx_snapshots_lookup ON market_snapshots (symbol, timeframe_secs, timestamp DESC);



-- user_trades
CREATE TABLE IF NOT EXISTS user_trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL,
    outcome TEXT NOT NULL,
    risk_multiplier REAL NOT NULL,
    reward_multiplier REAL NOT NULL
);

-- exchange_keys
CREATE TABLE IF NOT EXISTS exchange_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange TEXT NOT NULL,
    account_name TEXT NOT NULL,
    api_key TEXT NOT NULL,
    api_secret TEXT NOT NULL,
    passphrase TEXT NOT NULL DEFAULT '',
    referred_uid TEXT NOT NULL DEFAULT '',
    is_active INTEGER NOT NULL DEFAULT 0,
    last_sync_timestamp INTEGER
);

-- decision_profiles
CREATE TABLE IF NOT EXISTS decision_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_name TEXT NOT NULL UNIQUE,
    long_threshold INTEGER NOT NULL DEFAULT 40,
    short_threshold INTEGER NOT NULL DEFAULT -40
);

-- profile_indicators
CREATE TABLE IF NOT EXISTS profile_indicators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id INTEGER NOT NULL,
    indicator_name TEXT NOT NULL,
    weight INTEGER NOT NULL DEFAULT 10,
    override_status TEXT NOT NULL DEFAULT 'NONE',
    FOREIGN KEY (profile_id) REFERENCES decision_profiles(id) ON DELETE CASCADE
);

-- trade_telemetry_history
CREATE TABLE IF NOT EXISTS trade_telemetry_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange TEXT NOT NULL DEFAULT 'Hyperliquid',
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL,
    entry_timestamp INTEGER NOT NULL,
    exit_timestamp INTEGER NOT NULL,
    entry_price REAL NOT NULL,
    exit_price REAL NOT NULL,
    size REAL NOT NULL,
    commission_fees REAL NOT NULL DEFAULT 0.0,
    funding_fees REAL NOT NULL DEFAULT 0.0,
    realized_pnl REAL NOT NULL,
    roi_percentage REAL NOT NULL DEFAULT 0.0,
    trigger_source TEXT NOT NULL DEFAULT 'MANUAL'
);

-- risk_profiles
CREATE TABLE IF NOT EXISTS risk_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_name TEXT NOT NULL UNIQUE,
    capital REAL NOT NULL DEFAULT 1000.0,
    max_risk_pct REAL NOT NULL DEFAULT 2.0,
    leverage INTEGER NOT NULL DEFAULT 20,
    commission_pct REAL NOT NULL DEFAULT 0.06,
    funding_rate_8h REAL NOT NULL DEFAULT 0.0,
    spread REAL NOT NULL DEFAULT 0.0
);

-- support_resistance_levels
CREATE TABLE IF NOT EXISTS support_resistance_levels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    s1 REAL,
    s2 REAL,
    s3 REAL,
    r1 REAL,
    r2 REAL,
    r3 REAL,
    calculated_at INTEGER NOT NULL
);

-- (AI tables: master_assistant_records, automated_performance_tracker,
--  agent_thought_logs, decision_memory_buffer — removed; superseded by
--  2026XXXX_drop_legacy_ai.sql for existing databases.)
