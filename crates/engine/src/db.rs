use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use shared::models::MarketSnapshot;
use shared::TriggerType;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UserTrade {
    pub id: i64,
    pub timestamp: i64,
    pub symbol: String,
    pub direction: String,
    pub outcome: String,
    pub risk_multiplier: f64,
    pub reward_multiplier: f64,
}

#[derive(Debug)]
pub enum TelemetryMsg {
    InsertSnapshot(MarketSnapshot),
    InsertIndividualLog {
        master_record_id: i64,
        indicator_name: String,
        signal: String,
        reason: String,
    },
    UpdateMasterRecord {
        master_id: i64,
        general_trend: String,
        support_levels: String,
        resistance_levels: String,
        indicator_synthesis_summary: String,
        indicator_synthesis_evaluation: String,
        recommended_action: String,
        recommendation_rationale: String,
    },
    ConsoleLog(String),
    PaperOpenPosition {
        symbol: String,
        direction: String,
        entry_price: f64,
        size: f64,
        allocated_usd: f64,
    },
    PaperClosePosition {
        symbol: String,
        exit_price: f64,
        exit_timestamp: i64,
        trigger: String,
    },
    PaperUpdateBalance {
        symbol: String,
        current_cash: f64,
    },
}

pub async fn run_telemetry_logger(pool: SqlitePool, mut rx: tokio::sync::mpsc::Receiver<TelemetryMsg>) {
    println!("📝 Telemetry & Logging Worker: Background log thread running.");
    while let Some(msg) = rx.recv().await {
        match msg {
            TelemetryMsg::InsertSnapshot(snapshot) => {
                insert_snapshot_internal(&pool, &snapshot).await;
            }
            TelemetryMsg::InsertIndividualLog { master_record_id, indicator_name, signal, reason } => {
                insert_individual_log_internal(&pool, master_record_id, &indicator_name, &signal, &reason).await;
            }
            TelemetryMsg::UpdateMasterRecord {
                master_id,
                general_trend,
                support_levels,
                resistance_levels,
                indicator_synthesis_summary,
                indicator_synthesis_evaluation,
                recommended_action,
                recommendation_rationale,
            } => {
                update_master_record_internal(
                    &pool,
                    master_id,
                    &general_trend,
                    &support_levels,
                    &resistance_levels,
                    &indicator_synthesis_summary,
                    &indicator_synthesis_evaluation,
                    &recommended_action,
                    &recommendation_rationale,
                ).await;
            }
            TelemetryMsg::ConsoleLog(log_text) => {
                println!("{}", log_text);
            }
            TelemetryMsg::PaperOpenPosition { symbol, direction, entry_price, size, allocated_usd } => {
                paper_open_position_internal(&pool, &symbol, &direction, entry_price, size, allocated_usd).await;
            }
            TelemetryMsg::PaperClosePosition { symbol, exit_price, exit_timestamp, trigger } => {
                paper_close_position_internal(&pool, &symbol, exit_price, exit_timestamp, &trigger).await;
            }
            TelemetryMsg::PaperUpdateBalance { symbol, current_cash } => {
                paper_update_balance_internal(&pool, &symbol, current_cash).await;
            }
        }
    }
}

