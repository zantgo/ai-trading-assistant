// ─── JSON-RPC 2.0 Method Names (Standardized) ────────────────────────

// Layer 1-2 → Layer 3: Indicator data passed to AI agents
pub const METHOD_INDICATOR_ANALYZE: &str = "indicator.analyze";
pub const METHOD_DOMAIN_AGENT_EVALUATE: &str = "agent.evaluate";

// Layer 3 → Layer 4: Agent verdicts passed to orchestrator
pub const METHOD_AGENT_VERDICT: &str = "agent.verdict";

// Layer 4: Orchestrator decision
pub const METHOD_ORCHESTRATOR_DECIDE: &str = "orchestrator.decide";
pub const METHOD_ORCHESTRATOR_INTERVAL_SELECT: &str = "orchestrator.interval_select";

// Risk / Execution layer
pub const METHOD_EXECUTION_OPEN: &str = "execution.open_position";
pub const METHOD_EXECUTION_CLOSE: &str = "execution.close_position";
pub const METHOD_EXECUTION_ADJUST: &str = "execution.adjust_position";
pub const METHOD_EXECUTION_VALIDATE: &str = "execution.validate";

// Safety subsystem
pub const METHOD_SAFETY_CHECK: &str = "safety.check";
pub const METHOD_SAFETY_DROPOUT_TRIGGERED: &str = "safety.dropout_triggered";
pub const METHOD_SAFETY_RESET: &str = "safety.reset";

// Instance management
pub const METHOD_INSTANCE_CREATE: &str = "instance.create";
pub const METHOD_INSTANCE_PAUSE: &str = "instance.pause";
pub const METHOD_INSTANCE_STOP: &str = "instance.stop";
pub const METHOD_INSTANCE_DELETE: &str = "instance.delete";
pub const METHOD_INSTANCE_STATUS_CHANGED: &str = "instance.status_changed";

// Broadcast to frontend
pub const METHOD_BROADCAST_MARKET_SNAPSHOT: &str = "broadcast.market_snapshot";
pub const METHOD_BROADCAST_AGENT_RESULT: &str = "broadcast.agent_result";
pub const METHOD_BROADCAST_ORCHESTRATOR_RESULT: &str = "broadcast.orchestrator_result";
pub const METHOD_BROADCAST_SAFETY_EVENT: &str = "broadcast.safety_event";
pub const METHOD_BROADCAST_SYSTEM_STATUS: &str = "broadcast.system_status";

// Chat
pub const METHOD_CHAT_MESSAGE: &str = "chat.message";
pub const METHOD_CHAT_RESPONSE: &str = "chat.response";

// Historical analyst
pub const METHOD_HISTORICAL_ANALYSIS: &str = "historical.analysis";
pub const METHOD_HISTORICAL_RECOMMENDATION: &str = "historical.recommendation";

// Configuration
pub const METHOD_CONFIG_UPDATE: &str = "config.update";
pub const METHOD_CONFIG_QUERY: &str = "config.query";

// Session
pub const METHOD_SESSION_INIT: &str = "session.init";
pub const METHOD_SESSION_QUIT: &str = "session.quit";
pub const METHOD_SESSION_STATUS: &str = "session.status";

// ─── Agent Response Parameter Schemas ───────────────────────────────

/// Standard fields expected in every agent verdict response.
/// This is the contract: every agent MUST return confidence_score (0-100).
pub struct AgentResponseSchema;

impl AgentResponseSchema {
    pub const CONFIDENCE_SCORE: &str = "confidence_score";
    pub const SIGNAL: &str = "signal";
    pub const REASON: &str = "reason";
    pub const THOUGHT: &str = "thought";
    pub const DATA: &str = "data";
}

/// Orchestrator response must include next_interval selection.
pub struct OrchestratorResponseSchema;

impl OrchestratorResponseSchema {
    pub const ACTION: &str = "action";
    pub const RATIONALE: &str = "rationale";
    pub const NEXT_INTERVAL: &str = "next_interval";
    pub const CONFIDENCE_WEIGHTED_SCORE: &str = "confidence_weighted_score";
    pub const RISK_ADJUSTMENT: &str = "risk_adjustment";
}
