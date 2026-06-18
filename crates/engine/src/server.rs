use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use axum::{
    extract::{Path, State, WebSocketUpgrade, Query},
    extract::ws::{WebSocket, Message as AxumMessage},
    http::header,
    response::{IntoResponse, Redirect},
    routing::{delete, get, post},
    Router, Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc};
use sqlx::SqlitePool;
use tower_http::services::ServeDir;
use tower_http::cors::{CorsLayer, Any};
use shared::normalized::SymbolMapper;
use shared::TriggerType;
use shared::jsonrpc::JsonRpcNotification;
use crate::config::AppConfig;
use crate::analyzer;
use crate::llm::{LlmClient, ChatMessage, IndividualIndicatorResult, MultiAgentResults};
use crate::profile_evaluation::classify_market_regime;
use crate::db::{DecisionMemoryBufferRow, CompletedTradesBufferRow};

use crate::workspace::Workspace;
use crate::instance_registry;

pub struct AppState {
    pub workspace: Arc<Workspace>,
    pub config: Arc<RwLock<AppConfig>>,
    pub pool: SqlitePool,
    pub llm_client: Arc<RwLock<LlmClient>>,
    pub api_key_configured: Arc<AtomicBool>,
    pub symbol_mapper: Arc<SymbolMapper>,
    pub telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
    pub ws_url: String,
}

async fn get_active_pair(workspace: &Workspace, pair_key: &str) -> Option<Arc<analyzer::ActivePair>> {
    workspace.instances.read().await
        .get(pair_key)
        .map(|inst| inst.active_pair.clone())
}

/// Extract base symbol from a pair_key (e.g., "BTC-USDT" -> "BTC")
fn extract_base_symbol(pair_key: &str) -> String {
    pair_key.split('-').next().unwrap_or(pair_key).to_string()
}

/// Build pair_key from config symbol (e.g., config "Hyperliquid:BTC" or "BTC" -> "BTC-USDT")
fn default_pair_key(symbol_entry: &str) -> String {
    let raw = symbol_entry.split_once(':').map(|(_, s)| s).unwrap_or(symbol_entry);
    format!("{}-USDT", raw)
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub position: String,
    #[serde(default)]
    pub entry_price: String,
    pub historical_prices: Vec<f64>,
    pub indicators: IndicatorSnapshot,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timeframes: Option<MultiTimeframeIndicators>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MultiTimeframeIndicators {
    pub micro_term: IndicatorSnapshot,
    pub short_term: IndicatorSnapshot,
    #[serde(default)]
    pub medium_term: Option<IndicatorSnapshot>,
    #[serde(default)]
    pub large_term: Option<IndicatorSnapshot>,
}

#[derive(Debug, Deserialize)]
pub struct SetKeyRequest {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct SetRulesRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct RulesResponse {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub api_key_configured: bool,
    pub symbols: Vec<String>,
    pub candles: crate::config::CandlesConfig,
    pub indicators: crate::config::IndicatorsConfig,
    pub instances: std::collections::HashMap<String, crate::config::InstanceSpecificConfig>,
}

#[derive(Debug, Deserialize)]
pub struct AssistantRecordsQuery {
    #[serde(default)]
    pub trigger_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timeframe_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timeframe_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct IndicatorSnapshot {
    pub rsi: Option<f64>,
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<f64>,
    pub macd_line: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_histogram: Option<f64>,
    pub macd_histogram_trend: Option<String>,
    pub adx: Option<f64>,
    pub adx_plus: Option<f64>,
    pub adx_minus: Option<f64>,
    pub bb_upper: Option<f64>,
    pub bb_middle: Option<f64>,
    pub bb_lower: Option<f64>,
    pub atr: Option<f64>,
    pub atr_trend: Option<String>,
    pub atr_volatility_regime: Option<String>,
    pub current_price: Option<f64>,
    pub volume: Option<f64>,
    pub average_volume: Option<f64>,
    pub rvol: Option<f64>,
    pub ema_fast: Option<f64>,
    pub ema_medium: Option<f64>,
    pub ema_slow: Option<f64>,
    pub ema_long: Option<f64>,
    pub ema_stack_state: Option<String>,
    pub vwap: Option<f64>,
    pub vwap_bias: Option<String>,
    pub rsi_divergence_status: Option<String>,
    pub macd_divergence_status: Option<String>,
    pub macd_trend_state: Option<String>,
    pub macd_crossover_detected: Option<bool>,
    pub macd_crossover_direction: Option<String>,
    pub macd_histogram_peak: Option<f64>,
    pub squeeze_duration: Option<u32>,
    pub squeeze_release_trigger: Option<bool>,
    pub squeeze_momentum_direction: Option<String>,
    pub chart_pattern: Option<String>,
    pub chart_pattern_confidence: Option<f64>,
    pub bbwp: Option<f64>,
    pub adx_slope: Option<f64>,
    pub adx_regime: Option<String>,
    pub adx_di_crossover_detected: Option<bool>,
    pub adx_di_crossover_direction: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SupportResistanceResponse {
    pub detected_support_levels: Vec<String>,
    pub detected_resistance_levels: Vec<String>,
    pub structural_analysis: String,
}

#[derive(Debug, Serialize)]
pub struct IndicatorSynthesisResponse {
    pub summary_count: String,
    pub evaluation: String,
}

#[derive(Debug, Serialize)]
pub struct PositionRecommendationResponse {
    pub action: String,
    pub rationale: String,
}

#[derive(Debug, Serialize)]
pub struct PhaseTwoResponse {
    pub general_trend: String,
    pub support_and_resistance: SupportResistanceResponse,
    pub indicator_synthesis: IndicatorSynthesisResponse,
    pub position_recommendation: PositionRecommendationResponse,
}

#[derive(Debug, Serialize)]
pub struct MultiAgentAnalysisResponse {
    pub phase_one: Vec<IndividualIndicatorResult>,
    pub phase_two: PhaseTwoResponse,
}

#[derive(Debug, Serialize)]
pub struct HistoryCandle {
    pub time: u64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

#[derive(Debug, Serialize)]
pub struct IndicatorHistoryArrays {
    pub times: Vec<u64>,
    pub rsi_14: Vec<Option<String>>,
    pub squeeze_on: Vec<Option<bool>>,
    pub squeeze_momentum: Vec<Option<String>>,
    pub macd_line: Vec<Option<String>>,
    pub macd_signal: Vec<Option<String>>,
    pub macd_hist: Vec<Option<String>>,
    pub adx_14: Vec<Option<String>>,
    pub adx_plus: Vec<Option<String>>,
    pub adx_minus: Vec<Option<String>>,
    pub atr_14: Vec<Option<String>>,
    pub ema_fast: Vec<Option<String>>,
    pub ema_medium: Vec<Option<String>>,
    pub ema_slow: Vec<Option<String>>,
    pub ema_long: Vec<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub prices: Vec<String>,
    pub candles: Vec<HistoryCandle>,
    pub indicator_history: IndicatorHistoryArrays,
}

#[derive(Debug, Serialize)]
pub struct ChatReplResponse {
    pub reply: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatHistoryRequest {
    pub history: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct AddTradeRequest {
    pub symbol: String,
    pub direction: String,
    pub outcome: String,
    pub risk_multiplier: f64,
    pub reward_multiplier: f64,
}

#[derive(Debug, Serialize)]
pub struct MasterRecordJson {
    pub id: i64,
    pub created_at: String,
    pub position: String,
    pub entry_price: Option<String>,
    pub trend_classification: String,
    pub indicator_alignment: String,
    pub indicator_synthesis_summary: String,
    pub recommended_action: String,
    pub recommendation_rationale: String,
    pub price_at_analysis: String,
    pub support_levels: String,
    pub resistance_levels: String,
    pub symbol: String,
    pub trigger_type: String,
}

#[derive(Debug, Serialize)]
pub struct MasterHistoryResponse {
    pub records: Vec<MasterRecordJson>,
    pub latest_close: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeAcceptedResponse {
    pub master_id: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStatusResponse {
    pub connected: bool,
    pub latency_ms: u64,
    pub journal_mode: String,
    pub total_allocated_margin: f64,
    pub total_ai_token_costs_usd: f64,
    pub active_pairs_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservabilityBuffersResponse {
    pub symbol: String,
    pub recent_decisions: Vec<DecisionMemoryBufferRow>,
    pub completed_trades: Vec<CompletedTradesBufferRow>,
}

#[derive(Debug, Serialize)]
pub struct CostEstimateResponse {
    pub price_per_1m_input_tokens: f64,
    pub price_per_1m_output_tokens: f64,
    pub interval_seconds: u64,
    pub runs_per_day: f64,
    pub input_tokens_per_run: u64,
    pub output_tokens_per_run: u64,
    pub projected_daily_cost: f64,
    pub projected_weekly_cost: f64,
    pub projected_monthly_cost: f64,
    pub actual_input_tokens_used: u64,
    pub actual_output_tokens_used: u64,
    pub actual_total_cost: f64,
}

#[derive(Debug, Deserialize)]
pub struct CostEstimateQuery {
    pub pair_key: Option<String>,
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/session/status", get(serve_session_status))
        .route("/api/session/init", post(serve_session_init))
        .route("/api/session/quit", post(serve_session_quit))
        .route("/api/config", get(serve_config).post(update_config))
        .route("/api/config/key", post(serve_set_key))
        .route("/api/rules", get(serve_get_rules).post(serve_set_rules))
        .route("/api/history", get(serve_history))
        .route("/api/analyze", post(serve_analyze))
        .route("/api/chat", post(serve_chat))
        .route("/api/trades", get(serve_get_trades).post(serve_add_trade))
        .route("/api/assistant-records", get(serve_assistant_records))
        .route("/api/automated-performance", get(serve_automated_performance))
        .route("/api/paper/status", get(serve_paper_status))
        .route("/api/paper/config", post(serve_paper_config))
        .route("/api/paper/reset", post(serve_paper_reset))
        .route("/api/paper/order", post(serve_paper_order))
        .route("/api/paper/scale-in", post(serve_paper_scale_in))
        .route("/api/paper/scale-out", post(serve_paper_scale_out))
        .route("/api/paper/unrealized", get(serve_paper_unrealized))
        .route("/api/paper/performance", get(serve_paper_performance))
        .route("/api/instances", get(serve_list_instances).post(serve_add_instance))
        .route("/api/instances/:instance_id", get(serve_get_instance_detail).delete(serve_delete_instance))
        .route("/api/instances/:instance_id/config", post(serve_update_instance_config))
        .route("/api/instances/:instance_id/pause", post(serve_pause_instance))
        .route("/api/instances/:instance_id/stop", post(serve_stop_instance))
        .route("/api/instances/:instance_id/safety/reset", post(serve_reset_safety))
        .route("/api/instances/:instance_id/manual/open", post(serve_instance_manual_open))
        .route("/api/instances/:instance_id/manual/close", post(serve_instance_manual_close))
        .route("/api/instances/:instance_id/intervals", post(serve_instance_intervals))
        .route("/api/instances/:instance_id/api-key", post(serve_set_instance_api_key))
        .route("/api/instances/:instance_id/api-key", delete(serve_delete_instance_api_key))
        .route("/api/instances/:instance_id/usage", get(|State(state): State<Arc<AppState>>, Path(id): Path<String>| async move {
            let instances = state.workspace.instances.read().await;
            let instance = instances.values().find(|i| i.id == id).cloned();
            drop(instances);
            match instance {
                Some(inst) => {
                    let (input_tokens, output_tokens) = {
                        let usage = inst.token_tracker.lock().unwrap();
                        (usage.global.input_tokens, usage.global.output_tokens)
                    };
                    let active_source = inst.api_failover.active_source.read().await.clone();
                    Json(InstanceUsageResponse {
                        instance_id: id,
                        input_tokens,
                        output_tokens,
                        failover_active: active_source != crate::api_failover::KeySource::None,
                        failover_source: format!("{:?}", active_source),
                        consecutive_failures: inst.api_failover.consecutive_failures.load(std::sync::atomic::Ordering::Relaxed),
                    }).into_response()
                }
                None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
            }
        }))
        .route("/api/settings/backup-api-key", post(serve_set_backup_api_key))
        .route("/api/historical-recommendations", get(serve_historical_recommendations))
        .route("/api/instances/:instance_id/chat", post(serve_instance_chat))
        .route("/api/decision-profiles", get(serve_decision_profiles_list).post(serve_decision_profile_create))
        .route("/api/decision-profiles/:id", delete(serve_decision_profile_delete).post(serve_decision_profile_update))
        .route("/api/decision-profiles/:id/evaluate", post(serve_decision_evaluate))
        .route("/api/decision-profiles/:id/indicators", post(serve_profile_indicator_add))
        .route("/api/decision-profiles/:id/indicators/:iid", post(serve_profile_indicator_update).delete(serve_profile_indicator_delete))
        .route("/api/risk-profiles", get(serve_risk_profiles_list).post(serve_risk_profile_create))
        .route("/api/risk-profiles/:id", delete(serve_risk_profile_delete).post(serve_risk_profile_update))
        .route("/api/risk/calculate", post(serve_risk_calculate))
        .route("/api/risk/fee-table", get(serve_fee_table))
        .route("/api/risk/commission-projection", post(serve_commission_projection))
        .route("/api/exchange-keys", get(serve_exchange_keys_list).post(serve_exchange_keys_add))
        .route("/api/exchange-keys/:id", delete(serve_exchange_keys_delete).post(serve_exchange_keys_sync))
        .route("/api/dashboard/stats", get(serve_dashboard_stats))
        .route("/api/trade-ledger", get(serve_trade_ledger))
        .route("/api/trade-journal", get(serve_trade_journal))
        .route("/api/trade-journal/:id/notes", post(serve_update_journal_notes))
        .route("/api/trade-journal/export/csv", get(serve_export_journal_csv))
        .route("/api/trade-journal/export/json", get(serve_export_journal_json))
        .route("/api/trades/telemetry", post(serve_trade_telemetry_add))
        .route("/api/cost-estimate", get(serve_cost_estimate))
        .route("/api/system/status", get(serve_system_status))
        .route("/api/system/observability", get(serve_observability_buffers))
        .route("/ws", get(ws_handler))
        .route("/favicon.ico", get(|| async { Redirect::to("/favicon.svg") }))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .fallback_service(ServeDir::new("crates/frontend/dist"))
        .with_state(state)
}

async fn serve_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current_config = state.config.read().await.clone();
    let api_key_configured = state.api_key_configured.load(std::sync::atomic::Ordering::Relaxed);
    let response_body = ConfigResponse {
        api_key_configured,
        symbols: current_config.symbols.clone(),
        candles: current_config.candles.clone(),
        indicators: current_config.indicators.clone(),
        instances: current_config.instances.clone(),
    };
    let json = axum::Json(response_body);
    let mut response = json.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    response
}

async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AppConfig>,
) -> impl IntoResponse {
    match toml::to_string_pretty(&payload) {
        Ok(toml_str) => {
            if let Err(e) = std::fs::write("config.toml", toml_str) {
                eprintln!("❌ Database/Config Error: Failed to write configuration updates to config.toml: {}", e);
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to persist configuration file").into_response();
            }
            *state.config.write().await = payload;
            println!("✅ Configuration Updated: successfully synchronized config.toml dynamically.");
            (axum::http::StatusCode::OK, "Configuration successfully saved.").into_response()
        }
        Err(e) => {
            eprintln!("❌ TOML Serialization Error: {}", e);
            (axum::http::StatusCode::BAD_REQUEST, "Invalid configuration object structure").into_response()
        }
    }
}

async fn serve_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let pair_key = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    } else {
        query.symbol
    };

    let tf_secs = query.timeframe_secs.unwrap_or(60);
    let raw_symbol = extract_base_symbol(&pair_key);

    let config_guard = state.config.read().await;
    let pair_cfg = config_guard.instances.get(&pair_key);
    let current_limit = match tf_secs {
        300 => pair_cfg.map(|p| p.short_term.candles.analysis_limit).unwrap_or(config_guard.candles.analysis_limit),
        _ => pair_cfg.map(|p| p.micro_term.candles.analysis_limit).unwrap_or(config_guard.candles.analysis_limit),
    };

    let (prices, candles) = match get_active_pair(&state.workspace, &pair_key).await {
        Some(pair) => {
            let hist = if tf_secs == 300 {
                pair.short.history.read().await
            } else {
                pair.micro.history.read().await
            };
            let candles: Vec<HistoryCandle> = hist.iter().map(|c| HistoryCandle {
                time: c.start_time_ms,
                open: c.open.to_string(),
                high: c.high.to_string(),
                low: c.low.to_string(),
                close: c.close.to_string(),
                volume: c.volume.to_string(),
            }).collect();
            let price_list: Vec<String> = candles.iter().map(|c| c.close.clone()).collect();
            (price_list, candles)
        }
        None => (vec![], vec![]),
    };

    let indicator_rows = crate::db::query_indicator_snapshots(&state.pool, &raw_symbol, tf_secs, current_limit as u32).await;
    let mut indicator_history = IndicatorHistoryArrays {
        times: Vec::with_capacity(indicator_rows.len()),
        rsi_14: Vec::with_capacity(indicator_rows.len()),
        squeeze_on: Vec::with_capacity(indicator_rows.len()),
        squeeze_momentum: Vec::with_capacity(indicator_rows.len()),
        macd_line: Vec::with_capacity(indicator_rows.len()),
        macd_signal: Vec::with_capacity(indicator_rows.len()),
        macd_hist: Vec::with_capacity(indicator_rows.len()),
        adx_14: Vec::with_capacity(indicator_rows.len()),
        adx_plus: Vec::with_capacity(indicator_rows.len()),
        adx_minus: Vec::with_capacity(indicator_rows.len()),
        atr_14: Vec::with_capacity(indicator_rows.len()),
        ema_fast: Vec::with_capacity(indicator_rows.len()),
        ema_medium: Vec::with_capacity(indicator_rows.len()),
        ema_slow: Vec::with_capacity(indicator_rows.len()),
        ema_long: Vec::with_capacity(indicator_rows.len()),
    };
    for row in indicator_rows {
        indicator_history.times.push(row.timestamp as u64);
        indicator_history.rsi_14.push(row.rsi_14);
        indicator_history.squeeze_on.push(row.squeeze_on);
        indicator_history.squeeze_momentum.push(row.squeeze_momentum);
        indicator_history.macd_line.push(row.macd_line);
        indicator_history.macd_signal.push(row.macd_signal);
        indicator_history.macd_hist.push(row.macd_hist);
        indicator_history.adx_14.push(row.adx_14);
        indicator_history.adx_plus.push(row.adx_plus);
        indicator_history.adx_minus.push(row.adx_minus);
        indicator_history.atr_14.push(row.atr_14);
        indicator_history.ema_fast.push(row.ema_fast);
        indicator_history.ema_medium.push(row.ema_medium);
        indicator_history.ema_slow.push(row.ema_slow);
        indicator_history.ema_long.push(row.ema_long);
    }
    drop(config_guard);

    Json(HistoryResponse { prices, candles, indicator_history })
}

async fn serve_analyze(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AnalyzeRequest>,
) -> impl IntoResponse {
    let symbol = if payload.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    } else {
        payload.symbol.clone()
    };

    let prices = payload.historical_prices.clone();
    let indicators = payload.indicators.clone();
    let timeframes = payload.timeframes.clone();

    let last_close = {
        let pair_key = symbol.clone();
        match get_active_pair(&state.workspace, &pair_key).await {
            Some(pair) => {
                let hist = pair.micro.history.read().await;
                hist.back().map(|c| c.close.to_string()).unwrap_or_else(|| "0".to_string())
            }
            None => "0".to_string(),
        }
    };

    let entry_price = payload.entry_price.clone();
    let position = payload.position.clone();

    let master_id = crate::db::insert_master_placeholder(
        &state.pool,
        &position,
        &entry_price,
        &last_close,
        &symbol,
        TriggerType::Manual,
    )
    .await;

    let have_key = {
        let llm = state.llm_client.read().await;
        !llm.api_key.is_empty()
    };
    if !have_key {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "AI Assistant API Key is not configured. Heuristic fallback has been deprecated. Please configure your key in settings."
            })),
        ).into_response();
    }

    let raw_symbol = extract_base_symbol(&symbol);

    let last_close_f: f64 = last_close.parse().unwrap_or(0.0);
    let (support_levels, resistance_levels) = compute_support_resistance(&prices, last_close_f);
    let _atr_trend = determine_atr_trend(&state.pool, indicators.atr, 60).await;

    let empty_snap = IndicatorSnapshot::default();
    let mtf = timeframes.as_ref();
    let micro_snap = mtf.map(|t| &t.micro_term).unwrap_or(&indicators);
    let small_snap = mtf.map(|t| &t.short_term).unwrap_or(&indicators);
    let medium_snap = mtf.and_then(|t| t.medium_term.as_ref()).unwrap_or(&empty_snap);
    let large_snap = mtf.and_then(|t| t.large_term.as_ref()).unwrap_or(&empty_snap);

    let support_strings: Vec<String> = support_levels.iter().map(|s| s.to_string()).collect();
    let resistance_strings: Vec<String> = resistance_levels.iter().map(|s| s.to_string()).collect();
    let telemetry = compile_deterministic_telemetry(micro_snap, &support_strings, &resistance_strings);

    let multi_agent_results = match run_multi_agent_pipeline(
        state.llm_client.clone(),
        state.pool.clone(),
        &raw_symbol,
        micro_snap,
        small_snap,
        medium_snap,
        large_snap,
        &prices,
        master_id,
        &telemetry,
    ).await {
        Ok(res) => res,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Phase 1 analysis failed: {}", e)
                })),
            ).into_response();
        }
    };

    let legacy_signals = multi_agent_results.to_legacy_signals();
    let phase_one_json = serde_json::to_string(&legacy_signals).unwrap_or_else(|_| "[]".into());

    let journal_context = crate::db::query_recent_journal_for_context(&state.pool, &raw_symbol, 10).await;
    let journal_opt: Option<&str> = if journal_context.is_empty() { None } else { Some(&journal_context) };

    let llm = state.llm_client.read().await;
    let master_result = match llm.run_master_orchestrator(
        &position,
        &entry_price,
        &prices,
        &symbol,
        &phase_one_json,
        &telemetry.support_levels,
        &telemetry.resistance_levels,
        journal_opt,
        Some(&symbol),
    ).await {
        Ok(res) => res,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Phase 2 orchestrator failed: {}", e)
                })),
            ).into_response();
        }
    };
    drop(llm);

    let db_telemetry = state.telemetry_tx.clone();
    let db_pool = state.pool.clone();
    let db_master_id = master_id;
    let db_indicators = indicators.clone();
    let mr_general_trend = master_result.general_trend.clone();
    let mr_support = serde_json::to_string(&master_result.support_and_resistance.detected_support_levels).unwrap_or_default();
    let mr_resistance = serde_json::to_string(&master_result.support_and_resistance.detected_resistance_levels).unwrap_or_default();
    let mr_summary = master_result.indicator_synthesis.summary_count.clone();
    let mr_evaluation = master_result.indicator_synthesis.evaluation.clone();
    let mr_action = master_result.position_recommendation.action.clone();
    let mr_rationale = master_result.position_recommendation.rationale.clone();
    let mr_score = master_result.eight_factor_score;
    let mr_allocation = master_result.allocation_pct;

    tokio::spawn(async move {
        let local_snap = indicator_to_snapshot_local(&db_indicators);
        let regime = classify_market_regime(&local_snap);

        let _ = db_telemetry.send(crate::db::TelemetryMsg::UpdateMasterRecord {
            master_id: db_master_id,
            general_trend: mr_general_trend,
            support_levels: mr_support,
            resistance_levels: mr_resistance,
            indicator_synthesis_summary: mr_summary,
            indicator_synthesis_evaluation: mr_evaluation,
            recommended_action: mr_action,
            recommendation_rationale: mr_rationale,
            score_points: Some(mr_score),
            signals_json: None,
        }).await;

        let _ = sqlx::query(
            "UPDATE master_assistant_records SET market_regime = ?2, portfolio_allocation_pct = ?3 WHERE id = ?1"
        )
        .bind(db_master_id)
        .bind(regime.as_str())
        .bind(mr_allocation)
        .execute(&db_pool)
        .await;
    });

    let response = MultiAgentAnalysisResponse {
        phase_one: legacy_signals,
        phase_two: PhaseTwoResponse {
            general_trend: master_result.general_trend,
            support_and_resistance: SupportResistanceResponse {
                detected_support_levels: master_result.support_and_resistance.detected_support_levels,
                detected_resistance_levels: master_result.support_and_resistance.detected_resistance_levels,
                structural_analysis: master_result.support_and_resistance.structural_analysis,
            },
            indicator_synthesis: IndicatorSynthesisResponse {
                summary_count: master_result.indicator_synthesis.summary_count,
                evaluation: master_result.indicator_synthesis.evaluation,
            },
            position_recommendation: PositionRecommendationResponse {
                action: master_result.position_recommendation.action,
                rationale: master_result.position_recommendation.rationale,
            },
        },
    };

    (axum::http::StatusCode::OK, Json(response)).into_response()
}

