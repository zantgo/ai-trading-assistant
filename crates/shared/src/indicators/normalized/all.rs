//! Consolidation of the normalization mappers into a unified map.

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
    // Stochastic Oscillator (%K slowed, %D signal)
    pub stoch_k: Option<f64>,
    pub stoch_d: Option<f64>,
    // Chande Momentum Oscillator
    pub chandemo: Option<f64>,
    // Supertrend
    pub supertrend_line: Option<f64>,
    pub supertrend_dir: Option<i8>,
    // Keltner Channels
    pub keltner_upper: Option<f64>,
    pub keltner_middle: Option<f64>,
    pub keltner_lower: Option<f64>,
    // Donchian Channels
    pub donchian_upper: Option<f64>,
    pub donchian_middle: Option<f64>,
    pub donchian_lower: Option<f64>,
    // On-Balance Volume
    pub obv: Option<f64>,
    pub obv_sma: Option<f64>,
    // Chaikin Money Flow
    pub cmf: Option<f64>,
    // Money Flow Index
    pub mfi: Option<f64>,
    // Historical Volatility
    pub hv: Option<f64>,
    // Market Regime
    pub aroon_up: Option<f64>,
    pub aroon_down: Option<f64>,
    pub choppiness: Option<f64>,
    pub linreg_slope: Option<f64>,
    pub zscore: Option<f64>,
    // Generalized divergence states (Phase 2) for the extra oscillators.
    pub stochastic_divergence: DivergenceState,
    pub chandemo_divergence: DivergenceState,
    pub mfi_divergence: DivergenceState,
    pub cmf_divergence: DivergenceState,
    pub obv_divergence: DivergenceState,
    pub squeeze_divergence: DivergenceState,
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
    /// Keys: `rsi`, `stochastic`, `chandemo`, `macd`, `squeeze`, `adx`, `bbwp`,
    /// `rvol`, `ema_stack`, `vwap`, `fibonacci`, `patterns`,
    /// `support_resistance`.
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

        if let (Some(k), Some(d)) = (inputs.stoch_k, inputs.stoch_d) {
            out.insert("stochastic".into(), Self::normalize_stochastic(k, d));
        }

        if let Some(cmo) = inputs.chandemo {
            out.insert("chandemo".into(), Self::normalize_chandemo(cmo));
        }

        if let (Some(line), Some(dir)) = (inputs.supertrend_line, inputs.supertrend_dir) {
            out.insert(
                "supertrend".into(),
                Self::normalize_supertrend(ctx.price, line, dir),
            );
        }

        if let (Some(u), Some(m), Some(l)) =
            (inputs.keltner_upper, inputs.keltner_middle, inputs.keltner_lower)
        {
            out.insert("keltner".into(), Self::normalize_keltner(ctx.price, u, m, l));
        }

        if let (Some(u), Some(m), Some(l)) =
            (inputs.donchian_upper, inputs.donchian_middle, inputs.donchian_lower)
        {
            out.insert("donchian".into(), Self::normalize_donchian(ctx.price, u, m, l));
        }

        if let (Some(obv), Some(sma)) = (inputs.obv, inputs.obv_sma) {
            out.insert("obv".into(), Self::normalize_obv(obv, sma));
        }

        if let Some(cmf) = inputs.cmf {
            out.insert("cmf".into(), Self::normalize_cmf(cmf));
        }

        if let Some(mfi) = inputs.mfi {
            out.insert("mfi".into(), Self::normalize_mfi(mfi));
        }

        if let Some(hv) = inputs.hv {
            out.insert("hv".into(), Self::normalize_hv(hv));
        }

        if let (Some(up), Some(down)) = (inputs.aroon_up, inputs.aroon_down) {
            out.insert("aroon".into(), Self::normalize_aroon(up, down));
        }

        if let Some(chop) = inputs.choppiness {
            out.insert("choppiness".into(), Self::normalize_choppiness(chop));
        }

        if let Some(slope) = inputs.linreg_slope {
            out.insert(
                "linreg_slope".into(),
                Self::normalize_linreg_slope(slope, ctx.price),
            );
        }

        if let Some(z) = inputs.zscore {
            out.insert("zscore".into(), Self::normalize_zscore(z));
        }

        // Dedicated divergence confluence factors always present (neutral 0.0
        // when no divergence is active) for consistent frontend rendering.
        out.insert("rsi_divergence".into(), divergence_value(inputs.rsi_divergence));
        out.insert("macd_divergence".into(), divergence_value(inputs.macd_divergence));

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

        // Always present (neutral when no swing leg exists) for consistent
        // frontend rendering.
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

        out.insert(
            "patterns".into(),
            Self::normalize_patterns(
                inputs.pattern_bullish,
                inputs.pattern_bearish,
                inputs.pattern_confidence.unwrap_or(0.0),
                ctx.rvol.unwrap_or(0.0),
            ),
        );

        out.insert(
            "support_resistance".into(),
            Self::normalize_sr(
                    ctx.price,
                    &ctx.support_levels,
                    &ctx.resistance_levels,
                    ctx.rvol.unwrap_or(0.0),
                ),
            );

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

        // Generalized divergence scored entries (Phase 2). Always present
        // (neutral when no divergence is active) for consistent frontend
        // rendering.
        for (parent, key, state) in [
            ("stochastic", "stochastic_divergence", inputs.stochastic_divergence),
            ("chandemo", "chandemo_divergence", inputs.chandemo_divergence),
            ("mfi", "mfi_divergence", inputs.mfi_divergence),
            ("cmf", "cmf_divergence", inputs.cmf_divergence),
            ("obv", "obv_divergence", inputs.obv_divergence),
            ("squeeze", "squeeze_divergence", inputs.squeeze_divergence),
        ] {
            let v = super::signals::divergence_entry(&mut out, parent, state)
                .unwrap_or_else(|| NormalizedIndicatorValue::neutral("NEUTRAL"));
            out.insert(key.into(), v);
        }

        // Derive state-based discrete signals (threshold/breakout/etc.) from
        // each indicator's current label. Also surface the primary RSI/MACD
        // divergence as a signal on their parent oscillators.
        if let Some(v) = super::signals::divergence_entry(&mut out, "rsi", inputs.rsi_divergence) {
            let _ = v; // rsi_divergence scored entry already added above.
        }
        if let Some(v) = super::signals::divergence_entry(&mut out, "macd", inputs.macd_divergence) {
            let _ = v;
        }
        super::signals::derive_signals(&mut out);

        out
    }
}

/// Map a divergence classification to a dedicated normalized confluence value.
/// Confirmed divergences map to ±1.0, potential to ±0.5, none to neutral 0.0.
fn divergence_value(state: DivergenceState) -> NormalizedIndicatorValue {
    let (norm, label) = match state {
        DivergenceState::ConfirmedBullish => (1.0, "CONFIRMED_BULLISH_DIVERGENCE"),
        DivergenceState::PotentialBullish => (0.5, "POTENTIAL_BULLISH_DIVERGENCE"),
        DivergenceState::ConfirmedBearish => (-1.0, "CONFIRMED_BEARISH_DIVERGENCE"),
        DivergenceState::PotentialBearish => (-0.5, "POTENTIAL_BEARISH_DIVERGENCE"),
        DivergenceState::None => (0.0, "NEUTRAL"),
    };
    NormalizedIndicatorValue::scalar(norm, norm, label)
}
