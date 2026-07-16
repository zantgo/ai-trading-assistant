use sqlx::SqlitePool;

pub async fn dash_trade_timestamps(pool: &SqlitePool) -> Vec<(i64, f64, f64, String, String)> {
    sqlx::query_as(
        "SELECT exit_timestamp, realized_pnl, commission_fees, direction, trigger_source
         FROM trade_telemetry_history ORDER BY exit_timestamp ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TradeDetailRow {
    pub exit_timestamp: i64,
    pub symbol: String,
    pub direction: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub realized_pnl: f64,
    pub commission_fees: f64,
    pub roi_percentage: f64,
    pub trigger_source: String,
}

pub async fn dash_trade_detail(pool: &SqlitePool) -> Vec<TradeDetailRow> {
    sqlx::query_as::<_, TradeDetailRow>(
        "SELECT exit_timestamp, symbol, direction, entry_price, exit_price, size, realized_pnl, commission_fees, roi_percentage, trigger_source
         FROM trade_telemetry_history ORDER BY exit_timestamp ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClosedTradeRow {
    pub id: i64,
    pub symbol: String,
    pub direction: String,
    pub realized_pnl: f64,
    pub roi_pct: f64,
    pub allocated_usd: f64,
    pub market_regime: Option<String>,
}

pub async fn get_daily_pnl(pool: &SqlitePool) -> Option<f64> {
    let today_start = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)?
        .and_utc()
        .timestamp_millis();

    let row: (Option<f64>,) = sqlx::query_as(
        "SELECT COALESCE(SUM(realized_pnl), 0.0) FROM paper_trades WHERE exit_timestamp >= ?1",
    )
    .bind(today_start)
    .fetch_one(pool)
    .await
    .ok()?;
    row.0
}

pub async fn query_all_closed_trades(pool: &SqlitePool) -> Vec<ClosedTradeRow> {
    sqlx::query_as::<_, ClosedTradeRow>(
        "SELECT id, symbol, direction, realized_pnl, roi_pct,
                (entry_price * size) as allocated_usd,
                NULL as market_regime
         FROM paper_trades
         ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}
