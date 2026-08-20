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
    // v7.3: the significance treatment comes from `[workspace.analytics]` —
    // the same α / Monte Carlo runs the on-demand evaluator uses.
    let analytics = {
        let ws = state.workspace.config().await;
        performance_analytics::strategy_analytics::AnalyticsParams {
            alpha: ws.analytics.alpha,
            monte_carlo_runs: ws.analytics.monte_carlo_runs,
            min_trades_for_verdict: ws.analytics.min_trades_for_verdict,
        }
    };
    let rows = if let Some(ref pid) = query.policy_id {
        database_storage::query_strategy_analytics_history(
            &state.pool,
            Some(pid),
            query.limit.unwrap_or(50).min(crate::types::API_MAX_LIMIT),
        )
        .await
    } else {
        let on_demand =
            performance_analytics::performance_evaluator::compute_strategy_on_demand(
                &state.pool,
                analytics,
            )
            .await;
        if on_demand.is_empty() {
            database_storage::query_strategy_analytics_history(
                &state.pool,
                None,
                query.limit.unwrap_or(50).min(crate::types::API_MAX_LIMIT),
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
        query.limit.unwrap_or(100).min(crate::types::API_MAX_LIMIT),
    )
    .await;
    Json(rows)
}

pub async fn serve_risk_analytics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let persisted = database_storage::query_risk_analytics_latest(&state.pool).await;
    if let Some(risk) = persisted {
        return Json(risk);
    }
    let risk =
        performance_analytics::performance_evaluator::compute_risk_on_demand(&state.pool).await;
    Json(risk)
}

pub async fn serve_performance_matrix(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let rows = if let Some(ref pid) = query.policy_id {
        database_storage::query_performance_matrix_latest(&state.pool, Some(pid)).await
    } else {
        let on_demand =
            performance_analytics::performance_evaluator::compute_performance_on_demand(
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
        query.limit.unwrap_or(10).min(crate::types::API_MAX_LIMIT),
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

    // Single source of truth — shared with the scheduled optimizer task.
    let report = performance_analytics::strategy_optimizer::build_optimization_report(&trades);
    Json(vec![report])
}

pub async fn serve_trade_analytics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let trades =
        performance_analytics::performance_evaluator::get_trade_analytics(&state.pool).await;
    let filtered: Vec<_> = if let Some(ref pid) = query.policy_id {
        trades
            .into_iter()
            .filter(|t| t.trigger_source == *pid)
            .take(query.limit.unwrap_or(200).min(crate::types::API_MAX_LIMIT) as usize)
            .collect()
    } else {
        trades
            .into_iter()
            .take(query.limit.unwrap_or(200).min(crate::types::API_MAX_LIMIT) as usize)
            .collect()
    };
    Json(filtered)
}

pub async fn serve_performance_summary(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let summaries = if query.policy_id.is_some() {
        let _trades =
            performance_analytics::performance_evaluator::get_trade_analytics(&state.pool).await;
        let mut all =
            performance_analytics::performance_evaluator::compute_performance_summary_on_demand(
                &state.pool,
            )
            .await;
        all.retain(|s| Some(s.setup_type.clone()) == query.policy_id);
        all
    } else {
        performance_analytics::performance_evaluator::compute_performance_summary_on_demand(
            &state.pool,
        )
        .await
    };
    Json(summaries)
}

// ─── PAE L5: Backtest ────────────────────────────────────────────────

/// POST /api/backtest/run — replay recorded decisions through the executor.
#[derive(serde::Deserialize)]
pub struct BacktestRequest {
    pub symbol: String,
    pub timeframe_secs: u64,
    pub from_ms: i64,
    pub to_ms: i64,
    #[serde(default)]
    pub initial_capital: Option<f64>,
}

pub async fn serve_backtest_run(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BacktestRequest>,
) -> impl IntoResponse {
    if payload.symbol.trim().is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "symbol required" })),
        )
            .into_response();
    }

    let workspace = state.workspace.config().await;
    let fees = portfolio_supervisor::paper_trading::FeesConfig {
        maker_fee_pct: workspace.fees.maker_fee_pct,
        taker_fee_pct: workspace.fees.taker_fee_pct,
        funding_rate_8h: workspace.fees.funding_rate_8h,
        simulated_spread_pct: 0.01,
    };
    let cross_leverage = workspace.leverage.cross_leverage;

    let params = performance_analytics::backtest::BacktestParams {
        symbol: payload.symbol,
        timeframe_secs: payload.timeframe_secs,
        from_ms: payload.from_ms,
        to_ms: payload.to_ms,
        initial_capital: payload.initial_capital.unwrap_or(1000.0),
    };

    let result = performance_analytics::backtest::run_backtest(
        &state.pool,
        &params,
        &workspace.minimal_tae,
        &fees,
        cross_leverage,
        performance_analytics::strategy_analytics::AnalyticsParams {
            alpha: workspace.analytics.alpha,
            monte_carlo_runs: workspace.analytics.monte_carlo_runs,
            min_trades_for_verdict: workspace.analytics.min_trades_for_verdict,
        },
    )
    .await;

    let params_json = serde_json::to_string(&result.params).unwrap_or_default();
    let summary_json = serde_json::to_string(&serde_json::json!({
        "total_trades": result.total_trades,
        "win_count": result.win_count,
        "loss_count": result.loss_count,
        "win_rate": result.win_rate,
        "gross_profit": result.gross_profit,
        "gross_loss": result.gross_loss,
        "profit_factor": result.profit_factor,
        "expectancy": result.expectancy,
        "max_drawdown_pct": result.max_drawdown_pct,
    }))
    .unwrap_or_default();
    let stats_json = serde_json::to_string(&result.stats).unwrap_or_default();
    let trades_json = serde_json::to_string(&result.trades).unwrap_or_default();
    let equity_curve_json = serde_json::to_string(&result.equity_curve).unwrap_or_default();

    let backtest_id = database_storage::insert_backtest_run(
        &state.pool,
        &params_json,
        &summary_json,
        &stats_json,
        &trades_json,
        &equity_curve_json,
    )
    .await;

    Json(serde_json::json!({
        "backtest_id": backtest_id,
        "params": result.params,
        "summary": {
            "total_trades": result.total_trades,
            "win_count": result.win_count,
            "loss_count": result.loss_count,
            "win_rate": result.win_rate,
            "gross_profit": result.gross_profit,
            "gross_loss": result.gross_loss,
            "profit_factor": result.profit_factor,
            "expectancy": result.expectancy,
            "max_drawdown_pct": result.max_drawdown_pct,
        },
        "stats": result.stats,
        "trades": result.trades,
        "equity_curve": result.equity_curve,
    }))
    .into_response()
}

