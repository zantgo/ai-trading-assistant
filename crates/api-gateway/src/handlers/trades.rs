use crate::types::{
    AddTradeRequest, TradeJournalQuery, TradeLedgerQuery, TradeTelemetryRequest,
    UpdateJournalNotesRequest,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::header,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_add_trade(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddTradeRequest>,
) -> impl IntoResponse {
    let outcome_upper = payload.outcome.trim().to_uppercase();
    if outcome_upper != "WIN" && outcome_upper != "LOSS" {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "Outcome must be WIN or LOSS",
        )
            .into_response();
    }

    match database_storage::insert_user_trade(
        &state.pool,
        &payload.symbol,
        &payload.direction,
        &outcome_upper,
        payload.risk_multiplier,
        payload.reward_multiplier,
    )
    .await
    {
        Ok(id) => (
            axum::http::StatusCode::CREATED,
            format!("Trade logged with ID {}", id),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Web API Error: Failed to log trade record: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to persist trade record",
            )
                .into_response()
        }
    }
}

pub async fn serve_get_trades(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let trades = database_storage::query_user_trades(&state.pool, 100).await;
    Json(trades)
}

// ─── Trade Journal ───────────────────────────────────────────────

pub async fn serve_trade_journal(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TradeJournalQuery>,
) -> impl IntoResponse {
    let records = database_storage::query_trade_journal(&state.pool, query.limit).await;
    Json(records)
}

pub async fn serve_update_journal_notes(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateJournalNotesRequest>,
) -> impl IntoResponse {
    let score = payload.execution_score.clamp(0.0, 10.0);
    let ok = database_storage::update_journal_notes(&state.pool, id, &payload.human_notes, score).await;
    if ok {
        (axum::http::StatusCode::OK, "Journal notes updated").into_response()
    } else {
        (
            axum::http::StatusCode::NOT_FOUND,
            "Journal record not found",
        )
            .into_response()
    }
}

pub async fn serve_export_journal_csv(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let records = database_storage::query_trade_journal(&state.pool, 1000).await;
    let mut csv = String::from("id,trade_id,entry_date,exit_date,asset,direction,entry_reason,roe_percentage,final_analysis,execution_score,human_notes,symbol,realized_pnl,roi_percentage\n");
    for r in &records {
        let escaped_analysis = r.final_analysis.replace('"', "\"\"");
        let escaped_reason = r.entry_reason.replace('"', "\"\"");
        let escaped_notes = r.human_notes.replace('"', "\"\"");
        csv.push_str(&format!(
            "{},{},{},{},{},{},\"{}\",{:.2},\"{}\",{:.1},\"{}\",{},{:.2},{:.2}\n",
            r.id,
            r.trade_id,
            r.entry_date,
            r.exit_date,
            r.asset,
            r.direction,
            escaped_reason,
            r.roe_percentage,
            escaped_analysis,
            r.execution_score,
            escaped_notes,
            r.symbol,
            r.realized_pnl,
            r.roi_percentage,
        ));
    }
    ([(header::CONTENT_TYPE, "text/csv; charset=utf-8")], csv)
}

pub async fn serve_export_journal_json(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let records = database_storage::query_trade_journal(&state.pool, 1000).await;
    Json(records)
}

// ─── Trade Ledger ─────────────────────────────────────────────────

pub async fn serve_trade_ledger(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TradeLedgerQuery>,
) -> impl IntoResponse {
    let trades = database_storage::trade_telemetry_query_all(&state.pool, query.limit).await;
    Json(trades)
}

// ─── Trade Telemetry ──────────────────────────────────────────────

pub async fn serve_trade_telemetry_add(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TradeTelemetryRequest>,
) -> impl IntoResponse {
    let id = database_storage::trade_telemetry_insert(
        &state.pool,
        &payload.exchange,
        &payload.symbol,
        &payload.direction,
        payload.entry_timestamp,
        payload.exit_timestamp,
        payload.entry_price,
        payload.exit_price,
        payload.size,
        payload.commission_fees,
        payload.funding_fees,
        payload.realized_pnl,
        payload.roi_percentage,
        &payload.trigger_source,
    )
    .await;
    if id > 0 {
        (
            axum::http::StatusCode::CREATED,
            format!("Trade logged with id {}", id),
        )
            .into_response()
    } else {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to log trade",
        )
            .into_response()
    }
}
