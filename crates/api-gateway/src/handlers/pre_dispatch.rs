use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use config_models::OrderStatus;
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct PreDispatchOrder {
    pub order_id: String,
    pub symbol: String,
    pub direction: String,
    pub order_type: String,
    pub size: String,
    pub price: Option<String>,
    pub created_at: u64,
    pub reason: String,
}

pub async fn list_pre_dispatch(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let orders = state.execution_engine.orders.read().await;
    let pre_dispatch: Vec<PreDispatchOrder> = orders
        .iter()
        .filter(|(_, lifecycle)| lifecycle.status == OrderStatus::PreDispatch)
        .map(|(id, lifecycle)| PreDispatchOrder {
            order_id: id.clone(),
            symbol: lifecycle.packet.symbol.clone(),
            direction: format!("{:?}", lifecycle.packet.side),
            order_type: format!("{:?}", lifecycle.packet.order_type),
            size: lifecycle.packet.size.to_string(),
            price: lifecycle.packet.price.map(|p| p.to_string()),
            created_at: lifecycle.created_at,
            reason: lifecycle
                .transitions
                .last()
                .and_then(|t| t.metadata.clone())
                .unwrap_or_default(),
        })
        .collect();
    Json(serde_json::json!({
        "orders": pre_dispatch,
        "count": pre_dispatch.len(),
    }))
    .into_response()
}

pub async fn approve_pre_dispatch(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
) -> impl IntoResponse {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut orders = state.execution_engine.orders.write().await;
    let lifecycle = match orders.get_mut(&order_id) {
        Some(lifecycle) if lifecycle.status == OrderStatus::PreDispatch => lifecycle,
        Some(_) => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Order is not in PreDispatch state"
                })),
            )
                .into_response();
        }
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Order not found"
                })),
            )
                .into_response();
        }
    };

    lifecycle.status = OrderStatus::Pending;
    lifecycle.transitions.push(
        portfolio_supervisor::execution::state_machine::OrderTransition {
            from: OrderStatus::PreDispatch,
            to: OrderStatus::Pending,
            timestamp_ms: now_ms,
            metadata: Some("Approved via API override".into()),
        },
    );

    Json(serde_json::json!({
        "order_id": order_id,
        "status": "PENDING",
        "message": "Order approved and moved to Pending queue",
    }))
    .into_response()
}

pub async fn discard_pre_dispatch(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
) -> impl IntoResponse {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut orders = state.execution_engine.orders.write().await;
    let found = {
        let lifecycle = orders.get_mut(&order_id);
        match lifecycle {
            Some(lifecycle) if lifecycle.status == OrderStatus::PreDispatch => {
                lifecycle.transitions.push(
                    portfolio_supervisor::execution::state_machine::OrderTransition {
                        from: OrderStatus::PreDispatch,
                        to: OrderStatus::Rejected,
                        timestamp_ms: now_ms,
                        metadata: Some("Discarded via API".into()),
                    },
                );
                lifecycle.status = OrderStatus::Rejected;
                true
            }
            Some(_) => {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "Order is not in PreDispatch state"
                    })),
                )
                    .into_response();
            }
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": "Order not found"
                    })),
                )
                    .into_response();
            }
        }
    };

    if found {
        orders.remove(&order_id);
    }

    Json(serde_json::json!({
        "order_id": order_id,
        "status": "discarded",
    }))
    .into_response()
}