/// GET /api/backtest/:id — fetch a persisted run.
pub async fn serve_backtest_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    match database_storage::query_backtest_run(&state.pool, id).await {
        Some((params, summary, stats, trades, equity_curve)) => Json(serde_json::json!({
            "backtest_id": id,
            "params": serde_json::from_str::<serde_json::Value>(&params).unwrap_or_default(),
            "summary": serde_json::from_str::<serde_json::Value>(&summary).unwrap_or_default(),
            "stats": serde_json::from_str::<serde_json::Value>(&stats).unwrap_or_default(),
            "trades": serde_json::from_str::<serde_json::Value>(&trades).unwrap_or_default(),
            "equity_curve": serde_json::from_str::<serde_json::Value>(&equity_curve).unwrap_or_default(),
        }))
        .into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "Backtest run not found").into_response(),
    }
}

/// GET /api/backtest/list?limit=N — recent persisted runs (History tab).
pub async fn serve_backtest_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(20).min(200);
    let rows = database_storage::query_backtest_runs_list(&state.pool, limit).await;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "created_at": r.created_at,
                "params": serde_json::from_str::<serde_json::Value>(&r.params_json).unwrap_or_default(),
                "summary": serde_json::from_str::<serde_json::Value>(&r.summary_json).unwrap_or_default(),
            })
        })
        .collect();
    Json(items)
}

/// GET /api/backtest/coverage — recorded-snapshot coverage per symbol × TF
/// (data availability for the backtest replay source).
pub async fn serve_backtest_coverage(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rows = database_storage::query_backtest_coverage(&state.pool).await;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "symbol": r.symbol,
                "timeframe_secs": r.timeframe_secs,
                "snapshot_count": r.snapshot_count,
                "earliest_ms": r.earliest_ms,
                "latest_ms": r.latest_ms,
            })
        })
        .collect();
    Json(items)
}
