//! Bridge between the stateless indicator calculators and the centralized
//! [`NormalizationEngine`]. Builds the unified `indicators` map attached to
//! every [`MarketSnapshot`].

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;

use shared::indicators::{
    AdxOutput, AtrOutput, CrossoverDir, DivergenceResult, DivergenceState, DivergenceStatus,
    DivergenceType, FibonacciRange, IndicatorInputs, MacdOutput, NormalizationContext,
    NormalizationEngine, NormalizedIndicatorValue, PatternResult, SeriesDivergenceResult,
    SqueezeOutput,
};
use shared::indicators::normalized::PreviousBarState;

#[inline]
fn d2f(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}

#[inline]
fn od2f(d: Option<Decimal>) -> Option<f64> {
    d.and_then(|x| x.to_f64())
}

/// Map an RSI [`DivergenceResult`] to the engine's [`DivergenceState`].
pub fn rsi_divergence_state(div: &DivergenceResult) -> DivergenceState {
    let bullish = matches!(div.rsi_divergence, DivergenceType::RsiBullish | DivergenceType::RsiBullishHidden);
    let bearish = matches!(div.rsi_divergence, DivergenceType::RsiBearish | DivergenceType::RsiBearishHidden);
    match div.rsi_status {
        DivergenceStatus::Confirmed if bullish => DivergenceState::ConfirmedBullish,
        DivergenceStatus::Confirmed if bearish => DivergenceState::ConfirmedBearish,
        DivergenceStatus::Potential if bullish => DivergenceState::PotentialBullish,
        DivergenceStatus::Potential if bearish => DivergenceState::PotentialBearish,
        _ => DivergenceState::None,
    }
}

/// Map a MACD [`DivergenceResult`] to the engine's [`DivergenceState`].
pub fn macd_divergence_state(div: &DivergenceResult) -> DivergenceState {
    let bullish = matches!(div.macd_divergence, DivergenceType::MacdBullish | DivergenceType::MacdBullishHidden);
    let bearish = matches!(div.macd_divergence, DivergenceType::MacdBearish | DivergenceType::MacdBearishHidden);
    match div.macd_status {
        DivergenceStatus::Confirmed if bullish => DivergenceState::ConfirmedBullish,
        DivergenceStatus::Confirmed if bearish => DivergenceState::ConfirmedBearish,
        DivergenceStatus::Potential if bullish => DivergenceState::PotentialBullish,
        DivergenceStatus::Potential if bearish => DivergenceState::PotentialBearish,
        _ => DivergenceState::None,
    }
}

/// Bundle of generalized divergence states for the extra oscillators, mapped
/// into the normalization inputs. `None` when not computed (e.g. live ticks).
#[derive(Debug, Clone, Copy, Default)]
pub struct ExtraDivergence {
    pub stochastic: DivergenceState,
    pub chandemo: DivergenceState,
    pub mfi: DivergenceState,
    pub cmf: DivergenceState,
    pub obv: DivergenceState,
    pub squeeze: DivergenceState,
}

