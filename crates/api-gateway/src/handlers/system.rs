use crate::types::{DecisionMemoryBufferRow, ObservabilityBuffersResponse, SystemStatusResponse, WsQuery};
use crate::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_system_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let active_pairs_count = state.instance_count().await;
    let lat = state.latency_tracker.snapshot();

    let report = state.exchange_status.report().await;
    let connected = report.exchanges.iter().any(|e| matches!(e.state, network_adapters::exchange_status_tracker::ExchangeConnectionState::Connected));

    let capital = state.execution_engine.capital.read().await;
    let total_allocated_margin = capital.reserved_margin.to_string().parse::<f64>().unwrap_or(0.0);

    let response = SystemStatusResponse {
        connected,
        latency_ms: lat.observation_loop_latency_ms,
        ingest_skew_ms: lat.ingest_skew_ms,
        observation_loop_latency_ms: lat.observation_loop_latency_ms,
        system_heartbeat_latency_ms: lat.system_heartbeat_latency_ms,
        journal_mode: "WAL".to_string(),
        total_allocated_margin,
        active_pairs_count,
    };

    Json(response)
}

pub async fn serve_observability_buffers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    let symbol = if query.symbol.is_empty() {
        let _cfg = state.platform.read().await;
        state.workspace.config().await.declared_symbols().first().cloned().unwrap_or_default()
    } else {
        query.symbol
    };
    let raw_symbol = symbol
        .split_once(':')
        .map(|(_, s)| s)
        .unwrap_or(&symbol)
        .to_string();

    let overview = state.overview.read().await;
    let recent_decisions: Vec<DecisionMemoryBufferRow> = overview
        .as_ref()
        .map(|o| {
            o.asset_ranking
                .iter()
                .take(10)
                .map(|rank| DecisionMemoryBufferRow {
                    id: 0,
                    symbol: rank.symbol.clone(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    regime_classification: rank.regime.clone(),
                    orchestrator_decision: rank.bias.clone(),
                    confidence_score: rank.confidence,
                    eight_factor_score: (rank.score * 8.0) as i32,
                    portfolio_risk_pct: 0.0,
                })
                .collect()
        })
        .unwrap_or_default();

    let completed_trades: Vec<crate::types::CompletedTradesBufferRow> = sqlx::query_as(
        "SELECT \
            t.id, t.symbol, t.direction, t.entry_price, t.exit_price, \
            t.realized_pnl, t.roi_pct, \
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

    Json(ObservabilityBuffersResponse {
        symbol: raw_symbol,
        recent_decisions,
        completed_trades,
    })
}
