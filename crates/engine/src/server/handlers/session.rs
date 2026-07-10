use crate::server::types::{SessionInitRequest, SessionStatusResponse};
use crate::server::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use std::sync::Arc;

pub async fn serve_session_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let active = state
        .workspace
        .session
        .active
        .load(std::sync::atomic::Ordering::Relaxed);
    let currency = state.workspace.session.base_currency.read().await.clone();
    let exchange = state.workspace.session.exchange.read().await.clone();
    let instance_count = state.workspace.instance_count().await;
    let max_instances = state.workspace.max_instances().await;

    Json(SessionStatusResponse {
        active,
        currency: currency.map(|c| c.as_str().to_string()),
        exchange: exchange.map(|e| e.as_str().to_string()),
        instance_count,
        max_instances,
    })
}

pub async fn serve_session_init(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SessionInitRequest>,
) -> impl IntoResponse {
    let currency = match payload.currency.to_uppercase().as_str() {
        "USDT" => crate::workspace::Currency::USDT,
        "USDC" => crate::workspace::Currency::USDC,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid currency. Use 'USDT' or 'USDC'.",
            )
                .into_response()
        }
    };

    let exchange = match payload.exchange.to_lowercase().as_str() {
        "hyperliquid" => crate::workspace::ExchangeChoice::Hyperliquid,
        "bitget" => crate::workspace::ExchangeChoice::Bitget,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid exchange. Use 'Hyperliquid' or 'Bitget'.",
            )
                .into_response()
        }
    };

    match state
        .workspace
        .init_session(currency, exchange)
        .await
    {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Session initialized successfully.",
            })),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": e,
            })),
        )
            .into_response(),
    }
}

pub async fn serve_session_quit(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.workspace.quit_session().await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Session terminated. All instances stopped."
            })),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": e,
            })),
        )
            .into_response(),
    }
}
