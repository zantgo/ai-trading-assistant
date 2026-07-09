use crate::server::types::WsQuery;
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_system_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let active_pairs_count = state.workspace.instance_count().await;

    let mut total_allocated_margin = 0.0;
    let instances = state.workspace.instances.read().await;
    for instance in instances.values() {
        if let Some(pos) =
            crate::db::paper_get_active_position(&state.pool, &instance.symbol()).await
        {
            total_allocated_margin += pos.allocated_usd;
        }
    }

    let response = serde_json::json!({
        "connected": true,
        "latency_ms": 12,
        "journal_mode": "WAL",
        "total_allocated_margin": total_allocated_margin,
        "active_pairs_count": active_pairs_count,
    });

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

    let recent_decisions: Vec<crate::db::DecisionMemoryBufferRow> = sqlx::query_as(
        "SELECT id, symbol, timestamp, regime_classification, orchestrator_decision, confidence_score, eight_factor_score, portfolio_risk_pct \
         FROM decision_memory_buffer WHERE symbol = ?1 ORDER BY id DESC LIMIT 5"
    )
    .bind(&raw_symbol)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let completed_trades: Vec<crate::db::CompletedTradesBufferRow> = sqlx::query_as(
        "SELECT \
            t.id, t.symbol, t.direction, t.entry_price, t.exit_price, \
            t.realized_pnl, t.roi_percentage as roi_pct, \
            COALESCE(j.execution_score, 0.0) as execution_score, \
            COALESCE(j.final_analysis, '') as primary_mistake, \
            t.exit_timestamp as closed_at \
         FROM trade_telemetry_history t \
         LEFT JOIN trade_learning_journal j ON t.id = j.trade_id \
         WHERE t.symbol = ?1 \
         ORDER BY t.exit_timestamp DESC \
         LIMIT 5",
    )
    .bind(&raw_symbol)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({
        "symbol": raw_symbol,
        "recent_decisions": recent_decisions,
        "completed_trades": completed_trades,
    }))
}
