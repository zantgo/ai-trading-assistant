use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use shared::models::MarketSnapshot;

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
) -> i64 {
    match sqlx::query(
        "INSERT INTO master_assistant_records (
            position, entry_price, price_at_analysis, general_trend,
            support_levels, resistance_levels,
            indicator_synthesis_summary, indicator_synthesis_evaluation,
            recommended_action, recommendation_rationale, symbol
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
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
}

pub async fn query_master_records(pool: &SqlitePool, limit: u32) -> Vec<MasterRecord> {
    let rows = sqlx::query_as::<_, (i64, String, String, Option<String>, String, String, String, String, String, String, String, String, String)>(
        "SELECT id, created_at, position, entry_price, price_at_analysis,
                general_trend, support_levels, resistance_levels,
                indicator_synthesis_summary, indicator_synthesis_evaluation,
                recommended_action, recommendation_rationale, symbol
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
            .map(|(id, created_at, position, entry_price, price_at_analysis, general_trend, support_levels, resistance_levels, indicator_synthesis_summary, indicator_synthesis_evaluation, recommended_action, recommendation_rationale, symbol)| {
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
