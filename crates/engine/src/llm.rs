use serde::{Deserialize, Serialize};

const CHAT_SYSTEM_PROMPT: &str = r#"You are a conversational, professional trading assistant specializing in cryptocurrency technical analysis. You have real-time context of current market indicators (RSI, MACD, Squeeze Momentum, ADX, ATR, EMAs, Bollinger Bands, VWAP) and the user's current position. Your role is to help the user understand market conditions and make informed manual trading decisions.

Guidelines:
- Answer user questions concisely (2-4 sentences).
- Use the provided indicator context to support your answers.
- Never give financial advice or guarantee outcomes. Always frame responses as analysis, not directives.
- When the user asks about a specific indicator, explain what its current value means in the current market context.
- Be professional yet approachable. Use plain language where possible."#;

const MASTER_ORCHESTRATOR_PROMPT: &str = r#"You are the Master AI Trading Orchestrator. Your role is to synthesize individual technical indicator inputs, analyze general price action structure, and formulate a definitive trading recommendation.

RULES:
- If Position is Long or Short, only recommend Hold or Close. Never recommend opening a new position when one is already held.
- If Position is None, only recommend Wait, Open Long, or Open Short.
- Evaluate the last 100 prices to understand the trend structure. Use the provided support and resistance levels to frame your analysis.
- Consider the Phase 1 indicator signals as expert sub-agent opinions. Weight them by their alignment with each other and with price action.
- Output strictly JSON, no markdown fences, no conversational preambles.

OUTPUT SCHEMA:
{
  "general_trend": "UPWARD" | "DOWNWARD" | "SIDEWAYS",
  "support_and_resistance": {
    "structural_analysis": "A concise explanation of how the provided support/resistance levels constrain or influence the current price action. 1-2 sentences."
  },
  "indicator_synthesis": {
    "summary_count": "e.g., '4 Bullish, 1 Bearish, 2 Sideways'",
    "evaluation": "How the indicators converge or diverge from raw price action trend. Explain if the majority of signals support or conflict with the trend direction. 2-3 sentences."
  },
  "position_recommendation": {
    "action": "Hold" | "Close" | "Wait" | "Open Long" | "Open Short",
    "rationale": "Provide a highly clear, professional, conversational operational reasoning guiding the user on their next step given their position entry price, current price action, support/resistance constraints, and trend context. 2-4 sentences."
  }
}"#;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualIndicatorResult {
    pub indicator_name: String,
    pub signal: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SupportResistance {
    #[serde(default)]
    pub detected_support_levels: Vec<String>,
    #[serde(default)]
    pub detected_resistance_levels: Vec<String>,
    #[serde(default)]
    pub structural_analysis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorSynthesis {
    pub summary_count: String,
    pub evaluation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionRecommendation {
    pub action: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterOrchestratorResult {
    pub general_trend: String,
    #[serde(default)]
    pub support_and_resistance: SupportResistance,
    pub indicator_synthesis: IndicatorSynthesis,
    pub position_recommendation: PositionRecommendation,
}

pub struct LlmClient {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) indicators_guide: String,
}

impl LlmClient {
    pub fn from_dotenv() -> Result<Self, String> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| "DEEPSEEK_API_KEY not found in .env file. Create a .env file at the project root with: DEEPSEEK_API_KEY=sk-...".to_string())?;

        let api_key = api_key.trim().to_string();
        if api_key.is_empty() {
            return Err("DEEPSEEK_API_KEY is empty in .env file. Set your DeepSeek API key.".to_string());
        }
        if !api_key.starts_with("sk-") {
            return Err(format!(
                "DEEPSEEK_API_KEY does not look like a valid DeepSeek key (should start with 'sk-'). Got: {}...",
                &api_key[..api_key.len().min(10)]
            ));
        }

        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".into());
        let model = std::env::var("DEEPSEEK_MODEL")
            .unwrap_or_else(|_| "deepseek-chat".into());

        let indicators_guide = std::fs::read_to_string("docs/indicators-guide.md")
            .unwrap_or_else(|_| "No indicators guide found.".to_string());

        Ok(LlmClient {
            base_url,
            api_key,
            model,
            indicators_guide,
        })
    }

    pub async fn validate_key(&self) -> Result<(), String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| format!("Failed to reach DeepSeek API: {}", e))?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unreadable>".into());

        Err(format!(
            "DeepSeek API rejected the key (HTTP {}). Verify your DEEPSEEK_API_KEY in .env\nResponse: {}",
            status, body
        ))
    }

    pub async fn run_indicator_agent(
        &self,
        indicator_name: &str,
        indicator_section: &str,
        user_context: &str,
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
  "reason": "Provide a brief 1-2 sentence explanation of your decision using the rules and the provided numerical parameters."
}}

