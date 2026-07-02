//! Consolidation of the 11 normalization mappers into a unified map.

use super::super::squeeze::MomentumDirection;
use super::{DivergenceState, NormalizationContext, NormalizationEngine, NormalizedIndicatorValue};
use std::collections::HashMap;

/// Raw indicator inputs bundle consumed by [`NormalizationEngine::normalize_all`].
///
/// Every field is optional so partially-warmed pipelines can still produce a
/// consistent map (missing indicators are simply omitted from the output).
#[derive(Debug, Clone, Default)]
pub struct IndicatorInputs {
    // RSI
    pub rsi: Option<f64>,
    pub rsi_divergence: DivergenceState,
    // MACD
    pub macd_line: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_histogram: Option<f64>,
    pub macd_histogram_peak: Option<f64>,
    /// Crossover: Some(1) bullish, Some(-1) bearish, None none.
    pub macd_crossover: Option<i8>,
    pub macd_divergence: DivergenceState,
    // Squeeze
    pub squeeze_on: Option<bool>,
    pub squeeze_release_trigger: bool,
    pub squeeze_momentum: Option<f64>,
    pub squeeze_direction: Option<MomentumDirection>,
    // ADX
    pub adx: Option<f64>,
    pub adx_plus_di: Option<f64>,
    pub adx_minus_di: Option<f64>,
    pub adx_slope: Option<f64>,
    // BBWP
    pub bbwp: Option<f64>,
    // RVOL
    pub rvol: Option<f64>,
    // VWAP
    pub vwap: Option<f64>,
    // Fibonacci
    pub fib_gp_low: Option<f64>,
    pub fib_gp_high: Option<f64>,
    pub fib_ext_1618: Option<f64>,
    pub fib_ext_2618: Option<f64>,
    // Patterns
    pub pattern_bullish: bool,
    pub pattern_bearish: bool,
    pub pattern_confidence: Option<f64>,
    // Supplemental raw-only series preserved for frontend charts (normalized 0.0)
    pub atr_14: Option<f64>,
    pub atr_slope: Option<f64>,
    pub bb_upper: Option<f64>,
    pub bb_middle: Option<f64>,
    pub bb_lower: Option<f64>,
}

