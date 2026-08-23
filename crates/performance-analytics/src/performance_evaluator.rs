use database_storage;
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use core_domain::performance::{
    PerformanceMatrixRow, PerformanceMatrixSummary, RiskAnalyticsRow, StrategyAnalyticsRow,
    TradeAnalyticsRecord,
};

pub struct EvaluatorConfig {
    pub pool: SqlitePool,
    pub cancel: CancellationToken,
    pub eval_interval_secs: u64,
    /// v10.1: the configured significance-treatment parameters (the live
    /// pipeline previously hardcoded `AnalyticsParams::default()`).
    pub analytics_params: crate::strategy_analytics::AnalyticsParams,
    /// v10.1: annual risk-free rate (percent) from the default strategy's
    /// `pae.risk_math` — subtracted in Sharpe/Sortino numerators.
    pub risk_free_rate_pct: f64,
}

pub async fn run_performance_evaluator(cfg: EvaluatorConfig) {
    println!(
        "📊 Performance Evaluator: Started (interval: {}s)...",
        cfg.eval_interval_secs
    );

    loop {
        tokio::select! {
            biased;
            _ = cfg.cancel.cancelled() => {
                println!("🛑 Performance Evaluator: Terminated.");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(cfg.eval_interval_secs)) => {}
        }

        run_full_analytics_pipeline(&cfg.pool, cfg.analytics_params, cfg.risk_free_rate_pct).await;
    }
}

pub async fn run_full_analytics_pipeline(
    pool: &SqlitePool,
    analytics_params: crate::strategy_analytics::AnalyticsParams,
    risk_free_rate_pct: f64,
) {
    let trades = crate::trade_analytics::reconstruct_trades(pool).await;
    if trades.is_empty() {
        return;
    }

    let risk = crate::risk_analytics::compute_risk_analytics(pool, risk_free_rate_pct).await;
    database_storage::insert_risk_analytics(pool, &risk).await;

    let strategy_rows = crate::strategy_analytics::compute_strategy_analytics(
        pool,
        &trades,
        analytics_params,
    )
    .await;
    for row in &strategy_rows {
        database_storage::insert_strategy_analytics(pool, row).await;
    }

    let performance_rows =
        crate::performance_layer::compute_performance_matrix(pool, &trades).await;
    for row in &performance_rows {
        database_storage::insert_performance_matrix_snapshot(pool, row).await;
    }

    let summaries =
        crate::performance_layer::compute_performance_matrix_summary(pool, &trades).await;
    for summary in &summaries {
        database_storage::insert_performance_summary(pool, summary).await;
    }

    let strategy_count = strategy_rows.len();
    let regime_count = performance_rows.len();
    let summary_count = summaries.len();
    println!(
        "📊 Performance Evaluator: Pipeline complete — {} strategies, {} regime entries, {} performance summaries persisted",
        strategy_count, regime_count, summary_count
    );
}

pub async fn compute_risk_on_demand(
    pool: &SqlitePool,
    risk_free_rate_pct: f64,
) -> RiskAnalyticsRow {
    crate::risk_analytics::compute_risk_analytics(pool, risk_free_rate_pct).await
}

pub async fn compute_strategy_on_demand(
    pool: &SqlitePool,
    params: crate::strategy_analytics::AnalyticsParams,
) -> Vec<StrategyAnalyticsRow> {
    let trades = crate::trade_analytics::reconstruct_trades(pool).await;
    crate::strategy_analytics::compute_strategy_analytics(pool, &trades, params).await
}

pub async fn compute_performance_on_demand(pool: &SqlitePool) -> Vec<PerformanceMatrixRow> {
    let trades = crate::trade_analytics::reconstruct_trades(pool).await;
    crate::performance_layer::compute_performance_matrix(pool, &trades).await
}

pub async fn compute_performance_summary_on_demand(
    pool: &SqlitePool,
) -> Vec<PerformanceMatrixSummary> {
    let trades = crate::trade_analytics::reconstruct_trades(pool).await;
    crate::performance_layer::compute_performance_matrix_summary(pool, &trades).await
}

pub async fn get_trade_analytics(pool: &SqlitePool) -> Vec<TradeAnalyticsRecord> {
    crate::trade_analytics::reconstruct_trades(pool).await
}
