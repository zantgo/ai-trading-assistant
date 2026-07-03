use sqlx::SqlitePool;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::analyzer;
use crate::api_failover::ApiFailoverState;
use crate::automation;
use crate::config::{AppConfig, IntervalsConfig, SafetyConfig};
use crate::safety::SafetyManager;
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

#[derive(Debug, Clone)]
pub struct TradingState {
    pub initial_capital: f64,
    pub current_equity: f64,
}

impl Default for TradingState {
    fn default() -> Self {
        Self {
            initial_capital: 0.0,
            current_equity: 0.0,
        }
    }
}

impl TradingState {
    pub fn pnl_pct(&self) -> f64 {
        if self.initial_capital > 0.0 {
            ((self.current_equity - self.initial_capital) / self.initial_capital) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigState {
    pub status: InstanceStatus,
    pub intervals: IntervalsConfig,
    pub operational_mode: crate::config::OperationalMode,
}

impl ConfigState {
    pub fn new(intervals: IntervalsConfig, operational_mode: crate::config::OperationalMode) -> Self {
        Self {
            status: InstanceStatus::Running,
            intervals,
            operational_mode,
        }
    }
}

#[derive(Clone)]
pub struct TimeframeBuffers {
    pub history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub latest: Arc<RwLock<Option<MarketSnapshot>>>,
    pub snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
}

pub struct Instance {
    pub id: String,
    pub pair: (String, String),
    pub cancel: CancellationToken,

    pub trading: RwLock<TradingState>,

    pub config_state: RwLock<ConfigState>,
    pub safety_config: SafetyConfig,

    pub api_key: RwLock<Option<String>>,
    pub api_key_valid: AtomicBool,
    pub api_failover: Arc<ApiFailoverState>,
    pub token_tracker: Arc<crate::llm::TokenTracker>,

    pub safety: Arc<SafetyManager>,

    pub active_pair: Arc<analyzer::ActivePair>,
    pub automation_ctx: Arc<RwLock<Option<automation::AutomationContext>>>,

    pub pool: SqlitePool,
    pub config: Arc<RwLock<AppConfig>>,

    pub micro: TimeframeBuffers,
    pub fast: TimeframeBuffers,
    pub slow: TimeframeBuffers,
    pub r#macro: TimeframeBuffers,
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
        micro: TimeframeBuffers,
        fast: TimeframeBuffers,
        slow: TimeframeBuffers,
        r#macro: TimeframeBuffers,
        operational_mode: crate::config::OperationalMode,
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
            cancel: active_pair.cancel.clone(),
            trading: RwLock::new(TradingState::default()),
            config_state: RwLock::new(ConfigState::new(inter_config, operational_mode)),
            safety_config: safe_config,
            api_key: RwLock::new(None),
            api_key_valid: AtomicBool::new(false),
            api_failover: Arc::new(ApiFailoverState::new(None, None, 30, 5, 10, 300)),
            token_tracker: Arc::new(crate::llm::TokenTracker::default()),
            safety,
            active_pair,
            automation_ctx: Arc::new(RwLock::new(None)),
            pool,
            config,
            micro,
            fast,
            slow,
            r#macro,
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

    pub async fn latest_price(&self) -> Option<f64> {
        self.micro
            .latest
            .read()
            .await
            .as_ref()
            .and_then(|s| s.mid_price.to_string().parse::<f64>().ok())
    }

    pub async fn latest_close_str(&self) -> Option<String> {
        self.micro
            .latest
            .read()
            .await
            .as_ref()
            .and_then(|s| s.close.map(|d| d.to_string()))
    }

    pub async fn status(&self) -> InstanceStatus {
        self.config_state.read().await.status.clone()
    }

    pub async fn set_status(&self, status: InstanceStatus) {
        self.config_state.write().await.status = status;
    }

    pub async fn set_initial_capital(&self, capital: f64) {
        self.trading.write().await.initial_capital = capital;
    }

    pub async fn set_current_equity(&self, equity: f64) {
        self.trading.write().await.current_equity = equity;
    }
}
