use std::sync::Arc;

use crate::llm::{AnalystDocument, LlmClient, TraderDecision};

// ─── LLM Service Trait (v3.0) ──────────────────────────────────────
// Two-Agent Pipeline: Analyst (information prep) → Trader (decision).

#[async_trait::async_trait]
pub trait LlmService: Send + Sync {
    async fn run_analyst(
        &self,
        symbol: &str,
        current_price: f64,
        indicators_dto: &str,
        decision_context: &str,
        market_context: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        prices: &[f64],
        pair_key: Option<&str>,
    ) -> Result<AnalystDocument, String>;

    async fn run_trader(
        &self,
        analyst_document: &str,
        position: &str,
        entry_price: &str,
        symbol: &str,
        risk_profile: &str,
        pair_key: Option<&str>,
    ) -> Result<TraderDecision, String>;

    async fn has_api_key(&self) -> bool;
}

#[async_trait::async_trait]
impl LlmService for LlmClient {
    async fn run_analyst(
        &self,
        symbol: &str,
        current_price: f64,
        indicators_dto: &str,
        decision_context: &str,
        market_context: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        prices: &[f64],
        pair_key: Option<&str>,
    ) -> Result<AnalystDocument, String> {
        LlmClient::run_analyst_agent(
            self,
            symbol,
            current_price,
            indicators_dto,
            decision_context,
            market_context,
            support_levels,
            resistance_levels,
            prices,
            pair_key,
        )
        .await
    }

    async fn run_trader(
        &self,
        analyst_document: &str,
        position: &str,
        entry_price: &str,
        symbol: &str,
        risk_profile: &str,
        pair_key: Option<&str>,
    ) -> Result<TraderDecision, String> {
        LlmClient::run_trader_agent(
            self, analyst_document, position, entry_price, symbol, risk_profile, pair_key,
        )
        .await
    }

    async fn has_api_key(&self) -> bool {
        !self.api_key.read().await.is_empty()
    }
}

#[async_trait::async_trait]
impl LlmService for Arc<LlmClient> {
    async fn run_analyst(
        &self,
        symbol: &str,
        current_price: f64,
        indicators_dto: &str,
        decision_context: &str,
        market_context: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        prices: &[f64],
        pair_key: Option<&str>,
    ) -> Result<AnalystDocument, String> {
        LlmClient::run_analyst_agent(
            self,
            symbol,
            current_price,
            indicators_dto,
            decision_context,
            market_context,
            support_levels,
            resistance_levels,
            prices,
            pair_key,
        )
        .await
    }

    async fn run_trader(
        &self,
        analyst_document: &str,
        position: &str,
        entry_price: &str,
        symbol: &str,
        risk_profile: &str,
        pair_key: Option<&str>,
    ) -> Result<TraderDecision, String> {
        LlmClient::run_trader_agent(
            self, analyst_document, position, entry_price, symbol, risk_profile, pair_key,
        )
        .await
    }

    async fn has_api_key(&self) -> bool {
        !self.api_key.read().await.is_empty()
    }
}
