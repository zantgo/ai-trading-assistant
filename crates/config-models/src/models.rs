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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorsConfig {
    #[serde(default = "default_ema_fast")]
    pub ema_fast: usize,
    #[serde(default = "default_ema_medium")]
    pub ema_medium: usize,
    #[serde(default = "default_ema_slow")]
    pub ema_slow: usize,
    #[serde(default = "default_ema_long")]
    pub ema_long: usize,
    #[serde(default = "default_rsi_period")]
    pub rsi_period: usize,
    #[serde(default = "default_macd_fast")]
    pub macd_fast: usize,
    #[serde(default = "default_macd_slow")]
    pub macd_slow: usize,
    #[serde(default = "default_macd_signal")]
    pub macd_signal: usize,
    #[serde(default = "default_adx_period")]
    pub adx_period: usize,
    #[serde(default = "default_atr_period")]
    pub atr_period: usize,
    #[serde(default = "default_squeeze_period")]
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

impl Default for IndicatorsConfig {
    fn default() -> Self {
        Self {
            ema_fast: default_ema_fast(),
            ema_medium: default_ema_medium(),
            ema_slow: default_ema_slow(),
            ema_long: default_ema_long(),
            rsi_period: default_rsi_period(),
            macd_fast: default_macd_fast(),
            macd_slow: default_macd_slow(),
            macd_signal: default_macd_signal(),
            adx_period: default_adx_period(),
            atr_period: default_atr_period(),
            squeeze_period: default_squeeze_period(),
            stoch_k_period: default_stoch_k(),
            stoch_d_period: default_stoch_d(),
            stoch_s_period: default_stoch_s(),
            chandemo_period: default_chandemo(),
            supertrend_period: default_supertrend_period(),
            supertrend_multiplier: default_supertrend_multiplier(),
            keltner_ema_period: default_keltner_ema(),
            keltner_atr_period: default_keltner_atr(),
            keltner_multiplier: default_keltner_multiplier(),
            donchian_period: default_donchian_period(),
            obv_smoothing: default_obv_smoothing(),
            cmf_period: default_cmf_period(),
            mfi_period: default_mfi_period(),
            hv_period: default_hv_period(),
            aroon_period: default_aroon_period(),
            chop_period: default_chop_period(),
            linreg_period: default_linreg_period(),
            zscore_period: default_zscore_period(),
            bbwp_lookback: default_bbwp_lookback(),
            bbwp_period: default_bbwp_period(),
            macd_extreme_high_threshold: default_macd_extreme_high(),
            macd_extreme_low_threshold: default_macd_extreme_low(),
            macd_histogram_contraction_threshold: default_macd_contraction_threshold(),
            adx_trend_threshold: default_adx_trend_threshold(),
            adx_exhaustion_threshold: default_adx_exhaustion_threshold(),
            adx_slope_lookback: default_adx_slope_lookback(),
            squeeze_min_duration: default_squeeze_min_duration(),
            squeeze_bb_period: default_squeeze_bb_period(),
            squeeze_bb_std_dev: default_squeeze_bb_std_dev(),
            squeeze_kc_period: default_squeeze_kc_period(),
            squeeze_kc_atr_multiplier: default_squeeze_kc_atr_multiplier(),
            atr_multiplier_coefficient: default_atr_multiplier(),
            atr_target_rr_ratio: default_atr_target_rr(),
            volume_average_period: default_volume_average_period(),
            rvol_threshold_institutional: default_rvol_threshold_institutional(),
            rvol_threshold_climax: default_rvol_threshold_climax(),
            ichimoku_tenkan: default_ichimoku_tenkan(),
            ichimoku_kijun: default_ichimoku_kijun(),
            ichimoku_senkou_b: default_ichimoku_senkou_b(),
            ichimoku_displacement: default_ichimoku_displacement(),
            cci_period: default_cci_period(),
            psar_af_step: default_psar_af_step(),
            psar_af_max: default_psar_af_max(),
            williams_r_period: default_williams_r_period(),
            hull_ma_period: default_hull_ma_period(),
            force_index_smoothing: default_force_index_smoothing(),
            stddev_channel_period: default_stddev_channel_period(),
            smc_lookback: default_smc_lookback(),
            volume_profile_bins: default_volume_profile_bins(),
            volume_profile_window: default_volume_profile_window(),
            volume_profile_value_area: default_volume_profile_value_area(),
        }
    }
}

fn default_ema_fast() -> usize {
    10
}
fn default_ema_medium() -> usize {
    50
}
fn default_ema_slow() -> usize {
    100
}
fn default_ema_long() -> usize {
    200
}
fn default_rsi_period() -> usize {
    14
}
fn default_macd_fast() -> usize {
    12
}
fn default_macd_slow() -> usize {
    26
}
fn default_macd_signal() -> usize {
    9
}
fn default_adx_period() -> usize {
    14
}
fn default_atr_period() -> usize {
    14
}
fn default_squeeze_period() -> usize {
    20
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

/// Each instance runs in exactly one mode:
///
/// - **Advisory**: market monitor only — indicators, signals, snapshots are
///   computed and broadcast, but no trade orders are ever submitted. This is
///   the default and the safest mode for observation.
/// - **PaperTrading**: the paper trading engine executes simulated orders on
///   the internal matching engine. Portfolio, risk, and performance analytics
///   are updated as if real trades occurred.
/// - **LiveTrading**: the live exchange adapter (Hyperliquid or Bitget) submits
///   real orders. This mode is **not yet implemented** — enabling it currently
///   panics at the execution boundary.
///
/// PaperTrading and LiveTrading follow the **same code path** — the
/// execution layer is strategy-identical. Toggling between them changes only
/// the order-routing backend, ensuring a strategy that works in paper mode
/// works identically in live mode (when implemented).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalMode {
    #[default]
    Advisory,
    PaperTrading,
    LiveTrading,
}

impl OperationalMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationalMode::Advisory => "advisory",
            OperationalMode::PaperTrading => "paper_trading",
            OperationalMode::LiveTrading => "live_trading",
        }
    }

    /// True when this mode permits the execution layer to submit orders
    /// (either simulated or real).
    pub fn is_trading(&self) -> bool {
        matches!(self, OperationalMode::PaperTrading | OperationalMode::LiveTrading)
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
    #[serde(default = "default_drawdown_limit_pct")]
    pub drawdown_limit_pct: f64,
    #[serde(default = "default_max_daily_drawdown_pct")]
    pub max_daily_drawdown_pct: f64,
    #[serde(default = "default_systemic_risk_threshold")]
    pub systemic_risk_threshold: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_reset_cron: Option<String>,
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
fn default_drawdown_limit_pct() -> f64 {
    30.0
}
fn default_max_daily_drawdown_pct() -> f64 {
    5.0
}
fn default_systemic_risk_threshold() -> f64 {
    80.0
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            consecutive_loss_caution: default_consecutive_loss_caution(),
            consecutive_loss_dropout: default_consecutive_loss_dropout(),
            dropout_duration_hours: default_dropout_duration_hours(),
            drawdown_limit_pct: default_drawdown_limit_pct(),
            max_daily_drawdown_pct: default_max_daily_drawdown_pct(),
            systemic_risk_threshold: default_systemic_risk_threshold(),
            session_reset_cron: None,
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

/// Configurable activation: per-indicator, per-signal, and per-SignalKind
/// denylists. Omitting this section defaults to all-enabled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivationConfig {
    #[serde(default)]
    pub disabled_indicators: Vec<String>,
    #[serde(default)]
    pub disabled_signals: Vec<String>,
    #[serde(default)]
    pub disabled_signal_kinds: Vec<String>,
    #[serde(default = "default_true_bool")]
    pub liquidation_feed: bool,
    #[serde(default = "default_true_bool")]
    pub cluster_estimation: bool,
    #[serde(default = "default_true_bool")]
    pub liquidity_signals_enabled: bool,
}

impl Default for ActivationConfig {
    fn default() -> Self {
        Self {
            disabled_indicators: Vec::new(),
            disabled_signals: Vec::new(),
            disabled_signal_kinds: Vec::new(),
            liquidation_feed: true,
            cluster_estimation: true,
            liquidity_signals_enabled: true,
        }
    }
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
    #[serde(default = "default_true_bool")]
    pub liquidation_feed: bool,
    #[serde(default = "default_true_bool")]
    pub cluster_estimation: bool,
    #[serde(default = "default_true_bool")]
    pub signals: bool,
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
            liquidation_feed: true,
            cluster_estimation: true,
            signals: true,
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

// ─── Data Quality Configuration (DIE L3 median filter + outlier rejection) ────

/// Configuration block for the DIE L3 Data Quality Layer's median price filter
/// and outlier rejection. Maps to the TOML `[quality]` section.
///
/// All fields are optional and fall back to the spec-defined defaults.
/// When this section is absent, the median filter is disabled.
///
/// See `docs/engines/data-infrastructure-engine/03-01-04-die-layer3-data-quality.md` §4.1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityConfig {
    /// Size of the rolling window for the median price filter. Ticks are
    /// accepted unfiltered (warm-up) until this many ticks have been observed.
    #[serde(default = "default_median_window_size")]
    pub median_window_size: usize,

    /// Maximum allowed deviation from the rolling median (as a decimal fraction).
    /// A tick is rejected when `|p − median| / median > outlier_tolerance`.
    /// Default: 0.05 (5% deviation).
    #[serde(default = "default_outlier_tolerance")]
    pub outlier_tolerance: f64,

    /// When true, bypass the filter for a tick whose rolling median is exactly
    /// zero (rare venue reset edge case). Logged at debug level.
    #[serde(default = "default_bypass_on_zero_median")]
    pub bypass_on_zero_median: bool,

    /// Maximum age of the last trade (in seconds) before a completed candle is
    /// considered stale. A candle whose last observed trade occurred more than
    /// this many seconds before the candle close is flagged with `is_stale = true`.
    /// Default: 600 (10 minutes).
    #[serde(default = "default_staleness_threshold_secs")]
    pub staleness_threshold_secs: u64,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            median_window_size: default_median_window_size(),
            outlier_tolerance: default_outlier_tolerance(),
            bypass_on_zero_median: default_bypass_on_zero_median(),
            staleness_threshold_secs: default_staleness_threshold_secs(),
        }
    }
}