fn indicator_to_snapshot_local(snap: &IndicatorSnapshot) -> crate::profile_evaluation::SnapshotValues {
    crate::profile_evaluation::SnapshotValues {
        rsi: snap.rsi,
        squeeze_on: snap.squeeze_on,
        squeeze_momentum: snap.squeeze_momentum,
        squeeze_duration: snap.squeeze_duration,
        squeeze_release_trigger: snap.squeeze_release_trigger,
        squeeze_momentum_direction: snap.squeeze_momentum_direction.clone(),
        chart_pattern: snap.chart_pattern.clone(),
        chart_pattern_confidence: snap.chart_pattern_confidence,
        bbwp: snap.bbwp,
        macd_line: snap.macd_line,
        macd_signal: snap.macd_signal,
        macd_hist: snap.macd_histogram,
        adx: snap.adx,
        adx_plus: snap.adx_plus,
        adx_minus: snap.adx_minus,
        bb_upper: snap.bb_upper,
        bb_middle: snap.bb_middle,
        bb_lower: snap.bb_lower,
        atr: snap.atr,
        ema_fast: snap.ema_fast,
        ema_medium: snap.ema_medium,
        ema_slow: snap.ema_slow,
        ema_long: snap.ema_long,
        ema_stack_state: snap.ema_stack_state.clone(),
        vwap: snap.vwap,
        vwap_bias: snap.vwap_bias.clone(),
        close: snap.current_price,
        volume: snap.volume,
        average_volume: snap.average_volume,
        rvol: snap.rvol,
        current_price: snap.current_price.unwrap_or(0.0),
        rsi_divergence_status: None,
        macd_divergence_status: None,
        macd_trend_state: snap.macd_trend_state.clone(),
        macd_crossover_detected: snap.macd_crossover_detected,
        macd_crossover_direction: snap.macd_crossover_direction.clone(),
        macd_histogram_peak: snap.macd_histogram_peak,
        atr_volatility_regime: snap.atr_volatility_regime.clone(),
        adx_slope: None,
        adx_regime: None,
        adx_di_crossover_detected: None,
        adx_di_crossover_direction: None,
    }
}

