use crate::server::types::ExchangeKeyRequest;
use crate::server::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

pub async fn serve_exchange_keys_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match crate::db::exchange_keys_list(&state.pool).await {
        Ok(keys) => {
            let active_count = crate::db::exchange_keys_active_count(&state.pool).await;
            #[derive(Serialize)]
            struct ExchangeKeysResponse {
                accounts: Vec<crate::db::ExchangeKey>,
                active_count: i64,
                max_accounts: i64,
            }
            Json(ExchangeKeysResponse {
                accounts: keys,
                active_count,
                max_accounts: 3,
            })
            .into_response()
        }
        Err(e) => {
            eprintln!("Failed to list exchange keys: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
        }
    }
}

pub async fn serve_exchange_keys_add(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ExchangeKeyRequest>,
) -> impl IntoResponse {
    if payload.exchange.is_empty() || payload.account_name.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "Exchange and account name required",
        )
            .into_response();
    }
    match crate::db::exchange_keys_insert(
        &state.pool,
        &payload.exchange,
        &payload.account_name,
        &payload.api_key,
        &payload.api_secret,
        &payload.passphrase,
        &payload.referred_uid,
        payload.is_active,
    )
    .await
    {
        Ok(id) => (
            axum::http::StatusCode::CREATED,
            format!("Exchange key created with id {}", id),
        )
            .into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub async fn serve_exchange_keys_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    crate::db::exchange_keys_delete(&state.pool, id).await;
    (axum::http::StatusCode::OK, "Exchange key deleted").into_response()
}

pub async fn serve_exchange_keys_sync(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    crate::db::exchange_keys_update_sync(&state.pool, id, now).await;
    (axum::http::StatusCode::OK, "Sync timestamp updated").into_response()
}
