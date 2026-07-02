use std::collections::VecDeque;
use rust_decimal::Decimal;

use crate::config::TimeframeConfig;
use crate::config::FibonacciConfig;

use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedCandle, Exchange};
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum, BollingerBands, Atr, DivergenceDetector, FibonacciRange, Bbwp, detect_pattern};

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
    pub divergence_detector: DivergenceDetector,
    pub vwap_sum_tp_vol: Decimal,
    pub vwap_sum_vol: Decimal,
    pub volume_history: VecDeque<Decimal>,
    pub history: Vec<NormalizedCandle>,
    pub last_day_index: Option<u64>,
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
    let mut divergence_detector = DivergenceDetector::new(20);

    let mut vwap_sum_tp_vol = Decimal::ZERO;
    let mut vwap_sum_vol = Decimal::ZERO;
    let mut last_day_index: Option<u64> = None;
    let mut volume_history: VecDeque<Decimal> = VecDeque::with_capacity(20);

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

        let typical_price = (completed.high + completed.low + completed.close) / Decimal::from(3);
        vwap_sum_tp_vol += typical_price * completed.volume;
        vwap_sum_vol += completed.volume;

        let final_vwap = if vwap_sum_vol > Decimal::ZERO {
            Some(vwap_sum_tp_vol / vwap_sum_vol)
        } else {
            None
        };

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
            final_ema_fast, final_ema_medium, final_ema_slow, final_ema_long,
            ema_stack_state,
            final_rsi,
            &final_macd,
            final_adx.as_ref(),
            final_sqz.as_ref(),
            final_bb,
            final_atr.as_ref(),
            final_bbwp,
            rsi_divergence,
            macd_divergence,
            &volume_history,
            fib_config,
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
        divergence_detector,
        vwap_sum_tp_vol,
        vwap_sum_vol,
        volume_history,
        history,
        last_day_index,
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
    rsi_divergence: shared::indicators::DivergenceState,
    macd_divergence: shared::indicators::DivergenceState,
    volume_history: &VecDeque<Decimal>,
    fib_config: &FibonacciConfig,
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

    let indicators = crate::analyzer::normalize::build_indicator_map(
        crate::analyzer::normalize::NormalizeParams {
            close: completed.close,
            rsi: final_rsi,
            rsi_divergence,
            macd_divergence,
            macd: final_macd,
            sqz: final_sqz,
            adx: final_adx,
            bb: final_bb,
            atr: final_atr,
            bbwp: final_bbwp,
            vwap: final_vwap,
            ema_stack_state: ema_stack_state.as_deref(),
            ema_fast: Some(final_ema_fast),
            ema_medium: Some(final_ema_medium),
            ema_slow: Some(final_ema_slow),
            ema_long: Some(final_ema_long),
            rvol,
            fib: Some(&fib),
            pattern: Some(&pattern_result),
            support_levels: &[],
            resistance_levels: &[],
            active_position: None,
            adx_consecutive_deceleration: false,
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
        open: Some(completed.open),
        high: Some(completed.high),
        low: Some(completed.low),
        close: Some(completed.close),
        volume: Some(completed.volume),
        average_volume: avg_vol,
        indicators,
    }
}