pub async fn serve_system_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let active_pairs_count = state.workspace.instance_count().await;

    let costs = state.config.read().await.costs.clone();
    let total_ai_token_costs_usd = {
        let llm = state.llm_client.read().await;
        let tracker = llm.token_tracker.lock().unwrap();
        (tracker.global.input_tokens as f64 / 1_000_000.0) * costs.price_per_1m_input_tokens
            + (tracker.global.output_tokens as f64 / 1_000_000.0) * costs.price_per_1m_output_tokens
    };

    let mut total_allocated_margin = 0.0;
    let instances = state.workspace.instances.read().await;
    for instance in instances.values() {
        if let Some(pos) = crate::db::paper_get_active_position(&state.pool, &instance.symbol()).await {
            total_allocated_margin += pos.allocated_usd;
        }
    }

    let response = SystemStatusResponse {
        connected: state.api_key_configured.load(std::sync::atomic::Ordering::Relaxed),
        latency_ms: 12,
        journal_mode: "WAL".to_string(),
        total_allocated_margin,
        total_ai_token_costs_usd,
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
    let raw_symbol = symbol.split_once(':').map(|(_, s)| s).unwrap_or(&symbol).to_string();

    let recent_decisions: Vec<DecisionMemoryBufferRow> = sqlx::query_as(
        "SELECT id, symbol, timestamp, regime_classification, orchestrator_decision, confidence_score, eight_factor_score, portfolio_risk_pct \
         FROM decision_memory_buffer WHERE symbol = ?1 ORDER BY id DESC LIMIT 5"
    )
    .bind(&raw_symbol)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let completed_trades: Vec<CompletedTradesBufferRow> = sqlx::query_as(
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
         LIMIT 5"
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

pub fn compute_support_resistance(
    prices: &[f64],
    current_price: f64,
) -> (Vec<String>, Vec<String>) {
    if prices.len() < 10 {
        return (vec![], vec![]);
    }

    let mut local_mins: Vec<f64> = Vec::new();
    let mut local_maxs: Vec<f64> = Vec::new();

    for i in 1..prices.len() - 1 {
        let prev = prices[i - 1];
        let curr = prices[i];
        let next = prices[i + 1];

        if curr <= prev && curr <= next {
            local_mins.push(curr);
        }
        if curr >= prev && curr >= next {
            local_maxs.push(curr);
        }
    }

    local_mins.sort_by(|a, b| a.partial_cmp(b).unwrap());
    local_maxs.sort_by(|a, b| b.partial_cmp(a).unwrap());

    let step_size = if current_price >= 1000.0 {
        0.01
    } else if current_price >= 1.0 {
        0.0001
    } else {
        0.000001
    };

    let dedup_threshold = current_price * 0.002;

    let support_levels: Vec<String> = filter_levels(&local_mins, current_price, true, step_size, dedup_threshold);
    let resistance_levels: Vec<String> = filter_levels(&local_maxs, current_price, false, step_size, dedup_threshold);

    (support_levels, resistance_levels)
}

fn filter_levels(
    levels: &[f64],
    current_price: f64,
    is_support: bool,
    step_size: f64,
    dedup_thresh: f64,
) -> Vec<String> {
    let mut filtered: Vec<String> = Vec::new();

    for &level in levels {
        if is_support && level >= current_price {
            continue;
        }
        if !is_support && level <= current_price {
            continue;
        }

        let rounded = (level / step_size).round() * step_size;

        if filtered.iter().any(|existing: &String| {
            let existing_val: f64 = existing.parse().unwrap_or(0.0);
            (rounded - existing_val).abs() < dedup_thresh
        }) {
            continue;
        }

        let formatted = if step_size >= 0.01 {
            format!("{:.2}", rounded)
        } else if step_size >= 0.0001 {
            format!("{:.4}", rounded)
        } else {
            format!("{:.6}", rounded)
        };

        filtered.push(formatted);

        if filtered.len() >= 3 {
            break;
        }
    }

    filtered
}

pub async fn determine_atr_trend(pool: &SqlitePool, current_atr: Option<f64>, timeframe_secs: u64) -> String {
    let current_atr = match current_atr {
        Some(v) => v,
        None => return "flat".to_string(),
    };

    let rows = crate::db::query_atr_snapshots(pool, timeframe_secs, 5).await;

    if rows.len() < 5 {
        return "flat".to_string();
    }

    let previous_atrs: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.as_ref().and_then(|s| s.parse::<f64>().ok()))
        .collect();

    if previous_atrs.len() < 5 {
        return "flat".to_string();
    }

    let avg_previous: f64 = previous_atrs.iter().sum::<f64>() / previous_atrs.len() as f64;

    let pct_change = (current_atr - avg_previous) / avg_previous * 100.0;

    if pct_change > 2.0 {
        "rising".to_string()
    } else if pct_change < -2.0 {
        "falling".to_string()
    } else {
        "flat".to_string()
    }
}

pub async fn run_phase_one_agents_mtf(
    client: &LlmClient,
    symbol: &str,
    micro: &IndicatorSnapshot,
    small: &IndicatorSnapshot,
    medium: &IndicatorSnapshot,
    large: &IndicatorSnapshot,
    _prices: &[f64],
    master_id: i64,
    telemetry_tx: &mpsc::Sender<crate::db::TelemetryMsg>,
) -> Vec<IndividualIndicatorResult> {
    let rsi_section = client.get_guide_section("RSI");
    let macd_section = client.get_guide_section("MACD");
    let squeeze_section = client.get_guide_section("SQUEEZE");
    let adx_section = client.get_guide_section("ADX");
    let bb_atr_section = client.get_guide_section("BOLLINGER_ATR");
    let vol_ema_section = client.get_guide_section("VOLUME_EMA");
    let vwap_section = client.get_guide_section("VWAP");

    let indicator_names = ["RSI", "MACD", "SQUEEZE", "ADX", "BOLLINGER_ATR", "VOLUME_EMA", "VWAP"];
    let sections = [&rsi_section, &macd_section, &squeeze_section, &adx_section, &bb_atr_section, &vol_ema_section, &vwap_section];
    let medium_tf_secs = 900u64;
    let large_tf_secs = 3600u64;
    let timeframes: [(&str, &IndicatorSnapshot, u64); 4] = [
        ("micro", micro, 60),
        ("small", small, 300),
        ("medium", medium, medium_tf_secs),
        ("large", large, large_tf_secs),
    ];

    let mut handles = Vec::new();
    for (tf_label, indicator_snap, tf_secs) in &timeframes {
        for i in 0..7 {
            let name = indicator_names[i].to_string();
            let section = sections[i].to_string();
            let context = build_indicator_context(indicator_names[i], indicator_snap);
            let tf_label = tf_label.to_string();
            let _tf_secs = *tf_secs;
            let client_base = client.base_url.clone();
            let client_key = client.api_key.clone();
            let client_model = client.model.clone();
            let tracker = client.get_token_tracker();
            let pair_key = symbol.to_string();

            let handle = tokio::spawn(async move {
                let temp_client = LlmClient {
                failover_state: None,
                    base_url: client_base,
                    api_key: client_key,
                    model: client_model,
                    indicators_guide: String::new(),
                    token_tracker: tracker,
                };

                match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    temp_client.run_indicator_agent(&name, &section, &context, Some(&pair_key)),
                )
                .await
                {
                    Ok(Ok(result)) => IndividualIndicatorResult {
                        indicator_name: format!("{}-{}", tf_label, result.indicator_name),
                        signal: result.signal,
                        reason: result.reason,
                        confidence_score: result.confidence_score,
                        divergence_status: result.divergence_status.clone(),
                        divergence_type: result.divergence_type.clone(),
                        is_confirmed: result.is_confirmed.clone(),
                    },
                    Ok(Err(e)) => IndividualIndicatorResult {
                        indicator_name: format!("{}-{}", tf_label, name),
                        signal: "UNAVAILABLE".to_string(),
                        reason: format!("Agent error: {}", e),
                        confidence_score: 0,
                        divergence_status: None,
                        divergence_type: None,
                        is_confirmed: None,
                    },
                    Err(_) => IndividualIndicatorResult {
                        indicator_name: format!("{}-{}", tf_label, name),
                        signal: "UNAVAILABLE".to_string(),
                        reason: "Agent timed out after 10 seconds".to_string(),
                        confidence_score: 0,
                        divergence_status: None,
                        divergence_type: None,
                        is_confirmed: None,
                    },
                }
            });
            handles.push(handle);
        }
    }

    use futures_util::future::join_all;
    let results: Vec<IndividualIndicatorResult> = join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or_else(|e| IndividualIndicatorResult {
            indicator_name: "UNKNOWN".to_string(),
            signal: "UNAVAILABLE".to_string(),
            reason: format!("Task panic: {}", e),
            confidence_score: 0,
            divergence_status: None,
            divergence_type: None,
            is_confirmed: None,
        }))
        .collect();

    for (tf_label, _, tf_secs) in &timeframes {
        for result in &results {
            if result.indicator_name.starts_with(&format!("{}-", tf_label)) {
                let _ = telemetry_tx.send(crate::db::TelemetryMsg::InsertIndividualLog {
                    master_record_id: master_id,
                    indicator_name: result.indicator_name.clone(),
                    signal: result.signal.clone(),
                    reason: result.reason.clone(),
                    timeframe_secs: *tf_secs,
                }).await;
            }
        }
    }

    results
}

fn build_indicator_context(indicator_name: &str, snap: &IndicatorSnapshot) -> String {
    match indicator_name {
        "RSI" => format!(
            r#"{{ "rsi_value": {}, "current_price": {}, "rsi_divergence_status": "{}" }}"#,
            snap.rsi.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.current_price.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.rsi_divergence_status.as_deref().unwrap_or("none"),
        ),
        "MACD" => format!(
            r#"{{ "macd_line": {}, "signal_line": {}, "histogram_value": {}, "histogram_trend": "{}", "histogram_peak": {}, "crossover_detected": {}, "crossover_direction": "{}", "macd_divergence_status": "{}" }}"#,
            snap.macd_line.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_signal.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_histogram.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_trend_state.as_deref().unwrap_or("unknown"),
            snap.macd_histogram_peak.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_crossover_detected.unwrap_or(false),
            snap.macd_crossover_direction.as_deref().unwrap_or("NONE"),
            snap.macd_divergence_status.as_deref().unwrap_or("none"),
        ),
        "SQUEEZE" => format!(
            r#"{{ "squeeze_on": {}, "momentum_value": {}, "squeeze_duration": {}, "squeeze_release_trigger": {}, "momentum_direction": "{}" }}"#,
            snap.squeeze_on.map_or("null".to_string(), |v| v.to_string()),
            snap.squeeze_momentum.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.squeeze_duration.unwrap_or(0),
            snap.squeeze_release_trigger.unwrap_or(false),
            snap.squeeze_momentum_direction.as_deref().unwrap_or("Flat"),
        ),
        "ADX" => format!(
            r#"{{ "adx_line": {}, "di_plus": {}, "di_minus": {}, "adx_slope": {}, "adx_regime": "{}", "di_crossover_detected": {}, "di_crossover_direction": "{}" }}"#,
            snap.adx.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.adx_plus.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.adx_minus.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.adx_slope.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.adx_regime.as_deref().unwrap_or("unknown"),
            snap.adx_di_crossover_detected.unwrap_or(false),
            snap.adx_di_crossover_direction.as_deref().unwrap_or("NONE"),
        ),
        "BOLLINGER_ATR" => format!(
            r#"{{ "mid_price": {}, "bb_upper": {}, "bb_middle": {}, "bb_lower": {}, "atr_value": {} }}"#,
            snap.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.bb_upper.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.bb_middle.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.bb_lower.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.atr.map_or("null".to_string(), |v| format!("{:.4}", v)),
        ),
        "VOLUME_EMA" => format!(
            r#"{{ "close": {}, "ema_fast": {}, "ema_slow": {}, "volume": {}, "average_volume": {}, "rvol": {}, "ema_stack_state": "{}" }}"#,
            snap.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.ema_fast.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.ema_slow.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.volume.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.average_volume.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.rvol.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.ema_stack_state.as_deref().unwrap_or("tangled"),
        ),
        "VWAP" => format!(
            r#"{{ "close": {}, "vwap": {}, "vwap_bias": "{}" }}"#,
            snap.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.vwap.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.vwap_bias.as_deref().unwrap_or("equilibrium"),
        ),
        _ => "{}".to_string(),
    }
}

pub async fn run_phase_one_agents(
    client: &LlmClient,
    symbol: &str,
    indicators: &IndicatorSnapshot,
    prices: &[f64],
    atr_trend: &str,
    master_id: i64,
    telemetry_tx: &mpsc::Sender<crate::db::TelemetryMsg>,
) -> Vec<IndividualIndicatorResult> {
    let rsi_section = client.get_guide_section("RSI");
    let macd_section = client.get_guide_section("MACD");
    let squeeze_section = client.get_guide_section("SQUEEZE");
    let adx_section = client.get_guide_section("ADX");
    let bb_atr_section = client.get_guide_section("BOLLINGER_ATR");
    let vol_ema_section = client.get_guide_section("VOLUME_EMA");
    let vwap_section = client.get_guide_section("VWAP");

    let recent_closes_json = serde_json::to_string(
        &prices.iter().rev().take(10).rev().collect::<Vec<_>>()
    ).unwrap_or_else(|_| "[]".into());

    let rsi_context = format!(
        r#"{{ "rsi_value": {}, "recent_closes": {} }}"#,
        indicators.rsi.map_or("null".to_string(), |v| format!("{:.2}", v)),
        recent_closes_json,
    );

    let macd_hist_trend = compute_histogram_trend(prices, indicators.macd_histogram);
    let macd_context = format!(
        r#"{{ "macd_line": {}, "signal_line": {}, "histogram_value": {}, "histogram_trend": "{}" }}"#,
        indicators.macd_line.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.macd_signal.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.macd_histogram.map_or("null".to_string(), |v| format!("{:.4}", v)),
        macd_hist_trend,
    );

    let mom_trend = compute_squeeze_momentum_trend(indicators.squeeze_momentum);
    let squeeze_context = format!(
        r#"{{ "squeeze_on": {}, "momentum_value": {}, "momentum_trend": "{}" }}"#,
        indicators.squeeze_on.map_or("null".to_string(), |v| v.to_string()),
        indicators.squeeze_momentum.map_or("null".to_string(), |v| format!("{:.4}", v)),
        mom_trend,
    );

    let adx_context = format!(
        r#"{{ "adx_line": {}, "di_plus": {}, "di_minus": {} }}"#,
        indicators.adx.map_or("null".to_string(), |v| format!("{:.2}", v)),
        indicators.adx_plus.map_or("null".to_string(), |v| format!("{:.2}", v)),
        indicators.adx_minus.map_or("null".to_string(), |v| format!("{:.2}", v)),
    );

    let bb_atr_context = format!(
        r#"{{ "mid_price": {}, "bb_upper": {}, "bb_middle": {}, "bb_lower": {}, "atr_value": {}, "atr_trend": "{}" }}"#,
        indicators.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.bb_upper.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.bb_middle.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.bb_lower.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.atr.map_or("null".to_string(), |v| format!("{:.4}", v)),
        atr_trend,
    );

    let vol_ema_context = format!(
        r#"{{ "close": {}, "ema_fast": {}, "ema_medium": {}, "ema_slow": {}, "ema_long": {}, "volume": {}, "average_volume": {}, "rvol": {}, "ema_stack_state": "{}" }}"#,
        indicators.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.ema_fast.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.ema_medium.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.ema_slow.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.ema_long.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.volume.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.average_volume.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.rvol.map_or("null".to_string(), |v| format!("{:.2}", v)),
        indicators.ema_stack_state.as_deref().unwrap_or("tangled"),
    );

    let vwap_context = format!(
        r#"{{ "close": {}, "vwap": {}, "vwap_bias": "{}" }}"#,
        indicators.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.vwap.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.vwap_bias.as_deref().unwrap_or("equilibrium"),
    );

    let agents = vec![
        ("RSI", rsi_section, rsi_context),
        ("MACD", macd_section, macd_context),
        ("SQUEEZE", squeeze_section, squeeze_context),
        ("ADX", adx_section, adx_context),
        ("BOLLINGER_ATR", bb_atr_section, bb_atr_context),
        ("VOLUME_EMA", vol_ema_section, vol_ema_context),
        ("VWAP", vwap_section, vwap_context),
    ];

    let mut handles = Vec::new();
    for (name, section, context) in agents {
        let name = name.to_string();
        let section = section.to_string();
        let context = context;
        let client_base = client.base_url.clone();
        let client_key = client.api_key.clone();
        let client_model = client.model.clone();
        let tracker = client.get_token_tracker();
        let pair_key = symbol.to_string();

        let handle = tokio::spawn(async move {
            let temp_client = LlmClient {
                failover_state: None,
                base_url: client_base,
                api_key: client_key,
                model: client_model,
                indicators_guide: String::new(),
                token_tracker: tracker,
            };

            match tokio::time::timeout(
                std::time::Duration::from_secs(10),
                temp_client.run_indicator_agent(&name, &section, &context, Some(&pair_key)),
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(e)) => IndividualIndicatorResult {
                    indicator_name: name,
                    signal: "UNAVAILABLE".to_string(),
                    reason: format!("Agent error: {}", e),
                    confidence_score: 0,
                    divergence_status: None,
                    divergence_type: None,
                    is_confirmed: None,
                },
                Err(_) => IndividualIndicatorResult {
                    indicator_name: name,
                    signal: "UNAVAILABLE".to_string(),
                    reason: "Agent timed out after 10 seconds".to_string(),
                    confidence_score: 0,
                    divergence_status: None,
                    divergence_type: None,
                    is_confirmed: None,
                },
            }
        });
        handles.push(handle);
    }

    use futures_util::future::join_all;
    let results: Vec<IndividualIndicatorResult> = join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or_else(|e| IndividualIndicatorResult {
            indicator_name: "UNKNOWN".to_string(),
            signal: "UNAVAILABLE".to_string(),
            reason: format!("Task panic: {}", e),
            confidence_score: 0,
            divergence_status: None,
            divergence_type: None,
            is_confirmed: None,
        }))
        .collect();

    for result in &results {
        let _ = telemetry_tx.send(crate::db::TelemetryMsg::InsertIndividualLog {
            master_record_id: master_id,
            indicator_name: result.indicator_name.clone(),
            signal: result.signal.clone(),
            reason: result.reason.clone(),
            timeframe_secs: 60,
        }).await;
    }

    results
}

