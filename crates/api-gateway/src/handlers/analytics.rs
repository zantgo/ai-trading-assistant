use crate::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct AnalyticsQuery {
    pub policy_id: Option<String>,
    pub limit: Option<u32>,
}

pub async fn serve_strategy_analytics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let rows = if let Some(ref pid) = query.policy_id {
        database_storage::query_strategy_analytics_history(
            &state.pool,
            Some(pid),
            query.limit.unwrap_or(50),
        )
        .await
    } else {
        let on_demand = performance_analytics::performance_evaluator::compute_strategy_on_demand(
            &state.pool,
        )
        .await;
        if on_demand.is_empty() {
            database_storage::query_strategy_analytics_history(
                &state.pool,
                None,
                query.limit.unwrap_or(50),
            )
            .await
        } else {
            on_demand
        }
    };
    Json(rows)
}

pub async fn serve_strategy_analytics_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let rows = database_storage::query_strategy_analytics_history(
        &state.pool,
        query.policy_id.as_deref(),
        query.limit.unwrap_or(100),
    )
    .await;
    Json(rows)
}

pub async fn serve_risk_analytics(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let persisted = database_storage::query_risk_analytics_latest(&state.pool).await;
    if let Some(risk) = persisted {
        return Json(risk);
    }
    let risk = performance_analytics::performance_evaluator::compute_risk_on_demand(&state.pool).await;
    Json(risk)
}

pub async fn serve_performance_matrix(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let rows = if let Some(ref pid) = query.policy_id {
        database_storage::query_performance_matrix_latest(
            &state.pool,
            Some(pid),
        )
        .await
    } else {
        let on_demand = performance_analytics::performance_evaluator::compute_performance_on_demand(
            &state.pool,
        )
        .await;
        if on_demand.is_empty() {
            database_storage::query_performance_matrix_latest(&state.pool, None).await
        } else {
            on_demand
        }
    };
    Json(rows)
}

pub async fn serve_optimization_report(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let persisted = database_storage::query_optimization_reports(
        &state.pool,
        query.limit.unwrap_or(10),
    )
    .await;
    if !persisted.is_empty() {
        return Json(persisted);
    }

    let trades = database_storage::query_all_closed_trades(&state.pool).await;
    if trades.is_empty() {
        let report = core_domain::performance::OptimizationReport {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            total_trades: 0,
            regime_reports: vec![],
            recommendations: vec![],
        };
        return Json(vec![report]);
    }

    let mut by_regime: std::collections::HashMap<String, Vec<&database_storage::ClosedTradeRow>> =
        std::collections::HashMap::new();
    for t in &trades {
        let regime = t
            .market_regime
            .clone()
            .unwrap_or_else(|| "UNKNOWN".to_string());
        by_regime.entry(regime).or_default().push(t);
    }

    let mut regime_reports = Vec::new();
    let mut recommendations = Vec::new();

    for (regime, regime_trades) in &by_regime {
        let wins = regime_trades.iter().filter(|t| t.realized_pnl > 0.0).count();
        let gross_profit: f64 = regime_trades
            .iter()
            .filter(|t| t.realized_pnl > 0.0)
            .map(|t| t.realized_pnl)
            .sum();
        let gross_loss: f64 = regime_trades
            .iter()
            .filter(|t| t.realized_pnl < 0.0)
            .map(|t| t.realized_pnl.abs())
            .sum();
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else {
            f64::INFINITY
        };
        let total_pnl: f64 = regime_trades.iter().map(|t| t.realized_pnl).sum();
        let valid_r: Vec<f64> = regime_trades
            .iter()
            .filter_map(|t| {
                if t.allocated_usd > 0.0 {
                    Some(t.realized_pnl / t.allocated_usd)
                } else {
                    None
                }
            })
            .collect();
        let avg_r = if !valid_r.is_empty() {
            valid_r.iter().sum::<f64>() / valid_r.len() as f64
        } else {
            0.0
        };
        let win_rate = if !regime_trades.is_empty() {
            wins as f64 / regime_trades.len() as f64 * 100.0
        } else {
            0.0
        };

        regime_reports.push(core_domain::performance::RegimePerformanceReport {
            regime: regime.clone(),
            trade_count: regime_trades.len() as i64,
            win_rate,
            profit_factor,
            avg_r_multiple: avg_r,
            total_pnl,
        });

        if win_rate < 35.0 && regime_trades.len() > 5 {
            recommendations.push(format!(
                "REGIME {}: Low win rate ({:.1}%), consider reducing allocation",
                regime, win_rate
            ));
        }
    }

    let report = core_domain::performance::OptimizationReport {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
        total_trades: trades.len() as i64,
        regime_reports,
        recommendations,
    };
    Json(vec![report])
}

pub async fn serve_trade_analytics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let trades = performance_analytics::performance_evaluator::get_trade_analytics(&state.pool).await;
    let filtered: Vec<_> = if let Some(ref pid) = query.policy_id {
        trades
            .into_iter()
            .filter(|t| t.trigger_source == *pid)
            .take(query.limit.unwrap_or(200) as usize)
            .collect()
    } else {
        trades
            .into_iter()
            .take(query.limit.unwrap_or(200) as usize)
            .collect()
    };
    Json(filtered)
}

pub async fn serve_performance_summary(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let summaries = if query.policy_id.is_some() {
        let _trades = performance_analytics::performance_evaluator::get_trade_analytics(&state.pool).await;
        let mut all = performance_analytics::performance_evaluator::compute_performance_summary_on_demand(&state.pool).await;
        all.retain(|s| Some(s.policy_id.clone()) == query.policy_id);
        all
    } else {
        performance_analytics::performance_evaluator::compute_performance_summary_on_demand(&state.pool).await
    };
    Json(summaries)
}
