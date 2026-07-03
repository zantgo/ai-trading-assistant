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
pub struct CandlesConfig {
    pub duration_seconds: u64,
    #[serde(default = "default_analysis_limit")]
    pub analysis_limit: usize,
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
}

fn default_bbwp_lookback() -> usize { 252 }
fn default_bbwp_period() -> usize { 20 }
fn default_macd_extreme_high() -> f64 { 1000.0 }
fn default_macd_extreme_low() -> f64 { -1000.0 }
fn default_macd_contraction_threshold() -> f64 { 0.30 }
fn default_adx_trend_threshold() -> u32 { 20 }
fn default_adx_exhaustion_threshold() -> u32 { 40 }
fn default_adx_slope_lookback() -> usize { 3 }
fn default_squeeze_min_duration() -> u32 { 5 }
fn default_squeeze_bb_period() -> usize { 20 }
fn default_squeeze_bb_std_dev() -> f64 { 2.0 }
fn default_squeeze_kc_period() -> usize { 20 }
fn default_squeeze_kc_atr_multiplier() -> f64 { 1.5 }
fn default_atr_multiplier() -> f64 { 2.0 }
fn default_atr_target_rr() -> f64 { 2.5 }
fn default_volume_average_period() -> usize { 20 }
fn default_rvol_threshold_institutional() -> f64 { 1.5 }
fn default_rvol_threshold_climax() -> f64 { 3.0 }

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

fn default_swing_lookback() -> usize { 10 }
fn default_swing_scan_range() -> usize { 120 }
fn default_retracement_coefficients() -> Vec<f64> {
    vec![0.236, 0.382, 0.500, 0.618, 0.660, 0.786]
}
fn default_extension_coefficients() -> Vec<f64> {
    vec![1.272, 1.618, 2.000, 2.618]
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

fn default_pivot_strength_n() -> usize { 10 }
fn default_scan_range_candles() -> usize { 120 }
fn default_sr_proximity_threshold() -> f64 { 0.5 }
fn default_sr_flip_tolerance() -> f64 { 0.3 }
fn default_pattern_slope_tolerance() -> f64 { 0.2 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlowTimeframeConfig {
    #[serde(default = "default_enabled_true")]
    pub enabled: bool,
    pub duration_seconds: u64,
    #[serde(default = "default_analysis_limit")]
    pub analysis_limit: usize,
}

fn default_enabled_true() -> bool { true }

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

fn default_cross_leverage() -> u32 { 20 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringConfig {
    #[serde(default = "default_rsi_weight")]
    pub rsi_weight: i32,
    #[serde(default = "default_rsi_divergence_weight")]
    pub rsi_divergence_weight: i32,
    #[serde(default = "default_macd_weight")]
    pub macd_weight: i32,
    #[serde(default = "default_macd_divergence_weight")]
    pub macd_divergence_weight: i32,
    #[serde(default = "default_support_resistance_weight")]
    pub support_resistance_weight: i32,
    #[serde(default = "default_trend_weight")]
    pub trend_weight: i32,
    #[serde(default = "default_ema200_weight")]
    pub ema200_weight: i32,
    #[serde(default = "default_pattern_weight")]
    pub pattern_weight: i32,
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
            rsi_weight: default_rsi_weight(),
            rsi_divergence_weight: default_rsi_divergence_weight(),
            macd_weight: default_macd_weight(),
            macd_divergence_weight: default_macd_divergence_weight(),
            support_resistance_weight: default_support_resistance_weight(),
            trend_weight: default_trend_weight(),
            ema200_weight: default_ema200_weight(),
            pattern_weight: default_pattern_weight(),
            base_allocation_pct: default_base_allocation_pct(),
            micro_allocation_pct: default_micro_allocation_pct(),
            max_allocation_pct: default_max_allocation_pct(),
            base_score_threshold: default_base_score_threshold(),
            micro_score_threshold: default_micro_score_threshold(),
        }
    }
}

fn default_rsi_weight() -> i32 { 10 }
fn default_rsi_divergence_weight() -> i32 { 20 }
fn default_macd_weight() -> i32 { 10 }
fn default_macd_divergence_weight() -> i32 { 10 }
fn default_support_resistance_weight() -> i32 { 10 }
fn default_trend_weight() -> i32 { 20 }
fn default_ema200_weight() -> i32 { 10 }
fn default_pattern_weight() -> i32 { 10 }
fn default_base_allocation_pct() -> f64 { 1.0 }
fn default_micro_allocation_pct() -> f64 { 2.0 }
fn default_max_allocation_pct() -> f64 { 3.0 }
fn default_base_score_threshold() -> u32 { 40 }
fn default_micro_score_threshold() -> u32 { 60 }

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

fn default_maker_fee() -> f64 { 0.02 }
fn default_taker_fee() -> f64 { 0.06 }
fn default_funding_rate_8h() -> f64 { 0.01 }

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

fn default_max_opposite_exit_signals() -> usize { 5 }
fn default_automation_interval() -> u64 { 900 }

// ─── Operational Mode ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationalMode {
    ManualOnly,
    DeterministicHeuristics,
    HybridAiCopilot,
}

impl Default for OperationalMode {
    fn default() -> Self {
        OperationalMode::HybridAiCopilot
    }
}