fn compute_histogram_trend(_prices: &[f64], current_hist: Option<f64>) -> String {
    match current_hist {
        Some(v) if v > 0.0 => "increasing".to_string(),
        Some(v) if v < 0.0 => "decreasing".to_string(),
        _ => "flat".to_string(),
    }
}

fn compute_squeeze_momentum_trend(momentum: Option<f64>) -> String {
    match momentum {
        Some(v) if v > 0.0 => "rising".to_string(),
        Some(v) if v < 0.0 => "falling".to_string(),
        _ => "flat".to_string(),
    }
}

pub async fn run_multi_agent_pipeline(
    client: Arc<RwLock<LlmClient>>,
    pool: SqlitePool,
    symbol: &str,
    micro: &IndicatorSnapshot,
    _small: &IndicatorSnapshot,
    _medium: &IndicatorSnapshot,
    _large: &IndicatorSnapshot,
    prices: &[f64],
    master_id: i64,
    telemetry: &DeterministicTelemetry,
) -> Result<MultiAgentResults, String> {
    let client_guard = client.read().await;
    let prices_json = serde_json::to_string(&prices).unwrap_or_default();
    let pair_key = symbol.to_string();

    let context_trend = format!(
        r#"{{ "close": {}, "ema_stack_state": "{}", "deterministic_eight_factor_score": {}, "medium_trend_regime": "{}" }}"#,
        micro.current_price.unwrap_or(0.0),
        micro.ema_stack_state.as_deref().unwrap_or("tangled"),
        telemetry.total_confluence_score,
        telemetry.market_regime
    );

    let context_volatility = format!(
        r#"{{ "market_regime": "{}", "bbwp": {}, "atr": {}, "squeeze_on": {}, "rvol": {} }}"#,
        telemetry.market_regime,
        telemetry.bbwp_percentile,
        micro.atr.unwrap_or(0.0),
        telemetry.squeeze_on,
        telemetry.rvol
    );

    let context_structure = format!(
        r#"{{ "current_price": {}, "prices": {}, "squeeze_momentum_direction": "{}" }}"#,
        micro.current_price.unwrap_or(0.0),
        prices_json,
        micro.squeeze_momentum_direction.as_deref().unwrap_or("Flat")
    );

    let context_risk = format!(
        r#"{{ "leverage": 20, "max_risk_pct": 2.0 }}"#
    );

    let context_position = format!(
        r#"{{ "current_price": {} }}"#,
        micro.current_price.unwrap_or(0.0)
    );

    let client_trend = client_guard.api_key.clone();
    let client_vol = client_guard.api_key.clone();
    let client_struct = client_guard.api_key.clone();
    let client_risk = client_guard.api_key.clone();
    let client_pos = client_guard.api_key.clone();

    let base_url = client_guard.base_url.clone();
    let model = client_guard.model.clone();
    let tracker = client_guard.get_token_tracker();

    drop(client_guard);

    // Trend agent
    let trend_key = client_trend.clone();
    let trend_url = base_url.clone();
    let trend_model = model.clone();
    let trend_tracker = tracker.clone();
    let p_key = pair_key.clone();
    let trend_ctx = context_trend.clone();
    let h_trend = tokio::spawn(async move {
        let temp_client = LlmClient {
                failover_state: None,
            base_url: trend_url,
            api_key: trend_key,
            model: trend_model,
            indicators_guide: String::new(),
            token_tracker: trend_tracker,
        };
        temp_client.run_domain_agent::<crate::llm::TrendAgentData>(
            "Trend", crate::llm::TREND_AGENT_PROMPT, &trend_ctx, Some(&p_key)
        ).await
    });

    // Volatility agent
    let vol_key = client_vol.clone();
    let vol_url = base_url.clone();
    let vol_model = model.clone();
    let vol_tracker = tracker.clone();
    let p_key = pair_key.clone();
    let vol_ctx = context_volatility.clone();
    let h_vol = tokio::spawn(async move {
        let temp_client = LlmClient {
                failover_state: None,
            base_url: vol_url,
            api_key: vol_key,
            model: vol_model,
            indicators_guide: String::new(),
            token_tracker: vol_tracker,
        };
        temp_client.run_domain_agent::<crate::llm::VolatilityAgentData>(
            "Volatility", crate::llm::VOLATILITY_AGENT_PROMPT, &vol_ctx, Some(&p_key)
        ).await
    });

    // Structure agent
    let struct_key = client_struct.clone();
    let struct_url = base_url.clone();
    let struct_model = model.clone();
    let struct_tracker = tracker.clone();
    let p_key = pair_key.clone();
    let struct_ctx = context_structure.clone();
    let h_struct = tokio::spawn(async move {
        let temp_client = LlmClient {
                failover_state: None,
            base_url: struct_url,
            api_key: struct_key,
            model: struct_model,
            indicators_guide: String::new(),
            token_tracker: struct_tracker,
        };
        temp_client.run_domain_agent::<crate::llm::StructureAgentData>(
            "Structure", crate::llm::STRUCTURE_AGENT_PROMPT, &struct_ctx, Some(&p_key)
        ).await
    });

    // Risk agent
    let risk_key = client_risk.clone();
    let risk_url = base_url.clone();
    let risk_model = model.clone();
    let risk_tracker = tracker.clone();
    let p_key = pair_key.clone();
    let risk_ctx = context_risk.clone();
    let h_risk = tokio::spawn(async move {
        let temp_client = LlmClient {
                failover_state: None,
            base_url: risk_url,
            api_key: risk_key,
            model: risk_model,
            indicators_guide: String::new(),
            token_tracker: risk_tracker,
        };
        temp_client.run_domain_agent::<crate::llm::RiskAgentData>(
            "Risk", crate::llm::RISK_AGENT_PROMPT, &risk_ctx, Some(&p_key)
        ).await
    });

    // Position agent
    let pos_key = client_pos.clone();
    let pos_url = base_url.clone();
    let pos_model = model.clone();
    let pos_tracker = tracker.clone();
    let p_key = pair_key.clone();
    let pos_ctx = context_position.clone();
    let h_pos = tokio::spawn(async move {
        let temp_client = LlmClient {
                failover_state: None,
            base_url: pos_url,
            api_key: pos_key,
            model: pos_model,
            indicators_guide: String::new(),
            token_tracker: pos_tracker,
        };
        temp_client.run_domain_agent::<crate::llm::PositionAgentData>(
            "Position", crate::llm::POSITION_AGENT_PROMPT, &pos_ctx, Some(&p_key)
        ).await
    });

    let r_trend = h_trend.await.map_err(|e| format!("Task join error: {}", e))??;
    let r_vol = h_vol.await.map_err(|e| format!("Task join error: {}", e))??;
    let r_struct = h_struct.await.map_err(|e| format!("Task join error: {}", e))??;
    let r_risk = h_risk.await.map_err(|e| format!("Task join error: {}", e))??;
    let r_pos = h_pos.await.map_err(|e| format!("Task join error: {}", e))??;

    crate::db::insert_agent_thought_log(&pool, master_id, "Trend", &r_trend.thought, &serde_json::to_string(&r_trend.data).unwrap_or_default(), r_trend.data.confidence_score).await;
    crate::db::insert_agent_thought_log(&pool, master_id, "Volatility", &r_vol.thought, &serde_json::to_string(&r_vol.data).unwrap_or_default(), r_vol.data.volatility_score).await;
    crate::db::insert_agent_thought_log(&pool, master_id, "Structure", &r_struct.thought, &serde_json::to_string(&r_struct.data).unwrap_or_default(), r_struct.data.structural_score).await;
    crate::db::insert_agent_thought_log(&pool, master_id, "Risk", &r_risk.thought, &serde_json::to_string(&r_risk.data).unwrap_or_default(), r_risk.data.exposure_score).await;
    crate::db::insert_agent_thought_log(&pool, master_id, "Position", &r_pos.thought, &serde_json::to_string(&r_pos.data).unwrap_or_default(), 100).await;

    Ok(MultiAgentResults {
        trend: r_trend,
        volatility: r_vol,
        structure: r_struct,
        risk: r_risk,
        position: r_pos,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct DeterministicTelemetry {
    pub market_regime: String,
    pub total_confluence_score: i32,
    pub rvol: f64,
    pub adx_value: f64,
    pub adx_regime: String,
    pub bbwp_percentile: f64,
    pub squeeze_on: bool,
    pub vwap_bias: String,
    pub support_levels: Vec<String>,
    pub resistance_levels: Vec<String>,
    pub rsi_divergence_state: String,
    pub macd_divergence_state: String,
    pub macd_crossover_state: String,
    pub squeeze_release_state: String,
}

pub fn compile_deterministic_telemetry(
    mid: &IndicatorSnapshot,
    support_levels: &[String],
    resistance_levels: &[String],
) -> DeterministicTelemetry {
    let adx = mid.adx.unwrap_or(0.0);
    let bbwp = mid.bbwp.unwrap_or(50.0);
    let squeeze_on = mid.squeeze_on.unwrap_or(false);
    let rvol = mid.rvol.unwrap_or(1.0);
    let atr_regime = mid.atr_volatility_regime.as_deref();
    let ema_stack = mid.ema_stack_state.as_deref();
    let squeeze_released = mid.squeeze_release_trigger.unwrap_or(false);

    // 1. Regime Classification
    let regime = if bbwp < 10.0 || squeeze_on {
        "COMPRESSION"
    } else if squeeze_released || (bbwp > 90.0 && atr_regime == Some("expanding")) {
        "EXPANSION"
    } else if adx >= 25.0 && ema_stack != Some("tangled") && ema_stack.is_some() {
        "TRENDING"
    } else {
        "RANGE"
    };

    // 2. Resolve Trigger vs Confirmation States
    let is_completed = mid.current_price.is_some();

    let macd_crossover_state = if mid.macd_crossover_detected.unwrap_or(false) {
        if is_completed { "confirmed".to_string() } else { "trigger".to_string() }
    } else {
        "none".to_string()
    };

    let squeeze_release_state = if squeeze_released {
        if is_completed { "confirmed".to_string() } else { "trigger".to_string() }
    } else {
        "none".to_string()
    };

    let rsi_div_state = mid.rsi_divergence_status.clone().unwrap_or_else(|| "none".to_string());
    let macd_div_state = mid.macd_divergence_status.clone().unwrap_or_else(|| "none".to_string());

    // 3. Full 100-Point Scoring Protocol
    let mut score = 0;

    // A. RSI Alignment (10 pts)
    if mid.rsi.map_or(false, |r| r < 30.0) { score += 10; }
    else if mid.rsi.map_or(false, |r| r > 70.0) { score -= 10; }

    // B. RSI Divergence (20 pts)
    if rsi_div_state == "confirmed" { score += 20; }
    else if rsi_div_state == "potential" { score += 10; }

    // C. MACD Crossover (10 pts)
    if macd_crossover_state == "confirmed" {
        if mid.macd_crossover_direction.as_deref() == Some("BULLISH") { score += 10; }
        else if mid.macd_crossover_direction.as_deref() == Some("BEARISH") { score -= 10; }
    }

    // D. MACD Divergence (10 pts)
    if macd_div_state == "confirmed" { score += 10; }

    // E. Support/Resistance Alignment (10 pts)
    let cp = mid.current_price.unwrap_or(0.0);
    let s_f64: Vec<f64> = support_levels.iter().filter_map(|s| s.parse::<f64>().ok()).collect();
    let r_f64: Vec<f64> = resistance_levels.iter().filter_map(|r| r.parse::<f64>().ok()).collect();
    if s_f64.iter().any(|&s| (cp - s).abs() < s * 0.005) { score += 10; }
    if r_f64.iter().any(|&r| (cp - r).abs() < r * 0.005) { score -= 10; }

    // F. Macro Trend Alignment (20 pts)
    if let (Some(ema), Some(px)) = (mid.ema_long, mid.current_price) {
        if px > ema { score += 20; } else { score -= 20; }
    }

    // G. EMA Stacking (10 pts)
    if ema_stack == Some("bullish") { score += 10; }
    else if ema_stack == Some("bearish") { score -= 10; }

    // H. Chart Patterns / Volatility Breakout (10 pts)
    if squeeze_release_state == "confirmed" { score += 10; }

    DeterministicTelemetry {
        market_regime: regime.to_string(),
        total_confluence_score: score,
        rvol,
        adx_value: adx,
        adx_regime: mid.adx_regime.clone().unwrap_or_else(|| "congestion".to_string()),
        bbwp_percentile: bbwp,
        squeeze_on,
        vwap_bias: mid.vwap_bias.clone().unwrap_or_else(|| "equilibrium".to_string()),
        support_levels: support_levels.to_vec(),
        resistance_levels: resistance_levels.to_vec(),
        rsi_divergence_state: rsi_div_state,
        macd_divergence_state: macd_div_state,
        macd_crossover_state,
        squeeze_release_state,
    }
}

async fn serve_chat(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatHistoryRequest>,
) -> impl IntoResponse {
    let llm = state.llm_client.read().await;
    match llm.chat(payload.history, None).await {
        Ok(reply) => Json(ChatReplResponse { reply }).into_response(),
        Err(e) => {
            eprintln!("⚠️  LLM chat failed: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Chat request failed: {}", e),
            )
                .into_response()
        }
    }
}

async fn serve_assistant_records(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AssistantRecordsQuery>,
) -> impl IntoResponse {
    let records = match &query.trigger_type {
        Some(t) => crate::db::query_master_records_by_trigger(&state.pool, t, 50).await,
        None => crate::db::query_master_records(&state.pool, 50).await,
    };
    let default_symbol = state.config.read().await.symbols.first().cloned().unwrap_or_default();
    let latest_close = {
        let pair_key = default_pair_key(&default_symbol);
        match get_active_pair(&state.workspace, &pair_key).await {
            Some(pair) => {
                let hist = pair.micro.history.read().await;
                hist.back().map(|c| c.close.to_string()).unwrap_or_else(|| "0".to_string())
            }
            None => "0".to_string(),
        }
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
        }})
        .collect();

    Json(MasterHistoryResponse {
        records: records_json,
        latest_close,
    })
}