fn default_median_window_size() -> usize {
    20
}

fn default_outlier_tolerance() -> f64 {
    0.05
}

fn default_bypass_on_zero_median() -> bool {
    true
}

fn default_staleness_threshold_secs() -> u64 {
    600
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
    5000
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReconnectConfig {
    #[serde(default = "default_reconnect_initial_ms")]
    pub initial_backoff_ms: u64,
    #[serde(default = "default_reconnect_max_ms")]
    pub max_backoff_ms: u64,
    #[serde(default = "default_reconnect_jitter")]
    pub jitter_pct: f64,
}

fn default_reconnect_initial_ms() -> u64 { 1000 }
fn default_reconnect_max_ms() -> u64 { 30000 }
fn default_reconnect_jitter() -> f64 { 0.2 }

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
            jitter_pct: 0.2,
        }
    }
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

// ─── TAE: Lifecycle State ─────────────────────────────────────────

/// Per-instance lifecycle state. Four live values per
/// `03-03-06-tae-instance-lifecycle-spec.md §IL-01`.
///
/// Scoped-enum rule: `instance PAUSED` (lifecycle), not to be confused with
/// `AUTO_PAUSED` (policy) or `SUSPENDED` (safety). The serde names carry the
/// `lifecycle_` prefix to make the axis explicit in persisted TOML/JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Running,
    #[serde(rename = "lifecycle_paused")]
    LifecyclePaused,
    Stopping,
    Stopped,
}

