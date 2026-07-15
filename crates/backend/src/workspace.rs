use sqlx::SqlitePool;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::analyzer;
use crate::config::{AppConfig, IntervalsConfig, SafetyConfig};
use crate::safety::SafetyManager;
use shared::models::MarketSnapshot;
use shared::normalized::NormalizedCandle;

#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceStatus {
    Running,
    Paused,
    Stopped,
}

impl WorkspaceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkspaceStatus::Running => "running",
            WorkspaceStatus::Paused => "paused",
            WorkspaceStatus::Stopped => "stopped",
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
    pub status: WorkspaceStatus,
    pub intervals: IntervalsConfig,
    pub operational_mode: crate::config::OperationalMode,
}

impl ConfigState {
    pub fn new(intervals: IntervalsConfig, operational_mode: crate::config::OperationalMode) -> Self {
        Self {
            status: WorkspaceStatus::Running,
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

pub struct Workspace {
    pub id: String,
    pub pair: (String, String),
    pub cancel: CancellationToken,

    pub trading: RwLock<TradingState>,

    pub config_state: RwLock<ConfigState>,
    pub safety_config: SafetyConfig,

    pub safety: Arc<SafetyManager>,

    pub active_pair: Arc<analyzer::ActivePair>,

    pub pool: SqlitePool,
    pub config: Arc<RwLock<AppConfig>>,

    pub micro: TimeframeBuffers,
    pub fast: TimeframeBuffers,
    pub slow: TimeframeBuffers,
    pub r#macro: TimeframeBuffers,
}

impl Workspace {
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
            safety,
            active_pair,
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

    pub async fn status(&self) -> WorkspaceStatus {
        self.config_state.read().await.status.clone()
    }

    pub async fn set_status(&self, status: WorkspaceStatus) {
        self.config_state.write().await.status = status;
    }

    pub async fn set_initial_capital(&self, capital: f64) {
        self.trading.write().await.initial_capital = capital;
    }

    pub async fn set_current_equity(&self, equity: f64) {
        self.trading.write().await.current_equity = equity;
    }
}
