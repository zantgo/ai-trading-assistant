use std::collections::VecDeque;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::config::TimeframeConfig;
use crate::config::FibonacciConfig;

use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedCandle, Exchange};
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum, BollingerBands, Atr, DivergenceDetector, SeriesDivergence, FibonacciRange, Bbwp, Stochastic, ChandeMO, Supertrend, Keltner, Donchian, Obv, Cmf, Mfi, HistoricalVolatility, Aroon, Choppiness, LinRegSlope, ZScore, detect_pattern, PivotPoints, PivotMethod, Candlestick, CandlestickConfig, Ichimoku, Cci, ParabolicSar, WilliamsR, HullMA, AwesomeOscillator, ForceIndex, StdDevChannel, VolumeProfile, SmartMoney, AnchoredVwap};
use shared::indicators::normalized::PreviousBarState;
use crate::analyzer::normalize::{series_divergence_state, ExtraDivergence};
use crate::sr_engine::SrRoleTracker;
use crate::analyzer::update_sr_levels;

/// Maximum number of candles/snapshots retained in live memory buffers.
/// Bootstrap fetches up to `analysis_limit` (default 500); live buffers grow
/// naturally up to this hard cap before eviction.
pub const HIST_BUFFER_MAX: usize = 1000;

/// Pre-warmed indicator state produced by feeding historical candles through
/// all technical indicators before live WebSocket ingestion begins.
#[derive(Clone)]
pub struct WarmedPipelineState {
    pub ema_fast: Ema,
    pub ema_medium: Ema,
    pub ema_slow: Ema,
    pub ema_long: Ema,
    pub rsi_14: Rsi,
    pub macd: Macd,
    pub adx_14: Adx,
    pub sqz_mom: SqueezeMomentum,
    pub bollinger: BollingerBands,
    pub atr_standalone: Atr,
    pub bbwp_indicator: Bbwp,
    pub stochastic_indicator: Stochastic,
    pub chandemo_indicator: ChandeMO,
    pub supertrend_indicator: Supertrend,
    pub keltner_indicator: Keltner,
    pub donchian_indicator: Donchian,
    pub obv_indicator: Obv,
    pub cmf_indicator: Cmf,
    pub mfi_indicator: Mfi,
    pub hv_indicator: HistoricalVolatility,
    pub aroon_indicator: Aroon,
    pub choppiness_indicator: Choppiness,
    pub linreg_indicator: LinRegSlope,
    pub zscore_indicator: ZScore,
    pub divergence_detector: DivergenceDetector,
    pub stoch_div: SeriesDivergence,
    pub chandemo_div: SeriesDivergence,
    pub mfi_div: SeriesDivergence,
    pub cmf_div: SeriesDivergence,
    pub obv_div: SeriesDivergence,
    pub squeeze_div: SeriesDivergence,
    pub vwap_sum_tp_vol: Decimal,
    pub vwap_sum_vol: Decimal,
    pub volume_history: VecDeque<Decimal>,
    pub history: Vec<NormalizedCandle>,
    pub last_day_index: Option<u64>,
    pub sr_tracker: SrRoleTracker,
    pub pivot_points_indicator: PivotPoints,
    pub candlestick_indicator: Candlestick,
    pub ichimoku_indicator: Ichimoku,
    pub cci_indicator: Cci,
    pub psar_indicator: ParabolicSar,
    pub wr_indicator: WilliamsR,
    pub hma_indicator: HullMA,
    pub ao_indicator: AwesomeOscillator,
    pub fi_indicator: ForceIndex,
    pub sdc_indicator: StdDevChannel,
    pub volume_profile_indicator: VolumeProfile,
    pub smc_indicator: SmartMoney,
    pub anchored_vwap_indicator: AnchoredVwap,
    pub latest_snapshot: Option<MarketSnapshot>,
    pub snapshot_history: Vec<MarketSnapshot>,
}

