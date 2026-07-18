//! Read-path queries for the `connection_quality_samples` table.
//!
//! The in-memory registry serves the live `/api/connection-quality`
//! endpoint. These queries back historical views (reconstructed-candle
//! counts and gap statistics across persisted windows).

use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct HistoricalQualityRow {
    pub timestamp_ms: i64,
    pub window: String,
    pub uptime_pct: f64,
    pub disconnect_count: i64,
    pub avg_reconnect_ms: f64,
    pub total_data_loss_secs: i64,
    pub reconstructed_candles: i64,
    pub score: f64,
    pub pair_key: String,
    pub timeframe_secs: i64,
}

pub async fn query_recent_quality(
    pool: &SqlitePool,
    window: &str,
    limit: u32,
) -> Result<Vec<HistoricalQualityRow>, sqlx::Error> {
    sqlx::query_as::<_, HistoricalQualityRow>(
        r#"SELECT
            timestamp_ms,
            window,
            uptime_pct,
            disconnect_count,
            avg_reconnect_ms,
            total_data_loss_secs,
            reconstructed_candles,
            score,
            pair_key,
            timeframe_secs
        FROM connection_quality_samples
        WHERE window = ?1
        ORDER BY timestamp_ms DESC
        LIMIT ?2
        "#,
    )
    .bind(window)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
}

/// Recent samples for one `(pair_key, timeframe_secs)` scope.
pub async fn query_recent_quality_scoped(
    pool: &SqlitePool,
    pair_key: &str,
    timeframe_secs: u64,
    window: &str,
    limit: u32,
) -> Result<Vec<HistoricalQualityRow>, sqlx::Error> {
    sqlx::query_as::<_, HistoricalQualityRow>(
        r#"SELECT
            timestamp_ms,
            window,
            uptime_pct,
            disconnect_count,
            avg_reconnect_ms,
            total_data_loss_secs,
            reconstructed_candles,
            score,
            pair_key,
            timeframe_secs
        FROM connection_quality_samples
        WHERE pair_key = ?1 AND timeframe_secs = ?2 AND window = ?3
        ORDER BY timestamp_ms DESC
        LIMIT ?4
        "#,
    )
    .bind(pair_key)
    .bind(timeframe_secs as i64)
    .bind(window)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
}

pub async fn total_reconstructed_candles(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(reconstructed_candles), 0) FROM connection_quality_samples",
    )
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn total_gap_events(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(disconnect_count), 0) FROM connection_quality_samples",
    )
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
