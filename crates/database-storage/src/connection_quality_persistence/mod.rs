//! Connection-quality persistence (DB writer + read path).
//!
//! The in-memory tracker/registry lives in
//! `network_adapters::connection_quality_tracker` (no DB dep). Its 60 s
//! persistence loop INSERTs one row per `(pair_key, timeframe_secs)` scope ×
//! window into `connection_quality_samples` (see migrations
//! `20260715120000_connection_quality.sql` and
//! `20260718120000_connection_quality_pair_scope.sql`). This module owns the
//! schema contract: `insert_quality_sample` is the canonical writer for
//! callers that depend on `database-storage`, and `queries` is the
//! historical read path.

pub mod queries;

use sqlx::SqlitePool;

/// One `connection_quality_samples` row (write shape).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QualitySampleRow {
    pub timestamp_ms: u64,
    /// Window bucket name: ONE_HOUR | SIX_HOUR | TWENTY_FOUR_HOUR.
    pub window: String,
    pub uptime_pct: f64,
    pub disconnect_count: u32,
    pub avg_reconnect_ms: f64,
    pub total_data_loss_secs: u64,
    pub reconstructed_candles: u32,
    pub score: f64,
    /// Scope: unified pair key (e.g. "BTC-USDT"); "GLOBAL" for legacy rows.
    pub pair_key: String,
    /// Scope: timeframe seconds; 0 for legacy process-wide rows.
    pub timeframe_secs: u64,
}

/// Insert one per-scope sample row.
pub async fn insert_quality_sample(
    pool: &SqlitePool,
    row: &QualitySampleRow,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO connection_quality_samples \
            (timestamp_ms, window, uptime_pct, disconnect_count, avg_reconnect_ms, \
             total_data_loss_secs, reconstructed_candles, score, pair_key, timeframe_secs) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .bind(row.timestamp_ms as i64)
    .bind(&row.window)
    .bind(row.uptime_pct)
    .bind(row.disconnect_count as i64)
    .bind(row.avg_reconnect_ms)
    .bind(row.total_data_loss_secs as i64)
    .bind(row.reconstructed_candles as i64)
    .bind(row.score)
    .bind(&row.pair_key)
    .bind(row.timeframe_secs as i64)
    .execute(pool)
    .await?;
    Ok(())
}