pub async fn init_db() -> SqlitePool {
    let db_options = SqliteConnectOptions::new()
        .filename("telemetry.db")
        .create_if_missing(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePool::connect_with(db_options)
        .await
        .expect("❌ Database Setup: Failed to initialize SQLite database pool");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS market_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exchange TEXT NOT NULL DEFAULT 'Hyperliquid',
            symbol TEXT NOT NULL,
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
            squeeze_momentum TEXT
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build schema table");

    sqlx::query(
        "ALTER TABLE market_snapshots ADD COLUMN exchange TEXT NOT NULL DEFAULT 'Hyperliquid'"
    )
    .execute(&pool)
    .await
    .ok();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS individual_indicator_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            master_record_id INTEGER NOT NULL,
            indicator_name TEXT NOT NULL,
            signal TEXT NOT NULL,
            reason TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build individual_indicator_logs table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS master_assistant_records (
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
            symbol TEXT NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build master_assistant_records table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            symbol TEXT NOT NULL,
            direction TEXT NOT NULL,
            outcome TEXT NOT NULL,
            risk_multiplier REAL NOT NULL,
            reward_multiplier REAL NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build user_trades table");

    sqlx::query(
        "ALTER TABLE master_assistant_records ADD COLUMN trigger_type TEXT NOT NULL DEFAULT 'Manual'"
    )
    .execute(&pool)
    .await
    .ok();

    sqlx::query(
        "ALTER TABLE master_assistant_records ADD COLUMN stop_loss_trigger TEXT"
    )
    .execute(&pool)
    .await
    .ok();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS automated_performance_tracker (
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
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build automated_performance_tracker table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS paper_balances (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT NOT NULL UNIQUE,
            initial_usd REAL NOT NULL DEFAULT 10000.0,
            current_cash REAL NOT NULL DEFAULT 10000.0,
            allocation_pct REAL NOT NULL DEFAULT 10.0,
            auto_execute INTEGER NOT NULL DEFAULT 0
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build paper_balances table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS active_positions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT NOT NULL UNIQUE,
            direction TEXT NOT NULL,
            entry_price REAL NOT NULL,
            size REAL NOT NULL,
            allocated_usd REAL NOT NULL,
            entry_timestamp INTEGER NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build active_positions table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS paper_trades (
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
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build paper_trades table");

    // ─── Exchange Accounts ────────────────────────────────────────

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS exchange_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exchange TEXT NOT NULL,
            account_name TEXT NOT NULL,
            api_key TEXT NOT NULL,
            api_secret TEXT NOT NULL,
            passphrase TEXT NOT NULL DEFAULT '',
            referred_uid TEXT NOT NULL DEFAULT '',
            is_active INTEGER NOT NULL DEFAULT 0,
            last_sync_timestamp INTEGER
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build exchange_keys table");

    // ─── Decision Profiles ─────────────────────────────────────────

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS decision_profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            profile_name TEXT NOT NULL UNIQUE,
            long_threshold INTEGER NOT NULL DEFAULT 40,
            short_threshold INTEGER NOT NULL DEFAULT -40
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build decision_profiles table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS profile_indicators (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            profile_id INTEGER NOT NULL,
            indicator_name TEXT NOT NULL,
            weight INTEGER NOT NULL DEFAULT 10,
            override_status TEXT NOT NULL DEFAULT 'NONE',
            FOREIGN KEY (profile_id) REFERENCES decision_profiles(id) ON DELETE CASCADE
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build profile_indicators table");

    // ─── Trade Telemetry ───────────────────────────────────────────

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS trade_telemetry_history (
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
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build trade_telemetry_history table");

    // ─── Risk Profiles ─────────────────────────────────────────────

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS risk_profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            profile_name TEXT NOT NULL UNIQUE,
            capital REAL NOT NULL DEFAULT 1000.0,
            max_risk_pct REAL NOT NULL DEFAULT 2.0,
            leverage INTEGER NOT NULL DEFAULT 20,
            commission_pct REAL NOT NULL DEFAULT 0.06,
            funding_rate_8h REAL NOT NULL DEFAULT 0.0,
            spread REAL NOT NULL DEFAULT 0.0
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build risk_profiles table");

    // Seed default profiles if tables are empty
    seed_default_profiles(&pool).await;

    pool
}

async fn insert_snapshot_internal(pool: &SqlitePool, snapshot: &MarketSnapshot) {
    let sqz_on_db_val = snapshot.squeeze_on.map(|s| if s { 1 } else { 0 });
    let exchange_label = snapshot.exchange.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Hyperliquid".to_string());

    if let Err(e) = sqlx::query(
        "INSERT INTO market_snapshots (
            exchange, timestamp, symbol, mid_price, bid_price, ask_price,
            open, high, low, close, volume, average_volume,
            bb_upper, bb_middle, bb_lower, atr_14, vwap,
            ema_fast, ema_medium, ema_slow, ema_long, rsi_14,
            macd_line, macd_signal, macd_hist, adx_14, adx_plus, adx_minus,
            squeeze_on, squeeze_momentum
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30)"
    )
    .bind(exchange_label)
    .bind(snapshot.timestamp as i64)
    .bind(&snapshot.symbol)
    .bind(snapshot.mid_price.to_string())
    .bind(snapshot.bid_price.to_string())
    .bind(snapshot.ask_price.to_string())
    .bind(snapshot.open.map(|d| d.to_string()))
    .bind(snapshot.high.map(|d| d.to_string()))
    .bind(snapshot.low.map(|d| d.to_string()))
    .bind(snapshot.close.map(|d| d.to_string()))
    .bind(snapshot.volume.map(|d| d.to_string()))
    .bind(snapshot.average_volume.map(|d| d.to_string()))
    .bind(snapshot.bb_upper.map(|d| d.to_string()))
    .bind(snapshot.bb_middle.map(|d| d.to_string()))
    .bind(snapshot.bb_lower.map(|d| d.to_string()))
    .bind(snapshot.atr_14.map(|d| d.to_string()))
    .bind(snapshot.vwap.map(|d| d.to_string()))
    .bind(snapshot.ema_fast.map(|d| d.to_string()))
    .bind(snapshot.ema_medium.map(|d| d.to_string()))
    .bind(snapshot.ema_slow.map(|d| d.to_string()))
    .bind(snapshot.ema_long.map(|d| d.to_string()))
    .bind(snapshot.rsi_14.map(|d| d.to_string()))
    .bind(snapshot.macd_line.map(|d| d.to_string()))
    .bind(snapshot.macd_signal.map(|d| d.to_string()))
    .bind(snapshot.macd_hist.map(|d| d.to_string()))
    .bind(snapshot.adx_14.map(|d| d.to_string()))
    .bind(snapshot.adx_plus.map(|d| d.to_string()))
    .bind(snapshot.adx_minus.map(|d| d.to_string()))
    .bind(sqz_on_db_val)
    .bind(snapshot.squeeze_momentum.map(|d| d.to_string()))
    .execute(&*pool)
    .await
    {
        eprintln!("⚠️ Database Error: Failed to save completed snapshot: {}", e);
    }
}

pub async fn insert_master_placeholder(
    pool: &SqlitePool,
    position: &str,
    entry_price: &str,
    price_at_analysis: &str,
    symbol: &str,
    trigger_type: TriggerType,
) -> i64 {
    let trigger_str = trigger_type.to_string();
    match sqlx::query(
        "INSERT INTO master_assistant_records (
            position, entry_price, price_at_analysis, general_trend,
            support_levels, resistance_levels,
            indicator_synthesis_summary, indicator_synthesis_evaluation,
            recommended_action, recommendation_rationale, symbol, trigger_type
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
    )
    .bind(position)
    .bind(entry_price)
    .bind(price_at_analysis)
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind(symbol)
    .bind(&trigger_str)
    .execute(&*pool)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to insert master placeholder: {}", e);
            0
        }
    }
}

async fn insert_individual_log_internal(
    pool: &SqlitePool,
    master_record_id: i64,
    indicator_name: &str,
    signal: &str,
    reason: &str,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if let Err(e) = sqlx::query(
        "INSERT INTO individual_indicator_logs (
            master_record_id, indicator_name, signal, reason, timestamp
        ) VALUES (?1, ?2, ?3, ?4, ?5)"
    )
    .bind(master_record_id)
    .bind(indicator_name)
    .bind(signal)
    .bind(reason)
    .bind(now)
    .execute(&*pool)
    .await
    {
        eprintln!(
            "⚠️ Database Error: Failed to save individual indicator log for {}: {}",
            indicator_name, e
        );
    }
}

async fn update_master_record_internal(
    pool: &SqlitePool,
    master_id: i64,
    general_trend: &str,
    support_levels: &str,
    resistance_levels: &str,
    indicator_synthesis_summary: &str,
    indicator_synthesis_evaluation: &str,
    recommended_action: &str,
    recommendation_rationale: &str,
) {
    if let Err(e) = sqlx::query(
        "UPDATE master_assistant_records SET
            general_trend = ?2,
            support_levels = ?3,
            resistance_levels = ?4,
            indicator_synthesis_summary = ?5,
            indicator_synthesis_evaluation = ?6,
            recommended_action = ?7,
            recommendation_rationale = ?8
        WHERE id = ?1"
    )
    .bind(master_id)
    .bind(general_trend)
    .bind(support_levels)
    .bind(resistance_levels)
    .bind(indicator_synthesis_summary)
    .bind(indicator_synthesis_evaluation)
    .bind(recommended_action)
    .bind(recommendation_rationale)
    .execute(&*pool)
    .await
    {
        eprintln!("⚠️ Database Error: Failed to update master record {}: {}", master_id, e);
    }
}

pub struct MasterRecord {
    pub id: i64,
    pub created_at: String,
    pub position: String,
    pub entry_price: Option<String>,
    pub price_at_analysis: String,
    pub general_trend: String,
    pub support_levels: String,
    pub resistance_levels: String,
    pub indicator_synthesis_summary: String,
    #[allow(dead_code)]
    pub indicator_synthesis_evaluation: String,
    pub recommended_action: String,
    pub recommendation_rationale: String,
    pub symbol: String,
    pub trigger_type: String,
}

pub async fn query_master_records(pool: &SqlitePool, limit: u32) -> Vec<MasterRecord> {
    let rows = sqlx::query_as::<_, (i64, String, String, Option<String>, String, String, String, String, String, String, String, String, String, String)>(
        "SELECT id, created_at, position, entry_price, price_at_analysis,
                general_trend, support_levels, resistance_levels,
                indicator_synthesis_summary, indicator_synthesis_evaluation,
                recommended_action, recommendation_rationale, symbol, trigger_type
         FROM master_assistant_records
         WHERE general_trend != 'PENDING'
         ORDER BY id DESC
         LIMIT ?1"
    )
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await;

    match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|(id, created_at, position, entry_price, price_at_analysis, general_trend, support_levels, resistance_levels, indicator_synthesis_summary, indicator_synthesis_evaluation, recommended_action, recommendation_rationale, symbol, trigger_type)| {
                MasterRecord {
                    id,
                    created_at,
                    position,
                    entry_price,
                    price_at_analysis,
                    general_trend,
                    support_levels,
                    resistance_levels,
                    indicator_synthesis_summary,
                    indicator_synthesis_evaluation,
                    recommended_action,
                    recommendation_rationale,
                    symbol,
                    trigger_type,
                }
            })
            .collect(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to query master records: {}", e);
            vec![]
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndicatorSnapshotRow {
    pub timestamp: i64,
    pub rsi_14: Option<String>,
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<String>,
    pub macd_line: Option<String>,
    pub macd_signal: Option<String>,
    pub macd_hist: Option<String>,
    pub adx_14: Option<String>,
    pub adx_plus: Option<String>,
    pub adx_minus: Option<String>,
    pub atr_14: Option<String>,
    pub bb_upper: Option<String>,
    pub bb_middle: Option<String>,
    pub bb_lower: Option<String>,
    pub ema_fast: Option<String>,
    pub ema_medium: Option<String>,
    pub ema_slow: Option<String>,
    pub ema_long: Option<String>,
    pub average_volume: Option<String>,
}

pub async fn query_indicator_snapshots(
    pool: &SqlitePool,
    symbol: &str,
    limit: u32,
) -> Vec<IndicatorSnapshotRow> {
    let rows = sqlx::query(
        "SELECT timestamp, rsi_14, squeeze_on, squeeze_momentum,
                macd_line, macd_signal, macd_hist,
                adx_14, adx_plus, adx_minus,
                atr_14, bb_upper, bb_middle, bb_lower,
                ema_fast, ema_medium, ema_slow, ema_long,
                average_volume
         FROM market_snapshots
         WHERE symbol = ?1
           AND close IS NOT NULL
         ORDER BY timestamp ASC
         LIMIT ?2"
    )
    .bind(symbol)
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await;

    match rows {
        Ok(rows) => rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                IndicatorSnapshotRow {
                    timestamp: row.get(0),
                    rsi_14: row.get(1),
                    squeeze_on: row.get(2),
                    squeeze_momentum: row.get(3),
                    macd_line: row.get(4),
                    macd_signal: row.get(5),
                    macd_hist: row.get(6),
                    adx_14: row.get(7),
                    adx_plus: row.get(8),
                    adx_minus: row.get(9),
                    atr_14: row.get(10),
                    bb_upper: row.get(11),
                    bb_middle: row.get(12),
                    bb_lower: row.get(13),
                    ema_fast: row.get(14),
                    ema_medium: row.get(15),
                    ema_slow: row.get(16),
                    ema_long: row.get(17),
                    average_volume: row.get(18),
                }
            })
            .collect(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to query indicator snapshots: {}", e);
            vec![]
        }
    }
}

pub async fn query_atr_snapshots(pool: &SqlitePool, limit: u32) -> Vec<Option<String>> {
    let rows = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT atr_14 FROM market_snapshots
         WHERE atr_14 IS NOT NULL
         ORDER BY id DESC
         LIMIT ?1"
    )
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await;

    match rows {
        Ok(rows) => rows.into_iter().map(|(atr,)| atr).collect(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to query ATR snapshots: {}", e);
            vec![]
        }
    }
}

pub async fn query_master_records_by_trigger(
    pool: &SqlitePool,
    trigger_type: &str,
    limit: u32,
) -> Vec<MasterRecord> {
    let rows = sqlx::query_as::<_, (i64, String, String, Option<String>, String, String, String, String, String, String, String, String, String, String)>(
        "SELECT id, created_at, position, entry_price, price_at_analysis,
                general_trend, support_levels, resistance_levels,
                indicator_synthesis_summary, indicator_synthesis_evaluation,
                recommended_action, recommendation_rationale, symbol, trigger_type
         FROM master_assistant_records
         WHERE general_trend != 'PENDING' AND trigger_type = ?1
         ORDER BY id DESC
         LIMIT ?2"
    )
    .bind(trigger_type)
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await;

    match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|(id, created_at, position, entry_price, price_at_analysis, general_trend, support_levels, resistance_levels, indicator_synthesis_summary, indicator_synthesis_evaluation, recommended_action, recommendation_rationale, symbol, trigger_type)| {
                MasterRecord {
                    id,
                    created_at,
                    position,
                    entry_price,
                    price_at_analysis,
                    general_trend,
                    support_levels,
                    resistance_levels,
                    indicator_synthesis_summary,
                    indicator_synthesis_evaluation,
                    recommended_action,
                    recommendation_rationale,
                    symbol,
                    trigger_type,
                }
            })
            .collect(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to query master records by trigger: {}", e);
            vec![]
        }
    }
}

