use sqlx::SqlitePool;

use core_domain::performance::{
    OptimizationReport, PerformanceMatrixRow, PerformanceMatrixSummary, RiskAnalyticsRow,
    StrategyAnalyticsRow,
};

pub async fn insert_strategy_analytics(pool: &SqlitePool, row: &StrategyAnalyticsRow) -> i64 {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    match sqlx::query(
        "INSERT INTO strategy_analytics_history
         (timestamp, setup_type, total_trades, win_count, loss_count, win_rate,
          gross_profit, gross_loss, profit_factor, average_win, average_loss,
          avg_win_loss_ratio, expectancy, slippage_overhead, t_statistic, p_value,
          p_mc, monte_carlo_runs, is_significant, classification)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
    )
    .bind(ts)
    .bind(&row.setup_type)
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
            "SELECT setup_type, total_trades, win_count, loss_count, win_rate,
                    gross_profit, gross_loss, profit_factor, average_win, average_loss,
                    avg_win_loss_ratio, expectancy, slippage_overhead, t_statistic, p_value,
                    p_mc, monte_carlo_runs, is_significant, classification
             FROM strategy_analytics_history
             WHERE setup_type = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )
        .bind(pid)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, StrategyAnalyticsQueryRow>(
            "SELECT setup_type, total_trades, win_count, loss_count, win_rate,
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
                setup_type: r.setup_type,
                alpha: 0.05,
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
         (timestamp, setup_type, regime, trade_count, win_rate, profit_factor,
          avg_r_multiple, total_pnl, compatibility_label)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(ts)
    .bind(&row.setup_type)
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
             WHERE setup_type = ?1
             AND timestamp = (SELECT MAX(timestamp) FROM performance_matrix_snapshots WHERE setup_type = ?1)
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
                setup_type: r.setup_type,
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

pub async fn insert_risk_analytics(pool: &SqlitePool, row: &RiskAnalyticsRow) -> i64 {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    match sqlx::query(
        "INSERT INTO risk_analytics_history
         (timestamp, maximum_drawdown_pct, max_drawdown_duration_days,
          average_drawdown_pct, drawdown_count, sharpe_ratio, sortino_ratio,
          ulcer_index, calmar_ratio, daily_volatility, downside_deviation,
          value_at_risk_95, expected_shortfall_95)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )
    .bind(ts)
    .bind(row.maximum_drawdown_pct)
    .bind(row.max_drawdown_duration_days)
    .bind(row.average_drawdown_pct)
    .bind(row.drawdown_count as i64)
    .bind(row.sharpe_ratio)
    .bind(row.sortino_ratio)
    .bind(row.ulcer_index)
    .bind(row.calmar_ratio)
    .bind(row.daily_volatility)
    .bind(row.downside_deviation)
    .bind(row.value_at_risk_95)
    .bind(row.expected_shortfall_95)
    .execute(pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("DB: Failed to insert risk analytics: {}", e);
            0
        }
    }
}

pub async fn query_risk_analytics_latest(pool: &SqlitePool) -> Option<RiskAnalyticsRow> {
    let row = sqlx::query_as::<_, RiskAnalyticsQueryRow>(
        "SELECT maximum_drawdown_pct, max_drawdown_duration_days, average_drawdown_pct,
                drawdown_count, sharpe_ratio, sortino_ratio, ulcer_index, calmar_ratio,
                daily_volatility, downside_deviation, value_at_risk_95, expected_shortfall_95
         FROM risk_analytics_history
         ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(r)) => Some(RiskAnalyticsRow {
            maximum_drawdown_pct: r.maximum_drawdown_pct,
            max_drawdown_duration_days: r.max_drawdown_duration_days,
            average_drawdown_pct: r.average_drawdown_pct,
            drawdown_count: r.drawdown_count as u32,
            sharpe_ratio: r.sharpe_ratio,
            sortino_ratio: r.sortino_ratio,
            ulcer_index: r.ulcer_index,
            calmar_ratio: r.calmar_ratio,
            daily_volatility: r.daily_volatility,
            downside_deviation: r.downside_deviation,
            value_at_risk_95: r.value_at_risk_95,
            expected_shortfall_95: r.expected_shortfall_95,
        }),
        _ => None,
    }
}