/// Feed a sequence of historical candles through all technical indicators in
/// chronological order, returning fully warmed state ready for live ingestion.
pub fn warm_indicators_for_timeframe(
    mut candles: Vec<NormalizedCandle>,
    tf_config: &TimeframeConfig,
    fib_config: &FibonacciConfig,
    symbol: &str,
    timeframe_secs: u64,
) -> WarmedPipelineState {
    let active_indicators = tf_config.indicators.clone();

    let mut ema_fast = Ema::new(active_indicators.ema_fast);
    let mut ema_medium = Ema::new(active_indicators.ema_medium);
    let mut ema_slow = Ema::new(active_indicators.ema_slow);
    let mut ema_long = Ema::new(active_indicators.ema_long);
    let mut rsi_14 = Rsi::new(active_indicators.rsi_period);

    let mut macd = Macd::new();
    let mut adx_14 = Adx::new(active_indicators.adx_period);
    adx_14.set_thresholds(
        Decimal::from(active_indicators.adx_trend_threshold),
        Decimal::from(active_indicators.adx_exhaustion_threshold),
        active_indicators.adx_slope_lookback,
    );
    let mut sqz_mom = SqueezeMomentum::new(active_indicators.squeeze_period);
    sqz_mom.set_min_duration(active_indicators.squeeze_min_duration);
    let mut bollinger = BollingerBands::new(20);
    let mut atr_standalone = Atr::new(active_indicators.atr_period);
    let mut bbwp_indicator = Bbwp::new(active_indicators.bbwp_lookback, active_indicators.bbwp_period);
    let mut stochastic_indicator = Stochastic::new(
        active_indicators.stoch_k_period,
        active_indicators.stoch_d_period,
        active_indicators.stoch_s_period,
    );
    let mut chandemo_indicator = ChandeMO::new(active_indicators.chandemo_period);
    let mut supertrend_indicator = Supertrend::new(
        active_indicators.supertrend_period,
        active_indicators.supertrend_multiplier,
    );
    let mut keltner_indicator = Keltner::new(
        active_indicators.keltner_ema_period,
        active_indicators.keltner_atr_period,
        active_indicators.keltner_multiplier,
    );
    let mut donchian_indicator = Donchian::new(active_indicators.donchian_period);
    let mut obv_indicator = Obv::new(active_indicators.obv_smoothing);
    let mut cmf_indicator = Cmf::new(active_indicators.cmf_period);
    let mut mfi_indicator = Mfi::new(active_indicators.mfi_period);
    let mut hv_indicator = HistoricalVolatility::new(active_indicators.hv_period);
    let mut aroon_indicator = Aroon::new(active_indicators.aroon_period);
    let mut choppiness_indicator = Choppiness::new(active_indicators.chop_period);
    let mut linreg_indicator = LinRegSlope::new(active_indicators.linreg_period);
    let mut zscore_indicator = ZScore::new(active_indicators.zscore_period);
    let mut divergence_detector = DivergenceDetector::new(20);
    let mut stoch_div = SeriesDivergence::new(20);
    let mut chandemo_div = SeriesDivergence::new(20);
    let mut mfi_div = SeriesDivergence::new(20);
    let mut cmf_div = SeriesDivergence::new(20);
    let mut obv_div = SeriesDivergence::new(20);
    let mut squeeze_div = SeriesDivergence::new(20);

    let mut vwap_sum_tp_vol = Decimal::ZERO;
    let mut vwap_sum_vol = Decimal::ZERO;
    let mut last_day_index: Option<u64> = None;
    let mut volume_history: VecDeque<Decimal> = VecDeque::with_capacity(20);
    // S/R role-reversal tracker, warmed through the full history so live
    // ingestion inherits accurate flip-state (matches run_single tolerance).
    let mut sr_tracker = SrRoleTracker::new(0.003);
    // Session Pivot Points, warmed so live ingestion inherits published levels.
    let mut pivot_points_indicator = PivotPoints::new(PivotMethod::Classic);
    // Candlestick recognizer, warmed so its pending-confirmation buffer is live.
    let mut candlestick_indicator = Candlestick::new(CandlestickConfig::default());
    // Ichimoku Cloud, warmed so live ingestion inherits the 52-bar window.
    let mut ichimoku_indicator = Ichimoku::new(
        active_indicators.ichimoku_tenkan,
        active_indicators.ichimoku_kijun,
        active_indicators.ichimoku_senkou_b,
        active_indicators.ichimoku_displacement,
    );
    let mut cci_indicator = Cci::new(active_indicators.cci_period);
    let mut psar_indicator = ParabolicSar::new(active_indicators.psar_af_step, active_indicators.psar_af_max);
    let mut wr_indicator = WilliamsR::new(active_indicators.williams_r_period);
    let mut hma_indicator = HullMA::new(active_indicators.hull_ma_period);
    let mut ao_indicator = AwesomeOscillator::new();
    let mut fi_indicator = ForceIndex::new(active_indicators.force_index_smoothing);
    let mut sdc_indicator = StdDevChannel::new(active_indicators.stddev_channel_period);
    let mut volume_profile_indicator = VolumeProfile::new(active_indicators.volume_profile_window, active_indicators.volume_profile_bins, active_indicators.volume_profile_value_area);
    let mut smc_indicator = SmartMoney::new(active_indicators.smc_lookback);

    let mut anchored_vwap_indicator = AnchoredVwap::new();

    let mut latest_snapshot: Option<MarketSnapshot> = None;

    // Sort candles chronologically (oldest first)
    candles.sort_by_key(|c| c.start_time_ms);

    let analysis_limit = tf_config.candles.analysis_limit;
    let mut snapshot_history: Vec<MarketSnapshot> = Vec::with_capacity(analysis_limit);

    // Feed each historical candle through every indicator sequentially
    for completed in &candles {
        let candle_close_sec = completed.start_time_ms / 1000;
        let day_index = candle_close_sec / 86400;
        if let Some(prev_day) = last_day_index {
            if day_index > prev_day {
                vwap_sum_tp_vol = Decimal::ZERO;
                vwap_sum_vol = Decimal::ZERO;
            }
        }
        last_day_index = Some(day_index);

        // Session Pivot Points: accumulate H/L/C; publish on day rollover.
        let pivot_levels =
            pivot_points_indicator.update(completed.high, completed.low, completed.close, day_index);

        // Candlestick recognition (warmed through history).
        let candlestick_reading =
            candlestick_indicator.update(completed.open, completed.high, completed.low, completed.close);

        // Ichimoku Cloud (warmed through history).
        let ichimoku_reading =
            ichimoku_indicator.update(completed.high, completed.low, completed.close);

        // CCI (warmed through history).
        let cci_reading = cci_indicator.update(completed.high, completed.low, completed.close);

        // Parabolic SAR (warmed through history).
        let psar_reading = psar_indicator.update(completed.high, completed.low);

        let wr_reading = wr_indicator.update(completed.high, completed.low, completed.close);
        let hma_reading = hma_indicator.update(completed.close);
        let ao_reading = ao_indicator.update(completed.high, completed.low);
        let fi_reading = fi_indicator.update(completed.close, completed.volume);
        let sdc_reading = sdc_indicator.update(completed.close);

        let volume_profile_reading = volume_profile_indicator.update(completed.high, completed.low, completed.close, completed.volume);
        let smc_reading = smc_indicator.update(completed.open, completed.high, completed.low, completed.close);

        let typical_price = (completed.high + completed.low + completed.close) / Decimal::from(3);
        vwap_sum_tp_vol += typical_price * completed.volume;
        vwap_sum_vol += completed.volume;

        let final_vwap = if vwap_sum_vol > Decimal::ZERO {
            Some(vwap_sum_tp_vol / vwap_sum_vol)
        } else {
            None
        };

        let avwap_reading = anchored_vwap_indicator.update(
            completed.high, completed.low, completed.close, completed.volume,
            day_index,
            final_vwap.unwrap_or(Decimal::ZERO),
        );

        let final_ema_fast = ema_fast.update(completed.close);
        let final_ema_medium = ema_medium.update(completed.close);
        let final_ema_slow = ema_slow.update(completed.close);
        let final_ema_long = ema_long.update(completed.close);

        let ema_stack_state = if final_ema_fast > final_ema_medium
            && final_ema_medium > final_ema_slow
            && final_ema_slow > final_ema_long
            && completed.close > final_ema_fast
        {
            Some("bullish".to_string())
        } else if final_ema_fast < final_ema_medium
            && final_ema_medium < final_ema_slow
            && final_ema_slow < final_ema_long
            && completed.close < final_ema_fast
        {
            Some("bearish".to_string())
        } else {
            Some("tangled".to_string())
        };

        let final_rsi = rsi_14.update(completed.close);
        let final_macd = macd.update(completed.close);
        let final_adx = adx_14.update(completed.high, completed.low, completed.close);
        let final_sqz = sqz_mom.update(completed.high, completed.low, completed.close);
        let final_bb = bollinger.update(completed.close);
        let final_atr = atr_standalone.update(completed.high, completed.low, completed.close);
        let final_bbwp = bbwp_indicator.update(completed.close);
        let final_stoch = stochastic_indicator.update(completed.high, completed.low, completed.close);
        let final_cmo = chandemo_indicator.update(completed.close);
        let final_supertrend = supertrend_indicator.update(completed.high, completed.low, completed.close);
        let final_keltner = keltner_indicator.update(completed.high, completed.low, completed.close);
        let final_donchian = donchian_indicator.update(completed.high, completed.low);
        let final_obv = obv_indicator.update(completed.close, completed.volume);
        let final_cmf = cmf_indicator.update(completed.high, completed.low, completed.close, completed.volume);
        let final_mfi = mfi_indicator.update(completed.high, completed.low, completed.close, completed.volume);
        let final_hv = hv_indicator.update(completed.close);
        let final_aroon = aroon_indicator.update(completed.high, completed.low);
        let final_chop = choppiness_indicator.update(completed.high, completed.low, completed.close);
        let final_linreg = linreg_indicator.update(completed.close);
        let final_zscore = zscore_indicator.update(completed.close);

        let extra_div = ExtraDivergence {
            stochastic: final_stoch.as_ref().map(|s| series_divergence_state(&stoch_div.update(completed.close, s.k_value))).unwrap_or_default(),
            chandemo: final_cmo.map(|v| series_divergence_state(&chandemo_div.update(completed.close, v))).unwrap_or_default(),
            mfi: final_mfi.map(|v| series_divergence_state(&mfi_div.update(completed.close, v))).unwrap_or_default(),
            cmf: final_cmf.map(|v| series_divergence_state(&cmf_div.update(completed.close, v))).unwrap_or_default(),
            obv: final_obv.as_ref().map(|o| series_divergence_state(&obv_div.update(completed.close, o.obv))).unwrap_or_default(),
            squeeze: final_sqz.as_ref().map(|s| series_divergence_state(&squeeze_div.update(completed.close, s.momentum_value))).unwrap_or_default(),
        };

        let div_result = if let (Some(rsi), macd_hist) = (final_rsi, final_macd.histogram) {
            divergence_detector.update_full(completed.close, rsi, macd_hist)
        } else {
            shared::indicators::DivergenceResult::default_div()
        };
        let rsi_divergence = crate::analyzer::normalize::rsi_divergence_state(&div_result);
        let macd_divergence = crate::analyzer::normalize::macd_divergence_state(&div_result);

        volume_history.push_back(completed.volume);
        if volume_history.len() > 20 {
            volume_history.pop_front();
        }

        // Build snapshot using only the history accumulated so far
        let snapshot = build_historical_snapshot(
            &candles,
            completed,
            symbol,
            timeframe_secs,
            final_vwap,
            avwap_reading,
            final_ema_fast, final_ema_medium, final_ema_slow, final_ema_long,
            ema_stack_state,
            final_rsi,
            &final_macd,
            final_adx.as_ref(),
            final_sqz.as_ref(),
            final_bb,
            final_atr.as_ref(),
            final_bbwp,
            final_stoch.as_ref(),
            final_cmo,
            final_supertrend.as_ref(),
            final_keltner.as_ref(),
            final_donchian.as_ref(),
            final_obv.as_ref(),
            final_cmf,
            final_mfi,
            final_hv,
            final_aroon.as_ref(),
            final_chop,
            final_linreg,
            final_zscore,
            extra_div,
            rsi_divergence,
            macd_divergence,
            &volume_history,
            fib_config,
            &mut sr_tracker,
            pivot_levels,
            Some(candlestick_reading),
            ichimoku_reading,
            cci_reading,
            psar_reading,
            wr_reading, hma_reading, ao_reading, fi_reading, sdc_reading,
            volume_profile_reading,
            smc_reading,
        );

        latest_snapshot = Some(snapshot.clone());
        snapshot_history.push(snapshot);
    }

    // Limit history to analysis_limit, keeping the most recent candles
    let history: Vec<NormalizedCandle> = if candles.len() > analysis_limit {
        candles[candles.len() - analysis_limit..].to_vec()
    } else {
        candles
    };

    // Trim snapshot_history to match
    if snapshot_history.len() > analysis_limit {
        snapshot_history = snapshot_history[snapshot_history.len() - analysis_limit..].to_vec();
    }

    WarmedPipelineState {
        ema_fast,
        ema_medium,
        ema_slow,
        ema_long,
        rsi_14,
        macd,
        adx_14,
        sqz_mom,
        bollinger,
        atr_standalone,
        bbwp_indicator,
        stochastic_indicator,
        chandemo_indicator,
        supertrend_indicator,
        keltner_indicator,
        donchian_indicator,
        obv_indicator,
        cmf_indicator,
        mfi_indicator,
        hv_indicator,
        aroon_indicator,
        choppiness_indicator,
        linreg_indicator,
        zscore_indicator,
        divergence_detector,
        stoch_div,
        chandemo_div,
        mfi_div,
        cmf_div,
        obv_div,
        squeeze_div,
        vwap_sum_tp_vol,
        vwap_sum_vol,
        volume_history,
        history,
        last_day_index,
        sr_tracker,
        pivot_points_indicator,
        candlestick_indicator,
        ichimoku_indicator,
        cci_indicator,
        psar_indicator,
        wr_indicator,
        hma_indicator,
        ao_indicator,
        fi_indicator,
        sdc_indicator,
        volume_profile_indicator,
        smc_indicator,
        anchored_vwap_indicator,
        latest_snapshot,
        snapshot_history,
    }
}

