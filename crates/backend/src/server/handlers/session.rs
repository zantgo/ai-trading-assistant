use crate::server::types::{SessionInitRequest, SessionStatusResponse};
use crate::server::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use std::sync::Arc;

pub async fn serve_session_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let active = state
        .session
        .active
        .load(std::sync::atomic::Ordering::Relaxed);
    let currency = state.session.base_currency.read().await.clone();
    let exchange = state.session.exchange.read().await.clone();
    let workspace_count = state.workspace_count().await;

    Json(SessionStatusResponse {
        active,
        currency: currency.map(|c| c.as_str().to_string()),
        exchange: exchange.map(|e| e.as_str().to_string()),
        workspace_count,
    })
}

pub async fn serve_session_init(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SessionInitRequest>,
) -> impl IntoResponse {
    let currency = match payload.currency.to_uppercase().as_str() {
        "USDT" => crate::session::Currency::USDT,
        "USDC" => crate::session::Currency::USDC,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid currency. Use 'USDT' or 'USDC'.",
            )
                .into_response()
        }
    };

    let exchange = match payload.exchange.to_lowercase().as_str() {
        "hyperliquid" => crate::session::ExchangeChoice::Hyperliquid,
        "bitget" => crate::session::ExchangeChoice::Bitget,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid exchange. Use 'Hyperliquid' or 'Bitget'.",
            )
                .into_response()
        }
    };

    match state
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
    match state.quit_session().await {
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