impl Default for LifecycleState {
    fn default() -> Self {
        LifecycleState::Stopped
    }
}

impl LifecycleState {
    pub fn as_str(&self) -> &'static str {
        match self {
            LifecycleState::Running => "RUNNING",
            LifecycleState::LifecyclePaused => "PAUSED",
            LifecycleState::Stopping => "STOPPING",
            LifecycleState::Stopped => "STOPPED",
        }
    }
}

// ─── TAE: Per-Symbol Execution Stance ────────────────────────────
//
// Controls per-symbol execution authorization: whether a symbol may accept
// new entries, only close existing positions, or no orders at all.
// Managed by the PME Veto and the operator via REST API.
//
// This is the PME/TAE execution-authorization enum — NOT the L6 Decision
// Matrix `MarketStance` (environmental aggressiveness assessment).
// The only shared variant is `Avoid` (both AGGRESSIVE/CAUTIOUS/NON_AVOID
// are exclusive to MarketStance; CLOSE_ONLY is exclusive to this enum).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Stance {
    Active,
    CloseOnly,
    Avoid,
}

impl Default for Stance {
    fn default() -> Self {
        Stance::Active
    }
}

impl Stance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Stance::Active => "ACTIVE",
            Stance::CloseOnly => "CLOSE_ONLY",
            Stance::Avoid => "AVOID",
        }
    }
}

