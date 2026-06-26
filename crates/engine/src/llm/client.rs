use std::sync::Arc;
// LlmClient — HTTP client for DeepSeek API with failover, token tracking, and agent orchestration.

use super::token::*;


use super::token::TokenTracker;
use super::types::{
    ChatRequest, ChatResponse, Usage,
};

#[derive(Clone)]
pub struct LlmClient {
    pub(crate) base_url: Arc<String>,
    pub(crate) api_key: Arc<tokio::sync::RwLock<String>>,
    pub(crate) model: Arc<String>,
    pub(crate) indicators_guide: Arc<tokio::sync::RwLock<String>>,
    pub(crate) token_tracker: Arc<TokenTracker>,
    pub(crate) failover_state: Option<Arc<crate::api_failover::ApiFailoverState>>,
}

pub struct LlmClientConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
}

impl LlmClient {
    pub fn empty() -> Self {
        LlmClient {
            base_url: Arc::new(String::new()),
            api_key: Arc::new(tokio::sync::RwLock::new(String::new())),
            model: Arc::new(String::new()),
            indicators_guide: Arc::new(tokio::sync::RwLock::new(String::new())),
            token_tracker: Arc::new(TokenTracker::default()),
            failover_state: None,
        }
    }

    pub fn from_env() -> (Self, bool) {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .map(|k| k.trim().to_string())
            .unwrap_or_default();

        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".into());
        let model =
            std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".into());

        Self::from_config(LlmClientConfig {
            api_key: if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            },
            base_url,
            model,
        })
    }

    pub fn from_config(config: LlmClientConfig) -> (Self, bool) {
        let indicators_guide =
            std::fs::read_to_string("docs/indicators-guide.md").unwrap_or_else(|_| String::new());
        let key_present = config.api_key.is_some();

        (
            LlmClient {
                base_url: Arc::new(config.base_url),
                api_key: Arc::new(tokio::sync::RwLock::new(
                    config.api_key.unwrap_or_default(),
                )),
                model: Arc::new(config.model),
                indicators_guide: Arc::new(tokio::sync::RwLock::new(indicators_guide)),
                token_tracker: Arc::new(TokenTracker::default()),
                failover_state: None,
            },
            key_present,
        )
    }

    pub fn from_dotenv() -> Result<Self, String> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| "DEEPSEEK_API_KEY not found in .env file. Create a .env file at the project root with: DEEPSEEK_API_KEY=sk-...".to_string())?;

        let api_key = api_key.trim().to_string();
        if api_key.is_empty() {
            return Err(
                "DEEPSEEK_API_KEY is empty in .env file. Set your DeepSeek API key.".to_string(),
            );
        }
        if !api_key.starts_with("sk-") {
            return Err(format!(
                "DEEPSEEK_API_KEY does not look like a valid DeepSeek key (should start with 'sk-'). Got: {}...",
                &api_key[..api_key.len().min(10)]
            ));
        }

        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".into());
        let model = std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".into());

        let indicators_guide = std::fs::read_to_string("docs/indicators-guide.md")
            .unwrap_or_else(|_| "No indicators guide found.".to_string());

        Ok(LlmClient {
            base_url: Arc::new(base_url),
            api_key: Arc::new(tokio::sync::RwLock::new(api_key)),
            model: Arc::new(model),
            indicators_guide: Arc::new(tokio::sync::RwLock::new(indicators_guide)),
            token_tracker: Arc::new(TokenTracker::default()),
            failover_state: None,
        })
    }

    pub async fn set_api_key(&self, key: String) {
        *self.api_key.write().await = key;
    }

    pub async fn set_indicators_guide(&self, guide: String) {
        *self.indicators_guide.write().await = guide;
    }

    pub fn get_token_tracker(&self) -> Arc<TokenTracker> {
        self.token_tracker.clone()
    }

    pub fn reset_token_usage(&self) {
        self.token_tracker.reset();
    }

    pub fn get_token_usage_for_pair(&self, pair_key: &str) -> PairTokenUsage {
        self.token_tracker.get_per_pair(pair_key)
    }

    pub fn set_failover_state(&mut self, state: Arc<crate::api_failover::ApiFailoverState>) {
        self.failover_state = Some(state);
    }

    pub fn get_failover_state(&self) -> Option<Arc<crate::api_failover::ApiFailoverState>> {
        self.failover_state.clone()
    }

    /// Get the API key to use, preferring the failover state's active key if available.
    pub async fn active_api_key(&self) -> String {
        if let Some(ref fs) = self.failover_state {
            if let Some(key) = fs.active_key().await {
                return key;
            }
        }
        self.api_key.read().await.clone()
    }

    pub async fn validate_key(&self) -> Result<(), String> {
        let api_key = self.api_key.read().await;
        if api_key.is_empty() {
            return Err("No API key configured".to_string());
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key.as_str()))
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

    pub(crate) fn track_usage(&self, pair_key: Option<&str>, usage: &Option<Usage>) {
        if let Some(u) = usage {
            self.token_tracker
                .accumulate(pair_key, u.prompt_tokens, u.completion_tokens);
        }
    }

    /// Make a chat completion HTTP call with failover-aware key selection.
    /// Uses the failover state if configured, falls back to self.api_key.
    pub(crate) async fn call_chat_completion(
        &self,
        request_body: &ChatRequest,
        call_name: &str,
    ) -> Result<ChatResponse, String> {
        let base_url = self.base_url.as_ref().clone();

        // Resolve the API key: prefer failover state
        let api_key = self.active_api_key().await;
        if api_key.is_empty() {
            return Err(format!("[{}] No API key configured", call_name));
        }

        // If we have a failover state, use its retry/failover logic
        if let Some(ref fs) = self.failover_state {
            fs.execute_with_failover(call_name, move |key| {
                let url = format!("{}/chat/completions", base_url);
                let body = request_body.clone();
                async move {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(12))
                        .build()
                        .map_err(|e| format!("HTTP client build error: {}", e))?;

                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", key))
                        .header("Content-Type", "application/json")
                        .json(&body)
                        .send()
                        .await
                        .map_err(|e| format!("API request failed: {}", e))?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "<unreadable>".into());
                        return Err(format!("API returned HTTP {}: {}", status, body));
                    }

                    response
                        .json::<ChatResponse>()
                        .await
                        .map_err(|e| format!("Failed to parse API response: {}", e))
                }
            })
            .await
        } else {
            // No failover state: direct call with self.api_key
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .build()
                .map_err(|e| format!("HTTP client build error: {}", e))?;

            let response = client
                .post(format!("{}/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(request_body)
                .send()
                .await
                .map_err(|e| format!("API request failed: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<unreadable>".into());
                return Err(format!("API returned HTTP {}: {}", status, body));
            }

            response
                .json::<ChatResponse>()
                .await
                .map_err(|e| format!("Failed to parse API response: {}", e))
        }
    }

}
