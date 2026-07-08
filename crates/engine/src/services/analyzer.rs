use crate::profile_evaluation::snapshot_values_from_flat;
use crate::server::telemetry::compile_deterministic_telemetry;
use crate::server::types::{IndicatorSnapshot, WizardAnalysisResponse};
use crate::server::{math, AppState};
use crate::services::traits::LlmService;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AnalysisService {
    pool: SqlitePool,
    llm_client: Arc<dyn LlmService>,
    telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
    config: Arc<tokio::sync::RwLock<crate::config::AppConfig>>,
}

pub struct AnalysisRequest {
    pub symbol: String,
    pub position: String,
    pub entry_price: String,
    pub historical_prices: Vec<f64>,
    pub indicators: IndicatorSnapshot,
    pub timeframes: Option<crate::server::types::MultiTimeframeIndicators>,
    pub master_id: i64,
    pub last_close: f64,
}

impl AnalysisService {
    pub fn from_app_state(state: &Arc<AppState>) -> Self {
        Self {
            pool: state.pool.clone(),
            llm_client: state.llm_client.clone() as Arc<dyn LlmService>,
            telemetry_tx: state.telemetry_tx.clone(),
            config: state.config.clone(),
        }
    }

    pub async fn has_api_key(&self) -> bool {
        self.llm_client.has_api_key().await
    }

    pub async fn run_analysis(
        &self,
        req: AnalysisRequest,
    ) -> Result<WizardAnalysisResponse, String> {
        let symbol = req.symbol;
        let position = req.position;
        let entry_price = req.entry_price;
        let prices = req.historical_prices;
        let indicators = req.indicators;
        let master_id = req.master_id;
        let raw_symbol = crate::server::helpers::extract_base_symbol(&symbol);

        let last_close_f: f64 = req.last_close;
        let (support_levels, resistance_levels) =
            math::compute_support_resistance(&prices, last_close_f);
        let support_strings: Vec<String> = support_levels.iter().map(|s| s.to_string()).collect();
        let resistance_strings: Vec<String> =
            resistance_levels.iter().map(|s| s.to_string()).collect();

        let _empty_snap = IndicatorSnapshot::default();
        let mtf = req.timeframes.as_ref();
        let micro_snap = mtf.map(|t| &t.micro_term).unwrap_or(&indicators);

        let _telemetry = compile_deterministic_telemetry(
            micro_snap,
            &support_strings,
            &resistance_strings,
            None,
            None,
        );

        // Build indicator DTO array from normalized indicator map
        let indicators_dto = build_indicators_dto(micro_snap);

        // Build decision context from snapshot
        let decision_context = build_decision_context(micro_snap);

        // Build market context
        let market_context = build_market_context(micro_snap);

        // Step 1: Run Analyst Agent
        let analyst_document = self
            .llm_client
            .run_analyst(
                &raw_symbol,
                last_close_f,
                &indicators_dto,
                &decision_context,
                &market_context,
                &support_strings,
                &resistance_strings,
                &prices,
                Some(&raw_symbol),
            )
            .await?;

        let analyst_json = serde_json::to_string(&analyst_document)
            .map_err(|e| format!("Failed to serialize analyst document: {}", e))?;

        // Institutional Risk Management Layer — deterministic risk profile
        // (advisory) injected into the trader payload.
        let risk_profile_json = self
            .build_risk_profile_json(&raw_symbol, micro_snap, last_close_f, confluence_from(micro_snap))
            .await;

        // Step 2: Run Trader Agent
        let trader_decision = self
            .llm_client
            .run_trader(
                &analyst_json,
                &position,
                &entry_price,
                &raw_symbol,
                &risk_profile_json,
                Some(&raw_symbol),
            )
            .await?;

        self.spawn_background_updates(master_id, &analyst_document, &trader_decision)
            .await;

        Ok(WizardAnalysisResponse {
            analyst_document,
            trader_decision,
        })
    }

    /// Build the deterministic IRML risk profile for the pair and serialize it
    /// to JSON for the trader agent payload. Returns an empty string on failure
    /// (the trader treats this as `null`).
    async fn build_risk_profile_json(
        &self,
        symbol: &str,
        snap: &IndicatorSnapshot,
        price: f64,
        confluence: f64,
    ) -> String {
        let cfg = self.config.read().await;
        let risk_cfg = cfg.risk.clone();
        let suspend = cfg.safety.consecutive_loss_suspend;
        let drawdown_limit = cfg.safety.capital_drawdown_pct;
        let timeframe_secs = cfg.candles.duration_seconds as i64;
        drop(cfg);

        let indicators = &snap.indicators;
        let market = shared::market_context::MarketContext::synthesize(indicators);
        let atr_val = indicators.get("atr").map(|v| v.raw_value).unwrap_or(0.0);
        let decision = shared::decision_context::DecisionContext::compute(
            indicators, price, atr_val, confluence,
        );

        let engine = crate::risk_engine::RiskEngine::new(risk_cfg, suspend, drawdown_limit);
        let profile = engine
            .evaluate(
                &self.pool,
                symbol,
                symbol,
                timeframe_secs,
                indicators,
                Some(&market),
                Some(&decision),
                None,
                None,
            )
            .await;
        serde_json::to_string(&profile).unwrap_or_default()
    }

