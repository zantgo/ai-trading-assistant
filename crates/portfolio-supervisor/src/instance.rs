use sqlx::SqlitePool;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::lifecycle::LifecycleManager;
use crate::safety::SafetyManager;
use crate::WorkspaceState;
use config_models::{IntervalsConfig, SafetyConfig};
use core_domain::models::MarketSnapshot;
use core_domain::normalized::NormalizedCandle;
use market_analyzer::analyzer;

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
    pub portfolio_capital: f64,
    pub current_equity: f64,
}

impl Default for TradingState {
    fn default() -> Self {
        Self {
            portfolio_capital: 0.0,
            current_equity: 0.0,
        }
    }
}

impl TradingState {
    pub fn pnl_pct(&self) -> f64 {
        if self.portfolio_capital > 0.0 {
            ((self.current_equity - self.portfolio_capital) / self.portfolio_capital) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigState {
    pub status: InstanceStatus,
    pub intervals: IntervalsConfig,
    pub operational_mode: config_models::OperationalMode,
}

impl ConfigState {
    pub fn new(
        intervals: IntervalsConfig,
        operational_mode: config_models::OperationalMode,
    ) -> Self {
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
    /// Exchange this instance is wired to. Stamped at construction time so
    /// helpers that walk the workspace (e.g.
    /// `sync_exchange_status_active_pairs`) can bucket by exchange without
    /// having to derive it from `pair` or symbol conventions.
    pub exchange: crate::session::ExchangeChoice,
    pub cancel: CancellationToken,

    pub trading: RwLock<TradingState>,

    pub config_state: RwLock<ConfigState>,
    pub safety_config: SafetyConfig,

    pub safety: Arc<SafetyManager>,

    pub active_pair: Arc<analyzer::ActivePair>,

    pub pool: SqlitePool,
    pub workspace: WorkspaceState,

    pub micro: TimeframeBuffers,
    pub fast: TimeframeBuffers,
    pub slow: TimeframeBuffers,
    pub r#macro: TimeframeBuffers,

    pub lifecycle: RwLock<LifecycleManager>,

    /// Per-instance execution mode (Observe / Paper / Live). `Observe`
    /// instances are market-monitoring only: the TAE setup executor never
    /// evaluates or dispatches orders for them. Mirrors the persisted
    /// `InstanceEntry.mode` so the runtime gate needs no config round-trip.
    pub execution_mode: RwLock<config_models::ExecutionMode>,
}

impl Instance {
    pub fn new(
        id: String,
        pair: (String, String),
        exchange: crate::session::ExchangeChoice,
        active_pair: Arc<analyzer::ActivePair>,
        pool: SqlitePool,
        workspace: WorkspaceState,
        inter_config: IntervalsConfig,
        safe_config: SafetyConfig,
        micro: TimeframeBuffers,
        fast: TimeframeBuffers,
        slow: TimeframeBuffers,
        r#macro: TimeframeBuffers,
        operational_mode: config_models::OperationalMode,
    ) -> Self {
        let safety = Arc::new(SafetyManager::new(
            safe_config.consecutive_loss_caution,
            safe_config.consecutive_loss_dropout,
            safe_config.dropout_duration_hours,
            safe_config.drawdown_limit_pct,
            safe_config.max_daily_drawdown_pct,
            safe_config.systemic_risk_threshold,
        ));

        let mut lifecycle_mgr = LifecycleManager::new(None);
        lifecycle_mgr.set_db(id.clone(), Arc::new(pool.clone()));

        Self {
            id,
            pair,
            exchange,
            cancel: active_pair.cancel.clone(),
            trading: RwLock::new(TradingState::default()),
            config_state: RwLock::new(ConfigState::new(inter_config, operational_mode)),
            safety_config: safe_config,
            safety,
            active_pair,
            pool,
            workspace,
            micro,
            fast,
            slow,
            r#macro,
            lifecycle: RwLock::new(lifecycle_mgr),
            execution_mode: RwLock::new(config_models::ExecutionMode::Paper),
        }
    }

    pub async fn execution_mode(&self) -> config_models::ExecutionMode {
        *self.execution_mode.read().await
    }

    pub async fn set_execution_mode(&self, mode: config_models::ExecutionMode) {
        *self.execution_mode.write().await = mode;
    }

    /// v10.1: boot the lifecycle for the instance's execution mode —
    /// paper/live boots PAUSED (close-only, TAE not activated), observe
    /// boots RUNNING (ghost radar). Call after `set_execution_mode`.
    pub async fn boot_lifecycle(&self, mode: config_models::ExecutionMode) {
        let mut lc = LifecycleManager::new_for_mode(None, Some(mode));
        lc.set_db(self.id.clone(), Arc::new(self.pool.clone()));
        *self.lifecycle.write().await = lc;
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

    pub async fn set_portfolio_capital(&self, capital: f64) {
        self.trading.write().await.portfolio_capital = capital;
        self.safety
            .set_portfolio_capital(
                rust_decimal::Decimal::from_f64_retain(capital).unwrap_or_default(),
            )
            .await;
    }

    pub async fn set_current_equity(&self, equity: f64) {
        self.trading.write().await.current_equity = equity;
        self.safety
            .set_current_equity(rust_decimal::Decimal::from_f64_retain(equity).unwrap_or_default())
            .await;
    }

    /// Test-only constructor. Builds an `Instance` with empty buffers
    /// and a minimal ActivePair, suitable for unit-testing read paths
    /// without spinning up a real WS pipeline.
    #[doc(hidden)]
    pub fn new_test(id: String, pair: (String, String), micro: TimeframeBuffers) -> Self {
        use core_domain::models::MarketSnapshot;
        use core_domain::normalized::{NormalizedCandle, NormalizedEvent};
        use market_analyzer::analyzer::{ActivePair, TimeframePipeline};
        use std::collections::VecDeque;
        use std::sync::Arc;
        use tokio::sync::broadcast;
        use tokio::sync::RwLock;
        use tokio_util::sync::CancellationToken;

        let cancel = CancellationToken::new();
        let (bcast_tx, _) = broadcast::channel::<MarketSnapshot>(2);
        let new_pipeline = |slot: core_domain::models::TimeframeSlot| TimeframePipeline {
            slot,
            history: Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::new())),
            broadcast_tx: bcast_tx.clone(),
            latest_snapshot: Arc::new(RwLock::new(None)),
            snapshot_history: Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::new())),
            timeframe_secs: 60,
            timeframe_label: "Micro",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(
                market_analyzer::indicators::DivergenceDetector::new(20),
            )),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(
                market_analyzer::sr_engine::SrRoleTracker::new(0.003),
            )),
            fibonacci: config_models::FibonacciConfig::default(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
            active_set: Default::default(),
            // Per-TF cluster-matrix handle (Phase 2). Empty by default;
            // tests don't exercise cluster refresh so leaving this as
            // None is fine.
            cluster_matrix: Arc::new(RwLock::new(None)),
            // Per-TF cluster-refresh status snapshot (sibling to
            // `cluster_matrix`). Tests don't exercise refresh, so we
            // initialize as Pending with empty fields.
            cluster_status: Arc::new(RwLock::new(
                core_domain::liquidity::ClusterStatusSnapshot::pending(
                    &format!("{}-{}", pair.0, pair.1),
                    &slot.as_str(),
                ),
            )),
            pipeline_state: Arc::new(RwLock::new(
                core_domain::models::CandlePipelineState::Initializing,
            )),
            indicator_lifecycle: Arc::new(RwLock::new(std::collections::HashMap::new())),
            advisory: Arc::new(RwLock::new(None)),
            tf_leverage_config: Arc::new(config_models::TfLeverageConfig::default()),
            buffer_size: 500,
            stale_threshold_secs: 300,
        };
        let micro_pipe = new_pipeline(core_domain::models::TimeframeSlot::Micro);
        let fast_pipe = new_pipeline(core_domain::models::TimeframeSlot::Fast);
        let slow_pipe = new_pipeline(core_domain::models::TimeframeSlot::Slow);
        let macro_pipe = new_pipeline(core_domain::models::TimeframeSlot::Macro);
        let internal_symbol = format!("{}-{}", pair.0, pair.1);
        let active_pair = Arc::new(ActivePair {
            symbol: internal_symbol,
            custom_pipelines: std::collections::HashMap::new(),
            micro: micro_pipe,
            fast: fast_pipe,
            slow: slow_pipe,
            r#macro: macro_pipe,
            snapshot_tx: tokio::sync::mpsc::channel::<NormalizedEvent>(8).0,
            cancel: cancel.clone(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
            oi_history: Arc::new(RwLock::new(VecDeque::with_capacity(60))), // AUDIT-AIU-051: (timestamp_secs, value)
            funding_history: Arc::new(RwLock::new(VecDeque::with_capacity(8))),
            latency_tracker: Arc::new(core_domain::LatencyTracker::default()),
        });
        let empty_buffers = TimeframeBuffers::new();
        let workspace = WorkspaceState::empty();
        // Use a no-op sqlite pool for tests. We never hit the DB.
        let pool =
            sqlx::SqlitePool::connect_lazy("sqlite::memory:").expect("lazy sqlite memory pool");

        Self {
            id,
            pair,
            exchange: crate::session::ExchangeChoice::Hyperliquid,
            cancel,
            trading: RwLock::new(TradingState::default()),
            config_state: RwLock::new(ConfigState::new(
                config_models::IntervalsConfig::default(),
                config_models::OperationalMode::Advisory,
            )),
            safety_config: config_models::SafetyConfig::default(),
            safety: Arc::new(SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0)),
            active_pair,
            pool,
            workspace,
            micro,
            fast: empty_buffers.clone(),
            slow: empty_buffers.clone(),
            r#macro: empty_buffers,
            lifecycle: RwLock::new(LifecycleManager::new(None)),
            execution_mode: RwLock::new(config_models::ExecutionMode::Paper),
        }
    }
}

impl TimeframeBuffers {
    /// Default constructor for unit tests.
    pub fn new() -> Self {
        use core_domain::models::MarketSnapshot;
        use core_domain::normalized::NormalizedCandle;
        use std::collections::VecDeque;
        use std::sync::Arc;
        use tokio::sync::RwLock;
        Self {
            history: Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::new())),
            latest: Arc::new(RwLock::new(None::<MarketSnapshot>)),
            snapshot_history: Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::new())),
        }
    }
}

impl Default for TimeframeBuffers {
    fn default() -> Self {
        Self::new()
    }
}
