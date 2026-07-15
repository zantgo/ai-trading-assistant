use crate::server::AppState;
use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_get_trades(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let trades = crate::db::query_user_trades(&state.pool, 100).await;
    Json(trades)
}
