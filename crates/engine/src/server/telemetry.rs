use crate::server::types::IndicatorSnapshot;
use serde::Serialize;

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
}

pub fn compile_deterministic_telemetry(
    mid: &IndicatorSnapshot,
    support_levels: &[String],
    resistance_levels: &[String],
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
        if is_completed {
            "confirmed".to_string()
        } else {
            "trigger".to_string()
        }
    } else {
        "none".to_string()
    };

    let squeeze_release_state = if squeeze_released {
        if is_completed {
            "confirmed".to_string()
        } else {
            "trigger".to_string()
        }
    } else {
        "none".to_string()
    };

    let rsi_div_state = mid
        .rsi_divergence_status()
        .clone()
        .unwrap_or_else(|| "none".to_string());
    let macd_div_state = mid
        .macd_divergence_status()
        .clone()
        .unwrap_or_else(|| "none".to_string());

    // 3. Unified Registry-Driven Confluence (replaces legacy 100-point heuristic).
    // All 29 active directional indicators contribute continuously with
    // configurable weights, regime-aware multipliers, and the INACTIVE protocol.
    let snap = crate::profile_evaluation::indicator_to_snapshot_values(
        &mid.indicators,
        mid.current_price.unwrap_or(0.0),
    );
    let confluence = crate::profile_evaluation::calculate_registry_confluence(
        "BULLISH",
        &snap,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
        None,
    );

    DeterministicTelemetry {
        market_regime: regime.to_string(),
        total_confluence_score: confluence.score,
        rvol,
        adx_value: adx,
        adx_regime: mid
            .adx_regime()
            .clone()
            .unwrap_or_else(|| "congestion".to_string()),
        bbwp_percentile: bbwp,
        squeeze_on,
        vwap_bias: mid
            .vwap_bias()
            .clone()
            .unwrap_or_else(|| "equilibrium".to_string()),
        support_levels: support_levels.to_vec(),
        resistance_levels: resistance_levels.to_vec(),
        rsi_divergence_state: rsi_div_state,
        macd_divergence_state: macd_div_state,
        macd_crossover_state,
        squeeze_release_state,
    }
}
