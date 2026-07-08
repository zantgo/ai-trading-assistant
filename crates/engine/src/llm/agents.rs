use super::client::LlmClient;
use super::types::*;
use super::prompts::*;

impl LlmClient {
    // ─── Agent 1: Market Analyst (Information Preparation) ──────────

    pub async fn run_analyst_agent(
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
        let prices_str = serde_json::to_string(prices)
            .map_err(|e| format!("Failed to serialize prices: {}", e))?;
        let supports_str = serde_json::to_string(support_levels).unwrap_or_else(|_| "[]".into());
        let resistances_str =
            serde_json::to_string(resistance_levels).unwrap_or_else(|_| "[]".into());

        let user_message = format!(
            r#"{{"symbol":"{}","current_price":{},"indicators":{},"decision_context":{},"market_context":{},"support_levels":{},"resistance_levels":{},"price_history":{}}}"#,
            symbol,
            current_price,
            indicators_dto,
            decision_context,
            market_context,
            supports_str,
            resistances_str,
            prices_str,
        );

        let request_body = ChatRequest {
            model: self.model.as_ref().clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: ANALYST_AGENT_PROMPT.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: user_message,
                },
            ],
            temperature: 0.3,
            response_format: Some(ResponseFormat {
                format_type: "json_object".into(),
            }),
            max_tokens: 2048,
        };

        let chat_response = self
            .call_chat_completion(&request_body, "analyst-agent")
            .await?;

        let usage = chat_response.usage;
        let content = chat_response
            .choices
            .first()
            .ok_or("Analyst agent response had no choices")?
            .message
            .content
            .clone();

        let result: AnalystDocument = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse analyst agent JSON: {}. Raw content: {}",
                e, content
            )
        })?;

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

    // ─── Agent 2: Decision Trader (Decision Execution) ─────────────

    pub async fn run_trader_agent(
        &self,
        analyst_document: &str,
        position: &str,
        entry_price: &str,
        symbol: &str,
        risk_profile: &str,
        pair_key: Option<&str>,
    ) -> Result<TraderDecision, String> {
        let entry_info = if entry_price.is_empty() || entry_price == "0" || entry_price == "0.00" {
            "None (no open position)".to_string()
        } else {
            format!("${}", entry_price)
        };

        let risk_field = if risk_profile.trim().is_empty() {
            "null".to_string()
        } else {
            risk_profile.to_string()
        };

        let user_message = format!(
            r#"{{"analyst_document":{},"position":"{}","entry_price":"{}","symbol":"{}","risk_profile":{}}}"#,
            analyst_document, position, entry_info, symbol, risk_field,
        );

        let request_body = ChatRequest {
            model: self.model.as_ref().clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: TRADER_AGENT_PROMPT.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: user_message,
                },
            ],
            temperature: 0.2,
            response_format: Some(ResponseFormat {
                format_type: "json_object".into(),
            }),
            max_tokens: 1024,
        };

        let chat_response = self
            .call_chat_completion(&request_body, "trader-agent")
            .await?;

        let usage = chat_response.usage;
        let content = chat_response
            .choices
            .first()
            .ok_or("Trader agent response had no choices")?
            .message
            .content
            .clone();

        let result: TraderDecision = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse trader agent JSON: {}. Raw content: {}",
                e, content
            )
        })?;

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

    // ─── Journal Agent (Post-Trade Audit — kept) ───────────────────

    pub async fn run_journal_agent(
        &self,
        trade_context: &str,
        pair_key: Option<&str>,
    ) -> Result<JournalResult, String> {
        let request_body = ChatRequest {
            model: self.model.as_ref().clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: JOURNAL_AGENT_PROMPT.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: trade_context.to_string(),
                },
            ],
            temperature: 0.2,
            response_format: Some(ResponseFormat {
                format_type: "json_object".into(),
            }),
            max_tokens: 512,
        };

        let chat_response = self
            .call_chat_completion(&request_body, "journal-agent")
            .await?;

        let usage = chat_response.usage;

        let content = chat_response
            .choices
            .first()
            .ok_or("Journal agent response had no choices")?
            .message
            .content
            .clone();

        let mut result: JournalResult = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse journal agent JSON: {}. Raw content: {}",
                e, content
            )
        })?;

        result.execution_score = result.execution_score.clamp(0.0, 10.0);

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

    // ─── Chat Agent (Conversational — kept) ────────────────────────

    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        pair_key: Option<&str>,
    ) -> Result<String, String> {
        let mut full_messages: Vec<ChatMessage> = vec![ChatMessage {
            role: "system".into(),
            content: CHAT_SYSTEM_PROMPT.into(),
        }];
        full_messages.extend(messages);

        let request_body = ChatRequest {
            model: self.model.as_ref().clone(),
            messages: full_messages,
            temperature: 0.7,
            response_format: None,
            max_tokens: 1024,
        };

        let chat_response = self.call_chat_completion(&request_body, "chat").await?;

        let usage = chat_response.usage;

        let content = chat_response
            .choices
            .first()
            .ok_or("LLM response had no choices")?
            .message
            .content
            .clone();

        self.track_usage(pair_key, &usage);

        Ok(content)
    }
}
