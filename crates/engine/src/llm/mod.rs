// LLM module — AI client, prompts, types, and token tracking (v3.0).
// Two-Agent Pipeline: Analyst (information preparation) → Trader (decision execution).

pub mod prompts;
pub mod types;
pub mod token;
pub mod client;
pub mod agents;

// Public re-exports
pub use client::{LlmClient, LlmClientConfig};
pub use types::{
    AnalystDocument, ChatMessage, JournalResult, TraderDecision,
};
pub use token::{PairTokenUsage, TokenTracker};

// Crate-private re-exports for internal use
pub(crate) use types::{ChatRequest, ResponseFormat};
