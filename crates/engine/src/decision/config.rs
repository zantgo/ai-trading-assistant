use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DecisionConfig {
    #[serde(default = "default_exit_opposite_threshold")]
    pub exit_opposite_threshold: f64,
    #[serde(default = "default_anomaly_threshold")]
    pub anomaly_threshold: f64,
    #[serde(default = "default_max_signal_age_bars")]
    pub max_signal_age_bars: u32,

    // Composite weights (must sum to ~1.0)
    #[serde(default = "default_w_confluence")]
    pub w_confluence: f64,
    #[serde(default = "default_w_readiness")]
    pub w_readiness: f64,
    #[serde(default = "default_w_quality")]
    pub w_quality: f64,
    #[serde(default = "default_w_safety")]
    pub w_safety: f64,
    #[serde(default = "default_w_trend")]
    pub w_trend: f64,
    #[serde(default = "default_w_regime_conf")]
    pub w_regime_conf: f64,
    #[serde(default = "default_w_breakout")]
    pub w_breakout: f64,

    // Regime multipliers
    #[serde(default = "default_regime_mult_trending")]
    pub regime_mult_trending: f64,
    #[serde(default = "default_regime_mult_expansion")]
    pub regime_mult_expansion: f64,
    #[serde(default = "default_regime_mult_range")]
    pub regime_mult_range: f64,
    #[serde(default = "default_regime_mult_compression")]
    pub regime_mult_compression: f64,
    #[serde(default = "default_regime_mult_transitional")]
    pub regime_mult_transitional: f64,

    // Opening thresholds per regime
    #[serde(default = "default_open_threshold_trending")]
    pub open_threshold_trending: f64,
    #[serde(default = "default_open_threshold_expansion")]
    pub open_threshold_expansion: f64,
    #[serde(default = "default_open_threshold_range")]
    pub open_threshold_range: f64,
    #[serde(default = "default_open_threshold_compression")]
    pub open_threshold_compression: f64,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            exit_opposite_threshold: default_exit_opposite_threshold(),
            anomaly_threshold: default_anomaly_threshold(),
            max_signal_age_bars: default_max_signal_age_bars(),
            w_confluence: default_w_confluence(),
            w_readiness: default_w_readiness(),
            w_quality: default_w_quality(),
            w_safety: default_w_safety(),
            w_trend: default_w_trend(),
            w_regime_conf: default_w_regime_conf(),
            w_breakout: default_w_breakout(),
            regime_mult_trending: default_regime_mult_trending(),
            regime_mult_expansion: default_regime_mult_expansion(),
            regime_mult_range: default_regime_mult_range(),
            regime_mult_compression: default_regime_mult_compression(),
            regime_mult_transitional: default_regime_mult_transitional(),
            open_threshold_trending: default_open_threshold_trending(),
            open_threshold_expansion: default_open_threshold_expansion(),
            open_threshold_range: default_open_threshold_range(),
            open_threshold_compression: default_open_threshold_compression(),
        }
    }
}

fn default_exit_opposite_threshold() -> f64 { 60.0 }
fn default_anomaly_threshold() -> f64 { 0.85 }
fn default_max_signal_age_bars() -> u32 { 5 }

fn default_w_confluence() -> f64 { 0.25 }
fn default_w_readiness() -> f64 { 0.20 }
fn default_w_quality() -> f64 { 0.15 }
fn default_w_safety() -> f64 { 0.15 }
fn default_w_trend() -> f64 { 0.10 }
fn default_w_regime_conf() -> f64 { 0.10 }
fn default_w_breakout() -> f64 { 0.05 }

fn default_regime_mult_trending() -> f64 { 1.0 }
fn default_regime_mult_expansion() -> f64 { 0.9 }
fn default_regime_mult_range() -> f64 { 0.7 }
fn default_regime_mult_compression() -> f64 { 0.5 }
fn default_regime_mult_transitional() -> f64 { 0.0 }

fn default_open_threshold_trending() -> f64 { 0.55 }
fn default_open_threshold_expansion() -> f64 { 0.60 }
fn default_open_threshold_range() -> f64 { 0.70 }
fn default_open_threshold_compression() -> f64 { 999.0 }