pub async fn query_latest_snapshot(
    pool: &SqlitePool,
    symbol: &str,
) -> Option<MarketSnapshot> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT exchange, timestamp, symbol, mid_price, bid_price, ask_price,
                open, high, low, close, volume, average_volume,
                bb_upper, bb_middle, bb_lower, atr_14, vwap,
                ema_fast, ema_medium, ema_slow, ema_long, rsi_14,
                macd_line, macd_signal, macd_hist, adx_14, adx_plus, adx_minus,
                squeeze_on, squeeze_momentum
         FROM market_snapshots
         WHERE symbol = ?1 AND close IS NOT NULL
         ORDER BY id DESC
         LIMIT 1"
    )
    .bind(symbol)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten();

    row.map(|r| {
        let parse_dec = |val: Option<String>| val.and_then(|s| rust_decimal::Decimal::from_str_exact(&s).ok());
        MarketSnapshot {
            exchange: Some(shared::normalized::Exchange::Hyperliquid),
            timestamp: r.get::<i64, _>(1) as u64,
            symbol: r.get(2),
            mid_price: parse_dec(Some(r.get::<String, _>(3))).unwrap_or(rust_decimal::Decimal::ZERO),
            bid_price: parse_dec(Some(r.get::<String, _>(4))).unwrap_or(rust_decimal::Decimal::ZERO),
            ask_price: parse_dec(Some(r.get::<String, _>(5))).unwrap_or(rust_decimal::Decimal::ZERO),
            bid_size: None,
            ask_size: None,
            funding_rate: None,
            open: parse_dec(r.get::<Option<String>, _>(6)),
            high: parse_dec(r.get::<Option<String>, _>(7)),
            low: parse_dec(r.get::<Option<String>, _>(8)),
            close: parse_dec(r.get::<Option<String>, _>(9)),
            volume: parse_dec(r.get::<Option<String>, _>(10)),
            average_volume: parse_dec(r.get::<Option<String>, _>(11)),
            bb_upper: parse_dec(r.get::<Option<String>, _>(12)),
            bb_middle: parse_dec(r.get::<Option<String>, _>(13)),
            bb_lower: parse_dec(r.get::<Option<String>, _>(14)),
            atr_14: parse_dec(r.get::<Option<String>, _>(15)),
            vwap: parse_dec(r.get::<Option<String>, _>(16)),
            ema_fast: parse_dec(r.get::<Option<String>, _>(17)),
            ema_medium: parse_dec(r.get::<Option<String>, _>(18)),
            ema_slow: parse_dec(r.get::<Option<String>, _>(19)),
            ema_long: parse_dec(r.get::<Option<String>, _>(20)),
            rsi_14: parse_dec(r.get::<Option<String>, _>(21)),
            macd_line: parse_dec(r.get::<Option<String>, _>(22)),
            macd_signal: parse_dec(r.get::<Option<String>, _>(23)),
            macd_hist: parse_dec(r.get::<Option<String>, _>(24)),
            adx_14: parse_dec(r.get::<Option<String>, _>(25)),
            adx_plus: parse_dec(r.get::<Option<String>, _>(26)),
            adx_minus: parse_dec(r.get::<Option<String>, _>(27)),
            squeeze_on: r.get::<Option<i32>, _>(28).map(|v| v != 0),
            squeeze_momentum: parse_dec(r.get::<Option<String>, _>(29)),
        }
    })
}

