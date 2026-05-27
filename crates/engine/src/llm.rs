use serde::{Deserialize, Serialize};

const SYSTEM_PROMPT: &str = r#"You are a professional trading assistant analyzing cryptocurrency market data. You receive historical prices and current technical indicators for ETH-USD. Analyze the data and return ONLY a JSON object with no additional text. The JSON must follow this exact schema:

{
  "trend_analysis": {
    "classification": "trending upwards" | "trending downwards" | "sideways",
    "structural_reasoning": "brief description of the raw price action observed across the sequence"
  },
  "indicator_alignment": {
    "classification": "supportive" | "conflicting" | "neutral",
    "observation": "brief detail on how key variables like Squeeze Momentum, MACD Histogram, and RSI match or diverge from the identified trend"
  },
  "position_recommendation": {
    "action": "Hold" | "Close" | "Wait" | "Open Long" | "Open Short",
    "rationale": "clear operational reasoning guiding the user on the optimal step given their position context"
  }
}

Rules:
- If position is Long or Short, only recommend Hold or Close. Never recommend opening a new position when one is already held.
- If position is None, only recommend Wait, Open Long, or Open Short.
- Evaluate the last 100 price candles for trend direction. A 0.5% change over the sequence is the threshold between sideways and trending.
- Compare indicators against the trend: RSI above 50 supports upward, below 50 supports downward. MACD histogram positive supports upward, negative supports downward. Squeeze ON signals potential breakout.
- Be concise. Each reasoning string should be 1-2 sentences maximum.
- Output ONLY the JSON object. No markdown code fences, no preamble."#;

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
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

#[derive(Debug, Clone, Deserialize)]
pub struct LlmTrendAnalysis {
    pub classification: String,
    pub structural_reasoning: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmIndicatorAlignment {
    pub classification: String,
    pub observation: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmPositionRecommendation {
    pub action: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmAnalysisResult {
    pub trend_analysis: LlmTrendAnalysis,
    pub indicator_alignment: LlmIndicatorAlignment,
    pub position_recommendation: LlmPositionRecommendation,
}

pub struct LlmClient {
    base_url: String,
    api_key: String,
    model: String,
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

        Ok(LlmClient {
            base_url,
            api_key,
            model,
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

    pub async fn analyze(
        &self,
        position: &str,
        prices: &[f64],
        rsi: Option<f64>,
        squeeze_on: Option<bool>,
        macd_hist: Option<f64>,
        adx: Option<f64>,
        ema_fast: Option<f64>,
        ema_slow: Option<f64>,
    ) -> Result<LlmAnalysisResult, String> {
        let prices_str = serde_json::to_string(prices)
            .map_err(|e| format!("Failed to serialize prices: {}", e))?;

        let indicators_line = format!(
            "RSI: {}, Squeeze: {}, MACD Histogram: {}, ADX: {}, EMA Fast: {}, EMA Slow: {}",
            rsi.map_or("N/A".to_string(), |v| format!("{:.2}", v)),
            squeeze_on.map_or("N/A".to_string(), |v| if v { "ON".to_string() } else { "OFF".to_string() }),
            macd_hist.map_or("N/A".to_string(), |v| format!("{:.4}", v)),
            adx.map_or("N/A".to_string(), |v| format!("{:.2}", v)),
            ema_fast.map_or("N/A".to_string(), |v| format!("{:.2}", v)),
            ema_slow.map_or("N/A".to_string(), |v| format!("{:.2}", v)),
        );

        let user_message = format!(
            "Position: {}\nHistorical prices (last {} candles): {}\nCurrent indicators: {}",
            position,
            prices.len(),
            prices_str,
            indicators_line,
        );

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: SYSTEM_PROMPT.into(),
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
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable>".into());
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

        let result: LlmAnalysisResult = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse LLM JSON output: {}. Raw content: {}", e, content))?;

        Ok(result)
    }
}
