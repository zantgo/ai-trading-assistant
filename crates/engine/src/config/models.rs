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
    #[serde(default = "default_cci_period")]
    pub cci_period: usize,
    #[serde(default = "default_psar_af_step")]
    pub psar_af_step: f64,
    #[serde(default = "default_psar_af_max")]
    pub psar_af_max: f64,
    #[serde(default = "default_ichimoku_tenkan")]
    pub ichimoku_tenkan: usize,
    #[serde(default = "default_ichimoku_kijun")]
    pub ichimoku_kijun: usize,
    #[serde(default = "default_ichimoku_senkou_b")]
    pub ichimoku_senkou_b: usize,
    #[serde(default = "default_ichimoku_displacement")]
    pub ichimoku_displacement: usize,
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
    #[serde(default = "default_williams_r_period")]
    pub williams_r_period: usize,
    #[serde(default = "default_hull_ma_period")]
    pub hull_ma_period: usize,
    #[serde(default = "default_stddev_channel_period")]
    pub stddev_channel_period: usize,
    #[serde(default = "default_force_index_smoothing")]
    pub force_index_smoothing: usize,
    #[serde(default = "default_volume_profile_bins")]
    pub volume_profile_bins: usize,
    #[serde(default = "default_volume_profile_window")]
    pub volume_profile_window: usize,
    #[serde(default = "default_volume_profile_value_area")]
    pub volume_profile_value_area: f64,
    #[serde(default = "default_smc_lookback")]
    pub smc_lookback: usize,
}

impl Default for IndicatorsConfig {
    /// Sensible non-zero defaults consistent with the serde `default_*` fns and
    /// the canonical `config.toml`. Kept explicit (rather than derived) so that
    /// `..Default::default()` never yields zero periods — a zero period would
    /// cause divide-by-zero panics in period-normalized indicators (e.g. CCI).
    fn default() -> Self {
        Self {
            // Core periods (required in config.toml — mirror its canonical values).
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
            // Serde-defaulted fields — reuse the single-source default fns.
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
            cci_period: default_cci_period(),
            psar_af_step: default_psar_af_step(),
            psar_af_max: default_psar_af_max(),
            ichimoku_tenkan: default_ichimoku_tenkan(),
            ichimoku_kijun: default_ichimoku_kijun(),
            ichimoku_senkou_b: default_ichimoku_senkou_b(),
            ichimoku_displacement: default_ichimoku_displacement(),
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
            williams_r_period: default_williams_r_period(),
            hull_ma_period: default_hull_ma_period(),
            stddev_channel_period: default_stddev_channel_period(),
            force_index_smoothing: default_force_index_smoothing(),
            volume_profile_bins: default_volume_profile_bins(),
            volume_profile_window: default_volume_profile_window(),
            volume_profile_value_area: default_volume_profile_value_area(),
            smc_lookback: default_smc_lookback(),
        }
    }
}

fn default_bbwp_lookback() -> usize { 252 }
fn default_bbwp_period() -> usize { 20 }
fn default_stoch_k() -> usize { 18 }
fn default_stoch_d() -> usize { 5 }
fn default_stoch_s() -> usize { 9 }
fn default_chandemo() -> usize { 12 }
fn default_supertrend_period() -> usize { 10 }
fn default_supertrend_multiplier() -> f64 { 3.0 }
fn default_keltner_ema() -> usize { 20 }
fn default_keltner_atr() -> usize { 10 }
fn default_keltner_multiplier() -> f64 { 2.0 }
fn default_donchian_period() -> usize { 20 }
fn default_obv_smoothing() -> usize { 20 }
fn default_cmf_period() -> usize { 20 }
fn default_mfi_period() -> usize { 14 }
fn default_hv_period() -> usize { 20 }
fn default_aroon_period() -> usize { 25 }
fn default_chop_period() -> usize { 14 }
fn default_linreg_period() -> usize { 20 }
fn default_zscore_period() -> usize { 20 }
fn default_cci_period() -> usize { 20 }
fn default_psar_af_step() -> f64 { 0.02 }
fn default_psar_af_max() -> f64 { 0.20 }
fn default_ichimoku_tenkan() -> usize { 9 }
fn default_ichimoku_kijun() -> usize { 26 }
fn default_ichimoku_senkou_b() -> usize { 52 }
fn default_ichimoku_displacement() -> usize { 26 }
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
fn default_williams_r_period() -> usize { 14 }
fn default_hull_ma_period() -> usize { 16 }
fn default_stddev_channel_period() -> usize { 20 }
fn default_force_index_smoothing() -> usize { 13 }
fn default_volume_profile_bins() -> usize { 30 }
fn default_volume_profile_window() -> usize { 100 }
fn default_volume_profile_value_area() -> f64 { 0.70 }
fn default_smc_lookback() -> usize { 50 }

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

