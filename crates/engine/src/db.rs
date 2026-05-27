use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use shared::models::MarketSnapshot;

pub async fn init_db() -> SqlitePool {
    let db_options = SqliteConnectOptions::new()
        .filename("telemetry.db")
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(db_options)
        .await
        .expect("❌ Database Setup: Failed to initialize SQLite database pool");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS market_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            symbol TEXT NOT NULL,
            mid_price TEXT NOT NULL,
            bid_price TEXT NOT NULL,
            ask_price TEXT NOT NULL,
            open TEXT,
            high TEXT,
            low TEXT,
            close TEXT,
            volume TEXT,
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
        "CREATE TABLE IF NOT EXISTS assistant_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT (datetime('now')),
            position TEXT NOT NULL,
            trend_classification TEXT,
            indicator_alignment TEXT,
            recommended_action TEXT,
            recommendation_rationale TEXT,
            close_price TEXT,
            symbol TEXT
        )"
    )
    .execute(&pool)
    .await
    .expect("❌ Database Setup: Failed to build assistant_records table");

    pool
}

pub async fn insert_assistant_record(
    pool: &SqlitePool,
    position: &str,
    trend_classification: &str,
    indicator_alignment: &str,
    recommended_action: &str,
    recommendation_rationale: &str,
    close_price: &str,
    symbol: &str,
) {
    if let Err(e) = sqlx::query(
        "INSERT INTO assistant_records (
            position, trend_classification, indicator_alignment,
            recommended_action, recommendation_rationale, close_price, symbol
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )
    .bind(position)
    .bind(trend_classification)
    .bind(indicator_alignment)
    .bind(recommended_action)
    .bind(recommendation_rationale)
    .bind(close_price)
    .bind(symbol)
    .execute(&*pool)
    .await
    {
        eprintln!("⚠️ Database Error: Failed to save assistant record: {}", e);
    }
}

pub async fn insert_snapshot(pool: &SqlitePool, snapshot: &MarketSnapshot) {
    let sqz_on_db_val = snapshot.squeeze_on.map(|s| if s { 1 } else { 0 });

    if let Err(e) = sqlx::query(
        "INSERT INTO market_snapshots (
            timestamp, symbol, mid_price, bid_price, ask_price,
            open, high, low, close, volume,
            bb_upper, bb_middle, bb_lower, atr_14, vwap,
            ema_fast, ema_medium, ema_slow, ema_long, rsi_14,
            macd_line, macd_signal, macd_hist, adx_14, adx_plus, adx_minus,
            squeeze_on, squeeze_momentum
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)"
    )
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
