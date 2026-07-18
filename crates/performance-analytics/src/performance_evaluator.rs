use database_storage;
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use core_domain::performance::{
    RiskAnalyticsRow, StrategyAnalyticsRow, PerformanceMatrixRow, TradeAnalyticsRecord,
};

pub struct EvaluatorConfig {
    pub pool: SqlitePool,
    pub cancel: CancellationToken,
    pub eval_interval_secs: u64,
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

        run_full_analytics_pipeline(&cfg.pool).await;
    }
}

pub async fn run_full_analytics_pipeline(pool: &SqlitePool) {
    let trades = crate::trade_analytics::reconstruct_trades(pool).await;
    if trades.is_empty() {
        return;
    }

    let strategy_rows = crate::strategy_analytics::compute_strategy_analytics(pool, &trades).await;
    for row in &strategy_rows {
        database_storage::insert_strategy_analytics(pool, row).await;
    }

    let performance_rows = crate::performance_layer::compute_performance_matrix(pool, &trades).await;
    for row in &performance_rows {
        database_storage::insert_performance_matrix_snapshot(pool, row).await;
    }

    let strategy_count = strategy_rows.len();
    let regime_count = performance_rows.len();
    println!(
        "📊 Performance Evaluator: Pipeline complete — {} strategies, {} regime entries persisted",
        strategy_count, regime_count
    );
}

pub async fn compute_risk_on_demand(pool: &SqlitePool) -> RiskAnalyticsRow {
    crate::risk_analytics::compute_risk_analytics(pool).await
}

pub async fn compute_strategy_on_demand(pool: &SqlitePool) -> Vec<StrategyAnalyticsRow> {
    let trades = crate::trade_analytics::reconstruct_trades(pool).await;
    crate::strategy_analytics::compute_strategy_analytics(pool, &trades).await
}

pub async fn compute_performance_on_demand(pool: &SqlitePool) -> Vec<PerformanceMatrixRow> {
    let trades = crate::trade_analytics::reconstruct_trades(pool).await;
    crate::performance_layer::compute_performance_matrix(pool, &trades).await
}

pub async fn get_trade_analytics(pool: &SqlitePool) -> Vec<TradeAnalyticsRecord> {
    crate::trade_analytics::reconstruct_trades(pool).await
}
