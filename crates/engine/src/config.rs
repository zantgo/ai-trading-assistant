use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    100
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
pub struct MacroTimeframeConfig {
    #[serde(default = "default_enabled_true")]
    pub enabled: bool,
    pub duration_seconds: u64,
    #[serde(default = "default_analysis_limit")]
    pub analysis_limit: usize,
}

fn default_enabled_true() -> bool { true }

impl Default for MacroTimeframeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_seconds: 900,
            analysis_limit: 100,
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
    #[serde(default = "default_mid_allocation_pct")]
    pub mid_allocation_pct: f64,
    #[serde(default = "default_max_allocation_pct")]
    pub max_allocation_pct: f64,
    #[serde(default = "default_base_score_threshold")]
    pub base_score_threshold: u32,
    #[serde(default = "default_mid_score_threshold")]
    pub mid_score_threshold: u32,
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
            mid_allocation_pct: default_mid_allocation_pct(),
            max_allocation_pct: default_max_allocation_pct(),
            base_score_threshold: default_base_score_threshold(),
            mid_score_threshold: default_mid_score_threshold(),
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
fn default_mid_allocation_pct() -> f64 { 2.0 }
fn default_max_allocation_pct() -> f64 { 3.0 }
fn default_base_score_threshold() -> u32 { 40 }
fn default_mid_score_threshold() -> u32 { 60 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeesConfig {
    #[serde(default = "default_maker_fee")]
    pub maker_fee_pct: f64,
    #[serde(default = "default_taker_fee")]
    pub taker_fee_pct: f64,
}

impl Default for FeesConfig {
    fn default() -> Self {
        Self {
            maker_fee_pct: default_maker_fee(),
            taker_fee_pct: default_taker_fee(),
        }
    }
}

fn default_maker_fee() -> f64 { 0.02 }
fn default_taker_fee() -> f64 { 0.06 }

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
                analysis_limit: 100,
            },
            indicators,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairSpecificConfig {
    pub short_term: TimeframeConfig,
    pub mid_term: TimeframeConfig,
    pub long_term: TimeframeConfig,
    #[serde(default)]
    pub macro_term: Option<TimeframeConfig>,
    #[serde(default)]
    pub supermacro_term: Option<TimeframeConfig>,
    #[serde(default)]
    pub automation: AutomationConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub symbols: Vec<String>,
    pub candles: CandlesConfig,
    pub indicators: IndicatorsConfig,
    #[serde(default)]
    pub hyperliquid: HyperliquidConfig,
    #[serde(default)]
    pub fibonacci: FibonacciConfig,
    #[serde(default)]
    pub pivots: PivotsConfig,
    #[serde(default)]
    pub macro_timeframe: MacroTimeframeConfig,
    #[serde(default)]
    pub supermacro_timeframe: MacroTimeframeConfig,
    #[serde(default)]
    pub leverage: LeverageConfig,
    #[serde(default)]
    pub scoring: ScoringConfig,
    #[serde(default)]
    pub fees: FeesConfig,
    #[serde(default)]
    pub costs: CostsConfig,
    #[serde(default, skip_serializing)]
    pub pairs: HashMap<String, PairSpecificConfig>,
}

pub fn load_config() -> AppConfig {
    let config_raw = std::fs::read_to_string("config.toml")
        .expect("❌ Configuration Error: Failed to find \"config.toml\" in workspace root directory");

    toml::from_str(&config_raw)
        .expect("❌ Configuration Error: Failed to parse fields inside config.toml")
}

pub fn load_pairs() -> HashMap<String, PairSpecificConfig> {
    match std::fs::read_to_string("pairs.json") {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

pub fn save_pairs(pairs: &HashMap<String, PairSpecificConfig>) {
    match serde_json::to_string_pretty(pairs) {
        Ok(json_str) => {
            if let Err(e) = std::fs::write("pairs.json", json_str) {
                eprintln!("❌ Config Error: Failed to write pairs.json: {}", e);
            }
        }
        Err(e) => {
            eprintln!("❌ JSON Serialization Error for pairs: {}", e);
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
        assert_eq!(cfg.mid_allocation_pct, 2.0);
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
