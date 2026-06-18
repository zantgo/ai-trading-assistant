use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use sqlx::SqlitePool;

use crate::analyzer;
use crate::automation;
use crate::config::{AppConfig, IntervalsConfig, SafetyConfig};
use crate::api_failover::ApiFailoverState;
use crate::safety::SafetyManager;
use crate::llm::TokenTracker;
use shared::models::MarketSnapshot;
use shared::normalized::NormalizedCandle;

#[derive(Debug, Clone, PartialEq)]
pub enum InstanceStatus {
    Running,
    Paused,
    Stopped,
}

impl InstanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InstanceStatus::Running => "running",
            InstanceStatus::Paused => "paused",
            InstanceStatus::Stopped => "stopped",
        }
    }
}

pub struct Instance {
    pub id: String,
    pub pair: (String, String),
    pub status: RwLock<InstanceStatus>,
    pub cancel: CancellationToken,

    // Core pipeline (wraps ActivePair)
    pub active_pair: Arc<analyzer::ActivePair>,

    // Automation context
    pub automation_ctx: Arc<RwLock<Option<automation::AutomationContext>>>,

    // API key management
    pub api_key: RwLock<Option<String>>,
    pub api_key_valid: AtomicBool,
    pub api_failover: Arc<ApiFailoverState>,
    pub token_tracker: Arc<std::sync::Mutex<TokenTracker>>,

    // Unified safety subsystem
    pub safety: Arc<SafetyManager>,

    // Capital tracking
    pub initial_capital: RwLock<f64>,
    pub current_equity: RwLock<f64>,

    // Configuration
    pub tp_levels: RwLock<u8>,
    pub sl_levels: RwLock<u8>,
    pub intervals: RwLock<IntervalsConfig>,
    pub safety_config: RwLock<SafetyConfig>,

    // Shared state references
    pub pool: SqlitePool,
    pub config: Arc<RwLock<AppConfig>>,

    // Timeframe history and snapshots (shared from ActivePair pipelines)
    pub mid_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub long_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub macro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub supermacro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,

    pub mid_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub long_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub macro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub supermacro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
}

impl Instance {
    pub fn new(
        id: String,
        pair: (String, String),
        active_pair: Arc<analyzer::ActivePair>,
        pool: SqlitePool,
        config: Arc<RwLock<AppConfig>>,
        inter_config: IntervalsConfig,
        safe_config: SafetyConfig,
        mid_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
        long_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
        macro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
        supermacro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
        mid_latest: Arc<RwLock<Option<MarketSnapshot>>>,
        long_latest: Arc<RwLock<Option<MarketSnapshot>>>,
        macro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
        supermacro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    ) -> Self {
        let safety = Arc::new(SafetyManager::new(
            safe_config.consecutive_loss_caution,
            safe_config.consecutive_loss_dropout,
            safe_config.dropout_duration_hours,
            safe_config.capital_drawdown_pct,
        ));

        Self {
            id,
            pair,
            status: RwLock::new(InstanceStatus::Running),
            cancel: active_pair.cancel.clone(),
            active_pair,
            automation_ctx: Arc::new(RwLock::new(None)),
            api_key: RwLock::new(None),
            api_key_valid: AtomicBool::new(false),
            api_failover: Arc::new(ApiFailoverState::new(None, None, 30, 5, 10)),
            token_tracker: Arc::new(std::sync::Mutex::new(TokenTracker::default())),
            safety,
            initial_capital: RwLock::new(0.0),
            current_equity: RwLock::new(0.0),
            tp_levels: RwLock::new(1),
            sl_levels: RwLock::new(1),
            intervals: RwLock::new(inter_config),
            safety_config: RwLock::new(safe_config),
            pool,
            config,
            mid_history,
            long_history,
            macro_history,
            supermacro_history,
            mid_latest,
            long_latest,
            macro_latest,
            supermacro_latest,
        }
    }

    pub fn symbol(&self) -> String {
        self.active_pair.symbol.clone()
    }

    pub fn pair_key(&self) -> String {
        format!("{}-{}", self.pair.0, self.pair.1)
    }

    pub fn pair_display(&self) -> String {
        format!("{}/{}", self.pair.0, self.pair.1)
    }
}
