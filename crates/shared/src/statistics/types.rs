//! Shared types for the Statistical Intelligence Layer.
//!
//! Window sizes and engine configuration. Pure data — no logic.

use serde::{Deserialize, Serialize};

/// Supported rolling-window sizes for multi-horizon analysis.
pub const WINDOW_SIZES: &[usize] = &[20, 50, 100, 250, 500];

/// Configuration for the statistics engine, read from `[statistics]` in
/// `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsConfig {
    /// Master toggle. When `false` the SIL produces `StatisticalContext::default()`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Rolling-window capacities. Defaults to `[20, 50, 100, 250, 500]`.
    #[serde(default = "default_windows")]
    pub windows: Vec<usize>,

    /// Minimum number of observations before a probability is reported.
    #[serde(default = "default_min_obs")]
    pub probability_min_observations: usize,

    /// Number of forward bars used when evaluating probability outcomes.
    #[serde(default = "default_fwd_bars")]
    pub probability_forward_bars: usize,

    // ── Monte Carlo ────────────────────────────────────────────
    #[serde(default)]
    pub monte_carlo_enabled: bool,

    #[serde(default = "default_mc_paths")]
    pub monte_carlo_paths: usize,

    #[serde(default = "default_mc_steps")]
    pub monte_carlo_steps: usize,

    #[serde(default = "default_mc_target_atr")]
    pub monte_carlo_target_atr_mult: f64,

    #[serde(default = "default_mc_stop_atr")]
    pub monte_carlo_stop_atr_mult: f64,

    #[serde(default = "default_mc_interval")]
    pub monte_carlo_interval_seconds: u64,

    // ── Kalman drift estimation ────────────────────────────────
    #[serde(default = "default_true")]
    pub kalman_enabled: bool,

    #[serde(default = "default_kalman_q")]
    pub kalman_process_noise: f64,

    #[serde(default = "default_kalman_r")]
    pub kalman_measurement_noise: f64,

    #[serde(default = "default_kalman_rw")]
    pub kalman_residual_window: usize,

    // ── Online learning ────────────────────────────────────────
    #[serde(default = "default_true")]
    pub online_learning_enabled: bool,

    #[serde(default = "default_top_n")]
    pub feature_importance_top_n: usize,

    #[serde(default = "default_regimes")]
    pub clustering_regimes: usize,

    #[serde(default = "default_anomaly")]
    pub anomaly_threshold: f64,

    // ── Bayesian priors ────────────────────────────────────────
    #[serde(default = "default_one")]
    pub bayesian_prior_alpha: f64,

    #[serde(default = "default_one")]
    pub bayesian_prior_beta: f64,

    // ── Phase 16: Advanced Risk Modeling ──────────────────────
    /// Enable GARCH(1,1) volatility forecasting.
    #[serde(default = "default_true")]
    pub garch_enabled: bool,

    /// Window size for GARCH parameter estimation.
    #[serde(default = "default_garch_window")]
    pub garch_estimation_window: usize,

    /// Enable EVT tail risk modeling.
    #[serde(default = "default_true")]
    pub evt_enabled: bool,

    /// Percentile threshold for POT exceedance selection.
    #[serde(default = "default_evt_pct")]
    pub evt_threshold_percentile: f64,

    /// Enable Information Coefficient tracking.
    #[serde(default = "default_true")]
    pub ic_enabled: bool,

    /// Rolling window size for IC computation.
    #[serde(default = "default_ic_lookback")]
    pub ic_lookback: usize,

    /// Forward bars for IC outcome measurement.
    #[serde(default = "default_ic_forward")]
    pub ic_forward_bars: usize,
}

impl Default for StatisticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            windows: default_windows(),
            probability_min_observations: 30,
            probability_forward_bars: 5,
            monte_carlo_enabled: false,
            monte_carlo_paths: 1000,
            monte_carlo_steps: 50,
            monte_carlo_target_atr_mult: 2.0,
            monte_carlo_stop_atr_mult: 1.5,
            monte_carlo_interval_seconds: 300,
            kalman_enabled: true,
            kalman_process_noise: 0.00001,
            kalman_measurement_noise: 0.001,
            kalman_residual_window: 100,
            online_learning_enabled: true,
            feature_importance_top_n: 5,
            clustering_regimes: 5,
            anomaly_threshold: 0.8,
            bayesian_prior_alpha: 1.0,
            bayesian_prior_beta: 1.0,
            garch_enabled: true,
            garch_estimation_window: 252,
            evt_enabled: true,
            evt_threshold_percentile: 0.95,
            ic_enabled: true,
            ic_lookback: 50,
            ic_forward_bars: 5,
        }
    }
}

// ── Default helpers ────────────────────────────────────────────

fn default_true() -> bool { true }
fn default_one() -> f64 { 1.0 }
fn default_windows() -> Vec<usize> { vec![20, 50, 100, 250, 500] }
fn default_min_obs() -> usize { 30 }
fn default_fwd_bars() -> usize { 5 }
fn default_mc_paths() -> usize { 1000 }
fn default_mc_steps() -> usize { 50 }
fn default_mc_target_atr() -> f64 { 2.0 }
fn default_mc_stop_atr() -> f64 { 1.5 }
fn default_mc_interval() -> u64 { 300 }
fn default_top_n() -> usize { 5 }
fn default_regimes() -> usize { 5 }
fn default_anomaly() -> f64 { 0.8 }
fn default_kalman_q() -> f64 { 0.00001 }
fn default_kalman_r() -> f64 { 0.001 }
fn default_kalman_rw() -> usize { 100 }
fn default_garch_window() -> usize { 252 }
fn default_evt_pct() -> f64 { 0.95 }
fn default_ic_lookback() -> usize { 50 }
fn default_ic_forward() -> usize { 5 }
