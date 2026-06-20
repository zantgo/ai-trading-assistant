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
    pub micro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub short_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub medium_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub large_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,

    pub micro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub short_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub medium_latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub large_latest: Arc<RwLock<Option<MarketSnapshot>>>,

    // In-memory snapshot history (indicator timeseries aligned with candles)
    pub micro_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub short_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub medium_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub large_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
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
        micro_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
        short_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
        medium_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
        large_history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
        micro_latest: Arc<RwLock<Option<MarketSnapshot>>>,
        short_latest: Arc<RwLock<Option<MarketSnapshot>>>,
        medium_latest: Arc<RwLock<Option<MarketSnapshot>>>,
        large_latest: Arc<RwLock<Option<MarketSnapshot>>>,
        micro_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
        short_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
        medium_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
        large_snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
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
            micro_history,
            short_history,
            medium_history,
            large_history,
            micro_latest,
            short_latest,
            medium_latest,
            large_latest,
            micro_snapshot_history,
            short_snapshot_history,
            medium_snapshot_history,
            large_snapshot_history,
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
