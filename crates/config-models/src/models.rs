use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HyperliquidConfig {
    #[serde(default = "default_hyperliquid_ws_url")]
    pub ws_url: String,
}

impl Default for HyperliquidConfig {
    fn default() -> Self {
        Self {
            ws_url: default_hyperliquid_ws_url(),
        }
    }
}

impl HyperliquidConfig {
    pub fn rest_url(&self) -> String {
        self.ws_url
            .replace("wss://", "https://")
            .replace("ws://", "http://")
            .replace("/ws", "/info")
    }
}

fn default_hyperliquid_ws_url() -> String {
    "wss://api.hyperliquid.xyz/ws".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitgetConfig {
    #[serde(default = "default_bitget_ws_url")]
    pub ws_url: String,
}

impl Default for BitgetConfig {
    fn default() -> Self {
        Self {
            ws_url: default_bitget_ws_url(),
        }
    }
}

impl BitgetConfig {
    /// Base path for Bitget V2 mix (perpetual futures) market endpoints.
    pub fn mix_base_url(&self) -> String {
        "https://api.bitget.com/api/v2/mix/market".to_string()
    }

    pub fn rest_url(&self) -> String {
        format!("{}/candles", self.mix_base_url())
    }

    /// Ticker endpoint used to verify a contract symbol exists.
    pub fn ticker_url(&self) -> String {
        format!("{}/ticker", self.mix_base_url())
    }
}

fn default_bitget_ws_url() -> String {
    "wss://ws.bitget.com/v2/ws/public".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandlesConfig {
    #[serde(default = "default_candle_duration")]
    pub duration_seconds: u64,
    #[serde(default = "default_analysis_limit")]
    pub analysis_limit: usize,
}

impl Default for CandlesConfig {
    fn default() -> Self {
        Self {
            duration_seconds: default_candle_duration(),
            analysis_limit: default_analysis_limit(),
        }
    }
}

fn default_candle_duration() -> u64 {
    60
}

fn default_analysis_limit() -> usize {
    500
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IndicatorsConfig {
    pub ema_fast: usize,
    pub ema_medium: usize,
    pub ema_slow: usize,
    pub ema_long: usize,
    pub rsi_period: usize,
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    pub adx_period: usize,
    pub atr_period: usize,
    pub squeeze_period: usize,
    #[serde(default = "default_stoch_k")]
    pub stoch_k_period: usize,
    #[serde(default = "default_stoch_d")]
    pub stoch_d_period: usize,
    #[serde(default = "default_stoch_s")]
    pub stoch_s_period: usize,
    #[serde(default = "default_chandemo")]
    pub chandemo_period: usize,
    #[serde(default = "default_supertrend_period")]
    pub supertrend_period: usize,
    #[serde(default = "default_supertrend_multiplier")]
    pub supertrend_multiplier: f64,
    #[serde(default = "default_keltner_ema")]
    pub keltner_ema_period: usize,
    #[serde(default = "default_keltner_atr")]
    pub keltner_atr_period: usize,
    #[serde(default = "default_keltner_multiplier")]
    pub keltner_multiplier: f64,
    #[serde(default = "default_donchian_period")]
    pub donchian_period: usize,
    #[serde(default = "default_obv_smoothing")]
    pub obv_smoothing: usize,
    #[serde(default = "default_cmf_period")]
    pub cmf_period: usize,
    #[serde(default = "default_mfi_period")]
    pub mfi_period: usize,
    #[serde(default = "default_hv_period")]
    pub hv_period: usize,
    #[serde(default = "default_aroon_period")]
    pub aroon_period: usize,
    #[serde(default = "default_chop_period")]
    pub chop_period: usize,
    #[serde(default = "default_linreg_period")]
    pub linreg_period: usize,
    #[serde(default = "default_zscore_period")]
    pub zscore_period: usize,
    #[serde(default = "default_bbwp_lookback")]
    pub bbwp_lookback: usize,
    #[serde(default = "default_bbwp_period")]
    pub bbwp_period: usize,
    #[serde(default = "default_macd_extreme_high")]
    pub macd_extreme_high_threshold: f64,
    #[serde(default = "default_macd_extreme_low")]
    pub macd_extreme_low_threshold: f64,
    #[serde(default = "default_macd_contraction_threshold")]
    pub macd_histogram_contraction_threshold: f64,
    #[serde(default = "default_adx_trend_threshold")]
    pub adx_trend_threshold: u32,
    #[serde(default = "default_adx_exhaustion_threshold")]
    pub adx_exhaustion_threshold: u32,
    #[serde(default = "default_adx_slope_lookback")]
    pub adx_slope_lookback: usize,
    #[serde(default = "default_squeeze_min_duration")]
    pub squeeze_min_duration: u32,
    #[serde(default = "default_squeeze_bb_period")]
    pub squeeze_bb_period: usize,
    #[serde(default = "default_squeeze_bb_std_dev")]
    pub squeeze_bb_std_dev: f64,
    #[serde(default = "default_squeeze_kc_period")]
    pub squeeze_kc_period: usize,
    #[serde(default = "default_squeeze_kc_atr_multiplier")]
    pub squeeze_kc_atr_multiplier: f64,
    #[serde(default = "default_atr_multiplier")]
    pub atr_multiplier_coefficient: f64,
    #[serde(default = "default_atr_target_rr")]
    pub atr_target_rr_ratio: f64,
    #[serde(default = "default_volume_average_period")]
    pub volume_average_period: usize,
    #[serde(default = "default_rvol_threshold_institutional")]
    pub rvol_threshold_institutional: f64,
    #[serde(default = "default_rvol_threshold_climax")]
    pub rvol_threshold_climax: f64,
    #[serde(default = "default_ichimoku_tenkan")]
    pub ichimoku_tenkan: usize,
    #[serde(default = "default_ichimoku_kijun")]
    pub ichimoku_kijun: usize,
    #[serde(default = "default_ichimoku_senkou_b")]
    pub ichimoku_senkou_b: usize,
    #[serde(default = "default_ichimoku_displacement")]
    pub ichimoku_displacement: usize,
    #[serde(default = "default_cci_period")]
    pub cci_period: usize,
    #[serde(default = "default_psar_af_step")]
    pub psar_af_step: f64,
    #[serde(default = "default_psar_af_max")]
    pub psar_af_max: f64,
    #[serde(default = "default_williams_r_period")]
    pub williams_r_period: usize,
    #[serde(default = "default_hull_ma_period")]
    pub hull_ma_period: usize,
    #[serde(default = "default_force_index_smoothing")]
    pub force_index_smoothing: usize,
    #[serde(default = "default_stddev_channel_period")]
    pub stddev_channel_period: usize,
    #[serde(default = "default_smc_lookback")]
    pub smc_lookback: usize,
    #[serde(default = "default_volume_profile_bins")]
    pub volume_profile_bins: usize,
    #[serde(default = "default_volume_profile_window")]
    pub volume_profile_window: usize,
    #[serde(default = "default_volume_profile_value_area")]
    pub volume_profile_value_area: f64,
}

fn default_bbwp_lookback() -> usize {
    252
}
fn default_bbwp_period() -> usize {
    20
}
fn default_stoch_k() -> usize {
    18
}
fn default_stoch_d() -> usize {
    5
}
fn default_stoch_s() -> usize {
    9
}
fn default_chandemo() -> usize {
    12
}
fn default_supertrend_period() -> usize {
    10
}
fn default_supertrend_multiplier() -> f64 {
    3.0
}
fn default_keltner_ema() -> usize {
    20
}
fn default_keltner_atr() -> usize {
    10
}
fn default_keltner_multiplier() -> f64 {
    2.0
}
fn default_donchian_period() -> usize {
    20
}
fn default_obv_smoothing() -> usize {
    20
}
fn default_cmf_period() -> usize {
    20
}
fn default_mfi_period() -> usize {
    14
}
fn default_hv_period() -> usize {
    20
}
fn default_aroon_period() -> usize {
    25
}
fn default_chop_period() -> usize {
    14
}
fn default_linreg_period() -> usize {
    20
}
fn default_zscore_period() -> usize {
    20
}
fn default_macd_extreme_high() -> f64 {
    1000.0
}
fn default_macd_extreme_low() -> f64 {
    -1000.0
}
fn default_macd_contraction_threshold() -> f64 {
    0.30
}
fn default_adx_trend_threshold() -> u32 {
    20
}
fn default_adx_exhaustion_threshold() -> u32 {
    40
}
fn default_adx_slope_lookback() -> usize {
    3
}
fn default_squeeze_min_duration() -> u32 {
    5
}
fn default_squeeze_bb_period() -> usize {
    20
}
fn default_squeeze_bb_std_dev() -> f64 {
    2.0
}
fn default_squeeze_kc_period() -> usize {
    20
}
fn default_squeeze_kc_atr_multiplier() -> f64 {
    1.5
}
fn default_atr_multiplier() -> f64 {
    2.0
}
fn default_atr_target_rr() -> f64 {
    2.5
}
fn default_volume_average_period() -> usize {
    20
}
fn default_rvol_threshold_institutional() -> f64 {
    1.5
}
fn default_rvol_threshold_climax() -> f64 {
    3.0
}
fn default_ichimoku_tenkan() -> usize {
    9
}
fn default_ichimoku_kijun() -> usize {
    26
}
fn default_ichimoku_senkou_b() -> usize {
    52
}
fn default_ichimoku_displacement() -> usize {
    26
}
fn default_cci_period() -> usize {
    20
}
fn default_psar_af_step() -> f64 {
    0.02
}
fn default_psar_af_max() -> f64 {
    0.2
}
fn default_williams_r_period() -> usize {
    14
}
fn default_hull_ma_period() -> usize {
    21
}
fn default_force_index_smoothing() -> usize {
    13
}
fn default_stddev_channel_period() -> usize {
    20
}
fn default_smc_lookback() -> usize {
    20
}
fn default_volume_profile_bins() -> usize {
    50
}
fn default_volume_profile_window() -> usize {
    500
}
fn default_volume_profile_value_area() -> f64 {
    0.7
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FibonacciConfig {
    #[serde(default = "default_swing_lookback")]
    pub swing_lookback: usize,
    #[serde(default = "default_swing_scan_range")]
    pub swing_scan_range: usize,
    #[serde(default = "default_retracement_coefficients")]
    pub retracement_coefficients: Vec<f64>,
    #[serde(default = "default_extension_coefficients")]
    pub extension_coefficients: Vec<f64>,
}

impl Default for FibonacciConfig {
    fn default() -> Self {
        Self {
            swing_lookback: default_swing_lookback(),
            swing_scan_range: default_swing_scan_range(),
            retracement_coefficients: default_retracement_coefficients(),
            extension_coefficients: default_extension_coefficients(),
        }
    }
}

fn default_swing_lookback() -> usize {
    10
}
fn default_swing_scan_range() -> usize {
    120
}
fn default_retracement_coefficients() -> Vec<f64> {
    vec![0.236, 0.382, 0.500, 0.618, 0.660, 0.786]
}
fn default_extension_coefficients() -> Vec<f64> {
    vec![1.272, 1.618, 2.000, 2.618]
}

/// Order book configuration for depth analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBookConfig {
    #[serde(default = "default_ob_depth_levels")]
    pub depth_levels: usize,
    #[serde(default = "default_ob_imbalance_threshold")]
    pub imbalance_threshold: f64,
    #[serde(default = "default_ob_wall_threshold")]
    pub wall_threshold: f64,
    #[serde(default = "default_ob_spread_warning")]
    pub spread_warning_pct: f64,
    #[serde(default = "default_ob_spread_wide")]
    pub spread_wide_threshold_pct: f64,
}

impl Default for OrderBookConfig {
    fn default() -> Self {
        Self {
            depth_levels: default_ob_depth_levels(),
            imbalance_threshold: default_ob_imbalance_threshold(),
            wall_threshold: default_ob_wall_threshold(),
            spread_warning_pct: default_ob_spread_warning(),
            spread_wide_threshold_pct: default_ob_spread_wide(),
        }
    }
}

fn default_ob_depth_levels() -> usize {
    20
}
fn default_ob_imbalance_threshold() -> f64 {
    0.3
}
fn default_ob_wall_threshold() -> f64 {
    5.0
}
fn default_ob_spread_warning() -> f64 {
    0.1
}
fn default_ob_spread_wide() -> f64 {
    0.05
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PivotsConfig {
    #[serde(default = "default_pivot_strength_n")]
    pub strength_n: usize,
    #[serde(default = "default_scan_range_candles")]
    pub scan_range_candles: usize,
    #[serde(default = "default_sr_proximity_threshold")]
    pub sr_proximity_threshold_pct: f64,
    #[serde(default = "default_sr_flip_tolerance")]
    pub sr_flip_tolerance_pct: f64,
    #[serde(default = "default_pattern_slope_tolerance")]
    pub pattern_slope_tolerance: f64,
}

impl Default for PivotsConfig {
    fn default() -> Self {
        Self {
            strength_n: default_pivot_strength_n(),
            scan_range_candles: default_scan_range_candles(),
            sr_proximity_threshold_pct: default_sr_proximity_threshold(),
            sr_flip_tolerance_pct: default_sr_flip_tolerance(),
            pattern_slope_tolerance: default_pattern_slope_tolerance(),
        }
    }
}

fn default_pivot_strength_n() -> usize {
    10
}
fn default_scan_range_candles() -> usize {
    120
}
fn default_sr_proximity_threshold() -> f64 {
    0.5
}
fn default_sr_flip_tolerance() -> f64 {
    0.3
}
fn default_pattern_slope_tolerance() -> f64 {
    0.2
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlowTimeframeConfig {
    #[serde(default = "default_enabled_true")]
    pub enabled: bool,
    pub duration_seconds: u64,
    #[serde(default = "default_analysis_limit")]
    pub analysis_limit: usize,
}

fn default_enabled_true() -> bool {
    true
}

impl Default for SlowTimeframeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_seconds: 300,
            analysis_limit: default_analysis_limit(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeverageConfig {
    #[serde(default = "default_cross_leverage")]
    pub cross_leverage: u32,
}

impl Default for LeverageConfig {
    fn default() -> Self {
        Self {
            cross_leverage: default_cross_leverage(),
        }
    }
}

fn default_cross_leverage() -> u32 {
    20
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringConfig {
    #[serde(default = "default_base_allocation_pct")]
    pub base_allocation_pct: f64,
    #[serde(default = "default_micro_allocation_pct")]
    pub micro_allocation_pct: f64,
    #[serde(default = "default_max_allocation_pct")]
    pub max_allocation_pct: f64,
    #[serde(default = "default_base_score_threshold")]
    pub base_score_threshold: u32,
    #[serde(default = "default_micro_score_threshold")]
    pub micro_score_threshold: u32,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            base_allocation_pct: default_base_allocation_pct(),
            micro_allocation_pct: default_micro_allocation_pct(),
            max_allocation_pct: default_max_allocation_pct(),
            base_score_threshold: default_base_score_threshold(),
            micro_score_threshold: default_micro_score_threshold(),
        }
    }
}

fn default_base_allocation_pct() -> f64 {
    1.0
}
fn default_micro_allocation_pct() -> f64 {
    2.0
}
fn default_max_allocation_pct() -> f64 {
    3.0
}
fn default_base_score_threshold() -> u32 {
    40
}
fn default_micro_score_threshold() -> u32 {
    60
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeesConfig {
    #[serde(default = "default_maker_fee")]
    pub maker_fee_pct: f64,
    #[serde(default = "default_taker_fee")]
    pub taker_fee_pct: f64,
    #[serde(default = "default_funding_rate_8h")]
    pub funding_rate_8h: f64,
}

impl Default for FeesConfig {
    fn default() -> Self {
        Self {
            maker_fee_pct: default_maker_fee(),
            taker_fee_pct: default_taker_fee(),
            funding_rate_8h: default_funding_rate_8h(),
        }
    }
}

fn default_maker_fee() -> f64 {
    0.02
}
fn default_taker_fee() -> f64 {
    0.06
}
fn default_funding_rate_8h() -> f64 {
    0.01
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_automation_interval")]
    pub interval_seconds: u64,
    #[serde(default)]
    pub use_scoring_allocation: bool,
    #[serde(default = "default_max_opposite_exit_signals")]
    pub max_opposite_exit_signals: usize,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: default_automation_interval(),
            use_scoring_allocation: false,
            max_opposite_exit_signals: default_max_opposite_exit_signals(),
        }
    }
}

fn default_max_opposite_exit_signals() -> usize {
    5
}
fn default_automation_interval() -> u64 {
    900
}

// ─── Operational Mode ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OperationalMode {
    #[default]
    ManualOnly,
    DeterministicHeuristics,
}

impl OperationalMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationalMode::ManualOnly => "ManualOnly",
            OperationalMode::DeterministicHeuristics => "DeterministicHeuristics",
        }
    }
}