async fn serve_cost_estimate(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CostEstimateQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    let costs = config.costs.clone();
    let llm = state.llm_client.read().await;

    let pair_key = query.pair_key.unwrap_or_else(|| {
        let first = config.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    });

    let interval_seconds = config.instances.get(&pair_key)
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

    let projected_daily_cost =
        (daily_input_tokens / 1_000_000.0) * costs.price_per_1m_input_tokens
        + (daily_output_tokens / 1_000_000.0) * costs.price_per_1m_output_tokens;

    let usage = llm.get_token_usage_for_pair(&pair_key);
    let actual_input = usage.input_tokens;
    let actual_output = usage.output_tokens;
    let actual_total_cost =
        (actual_input as f64 / 1_000_000.0) * costs.price_per_1m_input_tokens
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

async fn serve_automated_performance(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let records = crate::db::query_automated_performance(&state.pool, 50).await;

    #[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize)]
pub struct PaperStatusQuery {
    #[serde(default)]
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct PaperConfigRequest {
    pub symbol: String,
    pub initial_usd: f64,
    pub allocation_pct: f64,
    pub auto_execute: bool,
    #[serde(default = "default_max_risk_pct")]
    pub max_risk_pct: f64,
    #[serde(default = "default_leverage")]
    pub leverage: i32,
    #[serde(default = "default_auto_execute_intervals")]
    pub auto_execute_intervals: i32,
    #[serde(default = "default_lookback_trades")]
    pub lookback_trades: i32,
}

fn default_max_risk_pct() -> f64 { 2.0 }
fn default_leverage() -> i32 { 20 }
fn default_auto_execute_intervals() -> i32 { 15 }
fn default_lookback_trades() -> i32 { 10 }

#[derive(Debug, Deserialize)]
pub struct PaperResetRequest {
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct PaperOrderRequest {
    pub symbol: String,
    pub direction: String,
    pub action: String,
}

#[derive(Debug, Deserialize)]
pub struct PaperScaleInRequest {
    pub symbol: String,
    pub direction: String,
    pub entry_price: f64,
    pub portion_number: i32,
    #[serde(default)]
    pub final_invalidation_level: f64,
}

#[derive(Debug, Deserialize)]
pub struct PaperScaleOutRequest {
    pub symbol: String,
    pub exit_price: f64,
    #[serde(default = "default_size_fraction")]
    pub size_fraction: f64,
    #[serde(default)]
    pub target_id: i64,
    #[serde(default = "default_trigger_source")]
    pub trigger_source: String,
}

fn default_size_fraction() -> f64 { 0.5 }
fn default_trigger_source() -> String { "AUTOMATED".to_string() }

#[derive(Debug, Deserialize)]
pub struct PaperPerformanceQuery {
    #[serde(default)]
    pub symbol: Option<String>,
}

async fn serve_paper_status(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaperStatusQuery>,
) -> impl IntoResponse {
    let symbol = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    } else {
        query.symbol
    };

    let pair_arc = get_active_pair(&state.workspace, &symbol).await;
    let current_price = if let Some(pair) = pair_arc {
        let snap = pair.micro.latest_snapshot.read().await;
        snap.as_ref()
            .and_then(|s| s.mid_price.to_string().parse::<f64>().ok())
            .unwrap_or(0.0)
    } else {
        0.0
    };

    let metrics = crate::db::paper_get_account_metrics(&state.pool, &symbol, current_price).await;

    Json(metrics)
}

async fn serve_paper_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperConfigRequest>,
) -> impl IntoResponse {
    let allocation = payload.allocation_pct.clamp(1.0, 100.0);
    crate::db::paper_set_advanced_config(
        &state.pool,
        &payload.symbol,
        payload.initial_usd,
        allocation,
        payload.auto_execute,
        payload.max_risk_pct,
        payload.leverage,
        payload.auto_execute_intervals,
        payload.lookback_trades,
    ).await;

    println!(
        "📄 Paper Config: {} initial=${:.2} allocation={:.1}% auto_execute={} risk={:.1}% leverage={}x interval={}m lookback={}",
        payload.symbol, payload.initial_usd, allocation, payload.auto_execute,
        payload.max_risk_pct, payload.leverage, payload.auto_execute_intervals, payload.lookback_trades
    );
    (axum::http::StatusCode::OK, "Paper trading config saved").into_response()
}

async fn serve_paper_reset(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperResetRequest>,
) -> impl IntoResponse {
    let position = crate::db::paper_get_active_position(&state.pool, &payload.symbol).await;
    if position.is_some() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let _ = state.telemetry_tx.send(crate::db::TelemetryMsg::PaperClosePosition {
            symbol: payload.symbol.clone(),
            exit_price: 0.0,
            exit_timestamp: now,
            trigger: "RESET".to_string(),
        }).await;
    }

    crate::db::paper_reset_account(&state.pool, &payload.symbol).await;
    (axum::http::StatusCode::OK, "Paper account reset").into_response()
}

async fn serve_paper_order(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperOrderRequest>,
) -> impl IntoResponse {
    let pair_arc = get_active_pair(&state.workspace, &payload.symbol).await;
    let current_price = if let Some(pair) = pair_arc {
        let snap = pair.micro.latest_snapshot.read().await;
        snap.as_ref()
            .and_then(|s| s.mid_price.to_string().parse::<f64>().ok())
            .unwrap_or(0.0)
    } else {
        0.0
    };

    if current_price <= 0.0 {
        return (axum::http::StatusCode::BAD_REQUEST, "No price data available for this pair").into_response();
    }

    if payload.action == "CLOSE" {
        let result = crate::paper_trading::close_paper_position(
            &state.pool,
            &state.telemetry_tx,
            &payload.symbol,
            current_price,
            "MANUAL",
        ).await;

        if result.success {
            (axum::http::StatusCode::OK, result.message).into_response()
        } else {
            (axum::http::StatusCode::BAD_REQUEST, result.message).into_response()
        }
    } else if payload.action == "OPEN" {
        let dir = payload.direction.to_uppercase();
        if dir != "LONG" && dir != "SHORT" {
            return (axum::http::StatusCode::BAD_REQUEST, "Direction must be LONG or SHORT").into_response();
        }

        let result = crate::paper_trading::verify_margin_and_open(
            &state.pool,
            &state.telemetry_tx,
            &payload.symbol,
            &dir,
            current_price,
        ).await;

        if result.success {
            (axum::http::StatusCode::CREATED, result.message).into_response()
        } else {
            (axum::http::StatusCode::BAD_REQUEST, result.message).into_response()
        }
    } else {
        (axum::http::StatusCode::BAD_REQUEST, "Action must be OPEN or CLOSE").into_response()
    }
}

async fn serve_paper_scale_in(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperScaleInRequest>,
) -> impl IntoResponse {
    let pair_arc = get_active_pair(&state.workspace, &payload.symbol).await;
    let current_price = if let Some(pair) = pair_arc {
        let snap = pair.micro.latest_snapshot.read().await;
        snap.as_ref()
            .and_then(|s| s.mid_price.to_string().parse::<f64>().ok())
            .unwrap_or(payload.entry_price)
    } else {
        payload.entry_price
    };

    let price = if payload.entry_price > 0.0 { payload.entry_price } else { current_price };

    if price <= 0.0 {
        return Json(serde_json::json!({"success": false, "message": "No price data available"})).into_response();
    }

    let dir = payload.direction.to_uppercase();
    if dir != "LONG" && dir != "SHORT" {
        return Json(serde_json::json!({"success": false, "message": "Direction must be LONG or SHORT"})).into_response();
    }

    let portion = payload.portion_number.max(1).min(3);

    let result = crate::paper_trading::scale_in_portion(
        &state.pool,
        &state.telemetry_tx,
        &payload.symbol,
        &dir,
        price,
        portion,
        payload.final_invalidation_level,
    ).await;

    Json(serde_json::json!({
        "success": result.success,
        "message": result.message,
        "new_average_entry_price": result.new_average_entry_price,
        "total_size": result.total_size,
        "portion_number": result.portion_number,
    })).into_response()
}

async fn serve_paper_scale_out(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperScaleOutRequest>,
) -> impl IntoResponse {
    let pair_arc = get_active_pair(&state.workspace, &payload.symbol).await;
    let current_price = if let Some(pair) = pair_arc {
        let snap = pair.micro.latest_snapshot.read().await;
        snap.as_ref()
            .and_then(|s| s.mid_price.to_string().parse::<f64>().ok())
            .unwrap_or(payload.exit_price)
    } else {
        payload.exit_price
    };

    let price = if payload.exit_price > 0.0 { payload.exit_price } else { current_price };
    let fraction = payload.size_fraction.clamp(0.01, 1.0);

    let result = crate::paper_trading::scale_out_portion(
        &state.pool,
        &state.telemetry_tx,
        &payload.symbol,
        price,
        fraction,
        payload.target_id,
        &payload.trigger_source,
    ).await;

    Json(serde_json::json!({
        "success": result.success,
        "message": result.message,
        "realized_pnl": result.realized_pnl,
        "remaining_size": result.remaining_size,
    }))
}

async fn serve_paper_unrealized(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaperStatusQuery>,
) -> impl IntoResponse {
    let symbol = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    } else {
        query.symbol
    };

    let pair_arc = get_active_pair(&state.workspace, &symbol).await;
    let current_price = if let Some(pair) = pair_arc {
        let snap = pair.micro.latest_snapshot.read().await;
        snap.as_ref()
            .and_then(|s| s.mid_price.to_string().parse::<f64>().ok())
            .unwrap_or(0.0)
    } else {
        0.0
    };

    let metrics = crate::db::paper_get_account_metrics(&state.pool, &symbol, current_price).await;

    #[derive(serde::Serialize)]
    struct UnrealizedResponse {
        symbol: String,
        direction: String,
        average_entry_price: f64,
        current_price: f64,
        size: f64,
        unrealized_pnl_usd: f64,
        unrealized_roi_pct: f64,
        final_invalidation_level: f64,
        filled_portions: i32,
        active_take_profit_targets: Vec<serde_json::Value>,
    }

    let direction = metrics.active_position.as_ref().map(|p| p.direction.clone()).unwrap_or_default();
    let avg_entry = metrics.active_position.as_ref().and_then(|p| p.average_entry_price).unwrap_or(0.0);
    let size = metrics.active_position.as_ref().map(|p| p.size).unwrap_or(0.0);
    let invalidation = metrics.active_position.as_ref().and_then(|p| p.final_invalidation_level).unwrap_or(0.0);
    let filled = metrics.active_position.as_ref().and_then(|p| p.current_portions).unwrap_or(0);

    let targets: Vec<serde_json::Value> = metrics.take_profit_targets.iter()
        .map(|t| serde_json::json!({
            "id": t.id,
            "target_price": t.target_price,
            "size_fraction": t.size_fraction,
            "is_hit": t.is_hit,
        }))
        .collect();

    Json(UnrealizedResponse {
        symbol,
        direction,
        average_entry_price: avg_entry,
        current_price,
        size,
        unrealized_pnl_usd: metrics.unrealized_pnl,
        unrealized_roi_pct: metrics.unrealized_roi_pct,
        final_invalidation_level: invalidation,
        filled_portions: filled,
        active_take_profit_targets: targets,
    })
}

async fn serve_paper_performance(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaperPerformanceQuery>,
) -> impl IntoResponse {
    let trades = crate::db::paper_query_trades(&state.pool, query.symbol.as_deref(), 100).await;

    #[derive(Debug, Serialize)]
    struct PaperPerformanceResponse {
        trades: Vec<crate::db::PaperTradeRecord>,
        total_trades: usize,
        wins: usize,
        losses: usize,
        win_rate: f64,
        profit_factor: f64,
        total_pnl: f64,
        avg_roi: f64,
        max_drawdown_pct: f64,
    }

    let total = trades.len();
    let wins = trades.iter().filter(|t| t.realized_pnl > 0.0).count();
    let losses = trades.iter().filter(|t| t.realized_pnl < 0.0).count();
    let win_rate = if total > 0 { wins as f64 / total as f64 } else { 0.0 };

    let gross_profit: f64 = trades.iter().filter(|t| t.realized_pnl > 0.0).map(|t| t.realized_pnl).sum();
    let gross_loss: f64 = trades.iter().filter(|t| t.realized_pnl < 0.0).map(|t| t.realized_pnl.abs()).sum();
    let profit_factor = if gross_loss > 0.0 { gross_profit / gross_loss } else if gross_profit > 0.0 { f64::INFINITY } else { 0.0 };

    let total_pnl: f64 = trades.iter().map(|t| t.realized_pnl).sum();
    let avg_roi = if total > 0 { trades.iter().map(|t| t.roi_pct).sum::<f64>() / total as f64 } else { 0.0 };

    let mut cumulative = 0.0;
    let mut peak = 0.0;
    let mut max_dd = 0.0;
    for t in trades.iter().rev() {
        cumulative += t.realized_pnl;
        if cumulative > peak { peak = cumulative; }
        let dd = peak - cumulative;
        if dd > max_dd { max_dd = dd; }
    }
    let max_drawdown_pct = if peak > 0.0 { (max_dd / peak) * 100.0 } else { 0.0 };

    Json(PaperPerformanceResponse {
        trades,
        total_trades: total,
        wins,
        losses,
        win_rate,
        profit_factor,
        total_pnl,
        avg_roi,
        max_drawdown_pct,
    })
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pair_key = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    } else {
        query.symbol
    };
    let tf_secs = query.timeframe_secs.unwrap_or(60);
    ws.on_upgrade(move |socket| handle_ws_socket(socket, state, pair_key, tf_secs))
}