/// Raw indicator outputs + stateful context needed to normalize a snapshot.
pub struct NormalizeParams<'a> {
    pub close: Decimal,
    pub rsi: Option<Decimal>,
    pub rsi_divergence: DivergenceState,
    pub macd_divergence: DivergenceState,
    pub stoch_k: Option<Decimal>,
    pub stoch_d: Option<Decimal>,
    pub chandemo: Option<Decimal>,
    pub supertrend_line: Option<Decimal>,
    pub supertrend_dir: Option<i8>,
    pub keltner: Option<(Decimal, Decimal, Decimal)>,
    pub donchian: Option<(Decimal, Decimal, Decimal)>,
    pub obv: Option<Decimal>,
    pub obv_sma: Option<Decimal>,
    pub cmf: Option<Decimal>,
    pub mfi: Option<Decimal>,
    pub hv: Option<Decimal>,
    pub aroon_up: Option<Decimal>,
    pub aroon_down: Option<Decimal>,
    pub choppiness: Option<Decimal>,
    pub linreg_slope: Option<Decimal>,
    pub zscore: Option<Decimal>,
    pub extra_div: ExtraDivergence,
    pub macd: &'a MacdOutput,
    pub sqz: Option<&'a SqueezeOutput>,
    pub adx: Option<&'a AdxOutput>,
    pub bb: Option<(Decimal, Decimal, Decimal)>,
    pub atr: Option<&'a AtrOutput>,
    pub bbwp: Option<Decimal>,
    pub vwap: Option<Decimal>,
    pub anchored_vwap: Option<shared::indicators::AvwapOutput>,
    pub ema_stack_state: Option<&'a str>,
    pub ema_fast: Option<Decimal>,
    pub ema_medium: Option<Decimal>,
    pub ema_slow: Option<Decimal>,
    pub ema_long: Option<Decimal>,
    pub rvol: Option<Decimal>,
    pub volume: Option<Decimal>,
    pub average_volume: Option<Decimal>,
    pub fib: Option<&'a FibonacciRange>,
    pub pattern: Option<&'a PatternResult>,
    pub support_levels: &'a [f64],
    pub resistance_levels: &'a [f64],
    pub active_position: Option<i8>,
    pub adx_consecutive_deceleration: bool,
    pub supertrend_flipped: bool,
    pub adx_di_crossover: Option<i8>,
    /// Session pivot levels (P/R1-3/S1-3), None until the first session finalizes.
    pub pivot_levels: Option<shared::indicators::PivotLevels>,
    pub pivot_proximity_pct: f64,
    /// Candlestick recognition reading for this bar (Stage 1 + Stage 3 output).
    pub candlestick: Option<shared::indicators::CandlestickResult>,
    pub candlestick_min_confidence: f64,
    /// Ichimoku Cloud output for this bar.
    pub ichimoku: Option<shared::indicators::IchimokuOutput>,
    /// CCI reading for this bar.
    pub cci: Option<Decimal>,
    /// PSAR reading for this bar.
    pub psar: Option<shared::indicators::PsarOutput>,
    /// Williams %R reading for this bar.
    pub williams_r: Option<Decimal>,
    /// Awesome Oscillator reading for this bar.
    pub awesome_oscillator: Option<shared::indicators::AoOutput>,
    /// Force Index reading for this bar.
    pub force_index: Option<Decimal>,
    /// Hull MA reading for this bar.
    pub hull_ma: Option<Decimal>,
    /// StdDev Channel reading for this bar.
    pub stddev_channel: Option<shared::indicators::SdChannelOutput>,
    /// Volume Profile reading for this bar.
    pub volume_profile: Option<shared::indicators::VolumeProfileOutput>,
    /// Smart Money Concepts reading for this bar.
    pub smc: Option<shared::indicators::SmcOutput>,
    pub prev: PreviousBarState,
}

/// Convert a generic series-divergence direction into a (potential) engine
/// [`DivergenceState`]. Confirmation upgrades are handled by the RSI/MACD
/// detector; the generalized oscillators surface potential divergences.
pub fn series_divergence_state(res: &SeriesDivergenceResult) -> DivergenceState {
    match res.direction {
        1 => DivergenceState::PotentialBullish,
        -1 => DivergenceState::PotentialBearish,
        _ => DivergenceState::None,
    }
}

/// Generalized divergence confirmation: wraps `series_divergence_state` and
/// upgrades `Potential → Confirmed` when the candle close decisively breaks
/// the nearest support (bullish divergence) or resistance (bearish) level,
/// matching the 0.2% tolerance used by `DivergenceDetector::check_divergence_confirmation`.
pub fn series_divergence_confirmed(
    res: &SeriesDivergenceResult,
    close: Decimal,
    supports: &[f64],
    resistances: &[f64],
) -> DivergenceState {
    let potential = series_divergence_state(res);
    let d = match potential {
        DivergenceState::PotentialBullish => 1i8,
        DivergenceState::PotentialBearish => -1i8,
        _ => return potential,
    };
    let close_f = d2f(close);
    if close_f <= 0.0 {
        return potential;
    }
    let tolerance_pct = 0.002;
    let confirmed = if d > 0 {
        // Bullish: close decisively below a support level.
        supports.iter().any(|s| {
            let buf = s * tolerance_pct;
            *s > 0.0 && close_f < *s && (*s - close_f) > buf
        })
    } else {
        // Bearish: close decisively above a resistance level.
        resistances.iter().any(|r| {
            let buf = r * tolerance_pct;
            *r > 0.0 && close_f > *r && (close_f - *r) > buf
        })
    };
    if confirmed {
        if d > 0 {
            DivergenceState::ConfirmedBullish
        } else {
            DivergenceState::ConfirmedBearish
        }
    } else {
        potential
    }
}