// ─── Trigger Configuration ─────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum TriggerMode {
    #[serde(rename = "interval")]
    Interval { seconds: u64 },
    #[serde(rename = "candle_close")]
    CandleClose { timeframe: String, count: u32 },
    #[serde(rename = "event_driven")]
    EventDriven { events: Vec<String> },
}

impl Default for TriggerMode {
    fn default() -> Self {
        TriggerMode::Interval { seconds: 900 }
    }
}

// ─── Position Sizing & Leverage Scaling ────────────────────────

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AllocationCurveModel {
    #[default]
    Stepped,
    Linear,
    Exponential,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllocationCurve {
    #[serde(default)]
    pub model: AllocationCurveModel,
    #[serde(default = "default_base_allocation_pct")]
    pub base_allocation_pct: f64,
    #[serde(default = "default_max_allocation_pct")]
    pub max_allocation_pct: f64,
    #[serde(default = "default_base_score_threshold")]
    pub base_score_threshold: u32,
    #[serde(default = "default_micro_score_threshold")]
    pub micro_score_threshold: u32,
    #[serde(default = "default_exponent")]
    pub exponent: f64,
}

impl Default for AllocationCurve {
    fn default() -> Self {
        Self {
            model: AllocationCurveModel::default(),
            base_allocation_pct: default_base_allocation_pct(),
            max_allocation_pct: default_max_allocation_pct(),
            base_score_threshold: default_base_score_threshold(),
            micro_score_threshold: default_micro_score_threshold(),
            exponent: default_exponent(),
        }
    }
}

fn default_exponent() -> f64 {
    2.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionScalingConfig {
    #[serde(default)]
    pub allocation_curve: AllocationCurve,
    #[serde(default = "default_leverage_mode")]
    pub leverage_mode: String,
    #[serde(default = "default_cross_leverage")]
    pub leverage_cap: u32,
    #[serde(default = "default_target_margin")]
    pub target_margin: f64,
}

impl Default for PositionScalingConfig {
    fn default() -> Self {
        Self {
            allocation_curve: AllocationCurve::default(),
            leverage_mode: default_leverage_mode(),
            leverage_cap: default_cross_leverage(),
            target_margin: default_target_margin(),
        }
    }
}

fn default_leverage_mode() -> String {
    "Fixed".to_string()
}

fn default_target_margin() -> f64 {
    0.02
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeframeConfig {
    pub candles: CandlesConfig,
    #[serde(default)]
    pub indicators: IndicatorsConfig,
}

impl TimeframeConfig {
    pub fn new(duration_seconds: u64, indicators: IndicatorsConfig) -> Self {
        Self {
            candles: CandlesConfig {
                duration_seconds,
                analysis_limit: default_analysis_limit(),
            },
            indicators,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSpecificConfig {
    pub micro_term: TimeframeConfig,
    pub fast_term: TimeframeConfig,
    #[serde(default)]
    pub slow_term: Option<TimeframeConfig>,
    #[serde(default)]
    pub macro_term: Option<TimeframeConfig>,
    #[serde(default)]
    pub automation: AutomationConfig,
    #[serde(default)]
    pub operational_mode: OperationalMode,
    #[serde(default)]
    pub weight_overrides: Option<std::collections::HashMap<String, i32>>,
    #[serde(default)]
    pub position_scaling: Option<PositionScalingConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_default_pair")]
    pub default_pair: String,
}

fn default_default_pair() -> String {
    "BTC/USDT".to_string()
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            default_pair: default_default_pair(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyConfig {
    #[serde(default = "default_consecutive_loss_caution")]
    pub consecutive_loss_caution: u32,
    #[serde(default = "default_consecutive_loss_dropout")]
    pub consecutive_loss_dropout: u32,
    #[serde(default = "default_dropout_duration_hours")]
    pub dropout_duration_hours: u64,
    #[serde(default = "default_capital_drawdown_pct")]
    pub capital_drawdown_pct: f64,
}

fn default_consecutive_loss_caution() -> u32 {
    3
}
fn default_consecutive_loss_dropout() -> u32 {
    5
}
fn default_dropout_duration_hours() -> u64 {
    8
}
fn default_capital_drawdown_pct() -> f64 {
    30.0
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            consecutive_loss_caution: default_consecutive_loss_caution(),
            consecutive_loss_dropout: default_consecutive_loss_dropout(),
            dropout_duration_hours: default_dropout_duration_hours(),
            capital_drawdown_pct: default_capital_drawdown_pct(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntervalsConfig {
    #[serde(default = "default_slow_seconds")]
    pub slow_seconds: u64,
    #[serde(default = "default_normal_seconds")]
    pub normal_seconds: u64,
    #[serde(default = "default_fast_seconds")]
    pub fast_seconds: u64,
}

fn default_slow_seconds() -> u64 {
    3600
}
fn default_normal_seconds() -> u64 {
    900
}
fn default_fast_seconds() -> u64 {
    300
}

/// Liquidity Intelligence configuration.
///
/// Controls derivatives telemetry activation, mark-price polling cadence,
/// liquidation event ingestion, and the assumptions used by the cluster
/// estimator. All fields have defaults; the platform remains fully
/// functional when this section is absent from legacy configs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidityConfig {
    #[serde(default = "default_liquidity_enabled")]
    pub enabled: bool,
    #[serde(default = "default_mark_poll_ms")]
    pub mark_price_poll_ms: u64,
    #[serde(default = "default_funding_refresh_ms")]
    pub funding_refresh_ms: u64,
    #[serde(default = "default_liquidation_retention_days")]
    pub event_retention_days: u32,
    #[serde(default = "default_liquidation_bucket_retention_days")]
    pub bucket_retention_days: u32,
    #[serde(default = "default_cluster_refresh_secs")]
    pub cluster_refresh_secs: u64,
    #[serde(default = "default_maintenance_margin_rate")]
    pub maintenance_margin_rate: f64,
    #[serde(default = "default_cascade_detected_zscore")]
    pub cascade_detected_zscore: f64,
    #[serde(default = "default_cascade_sustained_events")]
    pub cascade_sustained_events: u32,
    #[serde(default = "default_funding_extreme_pct")]
    pub funding_extreme_pct: f64,
    #[serde(default = "default_magnet_activation_distance_pct")]
    pub magnet_activation_distance_pct: f64,
    #[serde(default = "default_liquidity_vacuum_threshold")]
    pub liquidity_vacuum_threshold: f64,
    #[serde(default = "default_oi_funding_divergence_pct")]
    pub oi_funding_divergence_pct: f64,
}

impl Default for LiquidityConfig {
    fn default() -> Self {
        Self {
            enabled: default_liquidity_enabled(),
            mark_price_poll_ms: default_mark_poll_ms(),
            funding_refresh_ms: default_funding_refresh_ms(),
            event_retention_days: default_liquidation_retention_days(),
            bucket_retention_days: default_liquidation_bucket_retention_days(),
            cluster_refresh_secs: default_cluster_refresh_secs(),
            maintenance_margin_rate: default_maintenance_margin_rate(),
            cascade_detected_zscore: default_cascade_detected_zscore(),
            cascade_sustained_events: default_cascade_sustained_events(),
            funding_extreme_pct: default_funding_extreme_pct(),
            magnet_activation_distance_pct: default_magnet_activation_distance_pct(),
            liquidity_vacuum_threshold: default_liquidity_vacuum_threshold(),
            oi_funding_divergence_pct: default_oi_funding_divergence_pct(),
        }
    }
}

fn default_liquidity_enabled() -> bool {
    true
}
fn default_mark_poll_ms() -> u64 {
    60_000
}
fn default_funding_refresh_ms() -> u64 {
    60_000
}
fn default_liquidation_retention_days() -> u32 {
    90
}
fn default_liquidation_bucket_retention_days() -> u32 {
    7
}
fn default_cluster_refresh_secs() -> u64 {
    300
}
fn default_maintenance_margin_rate() -> f64 {
    0.005
}
fn default_cascade_detected_zscore() -> f64 {
    2.5
}
fn default_cascade_sustained_events() -> u32 {
    3
}
fn default_funding_extreme_pct() -> f64 {
    0.0005
}
fn default_magnet_activation_distance_pct() -> f64 {
    0.5
}
fn default_liquidity_vacuum_threshold() -> f64 {
    0.3
}
fn default_oi_funding_divergence_pct() -> f64 {
    2.0
}

impl Default for IntervalsConfig {
    fn default() -> Self {
        Self {
            slow_seconds: default_slow_seconds(),
            normal_seconds: default_normal_seconds(),
            fast_seconds: default_fast_seconds(),
        }
    }
}

// ─── Clock Drift Monitor (NTP-based UTC alignment enforcement) ────

/// Configuration block for the runtime `ClockMonitor`. Maps to the TOML
/// `[clock_monitor]` section (or `null`/missing to disable the monitor).
///
/// All fields are optional and fall back to conservative defaults, so legacy
/// `config.toml` files keep working without modification. When this section is
/// absent the engine simply skips spawning the monitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClockMonitorTomlConfig {
    #[serde(default = "default_clock_monitor_enabled")]
    pub enabled: bool,
    #[serde(default = "default_clock_monitor_servers")]
    pub ntp_servers: Vec<String>,
    #[serde(default = "default_clock_monitor_poll_secs")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_clock_monitor_threshold_micros")]
    pub threshold_micros: i64,
    #[serde(default = "default_clock_monitor_query_timeout_secs")]
    pub query_timeout_secs: u64,
    #[serde(default = "default_clock_monitor_jitter_window")]
    pub jitter_window_size: usize,
    #[serde(default = "default_clock_monitor_breach_action")]
    pub breach_action: ClockMonitorBreachAction,
    #[serde(default = "default_clock_monitor_warn_on_breach")]
    pub warn_on_breach: bool,
}

impl Default for ClockMonitorTomlConfig {
    fn default() -> Self {
        Self {
            enabled: default_clock_monitor_enabled(),
            ntp_servers: default_clock_monitor_servers(),
            poll_interval_secs: default_clock_monitor_poll_secs(),
            threshold_micros: default_clock_monitor_threshold_micros(),
            query_timeout_secs: default_clock_monitor_query_timeout_secs(),
            jitter_window_size: default_clock_monitor_jitter_window(),
            breach_action: default_clock_monitor_breach_action(),
            warn_on_breach: default_clock_monitor_warn_on_breach(),
        }
    }
}

impl ClockMonitorTomlConfig {
    pub fn is_active(&self) -> bool {
        self.enabled && !self.ntp_servers.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockMonitorBreachAction {
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "panic")]
    Panic,
}

fn default_clock_monitor_enabled() -> bool {
    true
}
fn default_clock_monitor_servers() -> Vec<String> {
    vec!["pool.ntp.org".to_string(), "time.aws.com".to_string()]
}
fn default_clock_monitor_poll_secs() -> u64 {
    30
}
fn default_clock_monitor_threshold_micros() -> i64 {
    50
}
fn default_clock_monitor_query_timeout_secs() -> u64 {
    5
}
fn default_clock_monitor_jitter_window() -> usize {
    20
}
fn default_clock_monitor_breach_action() -> ClockMonitorBreachAction {
    ClockMonitorBreachAction::Warn
}
fn default_clock_monitor_warn_on_breach() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scoring_config() {
        let cfg = ScoringConfig::default();
        assert_eq!(cfg.base_allocation_pct, 1.0);
        assert_eq!(cfg.micro_allocation_pct, 2.0);
        assert_eq!(cfg.max_allocation_pct, 3.0);
    }

    #[test]
    fn test_default_fibonacci_config() {
        let cfg = FibonacciConfig::default();
        assert_eq!(cfg.retracement_coefficients.len(), 6);
        assert_eq!(cfg.extension_coefficients.len(), 4);
    }

    #[test]
    fn test_default_pivots_config() {
        let cfg = PivotsConfig::default();
        assert_eq!(cfg.strength_n, 10);
        assert_eq!(cfg.scan_range_candles, 120);
    }

    #[test]
    fn test_default_leverage() {
        let cfg = LeverageConfig::default();
        assert_eq!(cfg.cross_leverage, 20);
    }
}

/// Type alias for the FAST timeframe configuration block. Structurally
/// identical to `SlowTimeframeConfig` (enabled flag + duration + analysis
/// limit) — the two are differentiated only by convention.
pub type FastTimeframeConfig = SlowTimeframeConfig;
