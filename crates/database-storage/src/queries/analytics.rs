use sqlx::SqlitePool;

use core_domain::performance::{PerformanceMatrixRow, StrategyAnalyticsRow};

pub async fn insert_strategy_analytics(
    pool: &SqlitePool,
    row: &StrategyAnalyticsRow,
) -> i64 {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    match sqlx::query(
        "INSERT INTO strategy_analytics_history
         (timestamp, policy_id, total_trades, win_count, loss_count, win_rate,
          gross_profit, gross_loss, profit_factor, average_win, average_loss,
          avg_win_loss_ratio, expectancy, slippage_overhead, t_statistic, p_value,
          p_mc, monte_carlo_runs, is_significant, classification)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
    )
    .bind(ts)
    .bind(&row.policy_id)
    .bind(row.total_trades as i64)
    .bind(row.win_count as i64)
    .bind(row.loss_count as i64)
    .bind(row.win_rate)
    .bind(row.gross_profit)
    .bind(row.gross_loss)
    .bind(row.profit_factor)
    .bind(row.average_win)
    .bind(row.average_loss)
    .bind(row.avg_win_loss_ratio)
    .bind(row.expectancy)
    .bind(row.slippage_overhead)
    .bind(row.t_statistic)
    .bind(row.p_value)
    .bind(row.p_mc)
    .bind(row.monte_carlo_runs as i64)
    .bind(row.is_significant as i64)
    .bind(format!("{:?}", row.classification))
    .execute(pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("DB: Failed to insert strategy analytics: {}", e);
            0
        }
    }
}

pub async fn query_strategy_analytics_history(
    pool: &SqlitePool,
    policy_id: Option<&str>,
    limit: u32,
) -> Vec<StrategyAnalyticsRow> {
    let rows: Result<Vec<_>, _> = if let Some(pid) = policy_id {
        sqlx::query_as::<_, StrategyAnalyticsQueryRow>(
            "SELECT policy_id, total_trades, win_count, loss_count, win_rate,
                    gross_profit, gross_loss, profit_factor, average_win, average_loss,
                    avg_win_loss_ratio, expectancy, slippage_overhead, t_statistic, p_value,
                    p_mc, monte_carlo_runs, is_significant, classification
             FROM strategy_analytics_history
             WHERE policy_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )
        .bind(pid)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, StrategyAnalyticsQueryRow>(
            "SELECT policy_id, total_trades, win_count, loss_count, win_rate,
                    gross_profit, gross_loss, profit_factor, average_win, average_loss,
                    avg_win_loss_ratio, expectancy, slippage_overhead, t_statistic, p_value,
                    p_mc, monte_carlo_runs, is_significant, classification
             FROM strategy_analytics_history
             ORDER BY timestamp DESC
             LIMIT ?1",
        )
        .bind(limit as i64)
        .fetch_all(pool)
        .await
    };

    match rows {
        Ok(r) => r
            .into_iter()
            .map(|r| StrategyAnalyticsRow {
                policy_id: r.policy_id,
                total_trades: r.total_trades as u32,
                win_count: r.win_count as u32,
                loss_count: r.loss_count as u32,
                win_rate: r.win_rate,
                gross_profit: r.gross_profit,
                gross_loss: r.gross_loss,
                profit_factor: r.profit_factor,
                average_win: r.average_win,
                average_loss: r.average_loss,
                avg_win_loss_ratio: r.avg_win_loss_ratio,
                expectancy: r.expectancy,
                slippage_overhead: r.slippage_overhead,
                t_statistic: r.t_statistic,
                p_value: r.p_value,
                p_mc: r.p_mc,
                monte_carlo_runs: r.monte_carlo_runs as u32,
                is_significant: r.is_significant != 0,
                classification: parse_classification(&r.classification),
            })
            .collect(),
        Err(e) => {
            eprintln!("DB: Failed to query strategy analytics history: {}", e);
            vec![]
        }
    }
}

