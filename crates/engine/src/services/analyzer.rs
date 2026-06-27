use crate::profile_evaluation::{classify_market_regime, indicator_to_snapshot_values};
use crate::server::telemetry::compile_deterministic_telemetry;
use crate::server::types::{
    IndicatorSnapshot, IndicatorSynthesisResponse, MultiAgentAnalysisResponse, PhaseTwoResponse,
    PositionRecommendationResponse, SupportResistanceResponse,
};
use crate::server::{math, AppState};
use crate::services::traits::LlmService;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AnalysisService {
    pool: SqlitePool,
    llm_client: Arc<dyn LlmService>,
    telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
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
        }
    }

    pub async fn has_api_key(&self) -> bool {
        self.llm_client.has_api_key().await
    }

    pub async fn run_analysis(
        &self,
        req: AnalysisRequest,
    ) -> Result<MultiAgentAnalysisResponse, String> {
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

        let empty_snap = IndicatorSnapshot::default();
        let mtf = req.timeframes.as_ref();
        let micro_snap = mtf.map(|t| &t.micro_term).unwrap_or(&indicators);
        let fast_snap = mtf.map(|t| &t.fast_term).unwrap_or(&indicators);
        let slow_snap = mtf
            .and_then(|t| t.slow_term.as_ref())
            .unwrap_or(&empty_snap);
        let macro_snap = mtf
            .and_then(|t| t.macro_term.as_ref())
            .unwrap_or(&empty_snap);

        let telemetry =
            compile_deterministic_telemetry(micro_snap, &support_strings, &resistance_strings);

        let multi_agent_results = self
            .llm_client
            .run_multi_agent_pipeline(
                self.pool.clone(),
                &raw_symbol,
                micro_snap,
                fast_snap,
                slow_snap,
                macro_snap,
                &prices,
                master_id,
                &telemetry,
            )
            .await?;

        let legacy_signals = multi_agent_results.to_legacy_signals();
        let phase_one_json =
            serde_json::to_string(&legacy_signals).unwrap_or_else(|_| "[]".into());

        let journal_context =
            crate::db::query_recent_journal_for_context(&self.pool, &raw_symbol, 10).await;
        let journal_opt: Option<&str> = if journal_context.is_empty() {
            None
        } else {
            Some(&journal_context)
        };

        let master_result = self
            .llm_client
            .run_master_orchestrator(
                &position,
                &entry_price,
                &prices,
                &symbol,
                &phase_one_json,
                &telemetry.support_levels,
                &telemetry.resistance_levels,
                journal_opt,
                Some(&symbol),
            )
            .await?;

        self.spawn_background_updates(
            master_id,
            &indicators,
            &master_result,
        )
        .await;

        Ok(MultiAgentAnalysisResponse {
            phase_one: legacy_signals,
            phase_two: PhaseTwoResponse {
                general_trend: master_result.general_trend,
                support_and_resistance: SupportResistanceResponse {
                    detected_support_levels: master_result
                        .support_and_resistance
                        .detected_support_levels,
                    detected_resistance_levels: master_result
                        .support_and_resistance
                        .detected_resistance_levels,
                    structural_analysis: master_result
                        .support_and_resistance
                        .structural_analysis,
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
        })
    }

    async fn spawn_background_updates(
        &self,
        master_id: i64,
        indicators: &IndicatorSnapshot,
        master_result: &crate::llm::MasterOrchestratorResult,
    ) {
        let db_telemetry = self.telemetry_tx.clone();
        let db_pool = self.pool.clone();
        let db_master_id = master_id;
        let db_indicators = indicators.clone();
        let mr_general_trend = master_result.general_trend.clone();
        let mr_support = serde_json::to_string(
            &master_result.support_and_resistance.detected_support_levels,
        )
        .unwrap_or_default();
        let mr_resistance = serde_json::to_string(
            &master_result
                .support_and_resistance
                .detected_resistance_levels,
        )
        .unwrap_or_default();
        let mr_summary = master_result.indicator_synthesis.summary_count.clone();
        let mr_evaluation = master_result.indicator_synthesis.evaluation.clone();
        let mr_action = master_result.position_recommendation.action.clone();
        let mr_rationale = master_result.position_recommendation.rationale.clone();
        let mr_score = master_result.eight_factor_score;
        let mr_allocation = master_result.allocation_pct;

        tokio::spawn(async move {
            let local_snap = indicator_to_snapshot_values(&db_indicators);
            let regime = classify_market_regime(&local_snap);

            let _ = db_telemetry
                .send(crate::db::TelemetryMsg::UpdateMasterRecord {
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
                })
                .await;

            let _ = sqlx::query(
                "UPDATE master_assistant_records SET market_regime = ?2, portfolio_allocation_pct = ?3 WHERE id = ?1"
            )
            .bind(db_master_id)
            .bind(regime.as_str())
            .bind(mr_allocation)
            .execute(&db_pool)
            .await;
        });
    }
}
