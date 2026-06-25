use serde::{Deserialize, Serialize};
// Data types for LLM API communication and orchestrator results.

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
    pub(crate) prompt_tokens: u64,
    pub(crate) completion_tokens: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualIndicatorResult {
    pub indicator_name: String,
    pub signal: String,
    pub reason: String,
    #[serde(default)]
    pub confidence_score: u8,
    #[serde(default)]
    pub divergence_status: Option<String>,
    #[serde(default)]
    pub divergence_type: Option<String>,
    #[serde(default)]
    pub is_confirmed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalResult {
    pub final_analysis: String,
    pub execution_score: f64,
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
    #[serde(default)]
    pub next_interval: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterOrchestratorResult {
    pub general_trend: String,
    #[serde(default)]
    pub support_and_resistance: SupportResistance,
    pub indicator_synthesis: IndicatorSynthesis,
    pub position_recommendation: PositionRecommendation,
    #[serde(default)]
    pub eight_factor_score: i32,
    #[serde(default)]
    pub allocation_pct: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentEvaluationResult<T> {
    pub thought: String,
    pub data: T,
}

// ─── Sub-Agent Output Schemas ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrendAgentData {
    pub directional_bias: String,
    pub confidence_score: i32,
    pub ema_slope_alignment: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VolatilityAgentData {
    pub regime_classification: String,
    pub volatility_score: i32,
    pub suggest_stop_multiplier: f64,
    pub is_actionable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructureAgentData {
    pub support_proximity_pct: f64,
    pub resistance_proximity_pct: f64,
    pub golden_pocket_status: String,
    pub structural_score: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskAgentData {
    pub suggested_sizing_pct: f64,
    pub leverage: i32,
    pub exposure_score: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionAgentData {
    pub recommended_action: String,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MasterOrchestrationData {
    pub market_regime: String,
    pub eight_factor_score: i32,
    pub decision: String,
    pub allocation_pct: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentResults {
    pub trend: AgentEvaluationResult<TrendAgentData>,
    pub volatility: AgentEvaluationResult<VolatilityAgentData>,
    pub structure: AgentEvaluationResult<StructureAgentData>,
    pub risk: AgentEvaluationResult<RiskAgentData>,
    pub position: AgentEvaluationResult<PositionAgentData>,
}