// ─── TAE: Trade Direction ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Long,
    Short,
}

impl Direction {
    pub fn sign(&self) -> rust_decimal::Decimal {
        use rust_decimal::Decimal;
        match self {
            Direction::Long => Decimal::ONE,
            Direction::Short => Decimal::NEGATIVE_ONE,
        }
    }
}

// ─── TAE: Execution Policy Conditions ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum ConditionGroup {
    #[serde(rename = "AND")]
    And(Vec<Condition>),
    #[serde(rename = "OR")]
    Or(Vec<Condition>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: Operator,
    pub value: ConditionValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Operator {
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
    In,
    Between,
    NotEq,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionValue {
    Number(f64),
    String(String),
    NumberList(Vec<f64>),
    StringList(Vec<String>),
}

// ─── TAE: Risk Parameters ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskParams {
    #[serde(default = "default_risk_per_trade_pct")]
    pub risk_per_trade_pct: f64,
    #[serde(default)]
    pub max_position_size_usd: Option<f64>,
    #[serde(default = "default_max_leverage")]
    pub max_leverage: u32,
    #[serde(default = "default_true_bool")]
    pub use_dynamic_stops: bool,
    #[serde(default)]
    pub fixed_stop_loss_pct: Option<f64>,
    #[serde(default = "default_target_rr_ratio")]
    pub target_rr_ratio: f64,
}

impl Default for RiskParams {
    fn default() -> Self {
        Self {
            risk_per_trade_pct: default_risk_per_trade_pct(),
            max_position_size_usd: None,
            max_leverage: default_max_leverage(),
            use_dynamic_stops: default_true_bool(),
            fixed_stop_loss_pct: None,
            target_rr_ratio: default_target_rr_ratio(),
        }
    }
}

fn default_risk_per_trade_pct() -> f64 {
    1.0
}

fn default_max_leverage() -> u32 {
    20
}

fn default_target_rr_ratio() -> f64 {
    2.5
}

fn default_true_bool() -> bool {
    true
}

// ─── TAE: Execution Policy ────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    pub policy_id: String,
    pub policy_name: String,
    #[serde(default)]
    pub description: String,
    pub symbol: String,
    pub direction: Direction,
    pub conditions: ConditionGroup,
    #[serde(default)]
    pub trigger_mode: TriggerMode,
    pub risk: RiskParams,
    #[serde(default = "default_true_bool")]
    pub enabled: bool,
    #[serde(default)]
    pub cooldown_seconds: u64,
    #[serde(default = "default_true_bool")]
    pub reduce_only_on_close_only: bool,
}

// ─── TAE: Order Types ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Market,
    Limit,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    PreDispatch,
    Pending,
    Submitted,
    Open,
    PartiallyFilled,
    Closed,
    Cancelled,
    Rejected,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::PreDispatch => "PRE_DISPATCH",
            OrderStatus::Pending => "PENDING",
            OrderStatus::Submitted => "SUBMITTED",
            OrderStatus::Open => "OPEN",
            OrderStatus::PartiallyFilled => "PARTIALLY_FILLED",
            OrderStatus::Closed => "CLOSED",
            OrderStatus::Cancelled => "CANCELLED",
            OrderStatus::Rejected => "REJECTED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderPacket {
    pub client_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<rust_decimal::Decimal>,
    pub size: rust_decimal::Decimal,
    pub reduce_only: bool,
    pub is_emergency_liquidation: bool,
    pub associated_position_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionMatrixRow {
    pub order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub order_type: String,
    pub direction: String,
    pub price: Option<rust_decimal::Decimal>,
    pub trigger_price: Option<rust_decimal::Decimal>,
    pub size: rust_decimal::Decimal,
    pub filled_size: rust_decimal::Decimal,
    pub status: String,
    pub is_reduce_only: bool,
    pub is_emergency_liquidation: bool,
    pub associated_position_id: Option<i64>,
    pub created_at: u64,
    pub updated_at: u64,
    pub slippage_bps: Option<f64>,
}

// ─── TAE: Execution Config ────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionConfig {
    #[serde(default = "default_slippage_ceiling_pct")]
    pub slippage_ceiling_pct: f64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            slippage_ceiling_pct: default_slippage_ceiling_pct(),
        }
    }
}

fn default_slippage_ceiling_pct() -> f64 {
    0.5
}
