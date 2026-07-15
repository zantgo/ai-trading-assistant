use crate::server::types::{ObservabilityBuffersResponse, SystemStatusResponse, WsQuery};
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_system_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let active_pairs_count = state.workspace_count().await;

    let response = SystemStatusResponse {
        connected: true,
        latency_ms: 12,
        journal_mode: "WAL".to_string(),
        total_allocated_margin: 0.0,
        active_pairs_count,
    };

    Json(response)
}

pub async fn serve_observability_buffers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    let symbol = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        cfg.symbols.first().cloned().unwrap_or_default()
    } else {
        query.symbol
    };
    let raw_symbol = symbol
        .split_once(':')
        .map(|(_, s)| s)
        .unwrap_or(&symbol)
        .to_string();

    let recent_decisions: Vec<crate::server::types::DecisionMemoryBufferRow> = Vec::new();

    let completed_trades: Vec<crate::server::types::CompletedTradesBufferRow> = sqlx::query_as(
        "SELECT \
            t.id, t.symbol, t.direction, t.entry_price, t.exit_price, \
            t.realized_pnl, t.roi_percentage as roi_pct, \
            0.0 as execution_score, \
            '' as primary_mistake, \
            t.exit_timestamp as closed_at \
         FROM trade_telemetry_history t \
         WHERE t.symbol = ?1 \
         ORDER BY t.exit_timestamp DESC \
         LIMIT 5",
    )
    .bind(&raw_symbol)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(ObservabilityBuffersResponse {
        symbol: raw_symbol,
        recent_decisions,
        completed_trades,
    })
}