impl NormalizationEngine {
    /// Consolidate all available indicators into the unified normalized map.
    ///
    /// Keys: `rsi`, `macd`, `squeeze`, `adx`, `bbwp`, `rvol`, `ema_stack`,
    /// `vwap`, `fibonacci`, `patterns`, `support_resistance`.
    pub fn normalize_all(
        inputs: &IndicatorInputs,
        ctx: &NormalizationContext,
    ) -> HashMap<String, NormalizedIndicatorValue> {
        let mut out: HashMap<String, NormalizedIndicatorValue> = HashMap::new();

        if let Some(rsi) = inputs.rsi {
            out.insert(
                "rsi".into(),
                Self::normalize_rsi(rsi, inputs.rsi_divergence),
            );
        }

        // Dedicated divergence confluence factors (distinct from the RSI/MACD
        // position folding) so the 8-factor scoring engine can weight them
        // independently per the spec (RSI-Div ±20, MACD-Div ±10).
        if let Some(v) = divergence_value(inputs.rsi_divergence) {
            out.insert("rsi_divergence".into(), v);
        }
        if let Some(v) = divergence_value(inputs.macd_divergence) {
            out.insert("macd_divergence".into(), v);
        }

        if let (Some(line), Some(signal), Some(hist)) =
            (inputs.macd_line, inputs.macd_signal, inputs.macd_histogram)
        {
            out.insert(
                "macd".into(),
                Self::normalize_macd(
                    line,
                    signal,
                    hist,
                    inputs.macd_histogram_peak.unwrap_or(0.0),
                    inputs.macd_crossover,
                ),
            );
        }

        if let (Some(on), Some(mom)) = (inputs.squeeze_on, inputs.squeeze_momentum) {
            out.insert(
                "squeeze".into(),
                Self::normalize_squeeze(
                    on,
                    inputs.squeeze_release_trigger,
                    mom,
                    inputs.squeeze_direction.unwrap_or(MomentumDirection::Flat),
                ),
            );
        }

        if let (Some(adx), Some(plus), Some(minus)) =
            (inputs.adx, inputs.adx_plus_di, inputs.adx_minus_di)
        {
            out.insert(
                "adx".into(),
                Self::normalize_adx(
                    adx,
                    plus,
                    minus,
                    inputs.adx_slope.unwrap_or(0.0),
                    ctx.adx_consecutive_deceleration,
                ),
            );
        }

        if let Some(bbwp) = inputs.bbwp {
            out.insert("bbwp".into(), Self::normalize_bbwp(bbwp, ctx.trend_bias));
        }

        if let Some(rvol) = inputs.rvol {
            out.insert("rvol".into(), Self::normalize_rvol(rvol));
        }

        if ctx.ema_stack_state.is_some() {
            out.insert("ema_stack".into(), Self::normalize_ema_stack(ctx));
        }

        if let Some(vwap) = inputs.vwap {
            out.insert("vwap".into(), Self::normalize_vwap(ctx.price, vwap));
        }

        if inputs.fib_gp_low.is_some()
            || inputs.fib_gp_high.is_some()
            || inputs.fib_ext_1618.is_some()
            || inputs.fib_ext_2618.is_some()
        {
            out.insert(
                "fibonacci".into(),
                Self::normalize_fibonacci(
                    ctx.price,
                    inputs.fib_gp_low,
                    inputs.fib_gp_high,
                    inputs.fib_ext_1618,
                    inputs.fib_ext_2618,
                    ctx.trend_bias,
                ),
            );
        }

        if inputs.pattern_bullish || inputs.pattern_bearish {
            out.insert(
                "patterns".into(),
                Self::normalize_patterns(
                    inputs.pattern_bullish,
                    inputs.pattern_bearish,
                    inputs.pattern_confidence.unwrap_or(0.0),
                    ctx.rvol.unwrap_or(0.0),
                ),
            );
        }

        if !ctx.support_levels.is_empty() || !ctx.resistance_levels.is_empty() {
            out.insert(
                "support_resistance".into(),
                Self::normalize_sr(
                    ctx.price,
                    &ctx.support_levels,
                    &ctx.resistance_levels,
                    ctx.rvol.unwrap_or(0.0),
                ),
            );
        }

        // Supplemental raw-only chart series (neutral normalized score so they
        // never influence the confluence engine).
        if inputs.atr_14.is_some() || inputs.atr_slope.is_some() {
            let mut values = HashMap::new();
            if let Some(a) = inputs.atr_14 {
                values.insert("atr_14".to_string(), a);
            }
            if let Some(s) = inputs.atr_slope {
                values.insert("atr_slope".to_string(), s);
            }
            out.insert(
                "atr".into(),
                NormalizedIndicatorValue::with_values(
                    inputs.atr_14.unwrap_or(0.0),
                    0.0,
                    "ATR_RAW",
                    values,
                ),
            );
        }

        if inputs.bb_upper.is_some() || inputs.bb_middle.is_some() || inputs.bb_lower.is_some() {
            let mut values = HashMap::new();
            if let Some(u) = inputs.bb_upper {
                values.insert("upper".to_string(), u);
            }
            if let Some(m) = inputs.bb_middle {
                values.insert("middle".to_string(), m);
            }
            if let Some(l) = inputs.bb_lower {
                values.insert("lower".to_string(), l);
            }
            out.insert(
                "bollinger".into(),
                NormalizedIndicatorValue::with_values(
                    inputs.bb_middle.unwrap_or(0.0),
                    0.0,
                    "BOLLINGER_RAW",
                    values,
                ),
            );
        }

        out
    }
}

/// Map a divergence classification to a dedicated normalized confluence value.
/// Confirmed divergences map to ±1.0, potential to ±0.5, none is omitted.
fn divergence_value(state: DivergenceState) -> Option<NormalizedIndicatorValue> {
    let (norm, label) = match state {
        DivergenceState::ConfirmedBullish => (1.0, "CONFIRMED_BULLISH_DIVERGENCE"),
        DivergenceState::PotentialBullish => (0.5, "POTENTIAL_BULLISH_DIVERGENCE"),
        DivergenceState::ConfirmedBearish => (-1.0, "CONFIRMED_BEARISH_DIVERGENCE"),
        DivergenceState::PotentialBearish => (-0.5, "POTENTIAL_BEARISH_DIVERGENCE"),
        DivergenceState::None => return None,
    };
    Some(NormalizedIndicatorValue::scalar(norm, norm, label))
}
