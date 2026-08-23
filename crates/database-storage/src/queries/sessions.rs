//! # Session identity queries (v10)
//!
//! Sessions are the join key for the whole data-science layer: every
//! telemetry row, trade, equity sample and risk event carries the
//! `session_id` of the run that produced it. Monotonic, persisted,
//! never reused.

use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: i64,
    pub mode: String,
    pub exchange: Option<String>,
    pub currency: Option<String>,
    pub portfolio_capital_usd: Option<f64>,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub status: String,
}

/// Create a session row and return its id (the session number).
pub async fn create_session(
    pool: &SqlitePool,
    mode: &str,
    exchange: Option<&str>,
    currency: Option<&str>,
    portfolio_capital_usd: Option<f64>,
    started_at_ms: i64,
    config_snapshot_json: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO sessions (mode, exchange, currency, portfolio_capital_usd, started_at_ms, status, config_snapshot_json)
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6)",
    )
    .bind(mode)
    .bind(exchange)
    .bind(currency)
    .bind(portfolio_capital_usd)
    .bind(started_at_ms)
    .bind(config_snapshot_json)
    .execute(pool)
    .await?;
    Ok(row.last_insert_rowid())
}

/// Close the active session (graceful shutdown / quit).
pub async fn close_session(pool: &SqlitePool, id: i64, ended_at_ms: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET ended_at_ms = ?2, status = 'closed' WHERE id = ?1")
        .bind(id)
        .bind(ended_at_ms)
        .execute(pool)
        .await?;
    Ok(())
}

/// The current active session id (highest `active` row), if any.
pub async fn current_session_id(pool: &SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM sessions WHERE status = 'active' ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}

/// All sessions, newest first.
pub async fn list_sessions(pool: &SqlitePool) -> Result<Vec<SessionRow>, sqlx::Error> {
    let rows: Vec<(i64, String, Option<String>, Option<String>, Option<f64>, i64, Option<i64>, String)> =
        sqlx::query_as(
            "SELECT id, mode, exchange, currency, portfolio_capital_usd, started_at_ms, ended_at_ms, status
             FROM sessions ORDER BY id DESC",
        )
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, mode, exchange, currency, portfolio_capital_usd, started_at_ms, ended_at_ms, status)| {
                SessionRow {
                    id,
                    mode,
                    exchange,
                    currency,
                    portfolio_capital_usd,
                    started_at_ms,
                    ended_at_ms,
                    status,
                }
            },
        )
        .collect())
}
