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
