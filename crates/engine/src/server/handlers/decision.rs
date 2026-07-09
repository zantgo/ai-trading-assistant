use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::decision::{DecisionConfig, DecisionMatrix};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct DecisionRequest {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub position: Option<String>,
    pub confluence_score: f64,
    pub opposite_score: f64,
    pub trade_readiness: f64,
    pub trade_quality: f64,
    pub trend_persistence: f64,
    pub risk_level: f64,
    pub regime: String,
    pub regime_confidence: f64,
    pub breakout_confidence: f64,
    pub anomaly_score: f64,
    #[serde(default)]
    pub compressed: bool,
    #[serde(default)]
    pub choppy: bool,
    #[serde(default)]
    pub confirmed_opposing_divergence: bool,
    #[serde(default)]
    pub signal_age_bars: u32,
}

#[derive(Debug, Serialize)]
pub struct DecisionResponse {
    pub action: String,
    pub confidence: f64,
    pub rationale: String,
    pub risk_notes: String,
    pub factor_breakdown: crate::decision::FactorBreakdown,
}

pub async fn serve_decision(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DecisionRequest>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    let decision_config = DecisionConfig::default();
    drop(config);

    let matrix = DecisionMatrix::new(decision_config);

    let positioned = payload.position.as_deref() == Some("Long") || payload.position.as_deref() == Some("Short");
    let position_dir = match payload.position.as_deref() {
        Some("Long") => Some(1.0),
        Some("Short") => Some(-1.0),
        _ => None,
    };

    let output = matrix.evaluate(
        positioned,
        position_dir,
        payload.confluence_score,
        payload.opposite_score,
        payload.trade_readiness,
        payload.trade_quality,
        payload.trend_persistence,
        payload.risk_level,
        &payload.regime,
        payload.regime_confidence,
        payload.breakout_confidence,
        payload.anomaly_score,
        payload.compressed,
        payload.choppy,
        payload.confirmed_opposing_divergence,
        payload.signal_age_bars,
        None,
    );

    Json(DecisionResponse {
        action: output.action.as_str().to_string(),
        confidence: output.confidence,
        rationale: output.rationale,
        risk_notes: output.risk_notes,
        factor_breakdown: output.factor_breakdown,
    })
}