RULES:
- Respond with JSON ONLY. Do not write markdown fences, preamble, or commentary.
- Be completely deterministic. Use the numerical parameters and apply them strictly against the criteria in the reference docs."#,
            indicator_name, indicator_section, indicator_name
        );

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: system_prompt },
                ChatMessage { role: "user".into(), content: user_context.to_string() },
            ],
            temperature: 0.1,
            response_format: Some(ResponseFormat { format_type: "json_object".into() }),
            max_tokens: 512,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("LLM API request failed for {}: {}", indicator_name, e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(format!("LLM API returned {} for {}: {}", status, indicator_name, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse LLM response for {}: {}", indicator_name, e))?;

        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| format!("LLM response for {} had no choices", indicator_name))?
            .message
            .content
            .clone();

        let result: IndividualIndicatorResult = serde_json::from_str(&content)
            .map_err(|e| format!(
                "Failed to parse LLM JSON output for {}: {}. Raw content: {}",
                indicator_name, e, content
            ))?;

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
    ) -> Result<MasterOrchestratorResult, String> {
        let prices_str = serde_json::to_string(prices)
            .map_err(|e| format!("Failed to serialize prices: {}", e))?;

        let supports_str = serde_json::to_string(support_levels)
            .unwrap_or_else(|_| "[]".into());
        let resistances_str = serde_json::to_string(resistance_levels)
            .unwrap_or_else(|_| "[]".into());

        let entry_info = if entry_price.is_empty() || entry_price == "0" || entry_price == "0.00" {
            "None (no open position)".to_string()
        } else {
            format!("${}", entry_price)
        };

        let user_message = format!(
            "CURRENT MARKET ASSET: {}\n\
             USER'S OPEN POSITION: {}\n\
             USER'S ENTRY PRICE: {}\n\
             RAW PRICE HISTORY (last {} closes): {}\n\
             COMPUTED SUPPORT LEVELS: {}\n\
             COMPUTED RESISTANCE LEVELS: {}\n\
             PHASE 1 INDIVIDUAL INDICATOR AGENT SIGNALS:\n{}",
            symbol, position, entry_info, prices.len(), prices_str,
            supports_str, resistances_str,
            phase_one_results_json,
        );

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: MASTER_ORCHESTRATOR_PROMPT.into() },
                ChatMessage { role: "user".into(), content: user_message },
            ],
            temperature: 0.3,
            response_format: Some(ResponseFormat { format_type: "json_object".into() }),
            max_tokens: 1024,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Master orchestrator request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(format!("Master orchestrator API returned {}: {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse master orchestrator response: {}", e))?;

        let content = chat_response
            .choices
            .first()
            .ok_or("Master orchestrator response had no choices")?
            .message
            .content
            .clone();

        let mut result: MasterOrchestratorResult = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse master orchestrator JSON: {}. Raw content: {}", e, content))?;

        result.support_and_resistance = SupportResistance {
            detected_support_levels: support_levels.to_vec(),
            detected_resistance_levels: resistance_levels.to_vec(),
            structural_analysis: result.support_and_resistance.structural_analysis,
        };

        Ok(result)
    }

    pub fn get_guide_section(&self, indicator_name: &str) -> String {
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

        let lines: Vec<&str> = self.indicators_guide.lines().collect();
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

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, String> {
        let mut full_messages: Vec<ChatMessage> = vec![
            ChatMessage {
                role: "system".into(),
                content: CHAT_SYSTEM_PROMPT.into(),
            },
        ];
        full_messages.extend(messages);

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: full_messages,
            temperature: 0.7,
            response_format: None,
            max_tokens: 1024,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("LLM API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(format!("LLM API returned {}: {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        let content = chat_response
            .choices
            .first()
            .ok_or("LLM response had no choices")?
            .message
            .content
            .clone();

        Ok(content)
    }
}
