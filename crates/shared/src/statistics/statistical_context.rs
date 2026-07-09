//! `StatisticalContext` — the output envelope of the Statistical Intelligence
//! Layer.  Attached to every completed `MarketSnapshot` as an optional field.
//!
//! All fields are populated incrementally per-candle by the
//! `StatisticsEngine::advance()` method.  Fields from modules that haven't
//! been implemented yet default to zero / empty / false and are filled in
//! by later phases.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::statistics::statistical_object::StatisticValue;

/// The complete statistical enrichment for a single candle snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalContext {
    // ── Module A: Distribution ─────────────────────────────────
    pub price_stats: StatisticValue,
    pub return_stats: StatisticValue,
    pub atr_stats: StatisticValue,
    pub rsi_stats: StatisticValue,
    pub bbwp_stats: StatisticValue,

    // ── Module D: Market Shape ─────────────────────────────────
    pub skewness: f64,
    pub kurtosis: f64,
    pub entropy: f64,
    pub tail_risk: f64,
    pub distribution_symmetry: f64,
    pub market_shape_label: String,
    pub volatility_percentile: f64,
    pub compression_percentile: f64,

    // ── Module B: Probabilities ────────────────────────────────
    pub trend_continuation_prob: f64,
    pub mean_reversion_prob: f64,
    pub breakout_success_prob: f64,
    pub reversal_prob: f64,
    pub atr_expansion_prob: f64,
    pub squeeze_release_prob: f64,
    pub volatility_expansion_prob: f64,
    pub stop_before_target_prob: f64,
    pub observation_counts: HashMap<String, usize>,

    // ── Module C: Confidence ───────────────────────────────────
    pub prediction_interval_68: (f64, f64),
    pub prediction_interval_95: (f64, f64),
    pub prediction_interval_99: (f64, f64),
    pub bootstrap_confidence_95: (f64, f64),
    pub historical_reliability: f64,
    pub confidence_score: f64,

    // ── Module E: Relationships ────────────────────────────────
    pub feature_agreement: f64,
    pub indicator_redundancy: f64,
    pub consensus_stability: f64,
    pub trend_consistency: f64,
    pub momentum_consistency: f64,

    // ── Module F: Monte Carlo ──────────────────────────────────
    pub mc_target_hit_prob: f64,
    pub mc_stop_hit_prob: f64,
    pub mc_max_drawdown_95: f64,
    pub mc_max_favorable_excursion_95: f64,
    pub mc_expected_movement: f64,
    pub mc_best_case: f64,
    pub mc_worst_case: f64,
    pub mc_median_outcome: f64,
    pub mc_confidence_95_range: (f64, f64),

    // ── Module F.2: Kalman Drift ──────────────────────────────
    pub kalman_drift: f64,
    pub kalman_noise_vol: f64,
    pub kalman_trend_strength: f64,

    // ── ML Layer ───────────────────────────────────────────────
    pub regime_label: String,
    pub regime_stability: f64,
    pub anomaly_score: f64,
    pub top_anomaly_reason: String,
    pub top_predictive_indicators: Vec<(String, f64)>,
    pub bayesian_posteriors: HashMap<String, (f64, f64, f64)>,

    // ── Derived Features ───────────────────────────────────────
    pub market_stretch_score: f64,
    pub trend_reliability: f64,
    pub momentum_stability: f64,
    pub volatility_shock_prob: f64,
    pub compression_probability: f64,
    pub expansion_probability: f64,
    pub breakout_confidence: f64,
    pub trend_confidence: f64,
    pub risk_confidence: f64,
    pub expected_opportunity: f64,
    pub market_predictability: f64,

    // ── Phase 16: Advanced Risk Modeling ──────────────────────
    // VaR/CVaR
    pub var_95: f64,
    pub var_99: f64,
    pub cvar_95: f64,
    pub cvar_99: f64,

    // GARCH
    pub garch_forecast_vol: f64,
    pub garch_long_run_vol: f64,
    pub garch_persistence: f64,

    // EVT
    pub evt_var_99: f64,
    pub evt_expected_shortfall_99: f64,
    pub evt_tail_index: f64,
    pub evt_scale: f64,

    // Information Coefficient
    pub ic_spearman: f64,
    pub ic_rank: f64,
    pub ic_significance: f64,
}

impl Default for StatisticalContext {
    fn default() -> Self {
        Self {
            price_stats: StatisticValue::default(),
            return_stats: StatisticValue::default(),
            atr_stats: StatisticValue::default(),
            rsi_stats: StatisticValue::default(),
            bbwp_stats: StatisticValue::default(),

            skewness: 0.0,
            kurtosis: 0.0,
            entropy: 0.0,
            tail_risk: 0.0,
            distribution_symmetry: 0.0,
            market_shape_label: "unknown".into(),
            volatility_percentile: 0.0,
            compression_percentile: 0.0,

            trend_continuation_prob: 0.0,
            mean_reversion_prob: 0.0,
            breakout_success_prob: 0.0,
            reversal_prob: 0.0,
            atr_expansion_prob: 0.0,
            squeeze_release_prob: 0.0,
            volatility_expansion_prob: 0.0,
            stop_before_target_prob: 0.0,
            observation_counts: HashMap::new(),

            prediction_interval_68: (0.0, 0.0),
            prediction_interval_95: (0.0, 0.0),
            prediction_interval_99: (0.0, 0.0),
            bootstrap_confidence_95: (0.0, 0.0),
            historical_reliability: 0.0,
            confidence_score: 0.0,

            feature_agreement: 0.0,
            indicator_redundancy: 0.0,
            consensus_stability: 0.0,
            trend_consistency: 0.0,
            momentum_consistency: 0.0,

            mc_target_hit_prob: 0.0,
            mc_stop_hit_prob: 0.0,
            mc_max_drawdown_95: 0.0,
            mc_max_favorable_excursion_95: 0.0,
            mc_expected_movement: 0.0,
            mc_best_case: 0.0,
            mc_worst_case: 0.0,
            mc_median_outcome: 0.0,
            mc_confidence_95_range: (0.0, 0.0),

            kalman_drift: 0.0,
            kalman_noise_vol: 0.0,
            kalman_trend_strength: 0.0,

            regime_label: "unknown".into(),
            regime_stability: 0.0,
            anomaly_score: 0.0,
            top_anomaly_reason: String::new(),
            top_predictive_indicators: Vec::new(),
            bayesian_posteriors: HashMap::new(),

            market_stretch_score: 0.0,
            trend_reliability: 0.0,
            momentum_stability: 0.0,
            volatility_shock_prob: 0.0,
            compression_probability: 0.0,
            expansion_probability: 0.0,
            breakout_confidence: 0.0,
            trend_confidence: 0.0,
            risk_confidence: 0.0,
            expected_opportunity: 0.0,
            market_predictability: 0.0,

            var_95: 0.0,
            var_99: 0.0,
            cvar_95: 0.0,
            cvar_99: 0.0,
            garch_forecast_vol: 0.0,
            garch_long_run_vol: 0.0,
            garch_persistence: 0.0,
            evt_var_99: 0.0,
            evt_expected_shortfall_99: 0.0,
            evt_tail_index: 0.0,
            evt_scale: 0.0,
            ic_spearman: 0.0,
            ic_rank: 0.0,
            ic_significance: 0.0,
        }
    }
}