async fn handle_ws_socket(mut socket: WebSocket, state: Arc<AppState>, pair_key: String, tf_secs: u64) {
    let rx = {
        let instances = state.workspace.instances.read().await;
        match instances.get(&pair_key) {
            Some(instance) => {
                let pair = &instance.active_pair;
                if tf_secs == 300 {
                    pair.short.broadcast_tx.subscribe()
                } else if tf_secs == 900 {
                    pair.medium.broadcast_tx.subscribe()
                } else {
                    pair.micro.broadcast_tx.subscribe()
                }
            }
            None => return,
        }
    };

    let mut rx_stream = rx;
    loop {
        match rx_stream.recv().await {
            Ok(snapshot) => {
                let symbol = snapshot.symbol.clone();
                let tf = snapshot.timeframe_secs;
                if let Ok(payload) = serde_json::to_value(&snapshot) {
                    let notif = JsonRpcNotification::new(
                        "broadcast.market_snapshot",
                        serde_json::json!({
                            "symbol": symbol,
                            "timeframe_secs": tf,
                            "snapshot": payload,
                        }),
                    );
                    if let Ok(json_str) = serde_json::to_string(&notif) {
                        if socket.send(AxumMessage::Text(json_str.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                eprintln!("⚠️ WS: Client fell behind by {} snapshots, resuming...", missed);
                continue;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                break;
            }
        }
    }
}

async fn serve_set_key(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetKeyRequest>,
) -> impl IntoResponse {
    let key = payload.api_key.trim().to_string();
    if key.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "API key cannot be empty").into_response();
    }

    {
        let mut llm = state.llm_client.write().await;
        llm.set_api_key(key.clone());
    }

    let llm = state.llm_client.read().await;
    match llm.validate_key().await {
        Ok(()) => {
            drop(llm);
            let mut llm = state.llm_client.write().await;
            llm.set_api_key(key.clone());
            drop(llm);

            let env_entry = format!("DEEPSEEK_API_KEY={}", key);
            if let Err(e) = std::fs::write(".env", &env_entry) {
                eprintln!("⚠️ Failed to persist API key to .env: {}", e);
            }

            state.api_key_configured.store(true, std::sync::atomic::Ordering::Relaxed);
            println!("✅ API key configured and validated successfully.");
            (axum::http::StatusCode::OK, "API key validated and saved.").into_response()
        }
        Err(e) => {
            state.api_key_configured.store(false, std::sync::atomic::Ordering::Relaxed);
            eprintln!("❌ API key validation failed: {}", e);
            (axum::http::StatusCode::UNAUTHORIZED, format!("Key validation failed: {}", e)).into_response()
        }
    }
}

async fn serve_get_rules() -> impl IntoResponse {
    match std::fs::read_to_string("docs/indicators-guide.md") {
        Ok(content) => Json(RulesResponse { content }).into_response(),
        Err(e) => {
            eprintln!("❌ Failed to read indicators guide: {}", e);
            (axum::http::StatusCode::NOT_FOUND, "Indicators guide not found").into_response()
        }
    }
}

async fn serve_set_rules(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetRulesRequest>,
) -> impl IntoResponse {
    if let Err(e) = std::fs::write("docs/indicators-guide.md", &payload.content) {
        eprintln!("❌ Failed to write indicators guide: {}", e);
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to save rules").into_response();
    }

    {
        let mut llm = state.llm_client.write().await;
        llm.set_indicators_guide(payload.content);
    }

    println!("✅ Indicators guide updated successfully.");
    (axum::http::StatusCode::OK, "Rules updated successfully.").into_response()
}

async fn serve_add_trade(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddTradeRequest>,
) -> impl IntoResponse {
    let outcome_upper = payload.outcome.trim().to_uppercase();
    if outcome_upper != "WIN" && outcome_upper != "LOSS" {
        return (axum::http::StatusCode::BAD_REQUEST, "Outcome must be WIN or LOSS").into_response();
    }

    match crate::db::insert_user_trade(
        &state.pool,
        &payload.symbol,
        &payload.direction,
        &outcome_upper,
        payload.risk_multiplier,
        payload.reward_multiplier,
    )
    .await
    {
        Ok(id) => {
            (axum::http::StatusCode::CREATED, format!("Trade logged with ID {}", id)).into_response()
        }
        Err(e) => {
            eprintln!("❌ Web API Error: Failed to log trade record: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to persist trade record")
                .into_response()
        }
    }
}

async fn serve_get_trades(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let trades = crate::db::query_user_trades(&state.pool, 100).await;
    Json(trades)
}

// ─── Decision Profiles ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DecisionProfileCreate {
    pub profile_name: String,
    #[serde(default = "default_long_threshold")]
    pub long_threshold: i32,
    #[serde(default = "default_short_threshold")]
    pub short_threshold: i32,
}
fn default_long_threshold() -> i32 { 40 }
fn default_short_threshold() -> i32 { -40 }

#[derive(Debug, Deserialize)]
pub struct DecisionProfileUpdate {
    pub profile_name: String,
    pub long_threshold: i32,
    pub short_threshold: i32,
}

#[derive(Debug, Deserialize)]
pub struct ProfileIndicatorAdd {
    pub indicator_name: String,
    #[serde(default = "default_weight")]
    pub weight: i32,
    #[serde(default)]
    pub override_status: String,
}
fn default_weight() -> i32 { 10 }

#[derive(Debug, Deserialize)]
pub struct ProfileIndicatorUpdate {
    pub weight: i32,
    #[serde(default)]
    pub override_status: String,
}

#[derive(Debug, Deserialize)]
pub struct EvaluateRequest {
    pub rsi: Option<f64>,
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<f64>,
    pub macd_line: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_hist: Option<f64>,
    pub adx: Option<f64>,
    pub adx_plus: Option<f64>,
    pub adx_minus: Option<f64>,
    pub bb_upper: Option<f64>,
    pub bb_middle: Option<f64>,
    pub bb_lower: Option<f64>,
    pub atr: Option<f64>,
    pub ema_fast: Option<f64>,
    pub ema_medium: Option<f64>,
    pub ema_slow: Option<f64>,
    pub ema_long: Option<f64>,
    #[serde(default)]
    pub ema_stack_state: Option<String>,
    pub vwap: Option<f64>,
    #[serde(default)]
    pub vwap_bias: Option<String>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub average_volume: Option<f64>,
    #[serde(default)]
    pub rvol: Option<f64>,
    pub current_price: f64,
    #[serde(default)]
    pub historical_prices: Vec<f64>,
    #[serde(default)]
    pub rsi_divergence_status: Option<String>,
    #[serde(default)]
    pub macd_divergence_status: Option<String>,
    #[serde(default)]
    pub macd_trend_state: Option<String>,
    #[serde(default)]
    pub macd_crossover_detected: Option<bool>,
    #[serde(default)]
    pub macd_crossover_direction: Option<String>,
    #[serde(default)]
    pub macd_histogram_peak: Option<f64>,
    #[serde(default)]
    pub squeeze_duration: Option<u32>,
    #[serde(default)]
    pub squeeze_release_trigger: Option<bool>,
    #[serde(default)]
    pub squeeze_momentum_direction: Option<String>,
    #[serde(default)]
    pub chart_pattern: Option<String>,
    #[serde(default)]
    pub chart_pattern_confidence: Option<f64>,
    #[serde(default)]
    pub bbwp: Option<f64>,
    #[serde(default)]
    pub atr_volatility_regime: Option<String>,
    #[serde(default)]
    pub atr_slope: Option<f64>,
    #[serde(default)]
    pub adx_slope: Option<f64>,
    #[serde(default)]
    pub adx_regime: Option<String>,
    #[serde(default)]
    pub adx_di_crossover_detected: Option<bool>,
    #[serde(default)]
    pub adx_di_crossover_direction: Option<String>,
}

async fn serve_decision_profiles_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let profiles = crate::db::decision_profiles_list(&state.pool).await;
    Json(profiles)
}

async fn serve_decision_profile_create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DecisionProfileCreate>,
) -> impl IntoResponse {
    if payload.profile_name.trim().is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Profile name required").into_response();
    }
    let id = crate::db::decision_profile_insert(
        &state.pool, payload.profile_name.trim(), payload.long_threshold, payload.short_threshold,
    ).await;
    if id > 0 {
        (axum::http::StatusCode::CREATED, format!("Profile created with id {}", id)).into_response()
    } else {
        (axum::http::StatusCode::CONFLICT, "Profile name already exists or DB error").into_response()
    }
}

async fn serve_decision_profile_update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<DecisionProfileUpdate>,
) -> impl IntoResponse {
    let ok = crate::db::decision_profile_update(
        &state.pool, id, &payload.profile_name, payload.long_threshold, payload.short_threshold,
    ).await;
    if ok {
        (axum::http::StatusCode::OK, "Profile updated").into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Profile not found").into_response()
    }
}

async fn serve_decision_profile_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    crate::db::decision_profile_delete(&state.pool, id).await;
    (axum::http::StatusCode::OK, "Profile deleted").into_response()
}

async fn serve_decision_evaluate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<EvaluateRequest>,
) -> impl IntoResponse {
    let snap = crate::profile_evaluation::SnapshotValues {
        rsi: payload.rsi,
        squeeze_on: payload.squeeze_on,
        squeeze_momentum: payload.squeeze_momentum,
        macd_line: payload.macd_line,
        macd_signal: payload.macd_signal,
        macd_hist: payload.macd_hist,
        adx: payload.adx,
        adx_plus: payload.adx_plus,
        adx_minus: payload.adx_minus,
        bb_upper: payload.bb_upper,
        bb_middle: payload.bb_middle,
        bb_lower: payload.bb_lower,
        atr: payload.atr,
        ema_fast: payload.ema_fast,
        ema_medium: payload.ema_medium,
        ema_slow: payload.ema_slow,
        ema_long: payload.ema_long,
        ema_stack_state: payload.ema_stack_state,
        vwap: payload.vwap,
        vwap_bias: payload.vwap_bias,
        close: payload.close,
        volume: payload.volume,
        average_volume: payload.average_volume,
        rvol: payload.rvol,
        current_price: payload.current_price,
        rsi_divergence_status: payload.rsi_divergence_status,
        macd_divergence_status: payload.macd_divergence_status,
        macd_trend_state: payload.macd_trend_state,
        macd_crossover_detected: payload.macd_crossover_detected,
        macd_crossover_direction: payload.macd_crossover_direction,
        macd_histogram_peak: payload.macd_histogram_peak,
        squeeze_duration: payload.squeeze_duration,
        squeeze_release_trigger: payload.squeeze_release_trigger,
        squeeze_momentum_direction: payload.squeeze_momentum_direction,
        chart_pattern: payload.chart_pattern,
        chart_pattern_confidence: payload.chart_pattern_confidence,
        bbwp: payload.bbwp,
        atr_volatility_regime: payload.atr_volatility_regime,
        adx_slope: payload.adx_slope,
        adx_regime: payload.adx_regime,
        adx_di_crossover_detected: payload.adx_di_crossover_detected,
        adx_di_crossover_direction: payload.adx_di_crossover_direction,
    };
    let score = crate::profile_evaluation::evaluate_profile(
        &state.pool, id, &snap, &payload.historical_prices,
    ).await;
    Json(score)
}

async fn serve_profile_indicator_add(
    State(state): State<Arc<AppState>>,
    Path(profile_id): Path<i64>,
    Json(payload): Json<ProfileIndicatorAdd>,
) -> impl IntoResponse {
    let status = if payload.override_status.is_empty() { "NONE" } else { &payload.override_status };
    let id = crate::db::profile_indicator_insert(
        &state.pool, profile_id, &payload.indicator_name, payload.weight, status,
    ).await;
    if id > 0 {
        (axum::http::StatusCode::CREATED, format!("Indicator added with id {}", id)).into_response()
    } else {
        (axum::http::StatusCode::BAD_REQUEST, "Failed to add indicator").into_response()
    }
}

async fn serve_profile_indicator_update(
    State(state): State<Arc<AppState>>,
    Path((_profile_id, indicator_id)): Path<(i64, i64)>,
    Json(payload): Json<ProfileIndicatorUpdate>,
) -> impl IntoResponse {
    let status = if payload.override_status.is_empty() { "NONE" } else { &payload.override_status };
    let ok = crate::db::profile_indicator_update(&state.pool, indicator_id, payload.weight, status).await;
    if ok {
        (axum::http::StatusCode::OK, "Indicator updated").into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Indicator not found").into_response()
    }
}

async fn serve_profile_indicator_delete(
    State(state): State<Arc<AppState>>,
    Path((_profile_id, indicator_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    crate::db::profile_indicator_delete(&state.pool, indicator_id).await;
    (axum::http::StatusCode::OK, "Indicator removed").into_response()
}

// ─── Risk Profiles ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RiskProfileCreate {
    pub profile_name: String,
    #[serde(default = "default_capital")]
    pub capital: f64,
    #[serde(default = "default_max_risk")]
    pub max_risk_pct: f64,
    #[serde(default = "default_leverage_i32")]
    pub leverage: i32,
    #[serde(default = "default_commission")]
    pub commission_pct: f64,
    #[serde(default)]
    pub funding_rate_8h: f64,
    #[serde(default)]
    pub spread: f64,
}
fn default_capital() -> f64 { 1000.0 }
fn default_max_risk() -> f64 { 2.0 }
fn default_leverage_i32() -> i32 { 20 }
fn default_commission() -> f64 { 0.06 }

#[derive(Debug, Deserialize)]
pub struct RiskCalculateRequest {
    pub direction: String,
    pub entry_price: f64,
    #[serde(default)]
    pub stop_loss_price: f64,
    #[serde(default)]
    pub take_profit_price: f64,
    #[serde(default)]
    pub profile_id: Option<i64>,
    pub capital: Option<f64>,
    pub max_risk_pct: Option<f64>,
    pub leverage: Option<i32>,
    pub commission_pct: Option<f64>,
    pub funding_rate_8h: Option<f64>,
    pub spread: Option<f64>,
    #[serde(default)]
    pub atr_value: Option<f64>,
    #[serde(default)]
    pub atr_multiplier: Option<f64>,
    #[serde(default)]
    pub atr_target_rr: Option<f64>,
    #[serde(default)]
    pub use_dynamic_atr: bool,
}

#[derive(Debug, Deserialize)]
pub struct CommissionProjectionPayload {
    pub direction: String,
    pub entry_1: f64,
    pub entry_2: f64,
    pub stop_loss_1: f64,
    pub stop_loss_2: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,
    #[serde(default)]
    pub profile_id: Option<i64>,
    pub capital: Option<f64>,
    pub max_risk_pct: Option<f64>,
    pub leverage: Option<i32>,
    pub capital_entry_1_pct: Option<f64>,
    pub order_type: Option<String>,
    pub commission_pct: Option<f64>,
    pub funding_rate_8h: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct FeeTableQuery {
    pub leverages: Option<Vec<u32>>,
    pub capitals: Option<Vec<f64>>,
    pub order_type: Option<String>,
}

async fn serve_risk_profiles_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let profiles = crate::db::risk_profiles_list(&state.pool).await;
    Json(profiles)
}

async fn serve_risk_profile_create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RiskProfileCreate>,
) -> impl IntoResponse {
    if payload.profile_name.trim().is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Profile name required").into_response();
    }
    let id = crate::db::risk_profile_insert(
        &state.pool, payload.profile_name.trim(),
        payload.capital, payload.max_risk_pct, payload.leverage,
        payload.commission_pct, payload.funding_rate_8h, payload.spread,
    ).await;
    if id > 0 {
        (axum::http::StatusCode::CREATED, format!("Risk profile created with id {}", id)).into_response()
    } else {
        (axum::http::StatusCode::CONFLICT, "Profile name already exists or DB error").into_response()
    }
}