/// Session-based Pivot Points configuration. V1 supports UTC-daily sessions with
/// the Classic method; `method` is forward-compatible for Fibonacci/Camarilla/
/// Woodie once implemented.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PivotPointsConfig {
    #[serde(default = "default_pivot_points_enabled")]
    pub enabled: bool,
    #[serde(default = "default_pivot_points_method")]
    pub method: String,
    #[serde(default = "default_pivot_points_proximity")]
    pub proximity_threshold_pct: f64,
}

impl Default for PivotPointsConfig {
    fn default() -> Self {
        Self {
            enabled: default_pivot_points_enabled(),
            method: default_pivot_points_method(),
            proximity_threshold_pct: default_pivot_points_proximity(),
        }
    }
}

fn default_pivot_points_enabled() -> bool { true }
fn default_pivot_points_method() -> String { "classic".to_string() }
fn default_pivot_points_proximity() -> f64 { 0.15 }

/// Candlestick pattern-recognition geometric thresholds (fractions).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandlestickConfig {
    #[serde(default = "default_cs_enabled")]
    pub enabled: bool,
    #[serde(default = "default_cs_doji_body_max")]
    pub doji_body_max: f64,
    #[serde(default = "default_cs_long_wick_mult")]
    pub long_wick_body_mult: f64,
    #[serde(default = "default_cs_small_wick_max")]
    pub small_wick_max: f64,
    #[serde(default = "default_cs_marubozu_wick_max")]
    pub marubozu_wick_max: f64,
    #[serde(default = "default_cs_spinning_body_max")]
    pub spinning_body_max: f64,
    #[serde(default = "default_cs_tweezer_eq_tol")]
    pub tweezer_eq_tol: f64,
    #[serde(default = "default_cs_min_confidence")]
    pub min_confidence: f64,
}

impl Default for CandlestickConfig {
    fn default() -> Self {
        Self {
            enabled: default_cs_enabled(),
            doji_body_max: default_cs_doji_body_max(),
            long_wick_body_mult: default_cs_long_wick_mult(),
            small_wick_max: default_cs_small_wick_max(),
            marubozu_wick_max: default_cs_marubozu_wick_max(),
            spinning_body_max: default_cs_spinning_body_max(),
            tweezer_eq_tol: default_cs_tweezer_eq_tol(),
            min_confidence: default_cs_min_confidence(),
        }
    }
}

fn default_cs_enabled() -> bool { true }
fn default_cs_doji_body_max() -> f64 { 0.1 }
fn default_cs_long_wick_mult() -> f64 { 2.0 }
fn default_cs_small_wick_max() -> f64 { 0.15 }
fn default_cs_marubozu_wick_max() -> f64 { 0.05 }
fn default_cs_spinning_body_max() -> f64 { 0.3 }
fn default_cs_tweezer_eq_tol() -> f64 { 0.001 }
fn default_cs_min_confidence() -> f64 { 0.3 }

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
    /// Registry-driven per-indicator weights (default 1.0 each). Keyed by
    /// indicator registry key; overrides the registry `default_weight`.
    #[serde(default)]
    pub indicator_weights: std::collections::HashMap<String, f64>,
    /// Registry-driven per-indicator enable flags. Absent = registry default.
    #[serde(default)]
    pub indicator_enabled: std::collections::HashMap<String, bool>,
    /// Regime-aware weight multipliers: regime label ("TRENDING"|"RANGE"|
    /// "EXPANSION"|"COMPRESSION") → { indicator_key → multiplier }. Absent = 1.0.
    #[serde(default = "default_regime_weight_multipliers")]
    pub regime_weight_multipliers: std::collections::HashMap<String, std::collections::HashMap<String, f64>>,
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
            indicator_weights: std::collections::HashMap::new(),
            indicator_enabled: std::collections::HashMap::new(),
            regime_weight_multipliers: default_regime_weight_multipliers(),
        }
    }
}