/// Build a `MarketSnapshot` from indicator state during historical pre-warming.
#[allow(clippy::too_many_arguments)]
fn build_historical_snapshot(
    all_candles: &[NormalizedCandle],
    completed: &NormalizedCandle,
    symbol: &str,
    timeframe_secs: u64,
    final_vwap: Option<Decimal>,
    avwap_reading: shared::indicators::AvwapOutput,
    final_ema_fast: Decimal,
    final_ema_medium: Decimal,
    final_ema_slow: Decimal,
    final_ema_long: Decimal,
    ema_stack_state: Option<String>,
    final_rsi: Option<Decimal>,
    final_macd: &shared::indicators::MacdOutput,
    final_adx: Option<&shared::indicators::AdxOutput>,
    final_sqz: Option<&shared::indicators::SqueezeOutput>,
    final_bb: Option<(Decimal, Decimal, Decimal)>,
    final_atr: Option<&shared::indicators::AtrOutput>,
    final_bbwp: Option<Decimal>,
    final_stoch: Option<&shared::indicators::StochasticOutput>,
    final_cmo: Option<Decimal>,
    final_supertrend: Option<&shared::indicators::SupertrendOutput>,
    final_keltner: Option<&shared::indicators::KeltnerOutput>,
    final_donchian: Option<&shared::indicators::DonchianOutput>,
    final_obv: Option<&shared::indicators::ObvOutput>,
    final_cmf: Option<Decimal>,
    final_mfi: Option<Decimal>,
    final_hv: Option<Decimal>,
    final_aroon: Option<&shared::indicators::AroonOutput>,
    final_chop: Option<Decimal>,
    final_linreg: Option<Decimal>,
    final_zscore: Option<Decimal>,
    extra_div: ExtraDivergence,
    rsi_divergence: shared::indicators::DivergenceState,
    macd_divergence: shared::indicators::DivergenceState,
    volume_history: &VecDeque<Decimal>,
    fib_config: &FibonacciConfig,
    sr_tracker: &mut SrRoleTracker,
    pivot_levels: Option<shared::indicators::PivotLevels>,
    candlestick: Option<shared::indicators::CandlestickResult>,
    ichimoku: Option<shared::indicators::IchimokuOutput>,
    cci: Option<Decimal>,
    psar: Option<shared::indicators::PsarOutput>,
    wr: Option<Decimal>,
    hma: Option<Decimal>,
    ao: Option<shared::indicators::AoOutput>,
    fi: Option<Decimal>,
    sdc: Option<shared::indicators::SdChannelOutput>,
    volume_profile: Option<shared::indicators::VolumeProfileOutput>,
    smc: Option<shared::indicators::SmcOutput>,
) -> MarketSnapshot {
    let candle_close_sec = completed.start_time_ms / 1000;

    let avg_vol = if !volume_history.is_empty() {
        let sum: Decimal = volume_history.iter().sum();
        Some(sum / Decimal::from(volume_history.len()))
    } else {
        None
    };

    let rvol = match (completed.volume, avg_vol) {
        (vol, Some(avg)) if avg > Decimal::ZERO => Some(vol / avg),
        _ => None,
    };

    let candles_high: Vec<Decimal> = all_candles.iter().map(|c| c.high).collect();
    let candles_low: Vec<Decimal> = all_candles.iter().map(|c| c.low).collect();

    let fib = FibonacciRange::compute_from_candles(
        &candles_high, &candles_low,
        fib_config.swing_lookback,
        fib_config.swing_scan_range,
        &fib_config.retracement_coefficients,
        &fib_config.extension_coefficients,
    );

    let pivots = FibonacciRange::detect_pivots(
        &candles_high, &candles_low,
        fib_config.swing_lookback,
        fib_config.swing_scan_range,
    );
    let pattern_result = detect_pattern(&pivots);

    // Support/Resistance zones: role-adjusted levels from the swing pivots,
    // updating the flip tracker on this historical close.
    let (sr_supports, sr_resistances) =
        update_sr_levels(sr_tracker, &pivots, completed.close, candle_close_sec);

    let indicators = crate::analyzer::normalize::build_indicator_map(
        crate::analyzer::normalize::NormalizeParams {
            close: completed.close,
            rsi: final_rsi,
            rsi_divergence,
            macd_divergence,
            stoch_k: final_stoch.map(|s| s.k_value),
            stoch_d: final_stoch.map(|s| s.d_value),
            chandemo: final_cmo,
            supertrend_line: final_supertrend.map(|s| s.line),
            supertrend_dir: final_supertrend.map(|s| s.direction),
            keltner: final_keltner.map(|k| (k.upper, k.middle, k.lower)),
            donchian: final_donchian.map(|d| (d.upper, d.middle, d.lower)),
            obv: final_obv.map(|o| o.obv),
            obv_sma: final_obv.map(|o| o.obv_sma),
            cmf: final_cmf,
            mfi: final_mfi,
            hv: final_hv,
            aroon_up: final_aroon.map(|a| a.up),
            aroon_down: final_aroon.map(|a| a.down),
            choppiness: final_chop,
            linreg_slope: final_linreg,
            zscore: final_zscore,
            extra_div,
            macd: final_macd,
            sqz: final_sqz,
            adx: final_adx,
            bb: final_bb,
            atr: final_atr,
            bbwp: final_bbwp,
            vwap: final_vwap,
            anchored_vwap: Some(avwap_reading),
            ema_stack_state: ema_stack_state.as_deref(),
            ema_fast: Some(final_ema_fast),
            ema_medium: Some(final_ema_medium),
            ema_slow: Some(final_ema_slow),
            ema_long: Some(final_ema_long),
            rvol,
            volume: Some(completed.volume),
            average_volume: avg_vol,
            fib: Some(&fib),
            pattern: Some(&pattern_result),
            support_levels: &sr_supports,
            resistance_levels: &sr_resistances,
            active_position: None,
            adx_consecutive_deceleration: false,
            supertrend_flipped: false,
            adx_di_crossover: None,
            pivot_levels,
            pivot_proximity_pct: 0.0015,
            candlestick,
            candlestick_min_confidence: 0.3,
            ichimoku,
            cci,
            psar,
            williams_r: wr,
            awesome_oscillator: ao,
            force_index: fi,
            hull_ma: hma,
            stddev_channel: sdc,
            volume_profile,
            smc,
            prev: PreviousBarState::default(),
        },
    );

    MarketSnapshot {
        exchange: Some(Exchange::Hyperliquid),
        timeframe_secs,
        timestamp: candle_close_sec,
        symbol: symbol.to_string(),
        is_completed: Some(true),
        mid_price: completed.close,
        bid_price: Decimal::ZERO,
        ask_price: Decimal::ZERO,
        bid_size: Some(completed.volume),
        ask_size: Some(completed.volume),
        funding_rate: None,
        open_interest: None,
        oi_delta_1h: None,
        mark_price: None,
        index_price: None,
        mark_index_spread_pct: None,
        prev_day_px: None,
        open: Some(completed.open),
        high: Some(completed.high),
        low: Some(completed.low),
        close: Some(completed.close),
        volume: Some(completed.volume),
        average_volume: avg_vol,
        context: Some(shared::market_context::MarketContext::synthesize(&indicators)),
        decision_context: Some({
            let atr_val = indicators.get("atr").map(|v| v.raw_value).unwrap_or(0.0);
        let mut sum = 0.0f64; let mut n = 0u32;
        for meta in shared::indicators::registry::INDICATORS {
            if meta.directional {
                if let Some(v) = indicators.get(meta.key) {
                    sum += v.normalized;
                    n += 1;
                }
            }
        }
        let conf = if n > 0 { (sum / n as f64 * 100.0).clamp(-100.0, 100.0) } else { 0.0 };
            let px = completed.close.to_f64().unwrap_or(0.0);
            shared::decision_context::DecisionContext::compute(&indicators, px, atr_val, conf)
        }),
        statistical_context: None,
        indicators,
        alignment: None,
                        risk: None,
        analysis: None,
                        advisory: None,
        risk_profile: None,
        liquidity: None,
        cluster: None,
    }
}
