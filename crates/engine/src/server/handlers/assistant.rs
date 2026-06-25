use crate::server::helpers::{default_pair_key, get_active_pair};
use crate::server::types::{
    AssistantRecordsQuery, CostEstimateQuery, CostEstimateResponse, MasterHistoryResponse,
    MasterRecordJson,
};
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_assistant_records(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AssistantRecordsQuery>,
) -> impl IntoResponse {
    let records = match &query.trigger_type {
        Some(t) => crate::db::query_master_records_by_trigger(&state.pool, t, 50).await,
        None => crate::db::query_master_records(&state.pool, 50).await,
    };
    let default_symbol = state
        .config
        .read()
        .await
        .symbols
        .first()
        .cloned()
        .unwrap_or_default();
    let pair_key = default_pair_key(&default_symbol);
    let latest_close = match get_active_pair(&state.workspace, &pair_key).await {
        Some(pair) => pair
            .latest_close_str()
            .await
            .unwrap_or_else(|| "0".to_string()),
        None => "0".to_string(),
    };

    let records_json: Vec<MasterRecordJson> = records
        .into_iter()
        .map(|r| {
            let summary = r.indicator_synthesis_summary.clone();
            MasterRecordJson {
                id: r.id,
                created_at: r.created_at,
                position: r.position,
                entry_price: r.entry_price,
                trend_classification: r.general_trend,
                indicator_alignment: summary.clone(),
                indicator_synthesis_summary: summary,
                recommended_action: r.recommended_action,
                recommendation_rationale: r.recommendation_rationale,
                price_at_analysis: r.price_at_analysis,
                support_levels: r.support_levels,
                resistance_levels: r.resistance_levels,
                symbol: r.symbol,
                trigger_type: r.trigger_type,
            }
        })
        .collect();

    Json(MasterHistoryResponse {
        records: records_json,
        latest_close,
    })
}

pub async fn serve_cost_estimate(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CostEstimateQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    let costs = config.costs.clone();
    let pair_key = query.pair_key.unwrap_or_else(|| {
        let first = config.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    });

    let interval_seconds = config
        .instances
        .get(&pair_key)
        .map(|p| p.automation.interval_seconds)
        .unwrap_or(900);

    const INPUT_TOKENS_PER_INDICATOR: u64 = 1024;
    const OUTPUT_TOKENS_PER_INDICATOR: u64 = 512;
    const NUM_INDICATORS: u64 = 35;
    const INPUT_TOKENS_PHASE2: u64 = 2048;
    const OUTPUT_TOKENS_PHASE2: u64 = 1024;

    let input_tokens_per_run = INPUT_TOKENS_PER_INDICATOR * NUM_INDICATORS + INPUT_TOKENS_PHASE2;
    let output_tokens_per_run = OUTPUT_TOKENS_PER_INDICATOR * NUM_INDICATORS + OUTPUT_TOKENS_PHASE2;

    let runs_per_day = if interval_seconds > 0 {
        86400.0 / interval_seconds as f64
    } else {
        0.0
    };

    let daily_input_tokens = input_tokens_per_run as f64 * runs_per_day;
    let daily_output_tokens = output_tokens_per_run as f64 * runs_per_day;

    let projected_daily_cost = (daily_input_tokens / 1_000_000.0) * costs.price_per_1m_input_tokens
        + (daily_output_tokens / 1_000_000.0) * costs.price_per_1m_output_tokens;

    let usage = state.llm_client.get_token_usage_for_pair(&pair_key);
    let (actual_input, actual_output) = usage.load();
    let actual_total_cost = (actual_input as f64 / 1_000_000.0) * costs.price_per_1m_input_tokens
        + (actual_output as f64 / 1_000_000.0) * costs.price_per_1m_output_tokens;

    let response = CostEstimateResponse {
        price_per_1m_input_tokens: costs.price_per_1m_input_tokens,
        price_per_1m_output_tokens: costs.price_per_1m_output_tokens,
        interval_seconds,
        runs_per_day,
        input_tokens_per_run,
        output_tokens_per_run,
        projected_daily_cost,
        projected_weekly_cost: projected_daily_cost * 7.0,
        projected_monthly_cost: projected_daily_cost * 30.0,
        actual_input_tokens_used: actual_input,
        actual_output_tokens_used: actual_output,
        actual_total_cost,
    };

    Json(response)
}

pub async fn serve_automated_performance(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let records = crate::db::query_automated_performance(&state.pool, 50).await;

    #[derive(Debug, serde::Serialize)]
    struct AutomatedPerformanceJson {
        id: i64,
        master_record_id: i64,
        symbol: String,
        price_at_signal: String,
        price_at_1h: Option<String>,
        price_at_4h: Option<String>,
        price_at_24h: Option<String>,
        direction_correct_1h: Option<bool>,
        direction_correct_4h: Option<bool>,
        direction_correct_24h: Option<bool>,
        created_at: String,
    }

    let records_json: Vec<AutomatedPerformanceJson> = records
        .into_iter()
        .map(|r| AutomatedPerformanceJson {
            id: r.id,
            master_record_id: r.master_record_id,
            symbol: r.symbol,
            price_at_signal: r.price_at_signal,
            price_at_1h: r.price_at_1h,
            price_at_4h: r.price_at_4h,
            price_at_24h: r.price_at_24h,
            direction_correct_1h: r.direction_correct_1h,
            direction_correct_4h: r.direction_correct_4h,
            direction_correct_24h: r.direction_correct_24h,
            created_at: r.created_at,
        })
        .collect();

    Json(records_json)
}

pub async fn serve_historical_recommendations(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            String,
            i64,
            f64,
            f64,
            f64,
            f64,
            f64,
            f64,
            String,
            String,
            String,
        ),
    >(
        "SELECT id, symbol, pair_key, generated_at, trades_analyzed, win_rate, avg_risk_reward, \
         avg_hold_time_minutes, profit_factor, suggested_rr, suggested_sizing_pct, \
         regime_analysis, key_improvements, risk_recommendation \
         FROM historical_recommendations ORDER BY generated_at DESC LIMIT 20",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let recommendations: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "symbol": r.1,
                "pair_key": r.2,
                "generated_at": r.3,
                "trades_analyzed": r.4,
                "win_rate": r.5,
                "avg_risk_reward": r.6,
                "avg_hold_time_minutes": r.7,
                "profit_factor": r.8,
                "suggested_rr": r.9,
                "suggested_sizing_pct": r.10,
                "regime_analysis": r.11,
                "key_improvements": r.12,
                "risk_recommendation": r.13,
            })
        })
        .collect();

    Json(serde_json::json!({
        "recommendations": recommendations,
    }))
}