/// Sensible default regime-aware weight multipliers. Trending regimes favor
/// trend/breakout indicators; ranging regimes favor mean-reversion oscillators.
fn default_regime_weight_multipliers(
) -> std::collections::HashMap<String, std::collections::HashMap<String, f64>> {
    use std::collections::HashMap;
    let mk = |pairs: &[(&str, f64)]| -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    };
    let mut m = HashMap::new();
    m.insert(
        "TRENDING".to_string(),
        mk(&[
            ("ema_stack", 1.5), ("supertrend", 1.5), ("donchian", 1.4), ("adx", 1.3),
            ("macd", 1.2), ("rsi", 0.7), ("stochastic", 0.6), ("zscore", 0.5),
        ]),
    );
    m.insert(
        "RANGE".to_string(),
        mk(&[
            ("rsi", 1.5), ("stochastic", 1.5), ("zscore", 1.5), ("bollinger", 1.4),
            ("mfi", 1.2), ("supertrend", 0.6), ("donchian", 0.5), ("ema_stack", 0.7),
        ]),
    );
    m.insert(
        "EXPANSION".to_string(),
        mk(&[
            ("supertrend", 1.4), ("donchian", 1.4), ("keltner", 1.3), ("macd", 1.2),
            ("patterns", 1.2),
        ]),
    );
    m.insert(
        "COMPRESSION".to_string(),
        mk(&[
            ("squeeze", 1.5), ("bbwp", 1.3), ("rsi", 1.1), ("stochastic", 1.1),
            ("supertrend", 0.7), ("donchian", 0.7),
        ]),
    );
    m
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

// ─── Position Sizing & Leverage Scaling ────────────────────────

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AllocationCurveModel {
    /// Legacy stepped: <40→base, 40–59→mid, ≥60→max
    #[default]
    Stepped,
    /// Linear interpolation between base and max pct.
    Linear,
    /// Exponential curve concentrating allocation at high scores.
    Exponential,
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

fn default_exponent() -> f64 { 2.0 }

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

// ─── Order Book Depth Analysis ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBookConfig {
    #[serde(default = "default_ob_depth_levels")]
    pub depth_levels: usize,
    #[serde(default = "default_ob_wall_threshold")]
    pub wall_threshold: f64,
    #[serde(default = "default_ob_spread_wide_threshold_pct")]
    pub spread_wide_threshold_pct: f64,
}

impl Default for OrderBookConfig {
    fn default() -> Self {
        Self {
            depth_levels: default_ob_depth_levels(),
            wall_threshold: default_ob_wall_threshold(),
            spread_wide_threshold_pct: default_ob_spread_wide_threshold_pct(),
        }
    }
}

fn default_ob_depth_levels() -> usize { 25 }
fn default_ob_wall_threshold() -> f64 { 0.15 }
fn default_ob_spread_wide_threshold_pct() -> f64 { 1.0 }

// ─────────────────────────── Portfolio Optimization ───────────────────────
// Phase 5: Kelly Criterion sizing + Risk Parity allocation layered on top of
// the IRML static exposure tiers. All fields carry sane defaults so existing
// config.toml files remain valid without a `[portfolio]` section.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioConfig {
    #[serde(default = "default_kelly_fraction")]
    pub kelly_fraction: f64,
    #[serde(default = "default_allocation_method")]
    pub allocation_method: String,
    #[serde(default = "default_min_alloc")]
    pub min_allocation_pct: f64,
    #[serde(default = "default_max_alloc")]
    pub max_allocation_pct: f64,
}

