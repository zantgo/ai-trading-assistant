use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::{HashMap, VecDeque};
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
use shared::normalized::{NormalizedEvent, NormalizedCandle, SymbolMapper};
use shared::models::MarketSnapshot;
use shared::TriggerType;
use crate::adapters;
use crate::config::{AppConfig, AutomationConfig, TimeframeConfig};
use crate::analyzer::{self, ActivePair};
use crate::llm::{LlmClient, ChatMessage, IndividualIndicatorResult, MasterOrchestratorResult};
use crate::automation;

use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub pairs: Arc<RwLock<HashMap<String, Arc<ActivePair>>>>,
    pub config: Arc<RwLock<AppConfig>>,
    pub pool: SqlitePool,
    pub llm_client: Arc<RwLock<LlmClient>>,
    pub api_key_configured: Arc<AtomicBool>,
    pub symbol_mapper: Arc<SymbolMapper>,
    pub telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
    pub ws_url: String,
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

#[derive(Debug, Deserialize)]
pub struct MultiTimeframeIndicators {
    pub short_term: IndicatorSnapshot,
    pub mid_term: IndicatorSnapshot,
    pub long_term: IndicatorSnapshot,
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

#[derive(Debug, Deserialize)]
pub struct AddPairRequest {
    pub symbol: String,
    #[serde(default)]
    pub exchange: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PairsListResponse {
    pub symbols: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub api_key_configured: bool,
    pub symbols: Vec<String>,
    pub candles: crate::config::CandlesConfig,
    pub indicators: crate::config::IndicatorsConfig,
    pub pairs: std::collections::HashMap<String, crate::config::PairSpecificConfig>,
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

#[derive(Debug, Deserialize)]
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
    pub current_price: Option<f64>,
    pub volume: Option<f64>,
    pub average_volume: Option<f64>,
    pub ema_fast: Option<f64>,
    pub ema_medium: Option<f64>,
    pub ema_slow: Option<f64>,
    pub ema_long: Option<f64>,
    pub vwap: Option<f64>,
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

#[derive(Debug, Deserialize)]
pub struct PairConfigPayload {
    pub short_term: crate::config::TimeframeConfig,
    pub mid_term: crate::config::TimeframeConfig,
    pub long_term: crate::config::TimeframeConfig,
    #[serde(default)]
    pub automation: AutomationConfig,
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

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
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
        .route("/api/paper/performance", get(serve_paper_performance))
        .route("/api/pairs", get(serve_list_pairs).post(serve_add_pair))
        .route("/api/pairs/:pair_key", delete(serve_remove_pair))
        .route("/api/pairs/:pair_key/config", post(serve_update_pair_config))
        .route("/api/decision-profiles", get(serve_decision_profiles_list).post(serve_decision_profile_create))
        .route("/api/decision-profiles/:id", delete(serve_decision_profile_delete).post(serve_decision_profile_update))
        .route("/api/decision-profiles/:id/evaluate", post(serve_decision_evaluate))
        .route("/api/decision-profiles/:id/indicators", post(serve_profile_indicator_add))
        .route("/api/decision-profiles/:id/indicators/:iid", post(serve_profile_indicator_update).delete(serve_profile_indicator_delete))
        .route("/api/risk-profiles", get(serve_risk_profiles_list).post(serve_risk_profile_create))
        .route("/api/risk-profiles/:id", delete(serve_risk_profile_delete).post(serve_risk_profile_update))
        .route("/api/risk/calculate", post(serve_risk_calculate))
        .route("/api/exchange-keys", get(serve_exchange_keys_list).post(serve_exchange_keys_add))
        .route("/api/exchange-keys/:id", delete(serve_exchange_keys_delete).post(serve_exchange_keys_sync))
        .route("/api/dashboard/stats", get(serve_dashboard_stats))
        .route("/api/trade-ledger", get(serve_trade_ledger))
        .route("/api/trades/telemetry", post(serve_trade_telemetry_add))
        .route("/ws", get(ws_handler))
        .route("/favicon.ico", get(|| async { Redirect::to("/favicon.svg") }))
        .fallback_service(ServeDir::new("crates/engine/frontend/dist"))
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
        pairs: current_config.pairs.clone(),
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
        let (ex, sym) = first.split_once(':').unwrap_or(("Hyperliquid", &first));
        format!("{}-{}", ex, sym.to_uppercase())
    } else {
        query.symbol
    };

    let tf_secs = query.timeframe_secs.unwrap_or(60);
    let raw_symbol = pair_key.split('-').nth(1).unwrap_or(&pair_key).to_string();

    let config_guard = state.config.read().await;
    let pair_cfg = config_guard.pairs.get(&pair_key);
    let current_limit = match tf_secs {
        15 => pair_cfg.map(|p| p.short_term.candles.analysis_limit).unwrap_or(config_guard.candles.analysis_limit),
        300 => pair_cfg.map(|p| p.long_term.candles.analysis_limit).unwrap_or(config_guard.candles.analysis_limit),
        _ => pair_cfg.map(|p| p.mid_term.candles.analysis_limit).unwrap_or(config_guard.candles.analysis_limit),
    };

    let pairs = state.pairs.read().await;
    let (prices, candles) = match pairs.get(&pair_key) {
        Some(pair) => {
            let hist = if tf_secs == 15 {
                pair.short.history.read().await
            } else if tf_secs == 300 {
                pair.long.history.read().await
            } else {
                pair.mid.history.read().await
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
        let (ex, sym) = first.split_once(':').unwrap_or(("Hyperliquid", &first));
        format!("{}-{}", ex, sym.to_uppercase())
    } else {
        payload.symbol.clone()
    };

    let prices = payload.historical_prices.clone();
    let indicators = &payload.indicators;

    let last_close = {
        let pair_key = symbol.clone();
        let pairs = state.pairs.read().await;
        if let Some(pair) = pairs.get(&pair_key) {
            let hist = pair.mid.history.read().await;
            hist.back().map(|c| c.close.to_string()).unwrap_or_else(|| "0".to_string())
        } else {
            "0".to_string()
        }
    };

    let current_price = indicators.current_price.unwrap_or_else(|| {
        prices.last().copied().unwrap_or(0.0)
    });

    let entry_price = payload.entry_price.clone();

    let (support_levels, resistance_levels) = compute_support_resistance(&prices, current_price);

    let atr_trend = determine_atr_trend(&state.pool, indicators.atr, 60).await;

    let master_id = crate::db::insert_master_placeholder(
        &state.pool,
        &payload.position,
        &entry_price,
        &last_close,
        &symbol,
        TriggerType::Manual,
    )
    .await;

    let llm = state.llm_client.read().await;

    let phase_one_results = if let Some(ref mtf) = payload.timeframes {
        run_phase_one_agents_mtf(
            &llm,
            &mtf.short_term,
            &mtf.mid_term,
            &mtf.long_term,
            &prices,
            master_id,
            &state.telemetry_tx,
        )
        .await
    } else {
        run_phase_one_agents(
            &llm,
            indicators,
            &prices,
            &atr_trend,
            master_id,
            &state.telemetry_tx,
        )
        .await
    };

    let phase_one_json = serde_json::to_string(&phase_one_results).unwrap_or_else(|_| "[]".into());

    let phase_two = match llm.run_master_orchestrator(
        &payload.position,
        &entry_price,
        &prices,
        &symbol,
        &phase_one_json,
        &support_levels.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        &resistance_levels.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    ).await {
        Ok(master_result) => {
            let _ = state.telemetry_tx.send(crate::db::TelemetryMsg::UpdateMasterRecord {
                master_id,
                general_trend: master_result.general_trend.clone(),
                support_levels: serde_json::to_string(&master_result.support_and_resistance.detected_support_levels).unwrap_or_default(),
                resistance_levels: serde_json::to_string(&master_result.support_and_resistance.detected_resistance_levels).unwrap_or_default(),
                indicator_synthesis_summary: master_result.indicator_synthesis.summary_count.clone(),
                indicator_synthesis_evaluation: master_result.indicator_synthesis.evaluation.clone(),
                recommended_action: master_result.position_recommendation.action.clone(),
                recommendation_rationale: master_result.position_recommendation.rationale.clone(),
            }).await;

            master_result
        }
        Err(e) => {
            eprintln!("⚠️  Master orchestrator failed, falling back to heuristics: {}", e);
            let heuristic = heuristic_master_synthesis(
                &payload.position,
                &prices,
                indicators,
                &support_levels,
                &resistance_levels,
                &phase_one_results,
            );

            let _ = state.telemetry_tx.send(crate::db::TelemetryMsg::UpdateMasterRecord {
                master_id,
                general_trend: heuristic.general_trend.clone(),
                support_levels: serde_json::to_string(&heuristic.support_and_resistance.detected_support_levels).unwrap_or_default(),
                resistance_levels: serde_json::to_string(&heuristic.support_and_resistance.detected_resistance_levels).unwrap_or_default(),
                indicator_synthesis_summary: heuristic.indicator_synthesis.summary_count.clone(),
                indicator_synthesis_evaluation: heuristic.indicator_synthesis.evaluation.clone(),
                recommended_action: heuristic.position_recommendation.action.clone(),
                recommendation_rationale: heuristic.position_recommendation.rationale.clone(),
            }).await;

            heuristic
        }
    };
    drop(llm);

    let response = MultiAgentAnalysisResponse {
        phase_one: phase_one_results.clone(),
        phase_two: PhaseTwoResponse {
            general_trend: phase_two.general_trend,
            support_and_resistance: SupportResistanceResponse {
                detected_support_levels: phase_two.support_and_resistance.detected_support_levels,
                detected_resistance_levels: phase_two.support_and_resistance.detected_resistance_levels,
                structural_analysis: phase_two.support_and_resistance.structural_analysis,
            },
            indicator_synthesis: IndicatorSynthesisResponse {
                summary_count: phase_two.indicator_synthesis.summary_count,
                evaluation: phase_two.indicator_synthesis.evaluation,
            },
            position_recommendation: PositionRecommendationResponse {
                action: phase_two.position_recommendation.action,
                rationale: phase_two.position_recommendation.rationale,
            },
        },
    };

    Json(response)
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
    short: &IndicatorSnapshot,
    mid: &IndicatorSnapshot,
    long: &IndicatorSnapshot,
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
    let timeframes: [(&str, &IndicatorSnapshot, u64); 3] = [
        ("short", short, 15),
        ("mid", mid, 60),
        ("long", long, 300),
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

            let handle = tokio::spawn(async move {
                let temp_client = LlmClient {
                    base_url: client_base,
                    api_key: client_key,
                    model: client_model,
                    indicators_guide: String::new(),
                };

                match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    temp_client.run_indicator_agent(&name, &section, &context),
                )
                .await
                {
                    Ok(Ok(result)) => IndividualIndicatorResult {
                        indicator_name: format!("{}-{}", tf_label, result.indicator_name),
                        signal: result.signal,
                        reason: result.reason,
                    },
                    Ok(Err(e)) => IndividualIndicatorResult {
                        indicator_name: format!("{}-{}", tf_label, name),
                        signal: "UNAVAILABLE".to_string(),
                        reason: format!("Agent error: {}", e),
                    },
                    Err(_) => IndividualIndicatorResult {
                        indicator_name: format!("{}-{}", tf_label, name),
                        signal: "UNAVAILABLE".to_string(),
                        reason: "Agent timed out after 10 seconds".to_string(),
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
            r#"{{ "rsi_value": {}, "current_price": {} }}"#,
            snap.rsi.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.current_price.map_or("null".to_string(), |v| format!("{:.2}", v)),
        ),
        "MACD" => format!(
            r#"{{ "macd_line": {}, "signal_line": {}, "histogram_value": {} }}"#,
            snap.macd_line.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_signal.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_histogram.map_or("null".to_string(), |v| format!("{:.4}", v)),
        ),
        "SQUEEZE" => format!(
            r#"{{ "squeeze_on": {}, "momentum_value": {} }}"#,
            snap.squeeze_on.map_or("null".to_string(), |v| v.to_string()),
            snap.squeeze_momentum.map_or("null".to_string(), |v| format!("{:.4}", v)),
        ),
        "ADX" => format!(
            r#"{{ "adx_line": {}, "di_plus": {}, "di_minus": {} }}"#,
            snap.adx.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.adx_plus.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.adx_minus.map_or("null".to_string(), |v| format!("{:.2}", v)),
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
            r#"{{ "close": {}, "ema_fast": {}, "ema_slow": {}, "volume": {}, "average_volume": {} }}"#,
            snap.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.ema_fast.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.ema_slow.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.volume.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.average_volume.map_or("null".to_string(), |v| format!("{:.4}", v)),
        ),
        "VWAP" => format!(
            r#"{{ "close": {}, "vwap": {} }}"#,
            snap.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.vwap.map_or("null".to_string(), |v| format!("{:.4}", v)),
        ),
        _ => "{}".to_string(),
    }
}

pub async fn run_phase_one_agents(
    client: &LlmClient,
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
        r#"{{ "close": {}, "ema_fast": {}, "ema_medium": {}, "ema_slow": {}, "ema_long": {}, "volume": {}, "average_volume": {} }}"#,
        indicators.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.ema_fast.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.ema_medium.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.ema_slow.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.ema_long.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.volume.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.average_volume.map_or("null".to_string(), |v| format!("{:.4}", v)),
    );

    let vwap_context = format!(
        r#"{{ "close": {}, "vwap": {} }}"#,
        indicators.current_price.map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.vwap.map_or("null".to_string(), |v| format!("{:.4}", v)),
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

        let handle = tokio::spawn(async move {
            let temp_client = LlmClient {
                base_url: client_base,
                api_key: client_key,
                model: client_model,
                indicators_guide: String::new(),
            };

            match tokio::time::timeout(
                std::time::Duration::from_secs(10),
                temp_client.run_indicator_agent(&name, &section, &context),
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(e)) => IndividualIndicatorResult {
                    indicator_name: name,
                    signal: "UNAVAILABLE".to_string(),
                    reason: format!("Agent error: {}", e),
                },
                Err(_) => IndividualIndicatorResult {
                    indicator_name: name,
                    signal: "UNAVAILABLE".to_string(),
                    reason: "Agent timed out after 10 seconds".to_string(),
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

pub fn heuristic_master_synthesis(
    position: &str,
    prices: &[f64],
    indicators: &IndicatorSnapshot,
    support_levels: &[String],
    resistance_levels: &[String],
    phase_one: &[IndividualIndicatorResult],
) -> MasterOrchestratorResult {
    let (trend_class, _) = classify_trend(prices);
    let (action, rationale) = compute_heuristic_recommendation(position, &trend_class, indicators);

    let bullish_count = phase_one.iter().filter(|r| r.signal == "BULLISH").count();
    let bearish_count = phase_one.iter().filter(|r| r.signal == "BEARISH").count();
    let sideways_count = phase_one.iter().filter(|r| r.signal == "SIDEWAYS").count();
    let summary = format!("{} Bullish, {} Bearish, {} Sideways", bullish_count, bearish_count, sideways_count);

    let evaluation = if bullish_count > bearish_count && bullish_count > sideways_count {
        "The majority of technical indicators signal bullish momentum, aligning with upward price pressure.".to_string()
    } else if bearish_count > bullish_count && bearish_count > sideways_count {
        "The majority of technical indicators signal bearish momentum, pointing to downward pressure.".to_string()
    } else {
        "Indicators are mixed with no dominant directional signal, suggesting a consolidating market.".to_string()
    };

    let s_and_r_analysis = format!(
        "Support levels ({}) and resistance levels ({}) frame the current price action. Price testing these boundaries will determine the next directional move.",
        support_levels.join(", "),
        resistance_levels.join(", "),
    );

    MasterOrchestratorResult {
        general_trend: trend_class,
        support_and_resistance: crate::llm::SupportResistance {
            detected_support_levels: support_levels.to_vec(),
            detected_resistance_levels: resistance_levels.to_vec(),
            structural_analysis: s_and_r_analysis,
        },
        indicator_synthesis: crate::llm::IndicatorSynthesis {
            summary_count: summary,
            evaluation,
        },
        position_recommendation: crate::llm::PositionRecommendation {
            action,
            rationale,
        },
    }
}

fn classify_trend(prices: &[f64]) -> (String, String) {
    if prices.len() < 10 {
        return ("SIDEWAYS".into(), "Insufficient price data.".into());
    }

    let first_quarter: f64 = prices.iter().take(prices.len() / 4).sum::<f64>() / (prices.len() / 4) as f64;
    let last_quarter: f64 = prices.iter().skip(3 * prices.len() / 4).sum::<f64>() / (prices.len() / 4) as f64;
    let change_pct = (last_quarter - first_quarter) / first_quarter * 100.0;

    if change_pct > 0.5 {
        ("UPWARD".into(), format!("Net increase of {:.2}% over the sequence.", change_pct))
    } else if change_pct < -0.5 {
        ("DOWNWARD".into(), format!("Net decrease of {:.2}% over the sequence.", change_pct))
    } else {
        ("SIDEWAYS".into(), format!("Minimal {:.2}% change, consolidation.", change_pct))
    }
}

fn compute_heuristic_recommendation(
    position: &str,
    trend: &str,
    indicators: &IndicatorSnapshot,
) -> (String, String) {
    let rsi_val = indicators.rsi.unwrap_or(50.0);
    let macd_hist = indicators.macd_histogram.unwrap_or(0.0);
    let squeeze_on = indicators.squeeze_on.unwrap_or(false);

    let mut bullish_signals = 0;
    let mut bearish_signals = 0;

    if rsi_val > 50.0 { bullish_signals += 1; } else { bearish_signals += 1; }
    if macd_hist > 0.0 { bullish_signals += 1; } else { bearish_signals += 1; }
    if squeeze_on { bullish_signals += 1; }

    let indicator_bias = if bullish_signals > bearish_signals { "supportive" } else { "conflicting" };

    match position {
        "Long" => {
            if trend == "UPWARD" && indicator_bias == "supportive" {
                ("Hold".into(), "Upward trend with supportive indicators. Maintain the long position.".into())
            } else if trend == "DOWNWARD" {
                ("Close".into(), "Downward trend detected while holding long. Consider closing to protect capital.".into())
            } else {
                ("Hold".into(), "Sideways trend. Hold and monitor for breakout direction.".into())
            }
        }
        "Short" => {
            if trend == "DOWNWARD" && indicator_bias == "conflicting" {
                ("Hold".into(), "Downward trend confirms the short position. Maintain it.".into())
            } else if trend == "UPWARD" {
                ("Close".into(), "Upward trend detected while holding short. Consider closing to limit losses.".into())
            } else {
                ("Hold".into(), "Sideways trend. Hold and monitor for breakout direction.".into())
            }
        }
        _ => {
            if trend == "UPWARD" && indicator_bias == "supportive" {
                ("Open Long".into(), "Strong upward trend with confirming indicators. Consider entering a long position.".into())
            } else if trend == "DOWNWARD" && indicator_bias == "conflicting" {
                ("Open Short".into(), "Strong downward trend. Consider entering a short position.".into())
            } else {
                ("Wait".into(), "Unclear market direction. Wait for a clearer signal before entering.".into())
            }
        }
    }
}

async fn serve_chat(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatHistoryRequest>,
) -> impl IntoResponse {
    let llm = state.llm_client.read().await;
    match llm.chat(payload.history).await {
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
        let (ex, sym) = default_symbol.split_once(':').unwrap_or(("Hyperliquid", &default_symbol));
        let pair_key = format!("{}-{}", ex, sym.to_uppercase());
        let pairs = state.pairs.read().await;
        if let Some(pair) = pairs.get(&pair_key) {
            let hist = pair.mid.history.read().await;
            hist.back().map(|c| c.close.to_string()).unwrap_or_else(|| "0".to_string())
        } else {
            "0".to_string()
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
}

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
        let (ex, sym) = first.split_once(':').unwrap_or(("Hyperliquid", &first));
        format!("{}-{}", ex, sym.to_uppercase())
    } else {
        query.symbol
    };

    let pair_arc = {
        let pairs = state.pairs.read().await;
        pairs.get(&symbol).cloned()
    };
    let current_price = if let Some(pair) = pair_arc {
        let snap = pair.mid.latest_snapshot.read().await;
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
    crate::db::paper_set_balance_config(
        &state.pool,
        &payload.symbol,
        payload.initial_usd,
        allocation,
        payload.auto_execute,
    ).await;

    println!(
        "📄 Paper Config: {} initial=${:.2} allocation={:.1}% auto_execute={}",
        payload.symbol, payload.initial_usd, allocation, payload.auto_execute
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
    let pair_arc = {
        let pairs = state.pairs.read().await;
        pairs.get(&payload.symbol).cloned()
    };
    let current_price = if let Some(pair) = pair_arc {
        let snap = pair.mid.latest_snapshot.read().await;
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
        let (ex, sym) = first.split_once(':').unwrap_or(("Hyperliquid", &first));
        format!("{}-{}", ex, sym.to_uppercase())
    } else {
        query.symbol
    };
    let tf_secs = query.timeframe_secs.unwrap_or(60);
    ws.on_upgrade(move |socket| handle_ws_socket(socket, state, pair_key, tf_secs))
}

async fn handle_ws_socket(mut socket: WebSocket, state: Arc<AppState>, pair_key: String, tf_secs: u64) {
    let rx = {
        let pairs = state.pairs.read().await;
        match pairs.get(&pair_key) {
            Some(pair) => {
                if tf_secs == 15 {
                    pair.short.broadcast_tx.subscribe()
                } else if tf_secs == 300 {
                    pair.long.broadcast_tx.subscribe()
                } else {
                    pair.mid.broadcast_tx.subscribe()
                }
            }
            None => return,
        }
    };

    let mut rx_stream = rx;
    loop {
        match rx_stream.recv().await {
            Ok(snapshot) => {
                if let Ok(json_str) = serde_json::to_string(&snapshot) {
                    if socket.send(AxumMessage::Text(json_str.into())).await.is_err() {
                        break;
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

async fn serve_add_pair(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddPairRequest>,
) -> impl IntoResponse {
    let raw_symbol = payload.symbol.trim().to_uppercase().to_string();
    if raw_symbol.is_empty() || raw_symbol.len() > 10 {
        return (axum::http::StatusCode::BAD_REQUEST, "Invalid symbol").into_response();
    }

    let exchange = payload.exchange.as_deref().unwrap_or("Hyperliquid");
    let pair_key = format!("{}-{}", exchange, raw_symbol);
    let normalized = format!("{}-USD", raw_symbol);

    // Resolve correct Exchange enum from payload string
    use shared::normalized::Exchange;
    let exchange_enum = match exchange {
        "Hyperliquid" => Exchange::Hyperliquid,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Unsupported exchange. Only Hyperliquid is currently supported."
            ).into_response();
        }
    };

    {
        let pairs = state.pairs.read().await;
        if pairs.contains_key(&pair_key) {
            return (axum::http::StatusCode::CONFLICT, "Pair already active").into_response();
        }

        // Register the normalized symbol mapping dynamically
        state.symbol_mapper.register(exchange_enum, &raw_symbol, &normalized).await;
    }

    // Persist Symbol addition: append new symbol to config.toml so it survives engine restarts
    {
        let mut config = state.config.write().await;
        let symbol_entry = format!("{}:{}", exchange, raw_symbol);
        if !config.symbols.contains(&symbol_entry) {
            config.symbols.push(symbol_entry);
            if let Ok(toml_str) = toml::to_string_pretty(&*config) {
                let _ = std::fs::write("config.toml", toml_str);
            }
        }
    }

    let (snapshot_tx, snapshot_rx) = mpsc::channel::<NormalizedEvent>(500);
    let config_guard = state.config.read().await;
    let pair_cfg = config_guard.pairs.get(&pair_key);
    let default_indicators = config_guard.indicators.clone();
    let cancel = CancellationToken::new();

    let short_cfg = pair_cfg
        .map(|p| p.short_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(15, default_indicators.clone()));
    let mid_cfg = pair_cfg
        .map(|p| p.mid_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(60, default_indicators.clone()));
    let long_cfg = pair_cfg
        .map(|p| p.long_term.clone())
        .unwrap_or_else(|| TimeframeConfig::new(300, default_indicators.clone()));
    drop(config_guard);

    let (short_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (mid_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
    let (long_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);

    let short_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(short_cfg.candles.analysis_limit)));
    let mid_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(mid_cfg.candles.analysis_limit)));
    let long_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(long_cfg.candles.analysis_limit)));

    let short_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let mid_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
    let long_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));

    let pair = Arc::new(ActivePair {
        symbol: raw_symbol.clone(),
        short: analyzer::TimeframePipeline {
            history: short_history.clone(),
            broadcast_tx: short_broadcast_tx.clone(),
            latest_snapshot: short_latest.clone(),
            timeframe_secs: 15,
            timeframe_label: "Short",
        },
        mid: analyzer::TimeframePipeline {
            history: mid_history.clone(),
            broadcast_tx: mid_broadcast_tx.clone(),
            latest_snapshot: mid_latest.clone(),
            timeframe_secs: 60,
            timeframe_label: "Mid",
        },
        long: analyzer::TimeframePipeline {
            history: long_history.clone(),
            broadcast_tx: long_broadcast_tx.clone(),
            latest_snapshot: long_latest.clone(),
            timeframe_secs: 300,
            timeframe_label: "Long",
        },
        snapshot_tx: snapshot_tx.clone(),
        cancel: cancel.clone(),
    });

    state.pairs.write().await.insert(pair_key.clone(), Arc::clone(&pair));

    // Spawn 3 pipeline channels from the router
    let (short_chan_tx, short_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (mid_chan_tx, mid_chan_rx) = mpsc::channel::<NormalizedEvent>(200);
    let (long_chan_tx, long_chan_rx) = mpsc::channel::<NormalizedEvent>(200);

    // Event router: fan out WS events to all 3 timeframes
    let router_symbol = raw_symbol.clone();
    let router_cancel = cancel.clone();
    tokio::spawn(async move {
        analyzer::run_event_router(
            snapshot_rx,
            short_chan_tx,
            mid_chan_tx,
            long_chan_tx,
            router_symbol,
            router_cancel,
        ).await;
    });

    // Spawn 3 independent pipeline tasks
    for (rx, tf_cfg, hist, snap, label, tf_secs, bcast) in [
        (short_chan_rx, short_cfg.clone(), short_history.clone(), short_latest.clone(), "Short", 15u64, short_broadcast_tx.clone()),
        (mid_chan_rx, mid_cfg.clone(), mid_history.clone(), mid_latest.clone(), "Mid", 60u64, mid_broadcast_tx.clone()),
        (long_chan_rx, long_cfg, long_history.clone(), long_latest.clone(), "Long", 300u64, long_broadcast_tx.clone()),
    ] {
        let a_symbol = raw_symbol.clone();
        let a_pair_key = pair_key.clone();
        let a_telemetry = state.telemetry_tx.clone();
        let a_cancel = cancel.clone();
        tokio::spawn(async move {
            analyzer::run_single(
                rx,
                a_telemetry,
                bcast,
                tf_cfg,
                hist,
                snap,
                a_symbol,
                a_pair_key,
                tf_secs,
                label,
                a_cancel,
            ).await;
        });
    }

    // WebSocket adapter
    let ws_symbol = raw_symbol.clone();
    let ws_tx = snapshot_tx.clone();
    let ws_cancel = cancel.clone();
    let ws_url = state.ws_url.clone();
    tokio::spawn(async move {
        adapters::hyperliquid::run_for_symbol(ws_symbol, ws_tx, ws_cancel, &ws_url).await;
    });

    let auto_ctx = automation::AutomationContext {
        pair_key: pair_key.clone(),
        symbol: raw_symbol.clone(),
        short_history: short_history.clone(),
        mid_history: mid_history.clone(),
        long_history: long_history.clone(),
        short_latest: short_latest.clone(),
        mid_latest: mid_latest.clone(),
        long_latest: long_latest.clone(),
        config: state.config.clone(),
        pool: state.pool.clone(),
        llm_client: state.llm_client.clone(),
        telemetry_tx: state.telemetry_tx.clone(),
        cancel: cancel.clone(),
        api_key_configured: state.api_key_configured.clone(),
    };
    tokio::spawn(async move {
        automation::run_pair_automation_loop(auto_ctx).await;
    });

    println!("✅ Pair added: {}", pair_key);
    (axum::http::StatusCode::CREATED, format!("Pair {} added", pair_key)).into_response()
}

async fn serve_remove_pair(
    State(state): State<Arc<AppState>>,
    Path(pair_key): Path<String>,
) -> impl IntoResponse {
    let pair = {
        let mut pairs = state.pairs.write().await;
        pairs.remove(&pair_key)
    };

    match pair {
        Some(pair) => {
            // Terminate backend analyzer loop task cleanly
            pair.cancel.cancel();

            // Persist Symbol removal and clean pairs.json
            {
                let mut config = state.config.write().await;

                // 1. Remove from config's active symbols array
                let parts: Vec<&str> = pair_key.split('-').collect();
                if parts.len() == 2 {
                    let symbol_entry = format!("{}:{}", parts[0], parts[1]);
                    if let Some(pos) = config.symbols.iter().position(|s| s == &symbol_entry) {
                        config.symbols.remove(pos);
                        if let Ok(toml_str) = toml::to_string_pretty(&*config) {
                            let _ = std::fs::write("config.toml", toml_str);
                        }
                    }
                }

                // 2. Remove pair-specific config block and save pairs.json
                config.pairs.remove(&pair_key);
                crate::config::save_pairs(&config.pairs);
            }

            println!("✅ Pair removed: {}", pair_key);
            (axum::http::StatusCode::OK, format!("Pair {} removed", pair_key)).into_response()
        }
        None => {
            (axum::http::StatusCode::NOT_FOUND, "Pair not found").into_response()
        }
    }
}

async fn serve_update_pair_config(
    State(state): State<Arc<AppState>>,
    Path(pair_key): Path<String>,
    Json(payload): Json<PairConfigPayload>,
) -> impl IntoResponse {
    let mut config = state.config.write().await;

    let specific_config = crate::config::PairSpecificConfig {
        short_term: payload.short_term,
        mid_term: payload.mid_term,
        long_term: payload.long_term,
        automation: payload.automation,
    };

    config.pairs.insert(pair_key.clone(), specific_config);
    crate::config::save_pairs(&config.pairs);
    println!("✅ Pair config saved: {}", pair_key);
    (axum::http::StatusCode::OK, "Pair configuration saved successfully").into_response()
}

async fn serve_list_pairs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pairs = state.pairs.read().await;
    let symbols: Vec<String> = pairs.keys().cloned().collect();
    Json(PairsListResponse { symbols })
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
    pub vwap: Option<f64>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub average_volume: Option<f64>,
    pub current_price: f64,
    #[serde(default)]
    pub historical_prices: Vec<f64>,
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
        vwap: payload.vwap,
        close: payload.close,
        volume: payload.volume,
        average_volume: payload.average_volume,
        current_price: payload.current_price,
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
    pub stop_loss_price: f64,
    pub take_profit_price: f64,
    #[serde(default)]
    pub profile_id: Option<i64>,
    pub capital: Option<f64>,
    pub max_risk_pct: Option<f64>,
    pub leverage: Option<i32>,
    pub commission_pct: Option<f64>,
    pub funding_rate_8h: Option<f64>,
    pub spread: Option<f64>,
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
    };

    match crate::risk_calculator::compute_risk(&input) {
        Ok(calc) => Json(calc).into_response(),
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
    fn test_heuristic_recommendation_logic() {
        let mut indicators = IndicatorSnapshot {
            rsi: Some(25.0),
            squeeze_on: Some(true),
            squeeze_momentum: Some(-0.05),
            macd_line: None,
            macd_signal: None,
            macd_histogram: Some(-1.2),
            macd_histogram_trend: None,
            adx: None,
            adx_plus: None,
            adx_minus: None,
            bb_upper: None,
            bb_middle: None,
            bb_lower: None,
            atr: None,
            atr_trend: None,
            current_price: Some(3125.0),
            volume: None,
            average_volume: None,
            ema_fast: None,
            ema_medium: None,
            ema_slow: None,
            ema_long: None,
            vwap: None,
        };

        let (action, _) = compute_heuristic_recommendation("Long", "DOWNWARD", &indicators);
        assert_eq!(action, "Close");

        indicators.rsi = Some(65.0);
        indicators.macd_histogram = Some(1.5);
        let (action_none, _) = compute_heuristic_recommendation("None", "UPWARD", &indicators);
        assert_eq!(action_none, "Open Long");
    }
}
