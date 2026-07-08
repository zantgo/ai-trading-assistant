use crate::services::analyzer::{AnalysisRequest, AnalysisService};
use crate::server::helpers::{default_pair_key, get_active_pair};
use crate::server::types::AnalyzeRequest;
use crate::server::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use shared::TriggerType;
use std::sync::Arc;

pub async fn serve_analyze(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AnalyzeRequest>,
) -> impl IntoResponse {
    let symbol = if payload.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    } else {
        payload.symbol.clone()
    };

    let last_close = match get_active_pair(&state.workspace, &symbol).await {
        Some(pair) => pair
            .latest_close_str()
            .await
            .unwrap_or_else(|| "0".to_string()),
        None => "0".to_string(),
    };
    let last_close_f: f64 = last_close.parse().unwrap_or(0.0);

    let master_id = crate::db::insert_master_placeholder(
        &state.pool,
        &payload.position,
        &payload.entry_price,
        &last_close,
        &symbol,
        TriggerType::Manual,
    )
    .await;

    let service = AnalysisService::from_app_state(&state);

    if !service.has_api_key().await {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "AI Assistant API Key is not configured. Please configure your key in settings."
            })),
        )
            .into_response();
    }

    let req = AnalysisRequest {
        symbol,
        position: payload.position,
        entry_price: payload.entry_price,
        historical_prices: payload.historical_prices,
        indicators: payload.indicators,
        timeframes: payload.timeframes,
        master_id,
        last_close: last_close_f,
    };

    match service.run_analysis(req).await {
        Ok(response) => (axum::http::StatusCode::OK, Json(response)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}