impl Default for PortfolioConfig {
    fn default() -> Self {
        Self {
            kelly_fraction: default_kelly_fraction(),
            allocation_method: default_allocation_method(),
            min_allocation_pct: default_min_alloc(),
            max_allocation_pct: default_max_alloc(),
        }
    }
}

fn default_kelly_fraction() -> f64 { 0.5 }
fn default_allocation_method() -> String { "kelly_risk_parity".to_string() }
fn default_min_alloc() -> f64 { 0.5 }
fn default_max_alloc() -> f64 { 5.0 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "default_max_instances")]
    pub max_instances: usize,
    #[serde(default = "default_default_pair")]
    pub default_pair: String,
}

fn default_max_instances() -> usize { 100 }
fn default_default_pair() -> String { "BTC/USDT".to_string() }

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            max_instances: default_max_instances(),
            default_pair: default_default_pair(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(default)]
    pub user_name: Option<String>,
    #[serde(default)]
    pub wallet_address: Option<String>,
    #[serde(default)]
    pub session_mode: Option<String>,
    #[serde(default)]
    pub session_currency: Option<String>,
    #[serde(default)]
    pub session_exchange: Option<String>,
    #[serde(default)]
    pub initial_capital: Option<f64>,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            user_name: None, wallet_address: None,
            session_mode: None, session_currency: None,
            session_exchange: None, initial_capital: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyConfig {
    #[serde(default = "default_consecutive_loss_caution")]
    pub consecutive_loss_caution: u32,
    #[serde(default = "default_consecutive_loss_dropout")]
    pub consecutive_loss_dropout: u32,
    #[serde(default = "default_consecutive_loss_suspend")]
    pub consecutive_loss_suspend: u32,
    #[serde(default = "default_dropout_duration_hours")]
    pub dropout_duration_hours: u64,
    #[serde(default = "default_capital_drawdown_pct")]
    pub capital_drawdown_pct: f64,
}

fn default_consecutive_loss_caution() -> u32 { 3 }
fn default_consecutive_loss_dropout() -> u32 { 5 }
fn default_consecutive_loss_suspend() -> u32 { 7 }
fn default_dropout_duration_hours() -> u64 { 8 }
fn default_capital_drawdown_pct() -> f64 { 30.0 }

// ─────────────────────────── Institutional Risk Management Layer ───────────
// Configuration surface for the IRML (see docs/institutional-risk-management-layer.md).
// All fields carry sane defaults so existing config.toml files remain valid.

/// Per-category weighting for the Position Risk Profile aggregation (Section 7.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskCategoryWeights {
    #[serde(default = "default_weight_one")]
    pub market: f64,
    #[serde(default = "default_weight_one")]
    pub structural: f64,
    #[serde(default = "default_weight_one")]
    pub momentum: f64,
    #[serde(default = "default_weight_volatility")]
    pub volatility: f64,
    #[serde(default = "default_weight_liquidity")]
    pub liquidity: f64,
    #[serde(default = "default_weight_one")]
    pub behavioral: f64,
}

fn default_weight_one() -> f64 { 1.0 }
fn default_weight_volatility() -> f64 { 1.2 }
fn default_weight_liquidity() -> f64 { 0.8 }

impl Default for RiskCategoryWeights {
    fn default() -> Self {
        Self {
            market: 1.0,
            structural: 1.0,
            momentum: 1.0,
            volatility: 1.2,
            liquidity: 0.8,
            behavioral: 1.0,
        }
    }
}