    async fn spawn_background_updates(
        &self,
        master_id: i64,
        analyst: &crate::llm::AnalystDocument,
        trader: &crate::llm::TraderDecision,
    ) {
        let db_telemetry = self.telemetry_tx.clone();
        let db_master_id = master_id;
        let general_trend = extract_trend_from_summary(&analyst.market_summary);
        let indicator_synthesis_summary = format!(
            "Action: {} (confidence: {})",
            trader.action, trader.confidence
        );
        let indicator_synthesis_evaluation = analyst.confluence_summary.clone();
        let mr_action = trader.action.clone();
        let mr_rationale = trader.rationale.clone();
        let mr_confidence = trader.confidence;

        tokio::spawn(async move {
            let _ = db_telemetry
                .send(crate::db::TelemetryMsg::UpdateMasterRecord {
                    master_id: db_master_id,
                    general_trend,
                    support_levels: "[]".to_string(),
                    resistance_levels: "[]".to_string(),
                    indicator_synthesis_summary,
                    indicator_synthesis_evaluation,
                    recommended_action: mr_action,
                    recommendation_rationale: mr_rationale,
                    score_points: Some(mr_confidence as i32),
                    signals_json: None,
                })
                .await;
        });
    }
}

/// Registry-weighted directional confluence `[-100,100]` from a snapshot map.
fn confluence_from(snap: &IndicatorSnapshot) -> f64 {
    let mut sum = 0.0f64;
    let mut wgt = 0.0f64;
    for meta in shared::indicators::registry::INDICATORS {
        if meta.directional {
            if let Some(v) = snap.indicators.get(meta.key) {
                sum += meta.default_weight * v.normalized;
                wgt += meta.default_weight;
            }
        }
    }
    if wgt > 0.0 {
        (sum / wgt * 100.0).clamp(-100.0, 100.0)
    } else {
        0.0
    }
}

fn extract_trend_from_summary(summary: &str) -> String {
    let lower = summary.to_lowercase();
    if lower.contains("bullish") || lower.contains("uptrend") || lower.contains("upward") {
        "UPWARD".to_string()
    } else if lower.contains("bearish") || lower.contains("downtrend") || lower.contains("downward") {
        "DOWNWARD".to_string()
    } else {
        "SIDEWAYS".to_string()
    }
}

/// Build a compact JSON array of indicator DTOs from the normalized indicator map.
fn build_indicators_dto(snap: &IndicatorSnapshot) -> String {
    let arr: Vec<serde_json::Value> = snap
        .indicators
        .iter()
        .map(|(key, v)| {
            let mut obj = serde_json::json!({
                "indicator_name": key,
                "normalized": v.normalized,
                "state_label": v.state_label,
            });
            if let Some(vals) = &v.values {
                obj["values"] = serde_json::to_value(vals).unwrap_or(serde_json::json!({}));
            }
            obj
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

/// Build decision context JSON from snapshot indicators.
fn build_decision_context(snap: &IndicatorSnapshot) -> String {
    // Try to synthesize from available data
    let local_snap = snapshot_values_from_flat(snap);

    let bullish_prob = local_snap
        .indicators
        .iter()
        .filter(|(_, v)| v.normalized > 0.0)
        .count() as f64
        / local_snap.indicators.len().max(1) as f64;

    let consensus = local_snap
        .indicators
        .values()
        .filter(|v| v.normalized.abs() > 0.2)
        .count() as f64
        / local_snap.indicators.len().max(1) as f64;

    let directional_sum: f64 = local_snap.indicators.values().map(|v| v.normalized).sum();
    let bias = (directional_sum / local_snap.indicators.len().max(1) as f64).clamp(-1.0, 1.0);

    let risk_level = if consensus < 0.5 { 0.6 } else { 0.3 };

    serde_json::to_string(&serde_json::json!({
        "bullish_probability": bullish_prob,
        "directional_bias": bias,
        "consensus": consensus,
        "risk_level": risk_level,
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

/// Build market context JSON from snapshot indicators.
fn build_market_context(snap: &IndicatorSnapshot) -> String {
    let regime = if let Some(adx) = snap.indicators.get("adx") {
        if adx.normalized > 0.4 {
            "TRENDING"
        } else if adx.normalized < -0.4 {
            "COMPRESSION"
        } else {
            "RANGE"
        }
    } else {
        "RANGE"
    };

    serde_json::to_string(&serde_json::json!({
        "regime": regime,
        "current_price": snap.current_price,
    }))
    .unwrap_or_else(|_| "{}".to_string())
}
