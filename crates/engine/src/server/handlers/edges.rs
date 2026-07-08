use crate::edges::types::{EdgeAnalyzeRequest, EdgeSaveRequest};
use crate::server::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_edges_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let pair_key = params
        .get("pair_key")
        .map(|s| s.as_str())
        .unwrap_or("BTC-USDT");

    let edges = crate::db::edges_list(&state.pool, pair_key).await;
    Json(serde_json::json!({ "edges": edges }))
}

pub async fn serve_edges_save(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<EdgeSaveRequest>,
) -> impl IntoResponse {
    if payload.name.trim().is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Edge name is required" })),
        )
            .into_response();
    }

    let config_json =
        match serde_json::to_string(&payload.config) {
            Ok(j) => j,
            Err(e) => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": format!("Invalid config: {}", e) })),
                )
                    .into_response();
            }
        };

    let id = crate::db::edges_insert(
        &state.pool,
        payload.name.trim(),
        &payload.pair_key,
        &payload.description,
        &config_json,
        payload.creator_name.as_deref(),
    )
    .await;

    if id > 0 {
        (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "id": id,
                "message": format!("Edge '{}' saved successfully", payload.name.trim())
            })),
        )
            .into_response()
    } else {
        (
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "Failed to save edge (name may already exist)" })),
        )
            .into_response()
    }
}

pub async fn serve_edges_analyze(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<EdgeAnalyzeRequest>,
) -> impl IntoResponse {
    if payload.edge_id <= 0 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Valid edge_id is required" })),
        )
            .into_response();
    }

    match crate::edges::run_analysis(
        &state.pool,
        payload.edge_id,
        &payload.symbol,
        payload.timeframe_secs,
    )
    .await
    {
        Ok(response) => (axum::http::StatusCode::OK, Json(serde_json::to_value(response).unwrap_or_default())).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn serve_edges_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if id <= 0 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Valid edge id is required" })),
        )
            .into_response();
    }

    let ok = crate::db::edges_delete(&state.pool, id).await;

    if ok {
        (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({ "success": true, "message": format!("Edge {} deleted", id) })),
        )
            .into_response()
    } else {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Edge {} not found", id) })),
        )
            .into_response()
    }
}
