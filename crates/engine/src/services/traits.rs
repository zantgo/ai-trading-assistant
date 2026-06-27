use std::sync::Arc;

use crate::llm::{LlmClient, MasterOrchestratorResult};
use crate::server::pipeline;
use crate::server::telemetry::DeterministicTelemetry;
use crate::server::types::IndicatorSnapshot;

// ─── LLM Service Trait ────────────────────────────────────────────

#[async_trait::async_trait]
pub trait LlmService: Send + Sync {
    async fn run_multi_agent_pipeline(
        &self,
        pool: sqlx::SqlitePool,
        symbol: &str,
        micro: &IndicatorSnapshot,
        fast: &IndicatorSnapshot,
        slow: &IndicatorSnapshot,
        macro_snap: &IndicatorSnapshot,
        prices: &[f64],
        master_id: i64,
        telemetry: &DeterministicTelemetry,
    ) -> Result<crate::llm::MultiAgentResults, String>;

    async fn run_master_orchestrator(
        &self,
        position: &str,
        entry_price: &str,
        prices: &[f64],
        symbol: &str,
        phase_one_json: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        journal_context: Option<&str>,
        pair_key: Option<&str>,
    ) -> Result<MasterOrchestratorResult, String>;

    async fn has_api_key(&self) -> bool;
}

#[async_trait::async_trait]
impl LlmService for LlmClient {
    async fn run_multi_agent_pipeline(
        &self,
        pool: sqlx::SqlitePool,
        symbol: &str,
        micro: &IndicatorSnapshot,
        fast: &IndicatorSnapshot,
        slow: &IndicatorSnapshot,
        macro_snap: &IndicatorSnapshot,
        prices: &[f64],
        master_id: i64,
        telemetry: &DeterministicTelemetry,
    ) -> Result<crate::llm::MultiAgentResults, String> {
        pipeline::run_multi_agent_pipeline(
            Arc::new(self.clone()),
            pool,
            symbol,
            micro,
            fast,
            slow,
            macro_snap,
            prices,
            master_id,
            telemetry,
        )
        .await
    }

    async fn run_master_orchestrator(
        &self,
        position: &str,
        entry_price: &str,
        prices: &[f64],
        symbol: &str,
        phase_one_json: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        journal_context: Option<&str>,
        pair_key: Option<&str>,
    ) -> Result<MasterOrchestratorResult, String> {
        LlmClient::run_master_orchestrator(
            self,
            position,
            entry_price,
            prices,
            symbol,
            phase_one_json,
            support_levels,
            resistance_levels,
            journal_context,
            pair_key,
        )
        .await
    }

    async fn has_api_key(&self) -> bool {
        !self.api_key.read().await.is_empty()
    }
}

#[async_trait::async_trait]
impl LlmService for Arc<LlmClient> {
    async fn run_multi_agent_pipeline(
        &self,
        pool: sqlx::SqlitePool,
        symbol: &str,
        micro: &IndicatorSnapshot,
        fast: &IndicatorSnapshot,
        slow: &IndicatorSnapshot,
        macro_snap: &IndicatorSnapshot,
        prices: &[f64],
        master_id: i64,
        telemetry: &DeterministicTelemetry,
    ) -> Result<crate::llm::MultiAgentResults, String> {
        pipeline::run_multi_agent_pipeline(
            self.clone(),
            pool,
            symbol,
            micro,
            fast,
            slow,
            macro_snap,
            prices,
            master_id,
            telemetry,
        )
        .await
    }

    async fn run_master_orchestrator(
        &self,
        position: &str,
        entry_price: &str,
        prices: &[f64],
        symbol: &str,
        phase_one_json: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        journal_context: Option<&str>,
        pair_key: Option<&str>,
    ) -> Result<MasterOrchestratorResult, String> {
        LlmClient::run_master_orchestrator(
            self,
            position,
            entry_price,
            prices,
            symbol,
            phase_one_json,
            support_levels,
            resistance_levels,
            journal_context,
            pair_key,
        )
        .await
    }

    async fn has_api_key(&self) -> bool {
        !self.api_key.read().await.is_empty()
    }
}