/// Institutional Risk Management Layer configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskConfig {
    // ── Aggregation ──
    #[serde(default)]
    pub category_weights: RiskCategoryWeights,
    #[serde(default = "default_worst_case_lambda")]
    pub worst_case_lambda: f64,
    #[serde(default = "default_transition_hysteresis")]
    pub transition_hysteresis: f64,

    // ── Adaptive Reward/Risk engine (Section 12) ──
    #[serde(default = "default_rr_prior_wins")]
    pub rr_prior_wins: f64,
    #[serde(default = "default_rr_prior_losses")]
    pub rr_prior_losses: f64,
    #[serde(default = "default_rr_safety_margin")]
    pub rr_safety_margin: f64,
    #[serde(default = "default_rr_block_size")]
    pub rr_block_size: u32,
    #[serde(default = "default_rr_lookback_trades")]
    pub rr_lookback_trades: u32,

    // ── Hard execution constraints (Section 16) ──
    #[serde(default = "default_max_daily_loss_pct")]
    pub max_daily_loss_pct: f64,
    #[serde(default = "default_max_trade_duration_secs")]
    pub max_trade_duration_secs: u64,
    #[serde(default = "default_min_trade_quality")]
    pub min_trade_quality: f64,
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
    #[serde(default = "default_max_volatility_percentile")]
    pub max_volatility_percentile: f64,
}

fn default_worst_case_lambda() -> f64 { 0.5 }
fn default_transition_hysteresis() -> f64 { 0.05 }
fn default_rr_prior_wins() -> f64 { 5.0 }
fn default_rr_prior_losses() -> f64 { 5.0 }
fn default_rr_safety_margin() -> f64 { 1.25 }
fn default_rr_block_size() -> u32 { 10 }
fn default_rr_lookback_trades() -> u32 { 0 }
fn default_max_daily_loss_pct() -> f64 { 5.0 }
fn default_max_trade_duration_secs() -> u64 { 86_400 }
fn default_min_trade_quality() -> f64 { 0.4 }
fn default_min_confidence() -> f64 { 0.5 }
fn default_max_volatility_percentile() -> f64 { 95.0 }

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            category_weights: RiskCategoryWeights::default(),
            worst_case_lambda: default_worst_case_lambda(),
            transition_hysteresis: default_transition_hysteresis(),
            rr_prior_wins: default_rr_prior_wins(),
            rr_prior_losses: default_rr_prior_losses(),
            rr_safety_margin: default_rr_safety_margin(),
            rr_block_size: default_rr_block_size(),
            rr_lookback_trades: default_rr_lookback_trades(),
            max_daily_loss_pct: default_max_daily_loss_pct(),
            max_trade_duration_secs: default_max_trade_duration_secs(),
            min_trade_quality: default_min_trade_quality(),
            min_confidence: default_min_confidence(),
            max_volatility_percentile: default_max_volatility_percentile(),
        }
    }
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            consecutive_loss_caution: default_consecutive_loss_caution(),
            consecutive_loss_dropout: default_consecutive_loss_dropout(),
            consecutive_loss_suspend: default_consecutive_loss_suspend(),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionConfig {
    #[serde(default = "default_exec_mode")]
    pub mode: String,
    #[serde(default = "default_max_slippage_pct")]
    pub max_slippage_pct: f64,
    #[serde(default = "default_order_timeout_secs")]
    pub order_timeout_secs: u64,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_orders_per_sec: u32,
    #[serde(default = "default_max_order_size")]
    pub max_order_size_usd: f64,
    #[serde(default = "default_max_position_value")]
    pub max_position_value_usd: f64,
}

fn default_exec_mode() -> String { "paper".into() }
fn default_max_slippage_pct() -> f64 { 0.5 }
fn default_order_timeout_secs() -> u64 { 30 }
fn default_rate_limit() -> u32 { 5 }
fn default_max_order_size() -> f64 { 50000.0 }
fn default_max_position_value() -> f64 { 200000.0 }

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            mode: default_exec_mode(),
            max_slippage_pct: default_max_slippage_pct(),
            order_timeout_secs: default_order_timeout_secs(),
            rate_limit_orders_per_sec: default_rate_limit(),
            max_order_size_usd: default_max_order_size(),
            max_position_value_usd: default_max_position_value(),
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
