use serde::{Deserialize, Serialize};
// Data types for LLM API communication — Two-Agent Pipeline (v3.0).
// Architecture: Analyst Agent (information preparation) → Trader Agent (decision execution).

// ─── Core API types ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ResponseFormat {
    #[serde(rename = "type")]
    pub(crate) format_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChatRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) response_format: Option<ResponseFormat>,
    pub(crate) max_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatResponse {
    pub(crate) choices: Vec<Choice>,
    #[serde(default)]
    pub(crate) usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    #[allow(dead_code)]
    total_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Choice {
    pub(crate) message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChoiceMessage {
    pub(crate) content: String,
}

// ─── Journal Agent (kept for post-trade audit) ─────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalResult {
    pub final_analysis: String,
    pub execution_score: f64,
}

// ─── Analyst Agent — Structured Market Document (v3.0) ─────────────
// Agent 1 receives ALL indicator data and produces a well-organized,
// human-readable document. NO trading decisions.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalystDocument {
    /// Overall market regime, directional bias, confluence score summary
    pub market_summary: String,
    /// EMA stack state, ADX strength, Ichimoku, Supertrend, trend indicators
    pub trend_indicators: String,
    /// RSI, MACD, Stochastic, CCI, Williams %R, momentum oscillators
    pub momentum_indicators: String,
    /// Bollinger Bands, BBWP, ATR regime, Squeeze status, Historical Volatility
    pub volatility_indicators: String,
    /// RVOL, OBV, CMF, MFI, VWAP bias, volume analysis
    pub volume_indicators: String,
    /// Support/Resistance levels, Fibonacci levels, Pivot Points, Chart Patterns
    pub structure_indicators: String,
    /// Active confirmed signals: divergences, crossovers, breakouts, squeeze releases
    pub active_signals: String,
    /// Weighted confluence score, consensus level, regime confidence, statistical context
    pub confluence_summary: String,
}

// ─── Trader Agent — Final Trading Decision (v3.0) ─────────────────
// Agent 2 makes strict trading decisions based solely on the Analyst's document.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderDecision {
    /// Hold | Close | Wait | Open Long | Open Short
    pub action: String,
    /// Confidence 0-100
    pub confidence: u8,
    /// Clear operational reasoning for the decision
    pub rationale: String,
    /// Any risk warnings or caveats
    pub risk_notes: String,
}
