use crate::backtest::engine::{BacktestConfig, BacktestEngine};
use crate::backtest::walk_forward::{ParameterRange, WalkForwardConfig, WalkForwardOptimizer};
use crate::server::AppState;
use axum::{response::IntoResponse, Json};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct BacktestRunRequest {
    pub symbol: String,
    pub start_time: i64,
    pub end_time: i64,
    pub initial_capital: Option<f64>,
    #[serde(default)]
    pub slippage_pct: Option<f64>,
    #[serde(default)]
    pub commission_pct: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct WalkForwardRequest {
    pub symbol: String,
    pub start_time: i64,
    pub end_time: i64,
    #[serde(default)]
    pub initial_capital: Option<f64>,
    #[serde(default)]
    pub parameters: Vec<ParameterRangeRequest>,
}

#[derive(Debug, Deserialize)]
pub struct ParameterRangeRequest {
    pub name: String,
    pub values: Vec<f64>,
}

pub async fn run_backtest(
    state: axum::extract::State<Arc<AppState>>,
    Json(payload): Json<BacktestRunRequest>,
) -> impl IntoResponse {
    let config = BacktestConfig {
        initial_capital: payload.initial_capital.unwrap_or(10000.0),
        slippage_pct: payload.slippage_pct.unwrap_or(0.05),
        commission_pct: payload.commission_pct.unwrap_or(0.04),
        risk_free_rate: 0.02,
        quorum_threshold: 60.0,
    };

    let mut engine = BacktestEngine::new(
        state.pool.clone(),
        payload.symbol.clone(),
        payload.start_time,
        payload.end_time,
        config,
    );

    match engine.run().await {
        Ok(result) => Json(serde_json::json!({ "success": true, "data": result })).into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e })),
        )
            .into_response(),
    }
}

pub async fn run_walk_forward(
    state: axum::extract::State<Arc<AppState>>,
    Json(payload): Json<WalkForwardRequest>,
) -> impl IntoResponse {
    let config = BacktestConfig {
        initial_capital: payload.initial_capital.unwrap_or(10000.0),
        slippage_pct: 0.05,
        commission_pct: 0.04,
        risk_free_rate: 0.02,
        quorum_threshold: 60.0,
    };

    let param_grid: Vec<ParameterRange> = payload
        .parameters
        .into_iter()
        .map(|p| ParameterRange {
            name: p.name,
            values: p.values,
        })
        .collect();

    let mut optimizer = WalkForwardOptimizer::new(
        state.pool.clone(),
        payload.symbol.clone(),
        payload.start_time,
        payload.end_time,
        param_grid,
    )
    .with_backtest_config(config)
    .with_walk_forward_config(WalkForwardConfig::default());

    match optimizer.optimize().await {
        Ok(result) => Json(serde_json::json!({ "success": true, "data": result })).into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e })),
        )
            .into_response(),
    }
}