async fn serve_risk_profile_update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<RiskProfileCreate>,
) -> impl IntoResponse {
    let ok = crate::db::risk_profile_update(
        &state.pool, id, &payload.profile_name,
        payload.capital, payload.max_risk_pct, payload.leverage,
        payload.commission_pct, payload.funding_rate_8h, payload.spread,
    ).await;
    if ok {
        (axum::http::StatusCode::OK, "Risk profile updated").into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Risk profile not found").into_response()
    }
}

async fn serve_risk_profile_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    crate::db::risk_profile_delete(&state.pool, id).await;
    (axum::http::StatusCode::OK, "Risk profile deleted").into_response()
}

async fn serve_risk_calculate(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RiskCalculateRequest>,
) -> impl IntoResponse {
    let (capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread) =
        if let Some(pid) = payload.profile_id {
            if let Some(profile) = crate::db::risk_profile_by_id(&state.pool, pid).await {
                (profile.capital, profile.max_risk_pct, profile.leverage,
                 profile.commission_pct, profile.funding_rate_8h, profile.spread)
            } else {
                (payload.capital.unwrap_or(1000.0), payload.max_risk_pct.unwrap_or(2.0),
                 payload.leverage.unwrap_or(20), payload.commission_pct.unwrap_or(0.06),
                 payload.funding_rate_8h.unwrap_or(0.0), payload.spread.unwrap_or(0.0))
            }
        } else {
            (payload.capital.unwrap_or(1000.0), payload.max_risk_pct.unwrap_or(2.0),
             payload.leverage.unwrap_or(20), payload.commission_pct.unwrap_or(0.06),
             payload.funding_rate_8h.unwrap_or(0.0), payload.spread.unwrap_or(0.0))
        };

    let input = crate::risk_calculator::RiskCalculationInput {
        capital,
        max_risk_pct,
        leverage,
        direction: payload.direction,
        entry_price: payload.entry_price,
        stop_loss_price: payload.stop_loss_price,
        take_profit_price: payload.take_profit_price,
        commission_pct,
        funding_rate_8h,
        spread,
        atr_value: payload.atr_value,
        atr_multiplier: payload.atr_multiplier,
        atr_target_rr: payload.atr_target_rr,
        use_dynamic_atr: payload.use_dynamic_atr,
    };

    let result = if payload.use_dynamic_atr && payload.atr_value.is_some() {
        crate::risk_calculator::compute_risk_with_atr(&input)
    } else {
        crate::risk_calculator::compute_risk(&input)
    };

    match result {
        Ok(calc) => Json(calc).into_response(),
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
    }
}

async fn serve_fee_table(
    State(state): State<Arc<AppState>>,
    Query(params): Query<FeeTableQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    let leverages = params.leverages.unwrap_or_else(|| vec![10, 20, 25, 40, 50]);
    let capitals = params.capitals.unwrap_or_else(|| vec![10.0, 50.0, 100.0, 500.0]);
    let order_type = params.order_type.unwrap_or_else(|| "taker".to_string());
    let table = crate::commission::generate_fee_table(&config.fees, &leverages, &capitals, &order_type);
    Json(table).into_response()
}

async fn serve_commission_projection(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CommissionProjectionPayload>,
) -> impl IntoResponse {
    let (capital, leverage, max_risk_pct, commission_pct, funding_rate_8h) =
        if let Some(pid) = payload.profile_id {
            if let Some(profile) = crate::db::risk_profile_by_id(&state.pool, pid).await {
                (profile.capital, profile.leverage, profile.max_risk_pct,
                 Some(profile.commission_pct), Some(profile.funding_rate_8h))
            } else {
                (payload.capital.unwrap_or(1000.0), payload.leverage.unwrap_or(20),
                 payload.max_risk_pct.unwrap_or(2.0), payload.commission_pct,
                 payload.funding_rate_8h)
            }
        } else {
            (payload.capital.unwrap_or(1000.0), payload.leverage.unwrap_or(20),
             payload.max_risk_pct.unwrap_or(2.0), payload.commission_pct,
             payload.funding_rate_8h)
        };

    let config = state.config.read().await;
    let input = crate::commission::CommissionProjectionRequest {
        direction: payload.direction,
        entry_1: payload.entry_1,
        entry_2: payload.entry_2,
        stop_loss_1: payload.stop_loss_1,
        stop_loss_2: payload.stop_loss_2,
        take_profit_1: payload.take_profit_1,
        take_profit_2: payload.take_profit_2,
        capital,
        leverage,
        max_risk_pct,
        capital_entry_1_pct: payload.capital_entry_1_pct.unwrap_or(50.0),
        order_type: payload.order_type.unwrap_or_else(|| "taker".to_string()),
        commission_pct,
        funding_rate_8h,
    };

    match crate::commission::compute_commission_projection(&input, &config.fees) {
        Ok(proj) => Json(proj).into_response(),
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
    }
}

// ─── Exchange Keys ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExchangeKeyRequest {
    pub exchange: String,
    pub account_name: String,
    pub api_key: String,
    pub api_secret: String,
    #[serde(default)]
    pub passphrase: String,
    #[serde(default)]
    pub referred_uid: String,
    #[serde(default)]
    pub is_active: bool,
}

async fn serve_exchange_keys_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let keys = crate::db::exchange_keys_list(&state.pool).await;
    let active_count = crate::db::exchange_keys_active_count(&state.pool).await;
    #[derive(Serialize)]
    struct ExchangeKeysResponse {
        accounts: Vec<crate::db::ExchangeKey>,
        active_count: i64,
        max_accounts: i64,
    }
    Json(ExchangeKeysResponse { accounts: keys, active_count, max_accounts: 3 })
}

async fn serve_exchange_keys_add(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ExchangeKeyRequest>,
) -> impl IntoResponse {
    if payload.exchange.is_empty() || payload.account_name.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Exchange and account name required").into_response();
    }
    let id = crate::db::exchange_keys_insert(
        &state.pool, &payload.exchange, &payload.account_name,
        &payload.api_key, &payload.api_secret, &payload.passphrase,
        &payload.referred_uid, payload.is_active,
    ).await;
    if id > 0 {
        (axum::http::StatusCode::CREATED, format!("Exchange key created with id {}", id)).into_response()
    } else {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to add exchange key").into_response()
    }
}

async fn serve_exchange_keys_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    crate::db::exchange_keys_delete(&state.pool, id).await;
    (axum::http::StatusCode::OK, "Exchange key deleted").into_response()
}

async fn serve_exchange_keys_sync(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    crate::db::exchange_keys_update_sync(&state.pool, id, now).await;
    (axum::http::StatusCode::OK, "Sync timestamp updated").into_response()
}

// ─── Dashboard Stats ──────────────────────────────────────────────

async fn serve_dashboard_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let stats = crate::stats_compiler::compile_dashboard_stats(&state.pool).await;
    Json(stats)
}

// ─── Trade Journal ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TradeJournalQuery {
    #[serde(default = "default_journal_limit")]
    pub limit: u32,
}
fn default_journal_limit() -> u32 { 50 }

#[derive(Debug, Deserialize)]
pub struct UpdateJournalNotesRequest {
    pub human_notes: String,
    pub execution_score: f64,
}

async fn serve_trade_journal(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TradeJournalQuery>,
) -> impl IntoResponse {
    let records = crate::db::query_trade_journal(&state.pool, query.limit).await;
    Json(records)
}

async fn serve_update_journal_notes(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateJournalNotesRequest>,
) -> impl IntoResponse {
    let score = payload.execution_score.clamp(0.0, 10.0);
    let ok = crate::db::update_journal_notes(&state.pool, id, &payload.human_notes, score).await;
    if ok {
        (axum::http::StatusCode::OK, "Journal notes updated").into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Journal record not found").into_response()
    }
}

async fn serve_export_journal_csv(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let records = crate::db::query_trade_journal(&state.pool, 1000).await;
    let mut csv = String::from("id,trade_id,entry_date,exit_date,asset,direction,entry_reason,roe_percentage,final_analysis,execution_score,human_notes,symbol,realized_pnl,roi_percentage\n");
    for r in &records {
        let escaped_analysis = r.final_analysis.replace('"', "\"\"");
        let escaped_reason = r.entry_reason.replace('"', "\"\"");
        let escaped_notes = r.human_notes.replace('"', "\"\"");
        csv.push_str(&format!(
            "{},{},{},{},{},{},\"{}\",{:.2},\"{}\",{:.1},\"{}\",{},{:.2},{:.2}\n",
            r.id, r.trade_id, r.entry_date, r.exit_date, r.asset, r.direction,
            escaped_reason, r.roe_percentage, escaped_analysis, r.execution_score,
            escaped_notes, r.symbol, r.realized_pnl, r.roi_percentage,
        ));
    }
    (
        [(header::CONTENT_TYPE, "text/csv; charset=utf-8")],
        csv,
    )
}

async fn serve_export_journal_json(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let records = crate::db::query_trade_journal(&state.pool, 1000).await;
    Json(records)
}

// ─── Trade Ledger ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TradeLedgerQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
}
fn default_limit() -> u32 { 200 }

async fn serve_trade_ledger(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TradeLedgerQuery>,
) -> impl IntoResponse {
    let trades = crate::db::trade_telemetry_query_all(&state.pool, query.limit).await;
    Json(trades)
}

// ─── Trade Telemetry ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TradeTelemetryRequest {
    pub exchange: String,
    pub symbol: String,
    pub direction: String,
    pub entry_timestamp: i64,
    pub exit_timestamp: i64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    #[serde(default)]
    pub commission_fees: f64,
    #[serde(default)]
    pub funding_fees: f64,
    pub realized_pnl: f64,
    #[serde(default)]
    pub roi_percentage: f64,
    #[serde(default = "default_trigger")]
    pub trigger_source: String,
}
fn default_trigger() -> String { "MANUAL".to_string() }

async fn serve_trade_telemetry_add(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TradeTelemetryRequest>,
) -> impl IntoResponse {
    let id = crate::db::trade_telemetry_insert(
        &state.pool, &payload.exchange, &payload.symbol, &payload.direction,
        payload.entry_timestamp, payload.exit_timestamp,
        payload.entry_price, payload.exit_price, payload.size,
        payload.commission_fees, payload.funding_fees,
        payload.realized_pnl, payload.roi_percentage, &payload.trigger_source,
    ).await;
    if id > 0 {
        (axum::http::StatusCode::CREATED, format!("Trade logged with id {}", id)).into_response()
    } else {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to log trade").into_response()
    }
}

// ─── Session Management ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SessionInitRequest {
    pub mode: String,
    pub currency: String,
    pub exchange: String,
    #[serde(default)]
    pub capital: f64,
}

#[derive(Debug, Serialize)]
pub struct SessionStatusResponse {
    pub active: bool,
    pub mode: Option<String>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub capital: Option<f64>,
    pub instance_count: usize,
    pub max_instances: usize,
}

async fn serve_session_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let active = state.workspace.session.active.load(std::sync::atomic::Ordering::Relaxed);
    let mode = state.workspace.session.trading_mode.read().await.clone();
    let currency = state.workspace.session.base_currency.read().await.clone();
    let exchange = state.workspace.session.exchange.read().await.clone();
    let capital = *state.workspace.session.initial_capital.read().await;
    let instance_count = state.workspace.instance_count().await;
    let max_instances = state.workspace.max_instances().await;

    Json(SessionStatusResponse {
        active,
        mode: mode.map(|m| m.as_str().to_string()),
        currency: currency.map(|c| c.as_str().to_string()),
        exchange: exchange.map(|e| e.as_str().to_string()),
        capital,
        instance_count,
        max_instances,
    })
}

async fn serve_session_init(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SessionInitRequest>,
) -> impl IntoResponse {
    let mode = match payload.mode.to_lowercase().as_str() {
        "paper" => crate::workspace::TradingMode::Paper,
        "live" => crate::workspace::TradingMode::Live,
        _ => return (axum::http::StatusCode::BAD_REQUEST, "Invalid mode. Use 'paper' or 'live'.").into_response(),
    };

    let currency = match payload.currency.to_uppercase().as_str() {
        "USDT" => crate::workspace::Currency::USDT,
        "USDC" => crate::workspace::Currency::USDC,
        _ => return (axum::http::StatusCode::BAD_REQUEST, "Invalid currency. Use 'USDT' or 'USDC'.").into_response(),
    };

    let exchange = match payload.exchange.to_lowercase().as_str() {
        "hyperliquid" => crate::workspace::ExchangeChoice::Hyperliquid,
        _ => return (axum::http::StatusCode::BAD_REQUEST, "Invalid exchange. Only 'Hyperliquid' is supported.").into_response(),
    };

    match state.workspace.init_session(mode, currency, exchange, payload.capital).await {
        Ok(()) => {
            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": "Session initialized successfully.",
            }))).into_response()
        }
        Err(e) => {
            (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "success": false,
                "error": e,
            }))).into_response()
        }
    }
}

async fn serve_session_quit(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.workspace.quit_session().await {
        Ok(()) => {
            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": "Session terminated. All instances stopped."
            }))).into_response()
        }
        Err(e) => {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "success": false,
                "error": e,
            }))).into_response()
        }
    }
}

// ─── Instance Management ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddInstanceRequest {
    pub base: String,
    pub quote: String,
}

#[derive(Debug, Serialize)]
pub struct InstanceListResponse {
    pub instances: Vec<instance_registry::InstanceSummary>,
    pub total_count: usize,
    pub max_count: usize,
}

