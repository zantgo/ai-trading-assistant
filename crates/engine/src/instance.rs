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
    pub fn new(
        intervals: IntervalsConfig,
        operational_mode: crate::config::OperationalMode,
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

    /// Test-only constructor. Builds an `Instance` with empty buffers
    /// and a minimal ActivePair, suitable for unit-testing read paths
    /// without spinning up a real WS pipeline.
    #[doc(hidden)]
    pub fn new_test(id: String, pair: (String, String), micro: TimeframeBuffers) -> Self {
        use crate::analyzer::{ActivePair, TimeframePipeline};
        use shared::models::MarketSnapshot;
        use shared::normalized::{NormalizedCandle, NormalizedEvent};
        use std::collections::VecDeque;
        use std::sync::Arc;
        use tokio::sync::broadcast;
        use tokio::sync::RwLock;
        use tokio_util::sync::CancellationToken;

        let cancel = CancellationToken::new();
        let (bcast_tx, _) = broadcast::channel::<MarketSnapshot>(2);
        let new_pipeline = || TimeframePipeline {
            history: Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::new())),
            broadcast_tx: bcast_tx.clone(),
            latest_snapshot: Arc::new(RwLock::new(None)),
            snapshot_history: Arc::new(RwLock::new(VecDeque::<MarketSnapshot>::new())),
            timeframe_secs: 60,
            timeframe_label: "Micro",
            divergence_detector: Arc::new(tokio::sync::Mutex::new(
                shared::indicators::DivergenceDetector::new(20),
            )),
            sr_tracker: Arc::new(tokio::sync::Mutex::new(
                crate::sr_engine::SrRoleTracker::new(0.003),
            )),
            fibonacci: crate::config::FibonacciConfig::default(),
            latest_oi: Arc::new(RwLock::new(None)),
            latest_funding: Arc::new(RwLock::new(None)),
            latest_mark_px: Arc::new(RwLock::new(None)),
            latest_index_px: Arc::new(RwLock::new(None)),
        };
        let micro_pipe = new_pipeline();
        let fast_pipe = new_pipeline();
        let slow_pipe = new_pipeline();
        let macro_pipe = new_pipeline();
        let internal_symbol = format!("{}-{}", pair.0, pair.1);
        let active_pair = Arc::new(ActivePair {
            symbol: internal_symbol,
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
            cluster_matrix: Arc::new(RwLock::new(None)),
        });
        let empty_buffers = TimeframeBuffers::new();
        let config = Arc::new(RwLock::new(crate::config::AppConfig {
            symbols: vec![],
            candles: crate::config::CandlesConfig {
                duration_seconds: 60,
                analysis_limit: 500,
            },
            indicators: crate::config::IndicatorsConfig {
                ema_fast: 10,
                ema_medium: 50,
                ema_slow: 100,
                ema_long: 200,
                rsi_period: 14,
                macd_fast: 12,
                macd_slow: 26,
                macd_signal: 9,
                adx_period: 14,
                atr_period: 14,
                squeeze_period: 20,
                stoch_k_period: 18,
                stoch_d_period: 5,
                stoch_s_period: 9,
                chandemo_period: 12,
                supertrend_period: 10,
                supertrend_multiplier: 3.0,
                keltner_ema_period: 20,
                keltner_atr_period: 10,
                keltner_multiplier: 2.0,
                donchian_period: 20,
                obv_smoothing: 20,
                cmf_period: 20,
                mfi_period: 14,
                hv_period: 20,
                aroon_period: 25,
                chop_period: 14,
                linreg_period: 20,
                zscore_period: 20,
                bbwp_lookback: 252,
                bbwp_period: 20,
                macd_extreme_high_threshold: 1000.0,
                macd_extreme_low_threshold: -1000.0,
                macd_histogram_contraction_threshold: 0.3,
                adx_trend_threshold: 20,
                adx_exhaustion_threshold: 40,
                adx_slope_lookback: 3,
                squeeze_min_duration: 5,
                squeeze_bb_period: 20,
                squeeze_bb_std_dev: 2.0,
                squeeze_kc_period: 20,
                squeeze_kc_atr_multiplier: 1.5,
                atr_multiplier_coefficient: 2.0,
                atr_target_rr_ratio: 2.5,
                volume_average_period: 20,
                rvol_threshold_institutional: 1.5,
                rvol_threshold_climax: 3.0,
                ichimoku_tenkan: 9,
                ichimoku_kijun: 26,
                ichimoku_senkou_b: 52,
                ichimoku_displacement: 26,
                cci_period: 20,
                psar_af_step: 0.02,
                psar_af_max: 0.2,
                williams_r_period: 14,
                hull_ma_period: 21,
                force_index_smoothing: 13,
                stddev_channel_period: 20,
                smc_lookback: 20,
                volume_profile_bins: 50,
                volume_profile_window: 500,
                volume_profile_value_area: 0.7,
            },
            hyperliquid: crate::config::HyperliquidConfig::default(),
            bitget: crate::config::BitgetConfig::default(),
            fibonacci: crate::config::FibonacciConfig::default(),
            pivots: crate::config::PivotsConfig::default(),
            slow_timeframe: crate::config::SlowTimeframeConfig::default(),
            macro_timeframe: crate::config::SlowTimeframeConfig::default(),
            leverage: crate::config::LeverageConfig::default(),
            scoring: crate::config::ScoringConfig::default(),
            fees: crate::config::FeesConfig::default(),
            defaults: crate::config::DefaultsConfig::default(),
            safety: crate::config::SafetyConfig::default(),
            intervals: crate::config::IntervalsConfig::default(),
            liquidity: crate::config::LiquidityConfig::default(),
            clock_monitor: None,
            instances: std::collections::HashMap::new(),
        }));
        // Use a no-op sqlite pool for tests. We never hit the DB.
        let pool =
            sqlx::SqlitePool::connect_lazy("sqlite::memory:").expect("lazy sqlite memory pool");
        Self {
            id,
            pair,
            cancel,
            trading: RwLock::new(TradingState::default()),
            config_state: RwLock::new(ConfigState::new(
                crate::config::IntervalsConfig::default(),
                crate::config::OperationalMode::ManualOnly,
            )),
            safety_config: crate::config::SafetyConfig::default(),
            safety: Arc::new(SafetyManager::new(3, 5, 8, 30.0)),
            active_pair,
            pool,
            config,
            micro,
            fast: empty_buffers.clone(),
            slow: empty_buffers.clone(),
            r#macro: empty_buffers,
        }
    }
}

impl TimeframeBuffers {
    /// Default constructor for unit tests.
    pub fn new() -> Self {
        use shared::models::MarketSnapshot;
        use shared::normalized::NormalizedCandle;
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