pub async fn insert_automated_performance_baseline(
    pool: &SqlitePool,
    master_record_id: i64,
    symbol: &str,
    price_at_signal: &str,
) {
    if let Err(e) = sqlx::query(
        "INSERT INTO automated_performance_tracker (master_record_id, symbol, price_at_signal)
         VALUES (?1, ?2, ?3)"
    )
    .bind(master_record_id)
    .bind(symbol)
    .bind(price_at_signal)
    .execute(&*pool)
    .await
    {
        eprintln!("⚠️ Database Error: Failed to insert automated performance baseline: {}", e);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AutomatedPerformanceRow {
    pub id: i64,
    pub master_record_id: i64,
    pub symbol: String,
    pub price_at_signal: String,
    pub price_at_1h: Option<String>,
    pub price_at_4h: Option<String>,
    pub price_at_24h: Option<String>,
    pub direction_correct_1h: Option<bool>,
    pub direction_correct_4h: Option<bool>,
    pub direction_correct_24h: Option<bool>,
    pub created_at: String,
}

pub async fn query_automated_performance(
    pool: &SqlitePool,
    limit: u32,
) -> Vec<AutomatedPerformanceRow> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, master_record_id, symbol, price_at_signal,
                price_at_1h, price_at_4h, price_at_24h,
                direction_correct_1h, direction_correct_4h, direction_correct_24h,
                created_at
         FROM automated_performance_tracker
         ORDER BY id DESC
         LIMIT ?1"
    )
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await;

    match rows {
        Ok(rows) => rows
            .iter()
            .map(|r| AutomatedPerformanceRow {
                id: r.get(0),
                master_record_id: r.get(1),
                symbol: r.get(2),
                price_at_signal: r.get(3),
                price_at_1h: r.get(4),
                price_at_4h: r.get(5),
                price_at_24h: r.get(6),
                direction_correct_1h: r.get::<Option<i32>, _>(7).map(|v| v != 0),
                direction_correct_4h: r.get::<Option<i32>, _>(8).map(|v| v != 0),
                direction_correct_24h: r.get::<Option<i32>, _>(9).map(|v| v != 0),
                created_at: r.get(10),
            })
            .collect(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to query automated performance: {}", e);
            vec![]
        }
    }
}

pub async fn update_performance_tracker_prices(
    pool: &SqlitePool,
    tracker_id: i64,
    price_at_1h: Option<&str>,
    direction_correct_1h: Option<bool>,
    price_at_4h: Option<&str>,
    direction_correct_4h: Option<bool>,
    price_at_24h: Option<&str>,
    direction_correct_24h: Option<bool>,
) {
    let corr_1h = direction_correct_1h.map(|v| if v { 1 } else { 0 });
    let corr_4h = direction_correct_4h.map(|v| if v { 1 } else { 0 });
    let corr_24h = direction_correct_24h.map(|v| if v { 1 } else { 0 });

    if let Err(e) = sqlx::query(
        "UPDATE automated_performance_tracker SET
            price_at_1h = COALESCE(?2, price_at_1h),
            direction_correct_1h = COALESCE(?3, direction_correct_1h),
            price_at_4h = COALESCE(?4, price_at_4h),
            direction_correct_4h = COALESCE(?5, direction_correct_4h),
            price_at_24h = COALESCE(?6, price_at_24h),
            direction_correct_24h = COALESCE(?7, direction_correct_24h)
         WHERE id = ?1"
    )
    .bind(tracker_id)
    .bind(price_at_1h)
    .bind(corr_1h)
    .bind(price_at_4h)
    .bind(corr_4h)
    .bind(price_at_24h)
    .bind(corr_24h)
    .execute(&*pool)
    .await
    {
        eprintln!("⚠️ Database Error: Failed to update performance tracker {}: {}", tracker_id, e);
    }
}

pub async fn query_pending_performance_entries(
    pool: &SqlitePool,
) -> Vec<AutomatedPerformanceRow> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, master_record_id, symbol, price_at_signal,
                price_at_1h, price_at_4h, price_at_24h,
                direction_correct_1h, direction_correct_4h, direction_correct_24h,
                created_at
         FROM automated_performance_tracker
         WHERE price_at_24h IS NULL
         ORDER BY id ASC"
    )
    .fetch_all(&*pool)
    .await;

    match rows {
        Ok(rows) => rows
            .iter()
            .map(|r| AutomatedPerformanceRow {
                id: r.get(0),
                master_record_id: r.get(1),
                symbol: r.get(2),
                price_at_signal: r.get(3),
                price_at_1h: r.get(4),
                price_at_4h: r.get(5),
                price_at_24h: r.get(6),
                direction_correct_1h: r.get::<Option<i32>, _>(7).map(|v| v != 0),
                direction_correct_4h: r.get::<Option<i32>, _>(8).map(|v| v != 0),
                direction_correct_24h: r.get::<Option<i32>, _>(9).map(|v| v != 0),
                created_at: r.get(10),
            })
            .collect(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to query pending performance entries: {}", e);
            vec![]
        }
    }
}

pub async fn query_master_action_by_id(pool: &SqlitePool, master_id: i64) -> Option<String> {
    use sqlx::Row;
    sqlx::query(
        "SELECT recommended_action FROM master_assistant_records WHERE id = ?1"
    )
    .bind(master_id)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten()
    .map(|r| r.get(0))
}

