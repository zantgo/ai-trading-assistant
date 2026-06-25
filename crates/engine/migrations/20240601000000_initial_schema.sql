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

-- individual_indicator_logs
CREATE TABLE IF NOT EXISTS individual_indicator_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    master_record_id INTEGER NOT NULL,
    indicator_name TEXT NOT NULL,
    signal TEXT NOT NULL,
    reason TEXT NOT NULL,
    timeframe_secs INTEGER NOT NULL DEFAULT 60,
    timestamp INTEGER NOT NULL
);

-- master_assistant_records (with all added columns)
CREATE TABLE IF NOT EXISTS master_assistant_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT DEFAULT (datetime('now')),
    position TEXT NOT NULL,
    entry_price TEXT,
    price_at_analysis TEXT NOT NULL,
    general_trend TEXT NOT NULL,
    support_levels TEXT NOT NULL,
    resistance_levels TEXT NOT NULL,
    indicator_synthesis_summary TEXT NOT NULL,
    indicator_synthesis_evaluation TEXT NOT NULL,
    recommended_action TEXT NOT NULL,
    recommendation_rationale TEXT NOT NULL,
    symbol TEXT NOT NULL,
    trigger_type TEXT NOT NULL DEFAULT 'Manual',
    stop_loss_trigger TEXT,
    micro_term_signal TEXT,
    long_term_signal TEXT,
    score_points INTEGER NOT NULL DEFAULT 0,
    signals_json TEXT NOT NULL DEFAULT '{}',
    market_regime TEXT DEFAULT 'stable',
    portfolio_allocation_pct REAL DEFAULT 0.0
);

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

-- automated_performance_tracker
CREATE TABLE IF NOT EXISTS automated_performance_tracker (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    master_record_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    price_at_signal TEXT NOT NULL,
    price_at_1h TEXT,
    price_at_4h TEXT,
    price_at_24h TEXT,
    direction_correct_1h INTEGER,
    direction_correct_4h INTEGER,
    direction_correct_24h INTEGER,
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (master_record_id) REFERENCES master_assistant_records(id)
);

-- paper_balances (with all added columns)
CREATE TABLE IF NOT EXISTS paper_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    initial_usd REAL NOT NULL DEFAULT 10000.0,
    current_cash REAL NOT NULL DEFAULT 10000.0,
    allocation_pct REAL NOT NULL DEFAULT 10.0,
    auto_execute INTEGER NOT NULL DEFAULT 0,
    max_risk_pct REAL NOT NULL DEFAULT 2.0,
    leverage INTEGER NOT NULL DEFAULT 20,
    auto_execute_intervals INTEGER NOT NULL DEFAULT 15,
    lookback_trades INTEGER NOT NULL DEFAULT 10
);

-- active_positions (with all added columns)
CREATE TABLE IF NOT EXISTS active_positions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    direction TEXT NOT NULL,
    entry_price REAL NOT NULL,
    size REAL NOT NULL,
    allocated_usd REAL NOT NULL,
    entry_timestamp INTEGER NOT NULL,
    final_invalidation_level REAL,
    target_profit_ratio REAL DEFAULT 2.0,
    current_portions INTEGER DEFAULT 1,
    average_entry_price REAL
);

-- paper_trades
CREATE TABLE IF NOT EXISTS paper_trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL,
    entry_price REAL NOT NULL,
    exit_price REAL NOT NULL,
    size REAL NOT NULL,
    realized_pnl REAL NOT NULL,
    roi_pct REAL NOT NULL,
    entry_timestamp INTEGER NOT NULL,
    exit_timestamp INTEGER NOT NULL,
    trigger TEXT NOT NULL DEFAULT 'MANUAL'
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

-- trade_learning_journal
CREATE TABLE IF NOT EXISTS trade_learning_journal (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trade_id INTEGER NOT NULL,
    entry_date TEXT NOT NULL,
    exit_date TEXT NOT NULL,
    asset TEXT NOT NULL,
    direction TEXT NOT NULL,
    entry_reason TEXT NOT NULL,
    roe_percentage REAL NOT NULL DEFAULT 0.0,
    final_analysis TEXT NOT NULL DEFAULT '',
    execution_score REAL NOT NULL DEFAULT 5.0,
    human_notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (trade_id) REFERENCES trade_telemetry_history(id)
);

CREATE INDEX IF NOT EXISTS idx_journal_lookup ON trade_learning_journal (asset, execution_score DESC);

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

-- active_position_portions
CREATE TABLE IF NOT EXISTS active_position_portions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER,
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL,
    entry_price REAL NOT NULL,
    size REAL NOT NULL,
    allocated_usd REAL NOT NULL,
    portion_number INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (position_id) REFERENCES active_positions(id)
);

-- position_take_profit_targets
CREATE TABLE IF NOT EXISTS position_take_profit_targets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER,
    symbol TEXT NOT NULL,
    target_price REAL NOT NULL,
    size_fraction REAL NOT NULL,
    is_hit INTEGER NOT NULL DEFAULT 0,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (position_id) REFERENCES active_positions(id)
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

-- agent_thought_logs
CREATE TABLE IF NOT EXISTS agent_thought_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    master_record_id INTEGER NOT NULL,
    agent_name TEXT NOT NULL,
    thought_process TEXT NOT NULL,
    json_rpc_payload TEXT NOT NULL,
    confidence_score INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (master_record_id) REFERENCES master_assistant_records(id)
);

-- decision_memory_buffer
CREATE TABLE IF NOT EXISTS decision_memory_buffer (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    regime_classification TEXT NOT NULL,
    orchestrator_decision TEXT NOT NULL,
    confidence_score INTEGER NOT NULL,
    eight_factor_score INTEGER NOT NULL,
    portfolio_risk_pct REAL NOT NULL
);
