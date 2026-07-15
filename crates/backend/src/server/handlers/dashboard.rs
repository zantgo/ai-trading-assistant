use crate::server::types::StatsQuery;
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_dashboard_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsQuery>,
) -> impl IntoResponse {
    let stats = crate::stats_compiler::compile_dashboard_stats(
        &state.pool,
        query.initial_capital.unwrap_or(10000.0),
    )
    .await;
    Json(stats)
}
