use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use config_models::OrderStatus;
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct OverrideReadinessRequest {
    pub new_readiness: String,
}

pub async fn override_readiness(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
    Json(req): Json<OverrideReadinessRequest>,
) -> impl IntoResponse {
    let valid_readiness = ["READY", "FORMING", "WATCH", "STAND_ASIDE"];
    if !valid_readiness.contains(&req.new_readiness.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid readiness. Must be READY, FORMING, WATCH, or STAND_ASIDE"
            })),
        )
            .into_response();
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut orders = state.execution_engine.orders.write().await;

    match orders.get_mut(&order_id) {
        Some(lifecycle) => {
            let current = lifecycle.status;
            if current == OrderStatus::PreDispatch || current == OrderStatus::Pending {
                lifecycle.transitions.push(
                    portfolio_supervisor::execution::state_machine::OrderTransition {
                        from: current,
                        to: current,
                        timestamp_ms: now_ms,
                        metadata: Some(format!(
                            "Gate 2 readiness override: {}",
                            req.new_readiness
                        )),
                    },
                );

                Json(serde_json::json!({
                    "order_id": order_id,
                    "new_readiness": req.new_readiness,
                    "message": "Readiness override applied"
                }))
                .into_response()
            } else {
                (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": format!("Cannot override readiness for order in state {:?}", current)
                    })),
                )
                    .into_response()
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Order not found"
            })),
        )
            .into_response(),
    }
}
