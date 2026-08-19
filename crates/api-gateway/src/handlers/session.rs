use crate::types::{SessionInitRequest, SessionStatusResponse};
use crate::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use std::sync::Arc;

pub async fn serve_session_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let active = state
        .session
        .active
        .load(std::sync::atomic::Ordering::Relaxed);
    let currency = state.session.base_currency.read().await.clone();
    let exchange = state.session.exchange.read().await.clone();
    let instance_count = state.instance_count().await;
    let mode = state.session.session_mode().await;
    let capital = state.session.session_capital().await;

    Json(SessionStatusResponse {
        active,
        currency: currency.map(|c| c.as_str().to_string()),
        exchange: exchange.map(|e| e.as_str().to_string()),
        instance_count,
        mode,
        capital,
    })
}

pub async fn serve_session_init(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SessionInitRequest>,
) -> impl IntoResponse {
    let currency = match payload.currency.to_uppercase().as_str() {
        "USDT" => portfolio_supervisor::session::Currency::USDT,
        "USDC" => portfolio_supervisor::session::Currency::USDC,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Invalid currency. Use 'USDT' or 'USDC'.",
                })),
            )
                .into_response()
        }
    };

    let exchange = match payload.exchange.to_lowercase().as_str() {
        "hyperliquid" => portfolio_supervisor::session::ExchangeChoice::Hyperliquid,
        "bitget" => portfolio_supervisor::session::ExchangeChoice::Bitget,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Invalid exchange. Use 'Hyperliquid' or 'Bitget'.",
                })),
            )
                .into_response()
        }
    };

    // v7.1 follow-up: mode + paper capital defaults for created instances.
    let mode = match payload.mode.as_deref() {
        None => None,
        Some("paper") | Some("live") => Some(payload.mode.clone().unwrap()),
        Some(other) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Invalid mode '{}'. Use 'paper' or 'live'.", other),
                })),
            )
                .into_response();
        }
    };
    if let Some(cap) = payload.initial_capital_usd {
        if !cap.is_finite() || cap <= 0.0 {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "error": "initial_capital_usd must be a positive number.",
                })),
            )
                .into_response();
        }
    }

    // Live session requires an active key for the chosen exchange.
    if mode.as_deref() == Some("live") {
        let exchange_name = exchange.as_str();
        let key_row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM exchange_keys WHERE exchange = ?1 AND is_active = 1",
        )
        .bind(exchange_name)
        .fetch_one(&state.pool)
        .await;
        let key_count = key_row.map(|r| r.0).unwrap_or(0);
        if key_count == 0 {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!(
                        "Live session requires an active {} API key — add one in Settings → Exchange API Keys (with EXCHANGE_SECRET_KEY set).",
                        exchange_name
                    ),
                })),
            )
                .into_response();
        }
    }

    state
        .session
        .set_session_defaults(mode.clone(), payload.initial_capital_usd)
        .await;

    match state.init_session(currency, exchange).await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Session initialized successfully.",
                "mode": mode,
                "capital": payload.initial_capital_usd,
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