pub async fn query_closest_close_price(
    pool: &SqlitePool,
    symbol: &str,
    target_timestamp_secs: u64,
) -> Option<f64> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT close FROM market_snapshots
         WHERE symbol = ?1 AND close IS NOT NULL AND timestamp >= ?2
         ORDER BY timestamp ASC LIMIT 1"
    )
    .bind(symbol)
    .bind(target_timestamp_secs as i64)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten();

    match row {
        Some(r) => {
            let s: String = r.get(0);
            s.parse::<f64>().ok()
        }
        None => {
            let fallback = sqlx::query(
                "SELECT close FROM market_snapshots
                 WHERE symbol = ?1 AND close IS NOT NULL AND timestamp <= ?2
                 ORDER BY timestamp DESC LIMIT 1"
            )
            .bind(symbol)
            .bind(target_timestamp_secs as i64)
            .fetch_optional(&*pool)
            .await
            .ok()
            .flatten();
            fallback.and_then(|r: sqlx::sqlite::SqliteRow| {
                let s: String = r.get(0);
                s.parse::<f64>().ok()
            })
        }
    }
}

pub async fn insert_user_trade(
    pool: &SqlitePool,
    symbol: &str,
    direction: &str,
    outcome: &str,
    risk: f64,
    reward: f64,
) -> Result<i64, sqlx::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let res = sqlx::query(
        "INSERT INTO user_trades (timestamp, symbol, direction, outcome, risk_multiplier, reward_multiplier)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )
    .bind(now)
    .bind(symbol)
    .bind(direction)
    .bind(outcome)
    .bind(risk)
    .bind(reward)
    .execute(pool)
    .await?;

    Ok(res.last_insert_rowid())
}

pub async fn query_user_trades(pool: &SqlitePool, limit: u32) -> Vec<UserTrade> {
    let rows = sqlx::query_as::<_, (i64, i64, String, String, String, f64, f64)>(
        "SELECT id, timestamp, symbol, direction, outcome, risk_multiplier, reward_multiplier
         FROM user_trades
         ORDER BY id DESC
         LIMIT ?1"
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|(id, timestamp, symbol, direction, outcome, risk_multiplier, reward_multiplier)| {
                UserTrade {
                    id,
                    timestamp,
                    symbol,
                    direction,
                    outcome,
                    risk_multiplier,
                    reward_multiplier,
                }
            })
            .collect(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to query user trades: {}", e);
            vec![]
        }
    }
}

// ─── Paper Trading Functions ───────────────────────────────────────

async fn paper_open_position_internal(
    pool: &SqlitePool,
    symbol: &str,
    direction: &str,
    entry_price: f64,
    size: f64,
    allocated_usd: f64,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO active_positions (symbol, direction, entry_price, size, allocated_usd, entry_timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )
    .bind(symbol)
    .bind(direction)
    .bind(entry_price)
    .bind(size)
    .bind(allocated_usd)
    .bind(now)
    .execute(&*pool)
    .await
    {
        eprintln!("⚠️ Paper DB: Failed to open position for {}: {}", symbol, e);
    } else {
        println!("📄 Paper Position: OPEN {} {} @ ${:.2} (Size: {:.4}, Allocated: ${:.2})",
            symbol, direction, entry_price, size, allocated_usd);
    }
}

async fn paper_close_position_internal(
    pool: &SqlitePool,
    symbol: &str,
    exit_price: f64,
    exit_timestamp: i64,
    trigger: &str,
) {
    let position = sqlx::query_as::<_, (i64, String, String, f64, f64, f64, i64)>(
        "SELECT id, symbol, direction, entry_price, size, allocated_usd, entry_timestamp
         FROM active_positions WHERE symbol = ?1"
    )
    .bind(symbol)
    .fetch_optional(&*pool)
    .await;

    match position {
        Ok(Some((_id, sym, direction, entry_price, size, allocated_usd, entry_ts))) => {
            let realized_pnl = if direction == "LONG" {
                (exit_price - entry_price) * size
            } else {
                (entry_price - exit_price) * size
            };
            let roi_pct = if allocated_usd > 0.0 {
                (realized_pnl / allocated_usd) * 100.0
            } else {
                0.0
            };

            if let Err(e) = sqlx::query(
                "INSERT INTO paper_trades (symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
            )
            .bind(&sym)
            .bind(&direction)
            .bind(entry_price)
            .bind(exit_price)
            .bind(size)
            .bind(realized_pnl)
            .bind(roi_pct)
            .bind(entry_ts)
            .bind(exit_timestamp)
            .bind(trigger)
            .execute(&*pool)
            .await
            {
                eprintln!("⚠️ Paper DB: Failed to record trade for {}: {}", symbol, e);
            }

            if let Err(e) = sqlx::query("DELETE FROM active_positions WHERE symbol = ?1")
                .bind(symbol)
                .execute(&*pool)
                .await
            {
                eprintln!("⚠️ Paper DB: Failed to clear active position for {}: {}", symbol, e);
            }

            sqlx::query(
                "UPDATE paper_balances SET current_cash = current_cash + ?2 WHERE symbol = ?1"
            )
            .bind(symbol)
            .bind(allocated_usd + realized_pnl)
            .execute(&*pool)
            .await
            .ok();

            println!(
                "📄 Paper Position: CLOSE {} {} @ ${:.2} → PnL: ${:.2} (ROI: {:.2}%) [{}]",
                symbol, direction, exit_price, realized_pnl, roi_pct, trigger
            );
        }
        Ok(None) => {
            eprintln!("⚠️ Paper DB: No active position to close for {}", symbol);
        }
        Err(e) => {
            eprintln!("⚠️ Paper DB: Error querying active position for {}: {}", symbol, e);
        }
    }
}

async fn paper_update_balance_internal(
    pool: &SqlitePool,
    symbol: &str,
    current_cash: f64,
) {
    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute)
         VALUES (?1, ?2, ?2, 10.0, 0)"
    )
    .bind(symbol)
    .bind(current_cash)
    .execute(&*pool)
    .await
    {
        eprintln!("⚠️ Paper DB: Failed to update balance for {}: {}", symbol, e);
    }
}