impl OperationalMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationalMode::ManualOnly => "ManualOnly",
            OperationalMode::DeterministicHeuristics => "DeterministicHeuristics",
            OperationalMode::HybridAiCopilot => "HybridAiCopilot",
        }
    }
}

// ─── AI Trigger Configuration ──────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum TriggerMode {
    /// Fixed time interval between AI runs (legacy behaviour).
    #[serde(rename = "interval")]
    Interval { seconds: u64 },
    /// Trigger after N closed candles of a specific timeframe.
    #[serde(rename = "candle_close")]
    CandleClose {
        /// Timeframe label: "micro", "fast", "slow", or "macro".
        timeframe: String,
        count: u32,
    },
    /// Trigger only when specific deterministic events fire.
    #[serde(rename = "event_driven")]
    EventDriven {
        events: Vec<String>,
    },
}

impl Default for TriggerMode {
    fn default() -> Self {
        TriggerMode::Interval { seconds: 900 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTriggerConfig {
    #[serde(default)]
    pub trigger: TriggerMode,
}

impl Default for AiTriggerConfig {
    fn default() -> Self {
        Self {
            trigger: TriggerMode::default(),
        }
    }
}

// ─── Position Sizing & Leverage Scaling ────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AllocationCurveModel {
    /// Legacy stepped: <40→base, 40–59→mid, ≥60→max
    Stepped,
    /// Linear interpolation between base and max pct.
    Linear,
    /// Exponential curve concentrating allocation at high scores.
    Exponential { exponent: f64 },
}

impl Default for AllocationCurveModel {
    fn default() -> Self {
        AllocationCurveModel::Stepped
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Default for AllocationCurve {
    fn default() -> Self {
        Self {
            model: AllocationCurveModel::default(),
            base_allocation_pct: default_base_allocation_pct(),
            max_allocation_pct: default_max_allocation_pct(),
            base_score_threshold: default_base_score_threshold(),
            micro_score_threshold: default_micro_score_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionScalingConfig {
    #[serde(default)]
    pub allocation_curve: AllocationCurve,
    #[serde(default = "default_leverage_mode")]
    pub leverage_mode: String,
    #[serde(default = "default_cross_leverage")]
    pub leverage_cap: u32,
    /// Margin target as fraction of capital for volatility-scaled leverage.
    /// Only used when leverage_mode is "VolatilityScaled".
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
    pub ai_trigger: AiTriggerConfig,
    #[serde(default)]
    pub weight_overrides: Option<std::collections::HashMap<String, i32>>,
    #[serde(default)]
    pub position_scaling: Option<PositionScalingConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostsConfig {
    #[serde(default = "default_price_per_1m_input_tokens")]
    pub price_per_1m_input_tokens: f64,
    #[serde(default = "default_price_per_1m_output_tokens")]
    pub price_per_1m_output_tokens: f64,
}

impl Default for CostsConfig {
    fn default() -> Self {
        Self {
            price_per_1m_input_tokens: default_price_per_1m_input_tokens(),
            price_per_1m_output_tokens: default_price_per_1m_output_tokens(),
        }
    }
}

fn default_price_per_1m_input_tokens() -> f64 { 0.27 }
fn default_price_per_1m_output_tokens() -> f64 { 1.10 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "default_max_instances")]
    pub max_instances: usize,
    #[serde(default = "default_default_pair")]
    pub default_pair: String,
    #[serde(default)]
    pub backup_api_key: Option<String>,
}

fn default_max_instances() -> usize { 100 }
fn default_default_pair() -> String { "BTC/USDT".to_string() }

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            max_instances: default_max_instances(),
            default_pair: default_default_pair(),
            backup_api_key: None,
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

fn default_consecutive_loss_caution() -> u32 { 3 }
fn default_consecutive_loss_dropout() -> u32 { 5 }
fn default_dropout_duration_hours() -> u64 { 8 }
fn default_capital_drawdown_pct() -> f64 { 30.0 }

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

fn default_slow_seconds() -> u64 { 3600 }
fn default_normal_seconds() -> u64 { 900 }
fn default_fast_seconds() -> u64 { 300 }

impl Default for IntervalsConfig {
    fn default() -> Self {
        Self {
            slow_seconds: default_slow_seconds(),
            normal_seconds: default_normal_seconds(),
            fast_seconds: default_fast_seconds(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiFailoverConfig {
    #[serde(default = "default_max_retries_per_call")]
    pub max_retries_per_call: u32,
    #[serde(default = "default_retry_delay_seconds")]
    pub retry_delay_seconds: u64,
    #[serde(default = "default_max_consecutive_failures")]
    pub max_consecutive_failures: u32,
}

fn default_max_retries_per_call() -> u32 { 5 }
fn default_retry_delay_seconds() -> u64 { 30 }
fn default_max_consecutive_failures() -> u32 { 10 }

impl Default for ApiFailoverConfig {
    fn default() -> Self {
        Self {
            max_retries_per_call: default_max_retries_per_call(),
            retry_delay_seconds: default_retry_delay_seconds(),
            max_consecutive_failures: default_max_consecutive_failures(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scoring_config() {
        let cfg = ScoringConfig::default();
        assert_eq!(cfg.rsi_weight, 10);
        assert_eq!(cfg.rsi_divergence_weight, 20);
        assert_eq!(cfg.trend_weight, 20);
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