pub async fn insert_performance_matrix_snapshot(
    pool: &SqlitePool,
    row: &PerformanceMatrixRow,
) -> i64 {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    match sqlx::query(
        "INSERT INTO performance_matrix_snapshots
         (timestamp, policy_id, regime, trade_count, win_rate, profit_factor,
          avg_r_multiple, total_pnl, compatibility_label)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(ts)
    .bind(&row.policy_id)
    .bind(&row.regime)
    .bind(row.trade_count as i64)
    .bind(row.win_rate)
    .bind(row.profit_factor)
    .bind(row.avg_r_multiple)
    .bind(row.total_pnl)
    .bind(format!("{:?}", row.compatibility_label))
    .execute(pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("DB: Failed to insert performance matrix snapshot: {}", e);
            0
        }
    }
}

pub async fn query_performance_matrix_latest(
    pool: &SqlitePool,
    policy_id: Option<&str>,
) -> Vec<PerformanceMatrixRow> {
    let result: Result<Vec<_>, _> = if let Some(pid) = policy_id {
        sqlx::query_as::<_, PerformanceMatrixQueryRow>(
            "SELECT policy_id, regime, trade_count, win_rate, profit_factor,
                    avg_r_multiple, total_pnl, compatibility_label
             FROM performance_matrix_snapshots
             WHERE policy_id = ?1
             AND timestamp = (SELECT MAX(timestamp) FROM performance_matrix_snapshots WHERE policy_id = ?1)
             ORDER BY trade_count DESC",
        )
        .bind(pid)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, PerformanceMatrixQueryRow>(
            "SELECT policy_id, regime, trade_count, win_rate, profit_factor,
                    avg_r_multiple, total_pnl, compatibility_label
             FROM performance_matrix_snapshots
             WHERE timestamp = (SELECT MAX(timestamp) FROM performance_matrix_snapshots)
             ORDER BY trade_count DESC",
        )
        .fetch_all(pool)
        .await
    };

    match result {
        Ok(r) => r
            .into_iter()
            .map(|r| PerformanceMatrixRow {
                policy_id: r.policy_id,
                regime: r.regime,
                trade_count: r.trade_count as u32,
                win_rate: r.win_rate,
                profit_factor: r.profit_factor,
                avg_r_multiple: r.avg_r_multiple,
                total_pnl: r.total_pnl,
                compatibility_label: parse_compatibility(&r.compatibility_label),
            })
            .collect(),
        Err(e) => {
            eprintln!("DB: Failed to query performance matrix: {}", e);
            vec![]
        }
    }
}

fn parse_classification(s: &str) -> core_domain::performance::PerformanceClassification {
    match s {
        "StrongEdge" => core_domain::performance::PerformanceClassification::StrongEdge,
        "ModerateEdge" => core_domain::performance::PerformanceClassification::ModerateEdge,
        "WeakMarginalEdge" => core_domain::performance::PerformanceClassification::WeakMarginalEdge,
        "NoEdgeNegative" => core_domain::performance::PerformanceClassification::NoEdgeNegative,
        _ => core_domain::performance::PerformanceClassification::InsufficientData,
    }
}

fn parse_compatibility(s: &str) -> core_domain::performance::RegimeCompatibility {
    match s {
        "Strong" | "STRONG" => core_domain::performance::RegimeCompatibility::Strong,
        "Favorable" | "FAVORABLE" => core_domain::performance::RegimeCompatibility::Favorable,
        "Marginal" | "MARGINAL" => core_domain::performance::RegimeCompatibility::Marginal,
        _ => core_domain::performance::RegimeCompatibility::Avoid,
    }
}

#[derive(Debug, sqlx::FromRow)]
struct StrategyAnalyticsQueryRow {
    policy_id: String,
    total_trades: i64,
    win_count: i64,
    loss_count: i64,
    win_rate: f64,
    gross_profit: f64,
    gross_loss: f64,
    profit_factor: Option<f64>,
    average_win: f64,
    average_loss: f64,
    avg_win_loss_ratio: f64,
    expectancy: f64,
    slippage_overhead: f64,
    t_statistic: f64,
    p_value: f64,
    p_mc: f64,
    monte_carlo_runs: i64,
    is_significant: i64,
    classification: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PerformanceMatrixQueryRow {
    policy_id: String,
    regime: String,
    trade_count: i64,
    win_rate: f64,
    profit_factor: Option<f64>,
    avg_r_multiple: f64,
    total_pnl: f64,
    compatibility_label: String,
}