pub async fn paper_ensure_balance(pool: &SqlitePool, symbol: &str) {
    sqlx::query(
        "INSERT OR IGNORE INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute)
         VALUES (?1, 10000.0, 10000.0, 10.0, 0)"
    )
    .bind(symbol)
    .execute(&*pool)
    .await
    .ok();
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaperBalance {
    pub id: i64,
    pub symbol: String,
    pub initial_usd: f64,
    pub current_cash: f64,
    pub allocation_pct: f64,
    pub auto_execute: bool,
}

pub async fn paper_get_balance(pool: &SqlitePool, symbol: &str) -> PaperBalance {
    use sqlx::Row;
    paper_ensure_balance(pool, symbol).await;
    let row = sqlx::query(
        "SELECT id, symbol, initial_usd, current_cash, allocation_pct, auto_execute
         FROM paper_balances WHERE symbol = ?1"
    )
    .bind(symbol)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten();

    match row {
        Some(r) => PaperBalance {
            id: r.get(0),
            symbol: r.get(1),
            initial_usd: r.get(2),
            current_cash: r.get(3),
            allocation_pct: r.get(4),
            auto_execute: r.get::<i32, _>(5) != 0,
        },
        None => PaperBalance {
            id: 0,
            symbol: symbol.to_string(),
            initial_usd: 10000.0,
            current_cash: 10000.0,
            allocation_pct: 10.0,
            auto_execute: false,
        },
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ActivePaperPosition {
    pub id: i64,
    pub symbol: String,
    pub direction: String,
    pub entry_price: f64,
    pub size: f64,
    pub allocated_usd: f64,
    pub entry_timestamp: i64,
}

pub async fn paper_get_active_position(pool: &SqlitePool, symbol: &str) -> Option<ActivePaperPosition> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT id, symbol, direction, entry_price, size, allocated_usd, entry_timestamp
         FROM active_positions WHERE symbol = ?1"
    )
    .bind(symbol)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten();

    row.map(|r| ActivePaperPosition {
        id: r.get(0),
        symbol: r.get(1),
        direction: r.get(2),
        entry_price: r.get(3),
        size: r.get(4),
        allocated_usd: r.get(5),
        entry_timestamp: r.get(6),
    })
}

pub async fn paper_set_balance_config(
    pool: &SqlitePool,
    symbol: &str,
    initial_usd: f64,
    allocation_pct: f64,
    auto_execute: bool,
) {
    let auto_val: i32 = if auto_execute { 1 } else { 0 };
    sqlx::query(
        "INSERT INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute)
         VALUES (?1, ?2, ?2, ?3, ?4)
         ON CONFLICT(symbol) DO UPDATE SET
            initial_usd = excluded.initial_usd,
            current_cash = excluded.initial_usd,
            allocation_pct = excluded.allocation_pct,
            auto_execute = excluded.auto_execute"
    )
    .bind(symbol)
    .bind(initial_usd)
    .bind(allocation_pct)
    .bind(auto_val)
    .execute(&*pool)
    .await
    .ok();
}

pub async fn paper_reset_account(pool: &SqlitePool, symbol: &str) {
    let balance = paper_get_balance(pool, symbol).await;
    sqlx::query(
        "UPDATE paper_balances SET current_cash = ?2 WHERE symbol = ?1"
    )
    .bind(symbol)
    .bind(balance.initial_usd)
    .execute(&*pool)
    .await
    .ok();

    sqlx::query("DELETE FROM active_positions WHERE symbol = ?1")
        .bind(symbol)
        .execute(&*pool)
        .await
        .ok();

    println!("📄 Paper Account: {} reset to initial balance ${:.2}", symbol, balance.initial_usd);
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaperTradeRecord {
    pub id: i64,
    pub symbol: String,
    pub direction: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub realized_pnl: f64,
    pub roi_pct: f64,
    pub entry_timestamp: i64,
    pub exit_timestamp: i64,
    pub trigger: String,
}

pub async fn paper_query_trades(pool: &SqlitePool, symbol: Option<&str>, limit: u32) -> Vec<PaperTradeRecord> {
    use sqlx::Row;
    let rows = if let Some(sym) = symbol {
        sqlx::query(
            "SELECT id, symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger
             FROM paper_trades WHERE symbol = ?1 ORDER BY id DESC LIMIT ?2"
        )
        .bind(sym)
        .bind(limit as i64)
        .fetch_all(&*pool)
        .await
    } else {
        sqlx::query(
            "SELECT id, symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger
             FROM paper_trades ORDER BY id DESC LIMIT ?1"
        )
        .bind(limit as i64)
        .fetch_all(&*pool)
        .await
    };

    match rows {
        Ok(rows) => rows
            .iter()
            .map(|r| PaperTradeRecord {
                id: r.get(0),
                symbol: r.get(1),
                direction: r.get(2),
                entry_price: r.get(3),
                exit_price: r.get(4),
                size: r.get(5),
                realized_pnl: r.get(6),
                roi_pct: r.get(7),
                entry_timestamp: r.get(8),
                exit_timestamp: r.get(9),
                trigger: r.get(10),
            })
            .collect(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to query paper trades: {}", e);
            vec![]
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaperAccountMetrics {
    pub symbol: String,
    pub initial_usd: f64,
    pub current_cash: f64,
    pub allocation_pct: f64,
    pub auto_execute: bool,
    pub unrealized_pnl: f64,
    pub unrealized_roi_pct: f64,
    pub total_account_value: f64,
    pub margin_used: f64,
    pub max_trades: u32,
    pub active_trades: u32,
    pub available_trades: u32,
    pub active_position: Option<ActivePaperPosition>,
}

pub async fn paper_get_account_metrics(
    pool: &SqlitePool,
    symbol: &str,
    current_price: f64,
) -> PaperAccountMetrics {
    let balance = paper_get_balance(pool, symbol).await;
    let position = paper_get_active_position(pool, symbol).await;

    let (unrealized_pnl, unrealized_roi, margin_used) = match &position {
        Some(pos) => {
            let pnl = if pos.direction == "LONG" {
                (current_price - pos.entry_price) * pos.size
            } else {
                (pos.entry_price - current_price) * pos.size
            };
            let roi = if pos.allocated_usd > 0.0 {
                (pnl / pos.allocated_usd) * 100.0
            } else {
                0.0
            };
            (pnl, roi, pos.allocated_usd)
        }
        None => (0.0, 0.0, 0.0),
    };

    let total_account_value = balance.current_cash + margin_used + unrealized_pnl;
    let max_trades = if balance.allocation_pct > 0.0 {
        (100.0 / balance.allocation_pct).floor() as u32
    } else {
        0
    };
    let active_trades = if position.is_some() { 1u32 } else { 0u32 };
    let available_trades = max_trades.saturating_sub(active_trades);

    PaperAccountMetrics {
        symbol: symbol.to_string(),
        initial_usd: balance.initial_usd,
        current_cash: balance.current_cash,
        allocation_pct: balance.allocation_pct,
        auto_execute: balance.auto_execute,
        unrealized_pnl,
        unrealized_roi_pct: unrealized_roi,
        total_account_value,
        margin_used,
        max_trades,
        active_trades,
        available_trades,
        active_position: position,
    }
}

// ─── Seed Default Profiles ─────────────────────────────────────────

async fn seed_default_profiles(pool: &SqlitePool) {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM decision_profiles")
        .fetch_one(&*pool)
        .await
        .unwrap_or((0,));
    if count.0 > 0 {
        return;
    }

    sqlx::query(
        "INSERT INTO decision_profiles (profile_name, long_threshold, short_threshold)
         VALUES ('Cryptobruj', 40, -40)"
    )
    .execute(&*pool)
    .await
    .ok();

    let indicators = vec![
        ("RSI (Oversold/Overbought)", 10, "NONE"),
        ("RSI (Divergence)", 20, "NONE"),
        ("MACD (Crossovers)", 10, "NONE"),
        ("MACD (Divergence)", 10, "NONE"),
        ("Support/Resistance", 10, "NONE"),
        ("Trend", 20, "NONE"),
        ("Patterns", 10, "NONE"),
    ];
    for (name, weight, ovr) in &indicators {
        sqlx::query(
            "INSERT INTO profile_indicators (profile_id, indicator_name, weight, override_status)
             VALUES (1, ?1, ?2, ?3)"
        )
        .bind(name)
        .bind(weight)
        .bind(ovr)
        .execute(&*pool)
        .await
        .ok();
    }

    let risk_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM risk_profiles")
        .fetch_one(&*pool)
        .await
        .unwrap_or((0,));
    if risk_count.0 == 0 {
        sqlx::query(
            "INSERT INTO risk_profiles (profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread)
             VALUES ('Risk Profile', 1000.0, 2.0, 20, 0.06, 0.0, 0.0)"
        )
        .execute(&*pool)
        .await
        .ok();
    }
}

// ─── Exchange Keys CRUD ────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExchangeKey {
    pub id: i64,
    pub exchange: String,
    pub account_name: String,
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: String,
    pub referred_uid: String,
    pub is_active: bool,
    pub last_sync_timestamp: Option<i64>,
}

pub async fn exchange_keys_list(pool: &SqlitePool) -> Vec<ExchangeKey> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, exchange, account_name, api_key, api_secret, passphrase, referred_uid, is_active, last_sync_timestamp
         FROM exchange_keys ORDER BY id DESC"
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or_default();

    rows.iter()
        .map(|r| ExchangeKey {
            id: r.get(0),
            exchange: r.get(1),
            account_name: r.get(2),
            api_key: r.get(3),
            api_secret: r.get(4),
            passphrase: r.get(5),
            referred_uid: r.get(6),
            is_active: r.get::<i32, _>(7) != 0,
            last_sync_timestamp: r.get(8),
        })
        .collect()
}

pub async fn exchange_keys_insert(
    pool: &SqlitePool,
    exchange: &str,
    account_name: &str,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    referred_uid: &str,
    is_active: bool,
) -> i64 {
    let active_val: i32 = if is_active { 1 } else { 0 };
    let result = sqlx::query(
        "INSERT INTO exchange_keys (exchange, account_name, api_key, api_secret, passphrase, referred_uid, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )
    .bind(exchange)
    .bind(account_name)
    .bind(api_key)
    .bind(api_secret)
    .bind(passphrase)
    .bind(referred_uid)
    .bind(active_val)
    .execute(&*pool)
    .await;

    match result {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("⚠️ DB: Failed to insert exchange key: {}", e);
            0
        }
    }
}

pub async fn exchange_keys_delete(pool: &SqlitePool, id: i64) -> bool {
    sqlx::query("DELETE FROM exchange_keys WHERE id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

pub async fn exchange_keys_update_sync(pool: &SqlitePool, id: i64, timestamp: i64) {
    sqlx::query("UPDATE exchange_keys SET last_sync_timestamp = ?2 WHERE id = ?1")
        .bind(id)
        .bind(timestamp)
        .execute(&*pool)
        .await
        .ok();
}

pub async fn exchange_keys_active_count(pool: &SqlitePool) -> i64 {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM exchange_keys WHERE is_active = 1")
        .fetch_one(&*pool)
        .await
        .unwrap_or((0,));
    row.0
}

// ─── Decision Profiles CRUD ────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionProfile {
    pub id: i64,
    pub profile_name: String,
    pub long_threshold: i32,
    pub short_threshold: i32,
    #[serde(default)]
    pub indicators: Vec<ProfileIndicator>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProfileIndicator {
    pub id: i64,
    pub profile_id: i64,
    pub indicator_name: String,
    pub weight: i32,
    pub override_status: String,
}

pub async fn decision_profiles_list(pool: &SqlitePool) -> Vec<DecisionProfile> {
    use sqlx::Row;
    let rows = sqlx::query("SELECT id, profile_name, long_threshold, short_threshold FROM decision_profiles ORDER BY id ASC")
        .fetch_all(&*pool)
        .await
        .unwrap_or_default();

    let mut profiles = Vec::new();
    for r in &rows {
        let profile_id: i64 = r.get(0);
        let indicators = get_profile_indicators_internal(pool, profile_id).await;
        profiles.push(DecisionProfile {
            id: profile_id,
            profile_name: r.get(1),
            long_threshold: r.get(2),
            short_threshold: r.get(3),
            indicators,
        });
    }
    profiles
}

async fn get_profile_indicators_internal(pool: &SqlitePool, profile_id: i64) -> Vec<ProfileIndicator> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, profile_id, indicator_name, weight, override_status
         FROM profile_indicators WHERE profile_id = ?1 ORDER BY id ASC"
    )
    .bind(profile_id)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default();

    rows.iter()
        .map(|r| ProfileIndicator {
            id: r.get(0),
            profile_id: r.get(1),
            indicator_name: r.get(2),
            weight: r.get(3),
            override_status: r.get(4),
        })
        .collect()
}

pub async fn decision_profile_insert(
    pool: &SqlitePool,
    profile_name: &str,
    long_threshold: i32,
    short_threshold: i32,
) -> i64 {
    match sqlx::query(
        "INSERT INTO decision_profiles (profile_name, long_threshold, short_threshold)
         VALUES (?1, ?2, ?3)"
    )
    .bind(profile_name)
    .bind(long_threshold)
    .bind(short_threshold)
    .execute(&*pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("⚠️ DB: Failed to insert decision profile: {}", e);
            0
        }
    }
}

pub async fn decision_profile_update(
    pool: &SqlitePool,
    id: i64,
    profile_name: &str,
    long_threshold: i32,
    short_threshold: i32,
) -> bool {
    sqlx::query(
        "UPDATE decision_profiles SET profile_name = ?2, long_threshold = ?3, short_threshold = ?4 WHERE id = ?1"
    )
    .bind(id)
    .bind(profile_name)
    .bind(long_threshold)
    .bind(short_threshold)
    .execute(&*pool)
    .await
    .map(|r| r.rows_affected() > 0)
    .unwrap_or(false)
}

pub async fn decision_profile_delete(pool: &SqlitePool, id: i64) -> bool {
    sqlx::query("DELETE FROM profile_indicators WHERE profile_id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .ok();

    sqlx::query("DELETE FROM decision_profiles WHERE id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

pub async fn profile_indicator_insert(
    pool: &SqlitePool,
    profile_id: i64,
    indicator_name: &str,
    weight: i32,
    override_status: &str,
) -> i64 {
    match sqlx::query(
        "INSERT INTO profile_indicators (profile_id, indicator_name, weight, override_status)
         VALUES (?1, ?2, ?3, ?4)"
    )
    .bind(profile_id)
    .bind(indicator_name)
    .bind(weight)
    .bind(override_status)
    .execute(&*pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("⚠️ DB: Failed to insert profile indicator: {}", e);
            0
        }
    }
}

pub async fn profile_indicator_update(
    pool: &SqlitePool,
    indicator_id: i64,
    weight: i32,
    override_status: &str,
) -> bool {
    sqlx::query(
        "UPDATE profile_indicators SET weight = ?2, override_status = ?3 WHERE id = ?1"
    )
    .bind(indicator_id)
    .bind(weight)
    .bind(override_status)
    .execute(&*pool)
    .await
    .map(|r| r.rows_affected() > 0)
    .unwrap_or(false)
}

pub async fn profile_indicator_delete(pool: &SqlitePool, indicator_id: i64) -> bool {
    sqlx::query("DELETE FROM profile_indicators WHERE id = ?1")
        .bind(indicator_id)
        .execute(&*pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

// ─── Trade Telemetry CRUD ──────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct TradeTelemetryRecord {
    pub id: i64,
    pub exchange: String,
    pub symbol: String,
    pub direction: String,
    pub entry_timestamp: i64,
    pub exit_timestamp: i64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub commission_fees: f64,
    pub funding_fees: f64,
    pub realized_pnl: f64,
    pub roi_percentage: f64,
    pub trigger_source: String,
}

pub async fn trade_telemetry_insert(
    pool: &SqlitePool,
    exchange: &str,
    symbol: &str,
    direction: &str,
    entry_timestamp: i64,
    exit_timestamp: i64,
    entry_price: f64,
    exit_price: f64,
    size: f64,
    commission_fees: f64,
    funding_fees: f64,
    realized_pnl: f64,
    roi_percentage: f64,
    trigger_source: &str,
) -> i64 {
    match sqlx::query(
        "INSERT INTO trade_telemetry_history
         (exchange, symbol, direction, entry_timestamp, exit_timestamp,
          entry_price, exit_price, size, commission_fees, funding_fees,
          realized_pnl, roi_percentage, trigger_source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"
    )
    .bind(exchange)
    .bind(symbol)
    .bind(direction)
    .bind(entry_timestamp)
    .bind(exit_timestamp)
    .bind(entry_price)
    .bind(exit_price)
    .bind(size)
    .bind(commission_fees)
    .bind(funding_fees)
    .bind(realized_pnl)
    .bind(roi_percentage)
    .bind(trigger_source)
    .execute(&*pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("⚠️ DB: Failed to insert trade telemetry: {}", e);
            0
        }
    }
}

pub async fn trade_telemetry_query_all(pool: &SqlitePool, limit: u32) -> Vec<TradeTelemetryRecord> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, exchange, symbol, direction, entry_timestamp, exit_timestamp,
                entry_price, exit_price, size, commission_fees, funding_fees,
                realized_pnl, roi_percentage, trigger_source
         FROM trade_telemetry_history
         ORDER BY id DESC LIMIT ?1"
    )
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default();

    rows.iter()
        .map(|r| TradeTelemetryRecord {
            id: r.get(0),
            exchange: r.get(1),
            symbol: r.get(2),
            direction: r.get(3),
            entry_timestamp: r.get(4),
            exit_timestamp: r.get(5),
            entry_price: r.get(6),
            exit_price: r.get(7),
            size: r.get(8),
            commission_fees: r.get(9),
            funding_fees: r.get(10),
            realized_pnl: r.get(11),
            roi_percentage: r.get(12),
            trigger_source: r.get(13),
        })
        .collect()
}

pub async fn trade_telemetry_count(pool: &SqlitePool) -> i64 {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM trade_telemetry_history")
        .fetch_one(&*pool)
        .await
        .unwrap_or((0,));
    row.0
}

// ─── Risk Profiles CRUD ────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskProfile {
    pub id: i64,
    pub profile_name: String,
    pub capital: f64,
    pub max_risk_pct: f64,
    pub leverage: i32,
    pub commission_pct: f64,
    pub funding_rate_8h: f64,
    pub spread: f64,
}

pub async fn risk_profiles_list(pool: &SqlitePool) -> Vec<RiskProfile> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread
         FROM risk_profiles ORDER BY id ASC"
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or_default();

    rows.iter()
        .map(|r| RiskProfile {
            id: r.get(0),
            profile_name: r.get(1),
            capital: r.get(2),
            max_risk_pct: r.get(3),
            leverage: r.get(4),
            commission_pct: r.get(5),
            funding_rate_8h: r.get(6),
            spread: r.get(7),
        })
        .collect()
}

pub async fn risk_profile_by_id(pool: &SqlitePool, id: i64) -> Option<RiskProfile> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT id, profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread
         FROM risk_profiles WHERE id = ?1"
    )
    .bind(id)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten();

    row.map(|r| RiskProfile {
        id: r.get(0),
        profile_name: r.get(1),
        capital: r.get(2),
        max_risk_pct: r.get(3),
        leverage: r.get(4),
        commission_pct: r.get(5),
        funding_rate_8h: r.get(6),
        spread: r.get(7),
    })
}

