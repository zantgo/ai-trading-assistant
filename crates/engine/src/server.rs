use std::sync::Arc;
use std::collections::VecDeque;
use axum::{
    extract::{State, WebSocketUpgrade},
    extract::ws::{WebSocket, Message as AxumMessage},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router, Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use sqlx::SqlitePool;
use tower_http::services::ServeDir;
use rust_decimal::Decimal;
use shared::models::MarketSnapshot;
use crate::config::AppConfig;
use crate::llm::LlmClient;

pub struct AppState {
    pub tx: tokio::sync::broadcast::Sender<MarketSnapshot>,
    pub config: Arc<AppConfig>,
    pub history: Arc<RwLock<VecDeque<Decimal>>>,
    pub pool: SqlitePool,
    pub llm_client: LlmClient,
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub position: String,
    pub historical_prices: Vec<f64>,
    pub indicators: IndicatorSnapshot,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct IndicatorSnapshot {
    pub rsi: Option<f64>,
    pub squeeze_on: Option<bool>,
    pub macd_histogram: Option<f64>,
    pub adx: Option<f64>,
    pub ema_fast: Option<f64>,
    pub ema_slow: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TrendAnalysis {
    pub classification: String,
    pub structural_reasoning: String,
}

#[derive(Debug, Serialize)]
pub struct IndicatorAlignment {
    pub classification: String,
    pub observation: String,
}

#[derive(Debug, Serialize)]
pub struct PositionRecommendation {
    pub action: String,
    pub rationale: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub trend_analysis: TrendAnalysis,
    pub indicator_alignment: IndicatorAlignment,
    pub position_recommendation: PositionRecommendation,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub prices: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AssistantRecordJson {
    pub id: i64,
    pub created_at: String,
    pub position: String,
    pub trend_classification: String,
    pub indicator_alignment: String,
    pub recommended_action: String,
    pub recommendation_rationale: String,
    pub close_price: String,
    pub symbol: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantHistoryResponse {
    pub records: Vec<AssistantRecordJson>,
    pub latest_close: String,
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/config", get(serve_config).post(update_config))
        .route("/api/history", get(serve_history))
        .route("/api/analyze", post(serve_analyze))
        .route("/api/assistant-records", get(serve_assistant_records))
        .route("/ws", get(ws_handler))
        .route("/favicon.ico", get(|| async { Redirect::to("/favicon.svg") }))
        .fallback_service(ServeDir::new("crates/engine/frontend/dist"))
        .with_state(state)
}

async fn serve_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    axum::Json(state.config.clone())
}

async fn update_config(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<AppConfig>,
) -> impl IntoResponse {
    match toml::to_string_pretty(&payload) {
        Ok(toml_str) => {
            if let Err(e) = std::fs::write("config.toml", toml_str) {
                eprintln!("❌ Database/Config Error: Failed to write configuration updates to config.toml: {}", e);
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to persist configuration file").into_response();
            }
            println!("✅ Configuration Updated: successfully synchronized config.toml dynamically.");
            (axum::http::StatusCode::OK, "Configuration successfully saved. Restart recommended for full indicator parameter re-initialization.").into_response()
        }
        Err(e) => {
            eprintln!("❌ TOML Serialization Error: {}", e);
            (axum::http::StatusCode::BAD_REQUEST, "Invalid configuration object structure").into_response()
        }
    }
}

async fn serve_history(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let hist = state.history.read().await;
    let prices: Vec<String> = hist.iter().map(|d| d.to_string()).collect();
    Json(HistoryResponse { prices })
}

async fn serve_analyze(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AnalyzeRequest>,
) -> impl IntoResponse {
    let hist = state.history.read().await;
    let last_close = hist.back().map(|d| d.to_string()).unwrap_or_else(|| "0".to_string());

    let position = payload.position.clone();
    let prices = payload.historical_prices.clone();
    let indicators = &payload.indicators;

    let analysis = match state
        .llm_client
        .analyze(
            &position,
            &prices,
            indicators.rsi,
            indicators.squeeze_on,
            indicators.macd_histogram,
            indicators.adx,
            indicators.ema_fast,
            indicators.ema_slow,
        )
        .await
    {
        Ok(result) => AnalyzeResponse {
            trend_analysis: TrendAnalysis {
                classification: result.trend_analysis.classification,
                structural_reasoning: result.trend_analysis.structural_reasoning,
            },
            indicator_alignment: IndicatorAlignment {
                classification: result.indicator_alignment.classification,
                observation: result.indicator_alignment.observation,
            },
            position_recommendation: PositionRecommendation {
                action: result.position_recommendation.action,
                rationale: result.position_recommendation.rationale,
            },
        },
        Err(e) => {
            eprintln!("⚠️  LLM analysis failed, falling back to heuristics: {}", e);
            heuristic_analysis(&position, &prices, indicators)
        }
    };

    crate::db::insert_assistant_record(
        &state.pool,
        &position,
        &analysis.trend_analysis.classification,
        &analysis.indicator_alignment.classification,
        &analysis.position_recommendation.action,
        &analysis.position_recommendation.rationale,
        &last_close,
        &state.config.symbol,
    )
    .await;

    Json(analysis)
}

fn heuristic_analysis(
    position: &str,
    prices: &[f64],
    indicators: &IndicatorSnapshot,
) -> AnalyzeResponse {
    let (trend_class, trend_reason) = classify_trend(prices);
    let (indicator_class, indicator_obs) =
        classify_indicators(prices, indicators, &trend_class);
    let (action, rationale) =
        compute_recommendation(position, &trend_class, &indicator_class, prices, indicators);

    AnalyzeResponse {
        trend_analysis: TrendAnalysis {
            classification: trend_class,
            structural_reasoning: trend_reason,
        },
        indicator_alignment: IndicatorAlignment {
            classification: indicator_class,
            observation: indicator_obs,
        },
        position_recommendation: PositionRecommendation {
            action,
            rationale,
        },
    }
}

fn classify_trend(prices: &[f64]) -> (String, String) {
    if prices.len() < 10 {
        return (
            "sideways".into(),
            "Insufficient price data to determine a reliable trend.".into(),
        );
    }

    let first_quarter: f64 = prices.iter().take(prices.len() / 4).sum::<f64>() / (prices.len() / 4) as f64;
    let last_quarter: f64 = prices.iter().skip(3 * prices.len() / 4).sum::<f64>() / (prices.len() / 4) as f64;
    let change_pct = (last_quarter - first_quarter) / first_quarter * 100.0;

    if change_pct > 0.5 {
        (
            "trending upwards".into(),
            format!(
                "The last 100 candles show a net price increase of {:.2}% when comparing the first and last quartiles, indicating bullish momentum.",
                change_pct
            ),
        )
    } else if change_pct < -0.5 {
        (
            "trending downwards".into(),
            format!(
                "The last 100 candles show a net price decrease of {:.2}% when comparing the first and last quartiles, indicating bearish momentum.",
                change_pct
            ),
        )
    } else {
        (
            "sideways".into(),
            format!(
                "Price change of {:.2}% between quartile averages is minimal, suggesting a consolidation or sideways market.",
                change_pct
            ),
        )
    }
}

fn classify_indicators(
    _prices: &[f64],
    indicators: &IndicatorSnapshot,
    trend: &str,
) -> (String, String) {
    let mut supportive = 0;
    let mut conflicting = 0;

    match trend {
        "trending upwards" => {
            if indicators.rsi.map_or(false, |r| r > 50.0) { supportive += 1; } else { conflicting += 1; }
            if indicators.macd_histogram.map_or(false, |h| h > 0.0) { supportive += 1; } else { conflicting += 1; }
            if indicators.squeeze_on.map_or(false, |s| s) { supportive += 1; }
        }
        "trending downwards" => {
            if indicators.rsi.map_or(false, |r| r < 50.0) { supportive += 1; } else { conflicting += 1; }
            if indicators.macd_histogram.map_or(false, |h| h < 0.0) { supportive += 1; } else { conflicting += 1; }
            if indicators.squeeze_on.map_or(false, |s| s) { supportive += 1; }
        }
        _ => {
            if indicators.squeeze_on.unwrap_or(false) {
                supportive += 1;
            }
        }
    }

    if supportive > conflicting {
        (
            "supportive".into(),
            "Key indicators (RSI, MACD Histogram, Squeeze Momentum) align with the identified price trend.".into(),
        )
    } else if conflicting > supportive {
        (
            "conflicting".into(),
            "Indicators diverge from the price trend; RSI or MACD histogram is signaling against the prevailing direction.".into(),
        )
    } else {
        (
            "neutral".into(),
            "No clear alignment or divergence between price action and technical indicators.".into(),
        )
    }
}

fn compute_recommendation(
    position: &str,
    trend: &str,
    indicator_align: &str,
    _prices: &[f64],
    _indicators: &IndicatorSnapshot,
) -> (String, String) {
    let is_strong = indicator_align == "supportive";
    let is_weak = indicator_align == "conflicting";

    match position {
        "Long" => {
            if trend == "trending upwards" && is_strong {
                ("Hold".into(), "Position is aligned with a strong upward trend and supportive indicators. Maintain the long.".into())
            } else if trend == "trending downwards" && is_weak {
                ("Close".into(), "Downward trend with conflicting indicators suggests momentum may be turning against the long position. Consider closing to protect capital.".into())
            } else if trend == "trending downwards" {
                ("Close".into(), "The market is trending downwards while holding a long position. Closing is advised to minimize drawdown.".into())
            } else {
                ("Hold".into(), "Trend is sideways with unclear indicator signals. Holding the long position while monitoring for a breakout is prudent.".into())
            }
        }
        "Short" => {
            if trend == "trending downwards" && is_strong {
                ("Hold".into(), "Position is aligned with a strong downward trend and supportive indicators. Maintain the short.".into())
            } else if trend == "trending upwards" && is_weak {
                ("Close".into(), "Upward trend with conflicting indicators suggests momentum may be turning against the short position. Consider closing to protect capital.".into())
            } else if trend == "trending upwards" {
                ("Close".into(), "The market is trending upwards while holding a short position. Closing is advised to minimize drawdown.".into())
            } else {
                ("Hold".into(), "Trend is sideways with unclear indicator signals. Holding the short position while monitoring for a breakout is prudent.".into())
            }
        }
        _ => {
            if trend == "trending upwards" && is_strong {
                ("Open Long".into(), "Strong upward trend with confirming indicators. Consider entering a long position.".into())
            } else if trend == "trending downwards" && is_strong {
                ("Open Short".into(), "Strong downward trend with confirming indicators. Consider entering a short position.".into())
            } else {
                ("Wait".into(), "Market conditions are unclear or sideways. Waiting for a clearer signal before entering a position is recommended.".into())
            }
        }
    }
}

async fn serve_assistant_records(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let records = crate::db::query_assistant_records(&state.pool, 50).await;
    let hist = state.history.read().await;
    let latest_close = hist.back().map(|d| d.to_string()).unwrap_or_else(|| "0".to_string());

    let records_json: Vec<AssistantRecordJson> = records
        .into_iter()
        .map(|r| AssistantRecordJson {
            id: r.id,
            created_at: r.created_at,
            position: r.position,
            trend_classification: r.trend_classification,
            indicator_alignment: r.indicator_alignment,
            recommended_action: r.recommended_action,
            recommendation_rationale: r.recommendation_rationale,
            close_price: r.close_price,
            symbol: r.symbol,
        })
        .collect();

    Json(AssistantHistoryResponse {
        records: records_json,
        latest_close,
    })
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws_socket(socket, state))
}

async fn handle_ws_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();

    while let Ok(snapshot) = rx.recv().await {
        if let Ok(json_str) = serde_json::to_string(&snapshot) {
            if socket.send(AxumMessage::Text(json_str.into())).await.is_err() {
                break;
            }
        }
    }
}
