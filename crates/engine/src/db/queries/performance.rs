use sqlx::SqlitePool;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
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

pub async fn insert_automated_performance_baseline(
    pool: &SqlitePool,
    master_record_id: i64,
    symbol: &str,
    price_at_signal: &str,
) {
    if let Err(e) = sqlx::query(
        "INSERT INTO automated_performance_tracker (master_record_id, symbol, price_at_signal) VALUES (?1, ?2, ?3)"
    )
    .bind(master_record_id).bind(symbol).bind(price_at_signal)
    .execute(&*pool).await
    {
        eprintln!("Database Error: Failed to insert automated performance baseline: {}", e);
    }
}

pub async fn query_automated_performance(
    pool: &SqlitePool,
    limit: u32,
) -> Vec<AutomatedPerformanceRow> {
    sqlx::query_as::<_, AutomatedPerformanceRow>(
        "SELECT id, master_record_id, symbol, price_at_signal,
                price_at_1h, price_at_4h, price_at_24h,
                direction_correct_1h, direction_correct_4h, direction_correct_24h,
                created_at
         FROM automated_performance_tracker
         ORDER BY id DESC LIMIT ?1",
    )
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await
    .unwrap_or_else(|e| {
        eprintln!(
            "Database Error: Failed to query automated performance: {}",
            e
        );
        vec![]
    })
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
         WHERE id = ?1",
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
        eprintln!(
            "Database Error: Failed to update performance tracker {}: {}",
            tracker_id, e
        );
    }
}

pub async fn query_pending_performance_entries(pool: &SqlitePool) -> Vec<AutomatedPerformanceRow> {
    sqlx::query_as::<_, AutomatedPerformanceRow>(
        "SELECT id, master_record_id, symbol, price_at_signal,
                price_at_1h, price_at_4h, price_at_24h,
                direction_correct_1h, direction_correct_4h, direction_correct_24h,
                created_at
         FROM automated_performance_tracker
         WHERE price_at_24h IS NULL
         ORDER BY id ASC",
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or_else(|e| {
        eprintln!(
            "Database Error: Failed to query pending performance entries: {}",
            e
        );
        vec![]
    })
}
