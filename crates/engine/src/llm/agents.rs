use super::client::LlmClient;
use super::types::*;
use super::prompts::*;

impl LlmClient {
    pub async fn run_indicator_agent(
        &self,
        indicator_name: &str,
        indicator_section: &str,
        user_context: &str,
        pair_key: Option<&str>,
    ) -> Result<IndividualIndicatorResult, String> {
        let system_prompt = format!(
            r#"You are a highly analytical trading sub-agent specializing strictly in evaluating the technical indicator: {}.
Refer to the provided rules in the indicator reference for interpretation thresholds.

INDICATOR REFERENCE RULES:
{}

CONTEXT:
Analyze the provided current market data. You must output a clean JSON structure conforming to the following schema:

{{
  "indicator_name": "{}",
  "signal": "BULLISH" | "BEARISH" | "SIDEWAYS",
  "confidence_score": <0-100 integer>,
  "reason": "Provide a brief 1-2 sentence explanation of your decision using the rules and the provided numerical parameters."
}}

RULES:
- Respond with JSON ONLY. Do not write markdown fences, preamble, or commentary.
- confidence_score is MANDATORY and must be an integer 0-100 reflecting how certain you are.
- Be completely deterministic. Use the numerical parameters and apply them strictly against the criteria in the reference docs."#,
            indicator_name, indicator_section, indicator_name
        );

        let request_body = ChatRequest {
            model: self.model.as_ref().clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user".into(),
                    content: user_context.to_string(),
                },
            ],
            temperature: 0.1,
            response_format: Some(ResponseFormat {
                format_type: "json_object".into(),
            }),
            max_tokens: 512,
        };

        let chat_response = self
            .call_chat_completion(&request_body, indicator_name)
            .await?;
        let usage = chat_response.usage;
        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| format!("LLM response for {} had no choices", indicator_name))?
            .message
            .content
            .clone();

        let result: IndividualIndicatorResult = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse LLM JSON output for {}: {}. Raw content: {}",
                indicator_name, e, content
            )
        })?;

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

    pub async fn run_master_orchestrator(
        &self,
        position: &str,
        entry_price: &str,
        prices: &[f64],
        symbol: &str,
        phase_one_results_json: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        journal_context: Option<&str>,
        pair_key: Option<&str>,
    ) -> Result<MasterOrchestratorResult, String> {
        let prices_str = serde_json::to_string(prices)
            .map_err(|e| format!("Failed to serialize prices: {}", e))?;

        let supports_str = serde_json::to_string(support_levels).unwrap_or_else(|_| "[]".into());
        let resistances_str =
            serde_json::to_string(resistance_levels).unwrap_or_else(|_| "[]".into());

        let entry_info = if entry_price.is_empty() || entry_price == "0" || entry_price == "0.00" {
            "None (no open position)".to_string()
        } else {
            format!("${}", entry_price)
        };

        let journal_section = match journal_context {
            Some(ctx) if !ctx.is_empty() => format!("\n\n{}", ctx),
            _ => String::new(),
        };

        let user_message = format!(
            "CURRENT MARKET ASSET: {}\n\
             USER'S OPEN POSITION: {}\n\
             USER'S ENTRY PRICE: {}\n\
             RAW PRICE HISTORY (last {} closes): {}\n\
             COMPUTED SUPPORT LEVELS: {}\n\
             COMPUTED RESISTANCE LEVELS: {}\n\
             PHASE 1 INDIVIDUAL INDICATOR AGENT SIGNALS:\n{}{}",
            symbol,
            position,
            entry_info,
            prices.len(),
            prices_str,
            supports_str,
            resistances_str,
            phase_one_results_json,
            journal_section,
        );

        let request_body = ChatRequest {
            model: self.model.as_ref().clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: MASTER_ORCHESTRATOR_PROMPT.into(),
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
            max_tokens: 1024,
        };

        let chat_response = self
            .call_chat_completion(&request_body, "orchestrator")
            .await?;

        let usage = chat_response.usage;

        let content = chat_response
            .choices
            .first()
            .ok_or("Master orchestrator response had no choices")?
            .message
            .content
            .clone();

        let mut result: MasterOrchestratorResult = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse master orchestrator JSON: {}. Raw content: {}",
                e, content
            )
        })?;

        result.support_and_resistance = SupportResistance {
            detected_support_levels: support_levels.to_vec(),
            detected_resistance_levels: resistance_levels.to_vec(),
            structural_analysis: result.support_and_resistance.structural_analysis,
        };

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

    pub async fn run_multi_timeframe_orchestrator(
        &self,
        position: &str,
        entry_price: &str,
        symbol: &str,
        phase_one_results_json: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        journal_context: Option<&str>,
        pair_key: Option<&str>,
    ) -> Result<MasterOrchestratorResult, String> {
        let supports_str = serde_json::to_string(support_levels).unwrap_or_else(|_| "[]".into());
        let resistances_str =
            serde_json::to_string(resistance_levels).unwrap_or_else(|_| "[]".into());

        let entry_info = if entry_price.is_empty() || entry_price == "0" || entry_price == "0.00" {
            "None (no open position)".to_string()
        } else {
            format!("${}", entry_price)
        };

        let journal_section = match journal_context {
            Some(ctx) if !ctx.is_empty() => format!("\n\n{}", ctx),
            _ => String::new(),
        };

        let user_message = format!(
            "CURRENT MARKET ASSET: {}\n\
             USER'S OPEN POSITION: {}\n\
             USER'S ENTRY PRICE: {}\n\
             COMPUTED SUPPORT LEVELS: {}\n\
             COMPUTED RESISTANCE LEVELS: {}\n\
             PHASE 1 MULTI-TIMEFRAME INDICATOR AGENT SIGNALS (micro/fast/slow/macro prefix):\n{}{}",
            symbol, position, entry_info,
            supports_str, resistances_str,
            phase_one_results_json,
            journal_section,
        );

        let request_body = ChatRequest {
            model: self.model.as_ref().clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: MULTI_TF_MASTER_ORCHESTRATOR_PROMPT.into(),
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
            max_tokens: 1024,
        };

        let chat_response = self
            .call_chat_completion(&request_body, "multi-tf-orchestrator")
            .await?;

        let usage = chat_response.usage;

        let content = chat_response
            .choices
            .first()
            .ok_or("Multi-TF orchestrator response had no choices")?
            .message
            .content
            .clone();

        let mut result: MasterOrchestratorResult = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse multi-TF orchestrator JSON: {}. Raw: {}",
                e, content
            )
        })?;

        result.support_and_resistance = SupportResistance {
            detected_support_levels: support_levels.to_vec(),
            detected_resistance_levels: resistance_levels.to_vec(),
            structural_analysis: result.support_and_resistance.structural_analysis,
        };

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

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

    pub async fn get_guide_section(&self, indicator_name: &str) -> String {
        let section_number = match indicator_name {
            "RSI" => "1.",
            "MACD" => "2.",
            "SQUEEZE" => "3.",
            "ADX" => "4.",
            "BOLLINGER_ATR" => "5.",
            "VOLUME_EMA" => "6.",
            "VWAP" => "7.",
            _ => return "No rules found.".to_string(),
        };

        let guide = self.indicators_guide.read().await;
        let lines: Vec<&str> = guide.lines().collect();
        let mut start_idx = None;
        let mut end_idx = None;

        for (i, line) in lines.iter().enumerate() {
            if line.starts_with(&format!("## {}", section_number)) {
                start_idx = Some(i);
            }
            if start_idx.is_some() && end_idx.is_none() && i > start_idx.unwrap() {
                if line.starts_with("## ") && !line.starts_with(&format!("## {}", section_number)) {
                    end_idx = Some(i);
                }
                if line.starts_with("---") && i > start_idx.unwrap() + 5 {
                    end_idx = Some(i);
                }
            }
        }

        match (start_idx, end_idx) {
            (Some(s), Some(e)) => lines[s..e].join("\n"),
            (Some(s), None) => lines[s..].join("\n"),
            _ => "Section not found in indicators guide.".to_string(),
        }
    }

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

    pub async fn run_domain_agent<T>(
        &self,
        agent_name: &str,
        system_prompt: &str,
        user_context: &str,
        pair_key: Option<&str>,
    ) -> Result<AgentEvaluationResult<T>, String>
    where
        T: for<'de> serde::Deserialize<'de> + serde::Serialize + Clone,
    {
        let request_body = ChatRequest {
            model: self.model.as_ref().clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: user_context.to_string(),
                },
            ],
            temperature: 0.1,
            response_format: Some(ResponseFormat {
                format_type: "json_object".into(),
            }),
            max_tokens: 1024,
        };

        let chat_response = self.call_chat_completion(&request_body, agent_name).await?;

        let usage = chat_response.usage;
        self.track_usage(pair_key, &usage);

        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| format!("LLM response for {} had no choices", agent_name))?
            .message
            .content
            .clone();

        let parsed_result: AgentEvaluationResult<T> =
            serde_json::from_str(&content).map_err(|e| {
                format!(
                    "Failed to parse JSON output for {}: {}. Raw content: {}",
                    agent_name, e, content
                )
            })?;

        Ok(parsed_result)
    }
}
