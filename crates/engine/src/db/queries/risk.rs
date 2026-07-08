//! Persistence for the Institutional Risk Management Layer (IRML):
//! `risk_events` (state snapshots) and `rr_calibration` (per-block adaptive R:R
//! ledger). See docs/institutional-risk-management-layer.md Section 19.2.

use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RiskEventRow {
    pub id: i64,
    pub pair_key: String,
    pub timestamp: i64,
    pub overall_risk: f64,
    pub overall_level: String,
    pub drawdown_state: String,
    pub permission: String,
    pub losing_streak: i64,
    pub winning_streak: i64,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RrCalibrationRow {
    pub id: i64,
    pub pair_key: String,
    pub block_index: i64,
    pub wins: i64,
    pub losses: i64,
    pub win_rate_estimate: f64,
    pub breakeven_ratio: f64,
    pub recommended_ratio: f64,
    pub confidence: f64,
    pub net_block_pnl: f64,
    pub timestamp: i64,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_risk_event(
    pool: &SqlitePool,
    pair_key: &str,
    timestamp: i64,
    overall_risk: f64,
    overall_level: &str,
    drawdown_state: &str,
    permission: &str,
    losing_streak: i64,
    winning_streak: i64,
    explanation: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO risk_events
         (pair_key, timestamp, overall_risk, overall_level, drawdown_state, permission,
          losing_streak, winning_streak, explanation)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(pair_key)
    .bind(timestamp)
    .bind(overall_risk)
    .bind(overall_level)
    .bind(drawdown_state)
    .bind(permission)
    .bind(losing_streak)
    .bind(winning_streak)
    .bind(explanation)
    .execute(pool)
    .await;
}

/// Most recent persisted risk event for a pair (for hysteresis continuity).
pub async fn latest_risk_event(pool: &SqlitePool, pair_key: &str) -> Option<RiskEventRow> {
    sqlx::query_as::<_, RiskEventRow>(
        "SELECT id, pair_key, timestamp, overall_risk, overall_level, drawdown_state, permission,
                losing_streak, winning_streak, explanation
         FROM risk_events WHERE pair_key = ?1 ORDER BY id DESC LIMIT 1",
    )
    .bind(pair_key)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_rr_calibration(
    pool: &SqlitePool,
    pair_key: &str,
    block_index: i64,
    wins: i64,
    losses: i64,
    win_rate_estimate: f64,
    breakeven_ratio: f64,
    recommended_ratio: f64,
    confidence: f64,
    net_block_pnl: f64,
    timestamp: i64,
) {
    let _ = sqlx::query(
        "INSERT INTO rr_calibration
         (pair_key, block_index, wins, losses, win_rate_estimate, breakeven_ratio,
          recommended_ratio, confidence, net_block_pnl, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .bind(pair_key)
    .bind(block_index)
    .bind(wins)
    .bind(losses)
    .bind(win_rate_estimate)
    .bind(breakeven_ratio)
    .bind(recommended_ratio)
    .bind(confidence)
    .bind(net_block_pnl)
    .bind(timestamp)
    .execute(pool)
    .await;
}

/// Highest block index recorded for a pair (−1 if none).
pub async fn latest_rr_block_index(pool: &SqlitePool, pair_key: &str) -> i64 {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT COALESCE(MAX(block_index), -1) FROM rr_calibration WHERE pair_key = ?1",
    )
    .bind(pair_key)
    .fetch_one(pool)
    .await
    .ok();
    row.map(|r| r.0).unwrap_or(-1)
}

pub async fn latest_rr_calibration(pool: &SqlitePool, pair_key: &str) -> Option<RrCalibrationRow> {
    sqlx::query_as::<_, RrCalibrationRow>(
        "SELECT id, pair_key, block_index, wins, losses, win_rate_estimate, breakeven_ratio,
                recommended_ratio, confidence, net_block_pnl, timestamp
         FROM rr_calibration WHERE pair_key = ?1 ORDER BY block_index DESC LIMIT 1",
    )
    .bind(pair_key)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// Realized PnL of a pair's closed trades ordered oldest→newest. If
/// `lookback > 0` only the most recent `lookback` trades are returned.
pub async fn pair_realized_pnls(pool: &SqlitePool, symbol: &str, lookback: u32) -> Vec<f64> {
    let rows: Vec<(f64,)> = if lookback > 0 {
        sqlx::query_as(
            "SELECT realized_pnl FROM (
                 SELECT realized_pnl, exit_timestamp FROM paper_trades
                 WHERE symbol = ?1 ORDER BY exit_timestamp DESC LIMIT ?2
             ) ORDER BY exit_timestamp ASC",
        )
        .bind(symbol)
        .bind(lookback as i64)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query_as(
            "SELECT realized_pnl FROM paper_trades WHERE symbol = ?1 ORDER BY exit_timestamp ASC",
        )
        .bind(symbol)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };
    rows.into_iter().map(|r| r.0).collect()
}

/// Total number of closed trades for a pair (used for block accounting).
pub async fn pair_trade_count(pool: &SqlitePool, symbol: &str) -> i64 {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT COUNT(*) FROM paper_trades WHERE symbol = ?1")
            .bind(symbol)
            .fetch_one(pool)
            .await
            .ok();
    row.map(|r| r.0).unwrap_or(0)
}

/// Initial capital allocated to a pair, if a paper balance exists.
pub async fn pair_initial_capital(pool: &SqlitePool, symbol: &str) -> Option<f64> {
    let row: Option<(f64,)> =
        sqlx::query_as("SELECT initial_usd FROM paper_balances WHERE symbol = ?1")
            .bind(symbol)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    row.map(|r| r.0)
}

/// Recent OHLC candles for a pair (oldest→newest) for liquidity-proxy scoring.
/// OHLC are stored as TEXT in `market_snapshots`; rows with unparsable values
/// are skipped.
pub async fn pair_recent_ohlc(
    pool: &SqlitePool,
    symbol: &str,
    timeframe_secs: i64,
    limit: i64,
) -> Vec<(f64, f64, f64, f64)> {
    let rows: Vec<(Option<String>, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT open, high, low, close FROM (
                 SELECT open, high, low, close, timestamp FROM market_snapshots
                 WHERE symbol = ?1 AND timeframe_secs = ?2
                 ORDER BY timestamp DESC LIMIT ?3
             ) ORDER BY timestamp ASC",
        )
        .bind(symbol)
        .bind(timeframe_secs)
        .bind(limit)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    rows.into_iter()
        .filter_map(|(o, h, l, c)| {
            let o = o?.parse::<f64>().ok()?;
            let h = h?.parse::<f64>().ok()?;
            let l = l?.parse::<f64>().ok()?;
            let c = c?.parse::<f64>().ok()?;
            Some((o, h, l, c))
        })
        .collect()
}
