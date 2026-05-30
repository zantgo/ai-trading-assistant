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
use shared::normalized::{NormalizedEvent, SymbolMapper};
use shared::models::MarketSnapshot;
use crate::config::AppConfig;
use crate::analyzer::{self, ActivePair};
use crate::llm::{LlmClient, ChatMessage, IndividualIndicatorResult, MasterOrchestratorResult};

use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub pairs: Arc<RwLock<HashMap<String, Arc<ActivePair>>>>,
    pub config: Arc<RwLock<AppConfig>>,
    pub pool: SqlitePool,
    pub llm_client: Arc<RwLock<LlmClient>>,
    pub api_key_configured: Arc<AtomicBool>,
    pub symbol_mapper: Arc<SymbolMapper>,
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
pub struct HistoryQuery {
    #[serde(default)]
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    #[serde(default)]
    pub symbol: String,
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
    pub candles: crate::config::CandlesConfig,
    pub indicators: crate::config::IndicatorsConfig,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub prices: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatReplResponse {
    pub reply: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatHistoryRequest {
    pub history: Vec<ChatMessage>,
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
        .route("/api/assistant-records", get(serve_assistant_records))
        .route("/api/pairs", get(serve_list_pairs).post(serve_add_pair))
        .route("/api/pairs/:pair_key", delete(serve_remove_pair))
        .route("/api/pairs/:pair_key/config", post(serve_update_pair_config))
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

    let pairs = state.pairs.read().await;
    let prices = match pairs.get(&pair_key) {
        Some(pair) => {
            let hist = pair.history.read().await;
            hist.iter().map(|d| d.to_string()).collect::<Vec<String>>()
        }
        None => vec![],
    };
    Json(HistoryResponse { prices })
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
            let hist = pair.history.read().await;
            hist.back().map(|d| d.to_string()).unwrap_or_else(|| "0".to_string())
        } else {
            "0".to_string()
        }
    };

    let current_price = indicators.current_price.unwrap_or_else(|| {
        prices.last().copied().unwrap_or(0.0)
    });

    let entry_price = payload.entry_price.clone();

    let (support_levels, resistance_levels) = compute_support_resistance(&prices, current_price);

    let atr_trend = determine_atr_trend(&state.pool, indicators.atr).await;

    let master_id = crate::db::insert_master_placeholder(
        &state.pool,
        &payload.position,
        &entry_price,
        &last_close,
        &symbol,
    )
    .await;

    let llm = state.llm_client.read().await;
    let phase_one_results = run_phase_one_agents(
        &llm,
        indicators,
        &prices,
        &atr_trend,
        master_id,
        &state.pool,
    )
    .await;

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
            crate::db::update_master_record(
                &state.pool,
                master_id,
                &master_result.general_trend,
                &serde_json::to_string(&master_result.support_and_resistance.detected_support_levels).unwrap_or_default(),
                &serde_json::to_string(&master_result.support_and_resistance.detected_resistance_levels).unwrap_or_default(),
                &master_result.indicator_synthesis.summary_count,
                &master_result.indicator_synthesis.evaluation,
                &master_result.position_recommendation.action,
                &master_result.position_recommendation.rationale,
            ).await;

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

            crate::db::update_master_record(
                &state.pool,
                master_id,
                &heuristic.general_trend,
                &serde_json::to_string(&heuristic.support_and_resistance.detected_support_levels).unwrap_or_default(),
                &serde_json::to_string(&heuristic.support_and_resistance.detected_resistance_levels).unwrap_or_default(),
                &heuristic.indicator_synthesis.summary_count,
                &heuristic.indicator_synthesis.evaluation,
                &heuristic.position_recommendation.action,
                &heuristic.position_recommendation.rationale,
            ).await;

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

fn compute_support_resistance(
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

async fn determine_atr_trend(pool: &SqlitePool, current_atr: Option<f64>) -> String {
    let current_atr = match current_atr {
        Some(v) => v,
        None => return "flat".to_string(),
    };

    let rows = crate::db::query_atr_snapshots(pool, 5).await;

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

async fn run_phase_one_agents(
    client: &LlmClient,
    indicators: &IndicatorSnapshot,
    prices: &[f64],
    atr_trend: &str,
    master_id: i64,
    pool: &SqlitePool,
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
        crate::db::insert_individual_log(
            pool,
            master_id,
            &result.indicator_name,
            &result.signal,
            &result.reason,
        )
        .await;
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

fn heuristic_master_synthesis(
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

async fn serve_assistant_records(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let records = crate::db::query_master_records(&state.pool, 50).await;
    let default_symbol = state.config.read().await.symbols.first().cloned().unwrap_or_default();
    let latest_close = {
        let (ex, sym) = default_symbol.split_once(':').unwrap_or(("Hyperliquid", &default_symbol));
        let pair_key = format!("{}-{}", ex, sym.to_uppercase());
        let pairs = state.pairs.read().await;
        if let Some(pair) = pairs.get(&pair_key) {
            let hist = pair.history.read().await;
            hist.back().map(|d| d.to_string()).unwrap_or_else(|| "0".to_string())
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
        }})
        .collect();

    Json(MasterHistoryResponse {
        records: records_json,
        latest_close,
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
    ws.on_upgrade(move |socket| handle_ws_socket(socket, state, pair_key))
}

async fn handle_ws_socket(mut socket: WebSocket, state: Arc<AppState>, pair_key: String) {
    let rx = {
        let pairs = state.pairs.read().await;
        match pairs.get(&pair_key) {
            Some(pair) => pair.broadcast_tx.subscribe(),
            None => return,
        }
    };

    let mut rx_stream = rx;
    while let Ok(snapshot) = rx_stream.recv().await {
        if let Ok(json_str) = serde_json::to_string(&snapshot) {
            if socket.send(AxumMessage::Text(json_str.into())).await.is_err() {
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

    {
        let pairs = state.pairs.read().await;
        if pairs.contains_key(&pair_key) {
            return (axum::http::StatusCode::CONFLICT, "Pair already active").into_response();
        }

        // Register the normalized symbol mapping
        use shared::normalized::Exchange;
        state.symbol_mapper.register(Exchange::Hyperliquid, &raw_symbol, &normalized).await;
    }

    let (snapshot_tx, snapshot_rx) = mpsc::channel::<NormalizedEvent>(100);
    let (broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(100);
    let history = Arc::new(RwLock::new(VecDeque::with_capacity(100)));
    let cancel = CancellationToken::new();

    let pair = Arc::new(ActivePair {
        symbol: raw_symbol.clone(),
        history: history.clone(),
        broadcast_tx: broadcast_tx.clone(),
        snapshot_tx: snapshot_tx.clone(),
        cancel: cancel.clone(),
    });

    state.pairs.write().await.insert(pair_key.clone(), Arc::clone(&pair));

    let analyzer_config = state.config.clone();
    let analyzer_pool = state.pool.clone();
    let analyzer_history = history.clone();
    let analyzer_broadcast = broadcast_tx.clone();
    let analyzer_cancel = cancel.clone();
    let analyzer_symbol = raw_symbol.clone();
    let analyzer_pair_key = pair_key.clone();
    tokio::spawn(async move {
        analyzer::run_single(
            snapshot_rx,
            analyzer_pool,
            analyzer_broadcast,
            analyzer_config,
            analyzer_history,
            analyzer_symbol,
            analyzer_pair_key,
            analyzer_cancel,
        ).await;
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
            pair.cancel.cancel();
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
        candles: payload.candles,
        indicators: payload.indicators,
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
