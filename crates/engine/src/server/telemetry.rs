use crate::server::types::IndicatorSnapshot;
use serde::Serialize;
use shared::statistics::statistical_context::StatisticalContext;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct StatisticalContextSummary {
    pub price_percentile: f64,
    pub volatility_percentile: f64,
    pub market_shape: String,
    pub anomaly_score: f64,
    pub top_anomaly: String,
    pub trend_reliability: f64,
    pub breakout_confidence: f64,
    pub trend_continuation_prob: f64,
    pub mean_reversion_prob: f64,
    pub reversal_prob: f64,
    pub confidence_score: f64,
    pub market_predictability: f64,
    pub expected_opportunity: f64,
    pub risk_confidence: f64,
    pub top_predictors: Vec<(String, f64)>,
    pub kalman_drift: f64,
    pub kalman_noise_vol: f64,
    pub kalman_trend_strength: f64,
}

impl Default for StatisticalContextSummary {
    fn default() -> Self {
        Self {
            price_percentile: 0.0,
            volatility_percentile: 0.0,
            market_shape: "unknown".into(),
            anomaly_score: 0.0,
            top_anomaly: String::new(),
            trend_reliability: 0.0,
            breakout_confidence: 0.0,
            trend_continuation_prob: 0.0,
            mean_reversion_prob: 0.0,
            reversal_prob: 0.0,
            confidence_score: 0.0,
            market_predictability: 0.0,
            expected_opportunity: 0.0,
            risk_confidence: 0.0,
            top_predictors: Vec::new(),
            kalman_drift: 0.0,
            kalman_noise_vol: 0.0,
            kalman_trend_strength: 0.0,
        }
    }
}

impl StatisticalContextSummary {
    pub fn from_sil_context(ctx: &StatisticalContext) -> Self {
        Self {
            price_percentile: ctx.price_stats.percentile,
            volatility_percentile: ctx.volatility_percentile,
            market_shape: ctx.market_shape_label.clone(),
            anomaly_score: ctx.anomaly_score,
            top_anomaly: ctx.top_anomaly_reason.clone(),
            trend_reliability: ctx.trend_reliability,
            breakout_confidence: ctx.breakout_confidence,
            trend_continuation_prob: ctx.trend_continuation_prob,
            mean_reversion_prob: ctx.mean_reversion_prob,
            reversal_prob: ctx.reversal_prob,
            confidence_score: ctx.confidence_score,
            market_predictability: ctx.market_predictability,
            expected_opportunity: ctx.expected_opportunity,
            risk_confidence: ctx.risk_confidence,
            top_predictors: ctx.top_predictive_indicators.clone(),
            kalman_drift: ctx.kalman_drift,
            kalman_noise_vol: ctx.kalman_noise_vol,
            kalman_trend_strength: ctx.kalman_trend_strength,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DeterministicTelemetry {
    pub market_regime: String,
    pub total_confluence_score: i32,
    pub rvol: f64,
    pub adx_value: f64,
    pub adx_regime: String,
    pub bbwp_percentile: f64,
    pub squeeze_on: bool,
    pub vwap_bias: String,
    pub support_levels: Vec<String>,
    pub resistance_levels: Vec<String>,
    pub rsi_divergence_state: String,
    pub macd_divergence_state: String,
    pub macd_crossover_state: String,
    pub squeeze_release_state: String,
    pub statistical_context: StatisticalContextSummary,
}

pub fn compile_deterministic_telemetry(
    mid: &IndicatorSnapshot,
    support_levels: &[String],
    resistance_levels: &[String],
    sil_ctx: Option<&StatisticalContext>,
    regime_multipliers: Option<&HashMap<String, HashMap<String, f64>>>,
) -> DeterministicTelemetry {
    let adx = mid.adx().unwrap_or(0.0);
    let bbwp = mid.bbwp().unwrap_or(50.0);
    let squeeze_on = mid.squeeze_on().unwrap_or(false);
    let rvol = mid.rvol().unwrap_or(1.0);
    let atr_regime_owned = mid.atr_volatility_regime();
    let atr_regime = atr_regime_owned.as_deref();
    let ema_stack_owned = mid.ema_stack_state();
    let ema_stack = ema_stack_owned.as_deref();
    let squeeze_released = mid.squeeze_release_trigger().unwrap_or(false);

    // 1. Regime Classification
    let regime = if bbwp < 10.0 || squeeze_on {
        "COMPRESSION"
    } else if squeeze_released || (bbwp > 90.0 && atr_regime == Some("expanding")) {
        "EXPANSION"
    } else if adx >= 25.0 && ema_stack != Some("tangled") && ema_stack.is_some() {
        "TRENDING"
    } else {
        "RANGE"
    };

    // 2. Resolve Trigger vs Confirmation States
    let is_completed = mid.current_price.is_some();

    let macd_crossover_state = if mid.macd_crossover_detected().unwrap_or(false) {
        if is_completed { "confirmed".to_string() } else { "trigger".to_string() }
    } else {
        "none".to_string()
    };

    let squeeze_release_state = if squeeze_released {
        if is_completed { "confirmed".to_string() } else { "trigger".to_string() }
    } else {
        "none".to_string()
    };

    let rsi_div_state = mid.rsi_divergence_status().clone().unwrap_or_else(|| "none".to_string());
    let macd_div_state = mid.macd_divergence_status().clone().unwrap_or_else(|| "none".to_string());

    let snap = crate::profile_evaluation::indicator_to_snapshot_values(
        &mid.indicators,
        mid.current_price.unwrap_or(0.0),
    );
    let confluence = crate::profile_evaluation::calculate_registry_confluence(
        "BULLISH", &snap,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
        regime_multipliers,
    );

    DeterministicTelemetry {
        market_regime: regime.to_string(),
        total_confluence_score: confluence.score,
        rvol,
        adx_value: adx,
        adx_regime: mid.adx_regime().clone().unwrap_or_else(|| "congestion".to_string()),
        bbwp_percentile: bbwp,
        squeeze_on,
        vwap_bias: mid.vwap_bias().clone().unwrap_or_else(|| "equilibrium".to_string()),
        support_levels: support_levels.to_vec(),
        resistance_levels: resistance_levels.to_vec(),
        rsi_divergence_state: rsi_div_state,
        macd_divergence_state: macd_div_state,
        macd_crossover_state,
        squeeze_release_state,
        statistical_context: sil_ctx
            .map(|c| StatisticalContextSummary::from_sil_context(c))
            .unwrap_or_default(),
    }
}