/// Consolidate raw indicator outputs into the unified normalized map.
pub fn build_indicator_map(p: NormalizeParams) -> HashMap<String, NormalizedIndicatorValue> {
    let price = d2f(p.close);
    let ema_bias = match p.ema_stack_state {
        Some("bullish") => 1i8,
        Some("bearish") => -1i8,
        _ => 0i8,
    };
    // Direction-aware bias: an active position overrides the EMA-stack bias so
    // BBWP / Fibonacci sign relative to the trade the operator actually holds.
    let trend_bias = match p.active_position {
        Some(pos) if pos != 0 => pos,
        _ => ema_bias,
    };

    let macd_crossover = p.macd.crossover.map(|c| match c {
        CrossoverDir::Bullish => 1i8,
        CrossoverDir::Bearish => -1i8,
    });

    let inputs = IndicatorInputs {
        rsi: od2f(p.rsi),
        rsi_divergence: p.rsi_divergence,
        macd_divergence: p.macd_divergence,
        stoch_k: od2f(p.stoch_k),
        stoch_d: od2f(p.stoch_d),
        chandemo: od2f(p.chandemo),
        supertrend_line: od2f(p.supertrend_line),
        supertrend_dir: p.supertrend_dir,
        supertrend_flipped: p.supertrend_flipped,
        keltner_upper: p.keltner.map(|k| d2f(k.0)),
        keltner_middle: p.keltner.map(|k| d2f(k.1)),
        keltner_lower: p.keltner.map(|k| d2f(k.2)),
        donchian_upper: p.donchian.map(|d| d2f(d.0)),
        donchian_middle: p.donchian.map(|d| d2f(d.1)),
        donchian_lower: p.donchian.map(|d| d2f(d.2)),
        obv: od2f(p.obv),
        obv_sma: od2f(p.obv_sma),
        cmf: od2f(p.cmf),
        mfi: od2f(p.mfi),
        hv: od2f(p.hv),
        aroon_up: od2f(p.aroon_up),
        aroon_down: od2f(p.aroon_down),
        choppiness: od2f(p.choppiness),
        linreg_slope: od2f(p.linreg_slope),
        zscore: od2f(p.zscore),
        stochastic_divergence: p.extra_div.stochastic,
        chandemo_divergence: p.extra_div.chandemo,
        mfi_divergence: p.extra_div.mfi,
        cmf_divergence: p.extra_div.cmf,
        obv_divergence: p.extra_div.obv,
        squeeze_divergence: p.extra_div.squeeze,
        macd_line: Some(d2f(p.macd.macd_line)),
        macd_signal: Some(d2f(p.macd.signal_line)),
        macd_histogram: Some(d2f(p.macd.histogram)),
        macd_histogram_peak: Some(d2f(p.macd.histogram_peak)),
        macd_crossover,
        squeeze_on: p.sqz.map(|s| s.squeeze_on),
        squeeze_release_trigger: p.sqz.map(|s| s.squeeze_release_trigger).unwrap_or(false),
        squeeze_momentum: p.sqz.map(|s| d2f(s.momentum_value)),
        squeeze_direction: p.sqz.map(|s| s.momentum_direction),
        adx: p.adx.map(|a| d2f(a.adx)),
        adx_plus_di: p.adx.map(|a| d2f(a.plus_di)),
        adx_minus_di: p.adx.map(|a| d2f(a.minus_di)),
        adx_slope: p.adx.map(|a| d2f(a.adx_slope)),
        adx_di_crossover: p.adx_di_crossover,
        bbwp: od2f(p.bbwp),
        rvol: od2f(p.rvol),
        vwap: od2f(p.vwap),
        avwap_weekly: p.anchored_vwap.as_ref().and_then(|a| a.vwap_weekly.map(|v| d2f(v))),
        avwap_monthly: p.anchored_vwap.as_ref().and_then(|a| a.vwap_monthly.map(|v| d2f(v))),
        avwap_swing: p.anchored_vwap.as_ref().and_then(|a| a.vwap_swing.map(|v| d2f(v))),
        fib_gp_low: p.fib.and_then(|f| od2f(f.golden_pocket_low)),
        fib_gp_high: p.fib.and_then(|f| od2f(f.golden_pocket_high)),
        fib_ext_1618: p.fib.and_then(|f| od2f(f.ext_1618)),
        fib_ext_2618: p.fib.and_then(|f| od2f(f.ext_2618)),
        pattern_bullish: p.pattern.map(|pt| pt.is_bullish).unwrap_or(false),
        pattern_bearish: p.pattern.map(|pt| pt.is_bearish).unwrap_or(false),
        pattern_confidence: p.pattern.map(|pt| pt.confidence),
        pattern_upper_slope: p.pattern.and_then(|pt| pt.upper_slope),
        pattern_upper_intercept: p.pattern.and_then(|pt| pt.upper_intercept),
        pattern_lower_slope: p.pattern.and_then(|pt| pt.lower_slope),
        pattern_lower_intercept: p.pattern.and_then(|pt| pt.lower_intercept),
        atr_14: p.atr.map(|a| d2f(a.atr_value)),
        atr_slope: p.atr.map(|a| d2f(a.atr_slope)),
        atr_regime: p.atr.map(|a| a.volatility_regime),
        bb_upper: p.bb.map(|b| d2f(b.0)),
        bb_middle: p.bb.map(|b| d2f(b.1)),
        bb_lower: p.bb.map(|b| d2f(b.2)),
        ema_fast: od2f(p.ema_fast),
        ema_medium: od2f(p.ema_medium),
        pivot: p.pivot_levels.map(|lv| d2f(lv.pivot)),
        pivot_r1: p.pivot_levels.map(|lv| d2f(lv.r1)),
        pivot_r2: p.pivot_levels.map(|lv| d2f(lv.r2)),
        pivot_r3: p.pivot_levels.map(|lv| d2f(lv.r3)),
        pivot_s1: p.pivot_levels.map(|lv| d2f(lv.s1)),
        pivot_s2: p.pivot_levels.map(|lv| d2f(lv.s2)),
        pivot_s3: p.pivot_levels.map(|lv| d2f(lv.s3)),
        pivot_proximity_pct: p.pivot_proximity_pct,
        candlestick: p.candlestick,
        candlestick_min_confidence: p.candlestick_min_confidence,
        ichimoku_tenkan: p.ichimoku.as_ref().map(|i| d2f(i.tenkan)),
        ichimoku_kijun: p.ichimoku.as_ref().map(|i| d2f(i.kijun)),
        ichimoku_senkou_a: p.ichimoku.as_ref().map(|i| d2f(i.senkou_a)),
        ichimoku_senkou_b: p.ichimoku.as_ref().map(|i| d2f(i.senkou_b)),
        ichimoku_chikou: p.ichimoku.as_ref().map(|i| d2f(i.chikou)),
        ichimoku_senkou_a_current: p.ichimoku.as_ref().map(|i| d2f(i.senkou_a_current)),
        ichimoku_senkou_b_current: p.ichimoku.as_ref().map(|i| d2f(i.senkou_b_current)),
        cci: od2f(p.cci),
        psar_sar: p.psar.as_ref().map(|p| d2f(p.sar)),
        psar_direction: p.psar.map(|p| p.direction),
        psar_flipped: p.psar.map(|p| p.flipped).unwrap_or(false),
        williams_r: od2f(p.williams_r),
        awesome_oscillator: p.awesome_oscillator.as_ref().map(|a| d2f(a.value)),
        ao_rising: p.awesome_oscillator.map(|a| a.rising).unwrap_or(false),
        force_index: od2f(p.force_index),
        hull_ma: od2f(p.hull_ma),
        stddev_upper: p.stddev_channel.as_ref().map(|s| d2f(s.upper)),
        stddev_center: p.stddev_channel.as_ref().map(|s| d2f(s.center)),
        stddev_lower: p.stddev_channel.as_ref().map(|s| d2f(s.lower)),
        volprofile_poc: p.volume_profile.as_ref().map(|vp| d2f(vp.poc)),
        volprofile_vah: p.volume_profile.as_ref().map(|vp| d2f(vp.vah)),
        volprofile_val: p.volume_profile.as_ref().map(|vp| d2f(vp.val)),
        volprofile_total_volume: p.volume_profile.as_ref().map(|vp| d2f(vp.total_volume)).unwrap_or(0.0),
        smc_structure_bullish: p.smc.as_ref().map(|s| matches!(s.structure, shared::indicators::MarketStructure::Bullish)).unwrap_or(false),
        smc_structure_bearish: p.smc.as_ref().map(|s| matches!(s.structure, shared::indicators::MarketStructure::Bearish)).unwrap_or(false),
        smc_bos_bullish: p.smc.as_ref().map(|s| s.bos_bullish).unwrap_or(false),
        smc_bos_bearish: p.smc.as_ref().map(|s| s.bos_bearish).unwrap_or(false),
        smc_choch_bullish: p.smc.as_ref().map(|s| s.choch_bullish).unwrap_or(false),
        smc_choch_bearish: p.smc.as_ref().map(|s| s.choch_bearish).unwrap_or(false),
        smc_liq_sweep_buy: p.smc.as_ref().map(|s| s.liquidity_sweep_buy).unwrap_or(false),
        smc_liq_sweep_sell: p.smc.as_ref().map(|s| s.liquidity_sweep_sell).unwrap_or(false),
        smc_ob_bullish_high: p.smc.as_ref().and_then(|s| od2f(s.active_ob_bullish_high)),
        smc_ob_bullish_low: p.smc.as_ref().and_then(|s| od2f(s.active_ob_bullish_low)),
        smc_ob_bearish_high: p.smc.as_ref().and_then(|s| od2f(s.active_ob_bearish_high)),
        smc_ob_bearish_low: p.smc.as_ref().and_then(|s| od2f(s.active_ob_bearish_low)),
        smc_fvg_top: p.smc.as_ref().and_then(|s| od2f(s.fvg_top)),
        smc_fvg_bottom: p.smc.as_ref().and_then(|s| od2f(s.fvg_bottom)),
        smc_fvg_bullish: p.smc.as_ref().map(|s| s.fvg_bullish).unwrap_or(false),
        smc_premium_discount: p.smc.as_ref().map(|s| s.premium_discount).unwrap_or(0.0),
    };

    let ctx = NormalizationContext {
        trend_bias,
        price,
        vwap: od2f(p.vwap),
        active_position: p.active_position,
        ema_stack_state: p.ema_stack_state.map(|s| s.to_string()),
        ema_medium: od2f(p.ema_medium),
        support_levels: p.support_levels.to_vec(),
        resistance_levels: p.resistance_levels.to_vec(),
        rvol: od2f(p.rvol),
        adx_consecutive_deceleration: p.adx_consecutive_deceleration,
        prev: p.prev,
    };

    let mut map = NormalizationEngine::normalize_all(&inputs, &ctx);
    inject_ema_values(
        &mut map,
        od2f(p.ema_fast),
        od2f(p.ema_medium),
        od2f(p.ema_slow),
        od2f(p.ema_long),
    );
    inject_volume(&mut map, od2f(p.volume), od2f(p.average_volume));

    // Preserve the raw Fibonacci resting levels on the fibonacci entry so they
    // can be persisted to dedicated DB columns and rendered on charts.
    if let (Some(fibr), Some(entry)) = (p.fib, map.get_mut("fibonacci")) {
        let mut vals = entry.values.take().unwrap_or_default();
        if let Some(v) = od2f(fibr.golden_pocket_high) {
            vals.insert("gp_top".to_string(), v);
        }
        if let Some(v) = od2f(fibr.golden_pocket_low) {
            vals.insert("gp_bottom".to_string(), v);
        }
        if let Some(v) = od2f(fibr.ext_1618) {
            vals.insert("ext_1618".to_string(), v);
        }
        if let Some(v) = od2f(fibr.ext_2618) {
            vals.insert("ext_2618".to_string(), v);
        }
        entry.values = Some(vals);
    }

    map
}

