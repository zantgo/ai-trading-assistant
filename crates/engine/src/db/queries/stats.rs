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
    pub market_regime: Option<String>,
}

pub async fn dash_trade_detail(pool: &SqlitePool) -> Vec<TradeDetailRow> {
    sqlx::query_as::<_, TradeDetailRow>(
        "SELECT t.exit_timestamp, t.symbol, t.direction, t.entry_price, t.exit_price, t.size,
                t.realized_pnl, t.commission_fees, t.roi_percentage, t.trigger_source,
                COALESCE(pt.market_regime, 'RANGE') as market_regime
         FROM trade_telemetry_history t
         LEFT JOIN paper_trades pt ON pt.symbol = t.symbol
             AND pt.exit_timestamp = t.exit_timestamp
             AND ABS(pt.realized_pnl - t.realized_pnl) < 0.01
         ORDER BY t.exit_timestamp ASC",
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
    let query = "
        SELECT
            pt.id, pt.symbol, pt.direction, pt.realized_pnl, pt.roi_pct,
            (pt.entry_price * pt.size) as allocated_usd,
            (SELECT mar.market_regime
             FROM master_assistant_records mar
             WHERE mar.symbol = pt.symbol
               AND mar.created_at <= datetime(pt.entry_timestamp / 1000, 'unixepoch')
             ORDER BY mar.id DESC LIMIT 1) as market_regime
        FROM paper_trades pt
        ORDER BY pt.id DESC
    ";
    sqlx::query_as::<_, ClosedTradeRow>(query)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

pub async fn insert_optimization_report(pool: &SqlitePool, report_json: &str) {
    let _ = sqlx::query(
        "INSERT INTO agent_thought_logs (master_record_id, agent_name, thought_process, json_rpc_payload, confidence_score) \
         VALUES (0, 'Optimizer', 'Periodic strategy weight optimization run', ?1, 0)"
    )
    .bind(report_json).execute(pool).await;
}