pub async fn insert_performance_summary(
    pool: &SqlitePool,
    summary: &PerformanceMatrixSummary,
) -> i64 {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    match sqlx::query(
        "INSERT INTO performance_matrix_summaries
         (timestamp, setup_type, total_trades, overall_profit_factor,
          overall_expectancy, overall_sharpe, overall_sortino,
          max_drawdown_pct, regime_strength_json, recommendations_json,
          overall_rating, last_evaluated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(ts)
    .bind(&summary.setup_type)
    .bind(summary.total_trades as i64)
    .bind(summary.overall_profit_factor)
    .bind(summary.overall_expectancy)
    .bind(summary.overall_sharpe)
    .bind(summary.overall_sortino)
    .bind(summary.max_drawdown_pct)
    .bind(serde_json::to_string(&summary.regime_strength_summary).unwrap_or_default())
    .bind(serde_json::to_string(&summary.optimization_recommendations).unwrap_or_default())
    .bind(format!("{:?}", summary.overall_rating))
    .bind(summary.last_evaluated_at)
    .execute(pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("DB: Failed to insert performance summary: {}", e);
            0
        }
    }
}

pub async fn insert_optimization_report(pool: &SqlitePool, report: &OptimizationReport) -> i64 {
    match sqlx::query(
        "INSERT INTO optimization_reports
         (timestamp, total_trades, regime_reports_json, recommendations_json)
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(report.timestamp)
    .bind(report.total_trades)
    .bind(serde_json::to_string(&report.regime_reports).unwrap_or_default())
    .bind(serde_json::to_string(&report.recommendations).unwrap_or_default())
    .execute(pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("DB: Failed to insert optimization report: {}", e);
            0
        }
    }
}

pub async fn query_optimization_reports(pool: &SqlitePool, limit: u32) -> Vec<OptimizationReport> {
    let rows: Result<Vec<OptimizationReportQueryRow>, _> = sqlx::query_as(
        "SELECT timestamp, total_trades, regime_reports_json, recommendations_json
         FROM optimization_reports
         ORDER BY timestamp DESC
         LIMIT ?1",
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|r| OptimizationReport {
                timestamp: r.timestamp,
                total_trades: r.total_trades,
                regime_reports: serde_json::from_str(&r.regime_reports_json).unwrap_or_default(),
                recommendations: serde_json::from_str(&r.recommendations_json).unwrap_or_default(),
            })
            .collect(),
        Err(e) => {
            eprintln!("DB: Failed to query optimization reports: {}", e);
            vec![]
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct StrategyAnalyticsQueryRow {
    setup_type: String,
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
    setup_type: String,
    regime: String,
    trade_count: i64,
    win_rate: f64,
    profit_factor: Option<f64>,
    avg_r_multiple: f64,
    total_pnl: f64,
    compatibility_label: String,
}

#[derive(Debug, sqlx::FromRow)]
struct RiskAnalyticsQueryRow {
    maximum_drawdown_pct: f64,
    max_drawdown_duration_days: f64,
    average_drawdown_pct: f64,
    drawdown_count: i64,
    sharpe_ratio: Option<f64>,
    sortino_ratio: Option<f64>,
    ulcer_index: f64,
    calmar_ratio: Option<f64>,
    daily_volatility: f64,
    downside_deviation: f64,
    value_at_risk_95: f64,
    expected_shortfall_95: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct OptimizationReportQueryRow {
    timestamp: i64,
    total_trades: i64,
    regime_reports_json: String,
    recommendations_json: String,
}

// ─── PAE L5: backtest_runs ───────────────────────────────────────────

/// Persist a backtest run; returns the new row id.
pub async fn insert_backtest_run(
    pool: &SqlitePool,
    params_json: &str,
    summary_json: &str,
    stats_json: &str,
    trades_json: &str,
    equity_curve_json: &str,
) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    match sqlx::query(
        "INSERT INTO backtest_runs \
         (params_json, summary_json, stats_json, trades_json, equity_curve_json, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(params_json)
    .bind(summary_json)
    .bind(stats_json)
    .bind(trades_json)
    .bind(equity_curve_json)
    .bind(now)
    .execute(pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("DB: Failed to insert backtest run: {}", e);
            0
        }
    }
}

/// Load a persisted backtest run: `(params, summary, stats, trades, equity_curve)`.
pub async fn query_backtest_run(
    pool: &SqlitePool,
    id: i64,
) -> Option<(String, String, String, String, String)> {
    sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT params_json, summary_json, stats_json, trades_json, equity_curve_json \
         FROM backtest_runs WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// One list row for the Backtest History tab: id, run timestamp, params and
/// the headline summary. The summary JSON is carried as-is so the frontend
/// renders the same numbers the run produced without re-querying each row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BacktestRunRow {
    pub id: i64,
    pub created_at: i64,
    pub params_json: String,
    pub summary_json: String,
}

/// List persisted backtest runs, newest first — the History tab source.
pub async fn query_backtest_runs_list(pool: &SqlitePool, limit: u32) -> Vec<BacktestRunRow> {
    sqlx::query_as::<_, BacktestRunRow>(
        "SELECT id, created_at, params_json, summary_json \
         FROM backtest_runs ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}