/// Attach the raw EMA ribbon line values to the `ema_stack` entry so chart
/// consumers can still render the ribbon.
fn inject_ema_values(
    map: &mut HashMap<String, NormalizedIndicatorValue>,
    fast: Option<f64>,
    medium: Option<f64>,
    slow: Option<f64>,
    long: Option<f64>,
) {
    if let Some(entry) = map.get_mut("ema_stack") {
        let mut vals = entry.values.take().unwrap_or_default();
        if let Some(f) = fast {
            vals.insert("fast".to_string(), f);
        }
        if let Some(m) = medium {
            vals.insert("medium".to_string(), m);
        }
        if let Some(s) = slow {
            vals.insert("slow".to_string(), s);
        }
        if let Some(l) = long {
            vals.insert("long".to_string(), l);
        }
        entry.values = Some(vals);
    }
}

/// Inject a raw-only `volume` entry (non-directional participation gate). The
/// normalized score stays 0.0; the state label reflects participation vs the
/// rolling average.
fn inject_volume(
    map: &mut HashMap<String, NormalizedIndicatorValue>,
    volume: Option<f64>,
    avg_volume: Option<f64>,
) {
    if let Some(vol) = volume {
        let label = match avg_volume {
            Some(avg) if avg > 0.0 && vol >= avg * 2.0 => "VOLUME_CLIMAX",
            Some(avg) if avg > 0.0 && vol >= avg * 1.5 => "HIGH_PARTICIPATION",
            Some(avg) if avg > 0.0 && vol < avg * 0.5 => "LOW_PARTICIPATION",
            _ => "NORMAL_PARTICIPATION",
        };
        let mut values = HashMap::new();
        if let Some(avg) = avg_volume {
            values.insert("average".to_string(), avg);
        }
        let mut niv = NormalizedIndicatorValue::with_values(vol, 0.0, label, values);
        if label == "VOLUME_CLIMAX" {
            use shared::indicators::normalized::{IndicatorSignal, SignalDirection, SignalKind, SignalStatus};
            niv.signals.push(IndicatorSignal::new(
                SignalKind::VolumeClimax, SignalDirection::Neutral, SignalStatus::Active, "VOLUME_CLIMAX",
            ));
        }
        map.insert("volume".into(), niv);
    }
}

