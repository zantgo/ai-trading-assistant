use core_domain::normalized::Exchange;
use sqlx::SqlitePool;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct UserTrade {
    pub id: i64,
    pub timestamp: i64,
    pub symbol: String,
    pub direction: String,
    pub outcome: String,
    pub risk_multiplier: f64,
    pub reward_multiplier: f64,
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
        .unwrap_or_default()
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
    sqlx::query_as::<_, UserTrade>(
        "SELECT id, timestamp, symbol, direction, outcome, risk_multiplier, reward_multiplier
         FROM user_trades
         ORDER BY id DESC
         LIMIT ?1",
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        eprintln!("Database Error: Failed to query user trades: {}", e);
        vec![]
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TradeTelemetryRecord {
    pub id: i64,
    pub exchange: Exchange,
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
    pub roi_pct: f64,
    pub trigger_source: String,
}

#[allow(clippy::too_many_arguments)]
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
    roi_pct: f64,
    trigger_source: &str,
) -> i64 {
    match sqlx::query(
        "INSERT INTO trade_telemetry_history
         (exchange, symbol, direction, entry_timestamp, exit_timestamp,
          entry_price, exit_price, size, commission_fees, funding_fees,
          realized_pnl, roi_pct, trigger_source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
    .bind(roi_pct)
    .bind(trigger_source)
    .execute(pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("DB: Failed to insert trade telemetry: {}", e);
            0
        }
    }
}

#[derive(sqlx::FromRow)]
struct TradeTelemetryQueryRow {
    id: i64,
    _exchange: String,
    symbol: String,
    direction: String,
    entry_timestamp: i64,
    exit_timestamp: i64,
    entry_price: f64,
    exit_price: f64,
    size: f64,
    commission_fees: f64,
    funding_fees: f64,
    realized_pnl: f64,
    roi_pct: f64,
    trigger_source: String,
}

pub async fn trade_telemetry_query_all(pool: &SqlitePool, limit: u32) -> Vec<TradeTelemetryRecord> {
    let rows = sqlx::query_as::<_, TradeTelemetryQueryRow>(
        "SELECT id, exchange, symbol, direction, entry_timestamp, exit_timestamp,
                entry_price, exit_price, size, commission_fees, funding_fees,
                realized_pnl, roi_pct, trigger_source
         FROM trade_telemetry_history
         ORDER BY id DESC LIMIT ?1",
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| TradeTelemetryRecord {
            id: r.id,
            exchange: Exchange::Hyperliquid,
            symbol: r.symbol,
            direction: r.direction,
            entry_timestamp: r.entry_timestamp,
            exit_timestamp: r.exit_timestamp,
            entry_price: r.entry_price,
            exit_price: r.exit_price,
            size: r.size,
            commission_fees: r.commission_fees,
            funding_fees: r.funding_fees,
            realized_pnl: r.realized_pnl,
            roi_pct: r.roi_pct,
            trigger_source: r.trigger_source,
        })
        .collect()
}

pub async fn trade_telemetry_count(pool: &SqlitePool) -> i64 {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM trade_telemetry_history")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    row.0
}