pub async fn risk_profile_insert(
    pool: &SqlitePool,
    profile_name: &str,
    capital: f64,
    max_risk_pct: f64,
    leverage: i32,
    commission_pct: f64,
    funding_rate_8h: f64,
    spread: f64,
) -> i64 {
    match sqlx::query(
        "INSERT INTO risk_profiles (profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )
    .bind(profile_name)
    .bind(capital)
    .bind(max_risk_pct)
    .bind(leverage)
    .bind(commission_pct)
    .bind(funding_rate_8h)
    .bind(spread)
    .execute(&*pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("⚠️ DB: Failed to insert risk profile: {}", e);
            0
        }
    }
}

pub async fn risk_profile_update(
    pool: &SqlitePool,
    id: i64,
    profile_name: &str,
    capital: f64,
    max_risk_pct: f64,
    leverage: i32,
    commission_pct: f64,
    funding_rate_8h: f64,
    spread: f64,
) -> bool {
    sqlx::query(
        "UPDATE risk_profiles SET profile_name = ?2, capital = ?3, max_risk_pct = ?4,
         leverage = ?5, commission_pct = ?6, funding_rate_8h = ?7, spread = ?8
         WHERE id = ?1"
    )
    .bind(id)
    .bind(profile_name)
    .bind(capital)
    .bind(max_risk_pct)
    .bind(leverage)
    .bind(commission_pct)
    .bind(funding_rate_8h)
    .bind(spread)
    .execute(&*pool)
    .await
    .map(|r| r.rows_affected() > 0)
    .unwrap_or(false)
}

pub async fn risk_profile_delete(pool: &SqlitePool, id: i64) -> bool {
    sqlx::query("DELETE FROM risk_profiles WHERE id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

// ─── Dashboard Stats Query Helpers ─────────────────────────────────

pub async fn dash_trade_timestamps(pool: &SqlitePool) -> Vec<(i64, f64, f64, String, String)> {
    sqlx::query_as(
        "SELECT exit_timestamp, realized_pnl, commission_fees, direction, trigger_source
         FROM trade_telemetry_history ORDER BY exit_timestamp ASC"
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or_default()
}

pub async fn dash_trade_detail(pool: &SqlitePool) -> Vec<(i64, String, String, f64, f64, f64, f64, f64, f64, String)> {
    sqlx::query_as(
        "SELECT exit_timestamp, symbol, direction, entry_price, exit_price, size, realized_pnl, commission_fees, roi_percentage, trigger_source
         FROM trade_telemetry_history ORDER BY exit_timestamp ASC"
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or_default()
}
