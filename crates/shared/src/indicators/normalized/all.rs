//! Consolidation of the normalization mappers into a unified map.

use super::super::squeeze::MomentumDirection;
use super::{
    DivergenceState, IndicatorSignal, NormalizationContext, NormalizationEngine,
    NormalizedIndicatorValue, SignalDirection, SignalKind, SignalStatus,
};
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
    /// True when Supertrend direction flipped this bar.
    pub supertrend_flipped: bool,
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
    /// DI crossover this bar: Some(1) +DI crossed above -DI (bullish),
    /// Some(-1) -DI crossed above +DI (bearish), None no cross.
    pub adx_di_crossover: Option<i8>,
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
    // EMA ribbon values for fast/medium crossover detection.
    pub ema_fast: Option<f64>,
    pub ema_medium: Option<f64>,
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
            // Structured Crossover signal: the MACD normalizer computes the state
            // label, but the crossover event itself is a structured boolean that
            // `derive_signals()` cannot detect from the label string alone.
            if let Some(cross_dir) = inputs.macd_crossover {
                if let Some(entry) = out.get_mut("macd") {
                    let (d, label) = if cross_dir > 0 {
                        (SignalDirection::Bullish, "BULLISH_CROSSOVER")
                    } else {
                        (SignalDirection::Bearish, "BEARISH_CROSSOVER")
                    };
                    entry.signals.push(IndicatorSignal::new(
                        SignalKind::Crossover, d, SignalStatus::Active, label,
                    ));
                }
            }
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

        // Generalized divergence scored entries (Phase 2). Each also pushes a
        // Divergence signal onto its parent oscillator.
        for (parent, key, state) in [
            ("stochastic", "stochastic_divergence", inputs.stochastic_divergence),
            ("chandemo", "chandemo_divergence", inputs.chandemo_divergence),
            ("mfi", "mfi_divergence", inputs.mfi_divergence),
            ("cmf", "cmf_divergence", inputs.cmf_divergence),
            ("obv", "obv_divergence", inputs.obv_divergence),
            ("squeeze", "squeeze_divergence", inputs.squeeze_divergence),
        ] {
            if let Some(v) = super::signals::divergence_entry(&mut out, parent, state) {
                out.insert(key.into(), v);
            }
        }

        // ── Structured cross-over / zero-cross detection ──
        // Every detector below compares current indicator values against the
        // previous completed bar (ctx.prev) and emits discrete signals when a
        // state transition is detected.

        // Supertrend flip (TrendFlip).
        if inputs.supertrend_flipped {
            if let Some(entry) = out.get_mut("supertrend") {
                let d = if inputs.supertrend_dir == Some(1) { SignalDirection::Bullish } else { SignalDirection::Bearish };
                entry.signals.push(IndicatorSignal::new(
                    SignalKind::TrendFlip, d, SignalStatus::Active,
                    if d == SignalDirection::Bullish { "SUPERTREND_BULLISH_FLIP" } else { "SUPERTREND_BEARISH_FLIP" },
                ));
            }
        }

        // ADX DI crossover (TrendFlip).
        if let Some(cross) = inputs.adx_di_crossover {
            if let Some(entry) = out.get_mut("adx") {
                let (d, label) = if cross > 0 {
                    (SignalDirection::Bullish, "ADX_DI_CROSSOVER_BULLISH")
                } else {
                    (SignalDirection::Bearish, "ADX_DI_CROSSOVER_BEARISH")
                };
                entry.signals.push(IndicatorSignal::new(
                    SignalKind::TrendFlip, d, SignalStatus::Active, label,
                ));
            }
        }

        // RSI midline cross (ZeroLineCross = RSI crosses 50).
        if let (Some(rsi), Some(prev_rsi)) = (inputs.rsi, ctx.prev.rsi) {
            if (prev_rsi <= 50.0 && rsi > 50.0) || (prev_rsi >= 50.0 && rsi < 50.0) {
                if let Some(entry) = out.get_mut("rsi") {
                    entry.signals.push(IndicatorSignal::new(SignalKind::ZeroLineCross,
                        if rsi > 50.0 { SignalDirection::Bullish } else { SignalDirection::Bearish },
                        SignalStatus::Active,
                        if rsi > 50.0 { "RSI_ZERO_CROSS_BULLISH" } else { "RSI_ZERO_CROSS_BEARISH" }));
                }
            }
        }

        // Stochastic K/D crossover.
        if let (Some(k), Some(d), Some(pk), Some(pd)) = (inputs.stoch_k, inputs.stoch_d, ctx.prev.stoch_k, ctx.prev.stoch_d) {
            if (pk <= pd && k > d) || (pk >= pd && k < d) {
                if let Some(entry) = out.get_mut("stochastic") {
                    entry.signals.push(IndicatorSignal::new(SignalKind::Crossover,
                        if k > d { SignalDirection::Bullish } else { SignalDirection::Bearish },
                        SignalStatus::Active,
                        if k > d { "STOCH_BULLISH_CROSSOVER" } else { "STOCH_BEARISH_CROSSOVER" }));
                }
            }
        }

        // ChandeMO / CMF / LinReg / Z-Score zero cross.
        for (key, current, prev_opt) in &[
            ("chandemo", inputs.chandemo, ctx.prev.chandemo),
            ("cmf", inputs.cmf, ctx.prev.cmf),
            ("linreg_slope", inputs.linreg_slope, ctx.prev.linreg_slope),
            ("zscore", inputs.zscore, ctx.prev.zscore),
        ] {
            if let (Some(cur), Some(prev)) = (*current, *prev_opt) {
                if (prev <= 0.0 && cur > 0.0) || (prev >= 0.0 && cur < 0.0) {
                    if let Some(entry) = out.get_mut(*key) {
                        entry.signals.push(IndicatorSignal::new(SignalKind::ZeroLineCross,
                            if cur > 0.0 { SignalDirection::Bullish } else { SignalDirection::Bearish },
                            SignalStatus::Active,
                            &format!("{}_ZERO_CROSS_{}", key.to_uppercase(), if cur > 0.0 { "BULLISH" } else { "BEARISH" })));
                    }
                }
            }
        }

        // OBV trend-flip: accumulation ↔ distribution transition.
        if let (Some(obv), Some(prev_obv), Some(sma), Some(prev_sma)) = (inputs.obv, ctx.prev.obv, inputs.obv_sma, ctx.prev.obv_sma) {
            let cur_above = obv > sma;
            let prev_above = prev_obv > prev_sma;
            if cur_above != prev_above {
                if let Some(entry) = out.get_mut("obv") {
                    entry.signals.push(IndicatorSignal::new(SignalKind::TrendFlip,
                        if cur_above { SignalDirection::Bullish } else { SignalDirection::Bearish },
                        SignalStatus::Active,
                        if cur_above { "OBV_TREND_FLIP_BULLISH" } else { "OBV_TREND_FLIP_BEARISH" }));
                }
            }
        }

        // Aroon crossover: Up crosses Down.
        if let (Some(up), Some(down), Some(pu), Some(pd)) = (inputs.aroon_up, inputs.aroon_down, ctx.prev.aroon_up, ctx.prev.aroon_down) {
            if (pu <= pd && up > down) || (pu >= pd && down > up) {
                if let Some(entry) = out.get_mut("aroon") {
                    entry.signals.push(IndicatorSignal::new(SignalKind::Crossover,
                        if up > down { SignalDirection::Bullish } else { SignalDirection::Bearish },
                        SignalStatus::Active,
                        if up > down { "AROON_BULLISH_CROSS" } else { "AROON_BEARISH_CROSS" }));
                }
            }
        }

        // MFI midline cross (50).
        if let (Some(mfi), Some(prev_mfi)) = (inputs.mfi, ctx.prev.mfi) {
            if (prev_mfi <= 50.0 && mfi > 50.0) || (prev_mfi >= 50.0 && mfi < 50.0) {
                if let Some(entry) = out.get_mut("mfi") {
                    entry.signals.push(IndicatorSignal::new(SignalKind::ZeroLineCross,
                        if mfi > 50.0 { SignalDirection::Bullish } else { SignalDirection::Bearish },
                        SignalStatus::Active,
                        if mfi > 50.0 { "MFI_CROSSOVER_BULLISH" } else { "MFI_CROSSOVER_BEARISH" }));
                }
            }
        }

        // Bollinger Band Touch (price at band edge).
        if let (Some(upper), Some(middle), Some(lower)) = (inputs.bb_upper, inputs.bb_middle, inputs.bb_lower) {
            let price = ctx.price;
            let inside = price >= lower && price <= upper;
            if !inside {
                if let Some(entry) = out.get_mut("bollinger") {
                    if price > upper {
                        entry.signals.push(IndicatorSignal::new(SignalKind::Breakout, SignalDirection::Bullish, SignalStatus::Active, "BOLLINGER_UPPER_BREAKOUT"));
                    } else {
                        entry.signals.push(IndicatorSignal::new(SignalKind::Breakout, SignalDirection::Bearish, SignalStatus::Active, "BOLLINGER_LOWER_BREAKOUT"));
                    }
                }
            } else {
                // Band touch (near band edge but inside)
                let band_w = upper - middle;
                if band_w > 0.0 {
                    let pct = (price - lower) / (upper - lower);
                    if pct > 0.90 {
                        if let Some(entry) = out.get_mut("bollinger") {
                            entry.signals.push(IndicatorSignal::new(SignalKind::BandTouch, SignalDirection::Bearish, SignalStatus::Active, "BOLLINGER_UPPER_BAND_TOUCH"));
                        }
                    } else if pct < 0.10 {
                        if let Some(entry) = out.get_mut("bollinger") {
                            entry.signals.push(IndicatorSignal::new(SignalKind::BandTouch, SignalDirection::Bullish, SignalStatus::Active, "BOLLINGER_LOWER_BAND_TOUCH"));
                        }
                    }
                }
            }
            // Normalize Bollinger: how far price is within bands (-1 bottom to +1 top)
            let norm = if upper > lower { ((price - middle) / (upper - middle)).clamp(-1.0, 1.0) } else { 0.0 };
            if let Some(entry) = out.get_mut("bollinger") {
                entry.normalized = norm;
                entry.state_label = if price > upper { "BOLLINGER_UPPER_BREAKOUT".into() }
                    else if price < lower { "BOLLINGER_LOWER_BREAKOUT".into() }
                    else if (price - lower) / (upper - lower).max(f64::EPSILON) > 0.90 { "BOLLINGER_UPPER_BAND_TOUCH".into() }
                    else if (price - lower) / (upper - lower).max(f64::EPSILON) < 0.10 { "BOLLINGER_LOWER_BAND_TOUCH".into() }
                    else { "BOLLINGER_INSIDE_BANDS".into() };
            }
        }

        // ATR expansion / contraction.
        if inputs.atr_14.is_some() {
            if let Some(slope) = inputs.atr_slope {
                let label = if slope > 0.01 { "ATR_EXPANDING" }
                    else if slope < -0.01 { "ATR_CONTRACTING" }
                    else { "ATR_STABLE" };
                if let Some(entry) = out.get_mut("atr") {
                    entry.state_label = label.into();
                    if slope > 0.01 {
                        entry.signals.push(IndicatorSignal::new(SignalKind::Threshold, SignalDirection::Neutral, SignalStatus::Active, "ATR_EXPANDING"));
                    } else if slope < -0.01 {
                        entry.signals.push(IndicatorSignal::new(SignalKind::CompressionRelease, SignalDirection::Neutral, SignalStatus::Active, "ATR_CONTRACTING"));
                    }
                }
            }
        }

        // ── Donchian BandTouch (distinct from Breakout): price near a band
        // edge but still inside the channel (mean-reversion proximity). ──
        if let (Some(u), Some(l)) = (inputs.donchian_upper, inputs.donchian_lower) {
            let price = ctx.price;
            if price < u && price > l && u > l {
                let pos = (price - l) / (u - l);
                if pos > 0.85 {
                    if let Some(e) = out.get_mut("donchian") {
                        e.signals.push(IndicatorSignal::new(SignalKind::BandTouch, SignalDirection::Bearish, SignalStatus::Active, "DONCHIAN_UPPER_BAND_TOUCH"));
                    }
                } else if pos < 0.15 {
                    if let Some(e) = out.get_mut("donchian") {
                        e.signals.push(IndicatorSignal::new(SignalKind::BandTouch, SignalDirection::Bullish, SignalStatus::Active, "DONCHIAN_LOWER_BAND_TOUCH"));
                    }
                }
            }
        }

        // ── Keltner BandTouch (distinct from Breakout). ──
        if let (Some(u), Some(l)) = (inputs.keltner_upper, inputs.keltner_lower) {
            let price = ctx.price;
            if price < u && price > l && u > l {
                let pos = (price - l) / (u - l);
                if pos > 0.85 {
                    if let Some(e) = out.get_mut("keltner") {
                        e.signals.push(IndicatorSignal::new(SignalKind::BandTouch, SignalDirection::Bearish, SignalStatus::Active, "KELTNER_UPPER_BAND_TOUCH"));
                    }
                } else if pos < 0.15 {
                    if let Some(e) = out.get_mut("keltner") {
                        e.signals.push(IndicatorSignal::new(SignalKind::BandTouch, SignalDirection::Bullish, SignalStatus::Active, "KELTNER_LOWER_BAND_TOUCH"));
                    }
                }
            }
        }

        // ── EMA fast/medium Crossover (distinct from StackChange). ──
        if let (Some(f), Some(m), Some(pf), Some(pm)) =
            (inputs.ema_fast, inputs.ema_medium, ctx.prev.ema_fast, ctx.prev.ema_medium)
        {
            if pf <= pm && f > m {
                if let Some(e) = out.get_mut("ema_stack") {
                    e.signals.push(IndicatorSignal::new(SignalKind::Crossover, SignalDirection::Bullish, SignalStatus::Active, "EMA_FAST_MEDIUM_BULLISH_CROSS"));
                }
            } else if pf >= pm && f < m {
                if let Some(e) = out.get_mut("ema_stack") {
                    e.signals.push(IndicatorSignal::new(SignalKind::Crossover, SignalDirection::Bearish, SignalStatus::Active, "EMA_FAST_MEDIUM_BEARISH_CROSS"));
                }
            }
        }

        // ── Supertrend price/line Crossover (distinct from TrendFlip). ──
        if let (Some(line), Some(pline), Some(pprice)) =
            (inputs.supertrend_line, ctx.prev.supertrend_line, ctx.prev.price)
        {
            let price = ctx.price;
            if pprice <= pline && price > line {
                if let Some(e) = out.get_mut("supertrend") {
                    e.signals.push(IndicatorSignal::new(SignalKind::Crossover, SignalDirection::Bullish, SignalStatus::Active, "SUPERTREND_PRICE_CROSS_BULLISH"));
                }
            } else if pprice >= pline && price < line {
                if let Some(e) = out.get_mut("supertrend") {
                    e.signals.push(IndicatorSignal::new(SignalKind::Crossover, SignalDirection::Bearish, SignalStatus::Active, "SUPERTREND_PRICE_CROSS_BEARISH"));
                }
            }
        }

        // ── Aroon TrendFlip (transition-only, distinct from Crossover). ──
        // Fires ONLY on the bar where Up/Down leadership crosses — a discrete
        // point-in-time flip event, then goes quiet until the next crossing.
        if let (Some(up), Some(down), Some(pu), Some(pd)) =
            (inputs.aroon_up, inputs.aroon_down, ctx.prev.aroon_up, ctx.prev.aroon_down)
        {
            if pu <= pd && up > down {
                if let Some(e) = out.get_mut("aroon") {
                    e.signals.push(IndicatorSignal::new(SignalKind::TrendFlip, SignalDirection::Bullish, SignalStatus::Active, "AROON_BULLISH_TREND_FLIP"));
                }
            } else if pu >= pd && up < down {
                if let Some(e) = out.get_mut("aroon") {
                    e.signals.push(IndicatorSignal::new(SignalKind::TrendFlip, SignalDirection::Bearish, SignalStatus::Active, "AROON_BEARISH_TREND_FLIP"));
                }
            }
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

        // ── INACTIVE fill (backend as single source of truth) ──
        // Event-driven directional indicators (divergences, Fibonacci, S/R,
        // Patterns) are omitted above when no trigger/level/pattern exists. Fill
        // any absent *directional* registry key with an explicit `INACTIVE`
        // placeholder (normalized 0.0, confidence 0) so the frontend always
        // renders a definitive state. The confluence engine skips `INACTIVE`
        // labels so these placeholders never dilute the weighted average.
        for meta in crate::indicators::registry::INDICATORS {
            if meta.directional && !out.contains_key(meta.key) {
                out.insert(
                    meta.key.into(),
                    NormalizedIndicatorValue::scalar(0.0, 0.0, "INACTIVE"),
                );
            }
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
