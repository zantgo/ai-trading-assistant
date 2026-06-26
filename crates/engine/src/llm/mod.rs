// LLM module — AI client, prompts, types, and token tracking.

pub mod prompts;
pub mod types;
pub mod token;
pub mod client;
pub mod agents;
pub mod results;

// Public re-exports
pub use prompts::{
    POSITION_AGENT_PROMPT, RISK_AGENT_PROMPT, STRUCTURE_AGENT_PROMPT, TREND_AGENT_PROMPT,
    VOLATILITY_AGENT_PROMPT,
};
pub use client::{LlmClient, LlmClientConfig};
pub use types::{
    AgentEvaluationResult, ChatMessage, IndividualIndicatorResult, IndicatorSynthesis,
    JournalResult, MasterOrchestrationData, MasterOrchestratorResult, MultiAgentResults,
    PositionAgentData, PositionRecommendation, RiskAgentData, StructureAgentData,
    SupportResistance, TrendAgentData, VolatilityAgentData,
};
pub use token::{PairTokenUsage, TokenTracker};

// Crate-private re-exports for internal use
pub(crate) use types::{ChatRequest, ResponseFormat};