async fn serve_list_instances(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InstanceDetailQuery>,
) -> impl IntoResponse {
    let all_summaries = instance_registry::list_instances(&state.workspace).await;
    let summaries: Vec<_> = if let Some(ref pk) = query.pair_key {
        all_summaries.into_iter().filter(|s| s.pair == *pk).collect()
    } else {
        all_summaries
    };
    let max_count = state.workspace.max_instances().await;
    Json(InstanceListResponse {
        total_count: summaries.len(),
        max_count,
        instances: summaries,
    })
}

async fn serve_add_instance(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddInstanceRequest>,
) -> impl IntoResponse {
    let base = payload.base.trim().to_uppercase();
    let quote = payload.quote.trim().to_uppercase();

    if base.is_empty() || quote.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Base and quote currency required").into_response();
    }
    if base.len() > 10 || quote.len() > 10 {
        return (axum::http::StatusCode::BAD_REQUEST, "Symbol too long").into_response();
    }

    match instance_registry::add_instance(
        &state.workspace,
        (base, quote),
        state.llm_client.clone(),
    ).await {
        Ok(instance) => {
            (
                axum::http::StatusCode::CREATED,
                Json(serde_json::json!({
                    "id": instance.id,
                    "pair": instance.pair_display(),
                    "message": format!("Instance {} created", instance.pair_display()),
                })),
            ).into_response()
        }
        Err(e) => {
            (axum::http::StatusCode::BAD_REQUEST, e).into_response()
        }
    }
}

async fn serve_delete_instance(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    match instance_registry::delete_instance(&state.workspace, &instance_id).await {
        Ok(()) => (axum::http::StatusCode::OK, format!("Instance {} deleted", instance_id)).into_response(),
        Err(e) => (axum::http::StatusCode::NOT_FOUND, e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct InstanceDetailQuery {
    pub pair_key: Option<String>,
}

async fn serve_get_instance_detail(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let instances = state.workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id).cloned();
    drop(instances);

    match instance {
        Some(inst) => {
            let status = inst.status.read().await.as_str().to_string();
            let paper = crate::db::paper_get_account_metrics(&state.pool, &inst.symbol(), 0.0).await;
            Json(serde_json::json!({
                "id": inst.id,
                "pair": inst.pair_display(),
                "symbol": inst.symbol(),
                "status": status,
                "initial_capital": *inst.initial_capital.read().await,
                "current_equity": *inst.current_equity.read().await,
                "paper_balance": paper.current_cash,
                "paper_equity": paper.total_account_value,
                "paper_unrealized_pnl": paper.unrealized_pnl,
                "tp_levels": *inst.tp_levels.read().await,
                "sl_levels": *inst.sl_levels.read().await,
                "consecutive_losses": inst.safety.consecutive_losses.load(std::sync::atomic::Ordering::Relaxed),
                "caution_level": inst.safety.caution_level.read().await.as_str().to_string(),
            }))
            .into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct InstanceConfigPayload {
    pub micro_term: crate::config::TimeframeConfig,
    pub short_term: crate::config::TimeframeConfig,
    #[serde(default)]
    pub medium_term: Option<crate::config::TimeframeConfig>,
    #[serde(default)]
    pub large_term: Option<crate::config::TimeframeConfig>,
    #[serde(default)]
    pub automation: crate::config::AutomationConfig,
}

async fn serve_update_instance_config(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceConfigPayload>,
) -> impl IntoResponse {
    let pair_key = {
        let instances = state.workspace.instances.read().await;
        // Try by instance ID first, then by pair_key directly
        instances.iter()
            .find(|(k, i)| i.id == instance_id || **k == instance_id)
            .map(|(k, _)| k.clone())
    };

    match pair_key {
        Some(pk) => {
            let mut config = state.config.write().await;
            let specific_config = crate::config::InstanceSpecificConfig {
                micro_term: payload.micro_term,
                short_term: payload.short_term,
                medium_term: payload.medium_term,
                large_term: payload.large_term,
                automation: payload.automation,
            };
            config.instances.insert(pk.clone(), specific_config);
            crate::config::save_instances(&config.instances);
            println!("✅ Instance config saved: {}", pk);
            (axum::http::StatusCode::OK, "Instance configuration saved successfully").into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

async fn serve_pause_instance(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    match instance_registry::pause_instance(&state.workspace, &instance_id).await {
        Ok(()) => (axum::http::StatusCode::OK, format!("Instance {} paused", instance_id)).into_response(),
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
    }
}

async fn serve_stop_instance(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    match instance_registry::stop_instance(&state.workspace, &instance_id).await {
        Ok(()) => (axum::http::StatusCode::OK, format!("Instance {} stopped", instance_id)).into_response(),
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
    }
}

async fn serve_reset_safety(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let instances = state.workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id).cloned();
    drop(instances);

    match instance {
        Some(inst) => {
            inst.safety.reset_consecutive_losses().await;
            (axum::http::StatusCode::OK, format!("Safety counter reset for instance {}", instance_id)).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct InstanceManualRequest {
    pub direction: Option<String>,
    pub price: Option<f64>,
}

async fn serve_instance_manual_open(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceManualRequest>,
) -> impl IntoResponse {
    let instances = state.workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id).cloned();
    drop(instances);

    match instance {
        Some(inst) => {
            // Manual entry always resets the safety counter
            inst.safety.reset_consecutive_losses().await;

            let dir = payload.direction.unwrap_or_else(|| "LONG".into());
            println!(
                "✋ Manual Open: {} {} direction={} price={:?}",
                instance_id, inst.pair_display(), dir, payload.price
            );

            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Manual open recorded for {} (safety counter reset)", inst.pair_display()),
                "instance_id": instance_id,
            }))).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

async fn serve_instance_manual_close(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceManualRequest>,
) -> impl IntoResponse {
    let instances = state.workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id).cloned();
    drop(instances);

    match instance {
        Some(inst) => {
            // Record the trade as a manual outcome
            let price = payload.price.unwrap_or(0.0);
            println!(
                "✋ Manual Close: {} {} price={}",
                instance_id, inst.pair_display(), price
            );

            // Manual close resets counter as well (human intervention)
            inst.safety.reset_consecutive_losses().await;

            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Manual close recorded for {} (safety counter reset)", inst.pair_display()),
                "instance_id": instance_id,
            }))).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

// ─── Instance API Key Management ──────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InstanceApiKeyRequest {
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InstanceUsageResponse {
    pub instance_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub failover_active: bool,
    pub failover_source: String,
    pub consecutive_failures: u32,
}

async fn serve_set_instance_api_key(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceApiKeyRequest>,
) -> impl IntoResponse {
    let instances = state.workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id).cloned();
    drop(instances);

    match instance {
        Some(inst) => {
            let key = payload.api_key.trim().to_string();
            if key.is_empty() {
                return (axum::http::StatusCode::BAD_REQUEST, "API key cannot be empty").into_response();
            }
            let _base_url = payload.base_url.unwrap_or_else(|| "https://api.deepseek.com/v1".into());
            let _model = payload.model.unwrap_or_else(|| "deepseek-chat".into());

            // Set the primary key in failover state
            inst.api_failover.set_primary_key(key.clone()).await;
            *inst.api_key.write().await = Some(key.clone());
            inst.api_key_valid.store(true, std::sync::atomic::Ordering::Relaxed);

            println!("🔑 Instance API key set for: {} ({})", inst.pair_display(), instance_id);
            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "instance_id": instance_id,
            }))).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

async fn serve_delete_instance_api_key(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let instances = state.workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id).cloned();
    drop(instances);

    match instance {
        Some(inst) => {
            *inst.api_key.write().await = None;
            inst.api_key_valid.store(false, std::sync::atomic::Ordering::Relaxed);
            println!("🗑️  Instance API key removed for: {} ({})", inst.pair_display(), instance_id);
            (axum::http::StatusCode::OK, "Instance API key removed").into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

// ─── Instance Intervals ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InstanceIntervalsRequest {
    pub slow_seconds: u64,
    pub normal_seconds: u64,
    pub fast_seconds: u64,
}

async fn serve_instance_intervals(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceIntervalsRequest>,
) -> impl IntoResponse {
    let instances = state.workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id).cloned();
    drop(instances);

    match instance {
        Some(inst) => {
            let mut intervals = inst.intervals.write().await;
            intervals.slow_seconds = payload.slow_seconds;
            intervals.normal_seconds = payload.normal_seconds;
            intervals.fast_seconds = payload.fast_seconds;

            println!("⏱️  Instance intervals updated for {} ({}) slow={}s normal={}s fast={}s",
                inst.pair_display(), instance_id,
                payload.slow_seconds, payload.normal_seconds, payload.fast_seconds);

            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "instance_id": instance_id,
                "intervals": {
                    "slow_seconds": payload.slow_seconds,
                    "normal_seconds": payload.normal_seconds,
                    "fast_seconds": payload.fast_seconds,
                },
            }))).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

// ─── Global Backup API Key ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BackupApiKeyRequest {
    pub api_key: String,
}

async fn serve_set_backup_api_key(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BackupApiKeyRequest>,
) -> impl IntoResponse {
    let key = payload.api_key.trim().to_string();
    {
        let mut config = state.config.write().await;
        config.workspace.backup_api_key = if key.is_empty() { None } else { Some(key) };
        if let Ok(toml_str) = toml::to_string_pretty(&*config) {
            let _ = std::fs::write("config.toml", toml_str);
        }
    }
    println!("🔑 Global backup API key updated");
    (axum::http::StatusCode::OK, "Backup API key saved").into_response()
}

// ─── Historical Recommendations ────────────────────────────────────

async fn serve_historical_recommendations(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, (i64, String, String, String, i64, f64, f64, f64, f64, f64, f64, String, String, String)>(
        "SELECT id, symbol, pair_key, generated_at, trades_analyzed, win_rate, avg_risk_reward, \
         avg_hold_time_minutes, profit_factor, suggested_rr, suggested_sizing_pct, \
         regime_analysis, key_improvements, risk_recommendation \
         FROM historical_recommendations ORDER BY generated_at DESC LIMIT 20"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let recommendations: Vec<serde_json::Value> = rows.into_iter().map(|r| {
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
    }).collect();

    Json(serde_json::json!({
        "recommendations": recommendations,
    }))
}

// ─── Instance AI Chat ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InstanceChatRequest {
    pub message: String,
    #[serde(default)]
    pub history: Vec<crate::llm::ChatMessage>,
}

async fn serve_instance_chat(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceChatRequest>,
) -> impl IntoResponse {
    let instances = state.workspace.instances.read().await;
    let instance = instances.values().find(|i| i.id == instance_id).cloned();
    drop(instances);

    match instance {
        Some(inst) => {
            let mut messages = payload.history.clone();
            messages.push(crate::llm::ChatMessage {
                role: "user".into(),
                content: payload.message,
            });

            let llm = state.llm_client.read().await;
            match llm.chat(messages, Some(&inst.pair_key())).await {
                Ok(reply) => {
                    Json(serde_json::json!({
                        "reply": reply,
                        "instance_id": instance_id,
                    })).into_response()
                }
                Err(e) => {
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                     Json(serde_json::json!({"error": e}))).into_response()
                }
            }
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_support_resistance_calculations() {
        let prices = vec![3110.0, 3135.0, 3105.0, 3140.0, 3100.0, 3150.0, 3115.0, 3145.0, 3120.0, 3130.0];
        let current_price = 3125.0;

        let (support, resistance) = compute_support_resistance(&prices, current_price);

        for s in &support {
            let s_val: f64 = s.parse().unwrap();
            assert!(s_val < current_price, "Support {} should be below current price", s_val);
        }

        for r in &resistance {
            let r_val: f64 = r.parse().unwrap();
            assert!(r_val > current_price, "Resistance {} should be above current price", r_val);
        }

        assert!(support.len() <= 3);
        assert!(resistance.len() <= 3);
    }

    #[test]
    fn test_compile_deterministic_telemetry() {
        let indicators = IndicatorSnapshot {
            rsi: Some(25.0),
            squeeze_on: Some(false),
            squeeze_momentum: Some(-0.05),
            squeeze_duration: Some(3),
            squeeze_release_trigger: Some(false),
            squeeze_momentum_direction: Some("Flat".to_string()),
            chart_pattern: None,
            chart_pattern_confidence: None,
            bbwp: Some(5.0),
            macd_line: Some(-0.5),
            macd_signal: Some(-0.3),
            macd_histogram: Some(-0.2),
            macd_histogram_trend: None,
            adx: Some(15.0),
            adx_plus: Some(12.0),
            adx_minus: Some(18.0),
            bb_upper: None,
            bb_middle: None,
            bb_lower: None,
            atr: Some(1.5),
            atr_trend: None,
            atr_volatility_regime: Some("contracting".to_string()),
            current_price: Some(3125.0),
            volume: None,
            average_volume: None,
            rvol: Some(0.8),
            ema_fast: None,
            ema_medium: None,
            ema_slow: None,
            ema_long: Some(3200.0),
            ema_stack_state: Some("bearish".to_string()),
            vwap: Some(3130.0),
            vwap_bias: Some("discount".to_string()),
            rsi_divergence_status: Some("potential".to_string()),
            macd_divergence_status: Some("none".to_string()),
            macd_trend_state: None,
            macd_crossover_detected: Some(false),
            macd_crossover_direction: None,
            macd_histogram_peak: None,
            adx_slope: None,
            adx_regime: Some("congestion".to_string()),
            adx_di_crossover_detected: None,
            adx_di_crossover_direction: None,
        };

        let support_levels: Vec<String> = vec!["3100.00".to_string(), "3050.00".to_string()];
        let resistance_levels: Vec<String> = vec!["3150.00".to_string(), "3200.00".to_string()];

        let telemetry = compile_deterministic_telemetry(&indicators, &support_levels, &resistance_levels);

        // bbwp < 10.0 → COMPRESSION regime
        assert_eq!(telemetry.market_regime, "COMPRESSION");
        // RSI < 30 → +10, RSI div potential → +10, bearish stack → -10, price < 200EMA → -20
        // total should be negative
        assert!(telemetry.total_confluence_score < 0);
        assert_eq!(telemetry.rvol, 0.8);
        assert_eq!(telemetry.adx_value, 15.0);
        assert_eq!(telemetry.adx_regime, "congestion");
        assert!((telemetry.bbwp_percentile - 5.0).abs() < 0.001);
        assert!(!telemetry.squeeze_on);
        assert_eq!(telemetry.vwap_bias, "discount");
        assert_eq!(telemetry.rsi_divergence_state, "potential");
        assert_eq!(telemetry.macd_divergence_state, "none");
        assert_eq!(telemetry.macd_crossover_state, "none");
        assert_eq!(telemetry.squeeze_release_state, "none");
        assert_eq!(telemetry.support_levels.len(), 2);
        assert_eq!(telemetry.resistance_levels.len(), 2);
    }
}