/// Flat raw indicator scalars (e.g. reconstructed from persisted DB columns).
#[derive(Debug, Clone, Default)]
pub struct RawScalarInputs {
    pub close: f64,
    pub rsi: Option<f64>,
    pub macd_line: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_hist: Option<f64>,
    pub adx: Option<f64>,
    pub adx_plus: Option<f64>,
    pub adx_minus: Option<f64>,
    pub bbwp: Option<f64>,
    pub bb_upper: Option<f64>,
    pub bb_middle: Option<f64>,
    pub bb_lower: Option<f64>,
    pub atr: Option<f64>,
    pub vwap: Option<f64>,
    pub ema_fast: Option<f64>,
    pub ema_medium: Option<f64>,
    pub ema_slow: Option<f64>,
    pub ema_long: Option<f64>,
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<f64>,
    pub rvol: Option<f64>,
}

/// Reconstruct the normalized indicator map from flat raw scalars. Crossover,
/// divergence, squeeze direction, Fibonacci, and pattern context are
/// unavailable from flat storage and default to neutral.
pub fn build_indicator_map_from_scalars(
    s: RawScalarInputs,
) -> HashMap<String, NormalizedIndicatorValue> {
    let ema_stack_state = match (s.ema_fast, s.ema_medium, s.ema_slow, s.ema_long) {
        (Some(f), Some(m), Some(sl), Some(l)) => {
            if f > m && m > sl && sl > l && s.close > f {
                Some("bullish".to_string())
            } else if f < m && m < sl && sl < l && s.close < f {
                Some("bearish".to_string())
            } else {
                Some("tangled".to_string())
            }
        }
        _ => None,
    };
    let trend_bias = match ema_stack_state.as_deref() {
        Some("bullish") => 1i8,
        Some("bearish") => -1i8,
        _ => 0i8,
    };

    let inputs = IndicatorInputs {
        rsi: s.rsi,
        macd_line: s.macd_line,
        macd_signal: s.macd_signal,
        macd_histogram: s.macd_hist,
        macd_histogram_peak: s.macd_hist,
        squeeze_on: s.squeeze_on,
        squeeze_momentum: s.squeeze_momentum,
        adx: s.adx,
        adx_plus_di: s.adx_plus,
        adx_minus_di: s.adx_minus,
        bbwp: s.bbwp,
        rvol: s.rvol,
        vwap: s.vwap,
        atr_14: s.atr,
        bb_upper: s.bb_upper,
        bb_middle: s.bb_middle,
        bb_lower: s.bb_lower,
        ..Default::default()
    };
    let ctx = NormalizationContext {
        trend_bias,
        price: s.close,
        vwap: s.vwap,
        ema_stack_state,
        ema_medium: s.ema_medium,
        rvol: s.rvol,
        ..Default::default()
    };

    let mut map = NormalizationEngine::normalize_all(&inputs, &ctx);
    inject_ema_values(&mut map, s.ema_fast, s.ema_medium, s.ema_slow, s.ema_long);
    map
}
