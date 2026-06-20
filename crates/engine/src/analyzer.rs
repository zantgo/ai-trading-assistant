use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{broadcast, RwLock};
use rust_decimal::Decimal;
use tokio_util::sync::CancellationToken;

use crate::config::TimeframeConfig;
use crate::config::FibonacciConfig;
use crate::db;

use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedEvent, NormalizedCandle, Exchange, CandleGenerator};
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum, BollingerBands, Atr, DivergenceDetector, FibonacciRange, Bbwp, detect_pattern};
use crate::sr_engine::SrRoleTracker;

/// Maximum number of candles/snapshots retained in live memory buffers.
/// Bootstrap fetches up to `analysis_limit` (default 500); live buffers grow
/// naturally up to this hard cap before eviction.
pub const HIST_BUFFER_MAX: usize = 1000;

/// Pre-warmed indicator state produced by feeding historical candles through
/// all technical indicators before live WebSocket ingestion begins.
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
    let mut bollinger = BollingerBands::new();
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

        let vwap_bias = match (final_vwap, completed.close) {
            (Some(v), cl) if cl > v * Decimal::new(1001, 3) => Some("premium".to_string()),
            (Some(v), cl) if cl < v * Decimal::new(999, 3) => Some("discount".to_string()),
            (Some(_), _) => Some("equilibrium".to_string()),
            (None, _) => None,
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

        let _div_result = if let (Some(rsi), macd_hist) = (final_rsi, final_macd.histogram) {
            divergence_detector.update_full(completed.close, rsi, macd_hist)
        } else {
            shared::indicators::DivergenceResult::default_div()
        };

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
            final_vwap, vwap_bias,
            final_ema_fast, final_ema_medium, final_ema_slow, final_ema_long,
            ema_stack_state,
            final_rsi,
            &final_macd,
            final_adx.as_ref(),
            final_sqz.as_ref(),
            final_bb,
            final_atr.as_ref(),
            final_bbwp,
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
    vwap_bias: Option<String>,
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
        rvol,
        bb_upper: final_bb.map(|b| b.0),
        bb_middle: final_bb.map(|b| b.1),
        bb_lower: final_bb.map(|b| b.2),
        atr_14: final_atr.map(|a| a.atr_value),
        atr_slope: final_atr.map(|a| a.atr_slope),
        atr_volatility_regime: final_atr.map(|a| format!("{:?}", a.volatility_regime).to_lowercase()),
        atr_stop_loss_level: None,
        atr_take_profit_level: None,
        vwap: final_vwap,
        vwap_bias,
        adx_14: final_adx.map(|a| a.adx),
        adx_plus: final_adx.map(|a| a.plus_di),
        adx_minus: final_adx.map(|a| a.minus_di),
        ema_fast: Some(final_ema_fast),
        ema_medium: Some(final_ema_medium),
        ema_slow: Some(final_ema_slow),
        ema_long: Some(final_ema_long),
        ema_stack_state,
        rsi_14: final_rsi,
        macd_line: Some(final_macd.macd_line),
        macd_signal: Some(final_macd.signal_line),
        macd_hist: Some(final_macd.histogram),
        squeeze_on: final_sqz.map(|s| s.squeeze_on),
        squeeze_momentum: final_sqz.map(|s| s.momentum_value),
        squeeze_duration: final_sqz.map(|s| s.squeeze_duration),
        squeeze_release_trigger: final_sqz.map(|s| s.squeeze_release_trigger),
        squeeze_momentum_direction: final_sqz.map(|s| format!("{:?}", s.momentum_direction)),
        bbwp: final_bbwp,
        support_levels: None,
        resistance_levels: None,
        sr_flip_events: None,
        fib_golden_pocket_low: fib.golden_pocket_low,
        fib_golden_pocket_high: fib.golden_pocket_high,
        fib_extension_1618: fib.ext_1618,
        fib_extension_2618: fib.ext_2618,
        swing_high: fib.swing_high,
        swing_low: fib.swing_low,
        chart_pattern: if pattern_result.pattern != shared::indicators::ChartPattern::None {
            Some(format!("{:?}", pattern_result.pattern))
        } else {
            None
        },
        chart_pattern_confidence: if pattern_result.confidence > 0.0 {
            Some(Decimal::from_f64_retain(pattern_result.confidence).unwrap_or(Decimal::ZERO))
        } else {
            None
        },
        rsi_divergence_status: None,
        rsi_divergence_coords: None,
        macd_divergence_status: None,
        macd_divergence_coords: None,
        macd_histogram_peak: Some(final_macd.histogram_peak),
        macd_trend_state: Some(format!("{:?}", final_macd.trend_state).to_lowercase()),
        macd_crossover_detected: Some(final_macd.crossover.is_some()),
        macd_crossover_direction: final_macd.crossover.map(|c| match c {
            shared::indicators::CrossoverDir::Bullish => "BULLISH",
            shared::indicators::CrossoverDir::Bearish => "BEARISH",
        }.to_string()),
        adx_slope: final_adx.map(|a| a.adx_slope),
        adx_peak: final_adx.map(|a| a.adx_peak),
        adx_regime: final_adx.map(|a| format!("{:?}", a.trending_regime).to_lowercase()),
        adx_di_crossover_detected: final_adx.map(|a| a.di_crossover.is_some()),
        adx_di_crossover_direction: final_adx.and_then(|a| a.di_crossover.map(|c| match c {
            shared::indicators::DiCrossoverDir::Bullish => "BULLISH",
            shared::indicators::DiCrossoverDir::Bearish => "BEARISH",
        }.to_string())),
    }
}

pub struct TimeframePipeline {
    pub history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub broadcast_tx: broadcast::Sender<MarketSnapshot>,
    pub latest_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
    pub snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub timeframe_secs: u64,
    pub timeframe_label: &'static str,
    pub divergence_detector: Arc<tokio::sync::Mutex<DivergenceDetector>>,
    pub sr_tracker: Arc<tokio::sync::Mutex<SrRoleTracker>>,
    pub fibonacci: FibonacciConfig,
}

pub struct ActivePair {
    pub symbol: String,
    pub micro: TimeframePipeline,
    pub short: TimeframePipeline,
    pub medium: TimeframePipeline,
    pub large: TimeframePipeline,
    pub snapshot_tx: tokio::sync::mpsc::Sender<NormalizedEvent>,
    pub cancel: CancellationToken,
}

pub async fn run_event_router(
    mut rx: Receiver<NormalizedEvent>,
    micro_tx: Sender<NormalizedEvent>,
    short_tx: Sender<NormalizedEvent>,
    medium_tx: Sender<NormalizedEvent>,
    large_tx: Sender<NormalizedEvent>,
    symbol: String,
    cancel: CancellationToken,
) {
    println!("🔄 Event Router: Started for {} (fanning out to 4 timeframes)...", symbol);

    loop {
        let event = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                println!("🛑 Event Router: {} cancelled, shutting down.", symbol);
                break;
            }
            result = rx.recv() => {
                match result {
                    Some(e) => e,
                    None => {
                        println!("🛑 Event Router: {} upstream channel closed.", symbol);
                        break;
                    }
                }
            }
        };

        let _ = micro_tx.send(event.clone()).await;
        let _ = short_tx.send(event.clone()).await;
        let _ = medium_tx.send(event.clone()).await;
        let _ = large_tx.send(event).await;
    }
}

pub async fn run_single(
    mut rx: Receiver<NormalizedEvent>,
    telemetry_tx: tokio::sync::mpsc::Sender<db::TelemetryMsg>,
    broadcast_tx: broadcast::Sender<MarketSnapshot>,
    tf_config: TimeframeConfig,
    fib_config: FibonacciConfig,
    divergence_detector: Arc<tokio::sync::Mutex<DivergenceDetector>>,
    history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    latest_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
    snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    symbol: String,
    pair_key: String,
    timeframe_secs: u64,
    timeframe_label: &'static str,
    cancel: CancellationToken,
    candle_forward: Option<tokio::sync::mpsc::UnboundedSender<NormalizedCandle>>,
    warmed: Option<WarmedPipelineState>,
) {
    println!(
        "📊 Analysis Task: Started {} ({}) — {} ({})s candles{}...",
        symbol, pair_key, timeframe_label, tf_config.candles.duration_seconds,
        if warmed.is_some() { " [pre-warmed]" } else { "" }
    );

    let active_indicators = tf_config.indicators.clone();

    let (mut ema_fast, mut ema_medium, mut ema_slow, mut ema_long,
         mut rsi_14, mut macd, mut adx_14, mut sqz_mom,
         mut bollinger, mut atr_standalone, mut bbwp_indicator,
         mut vwap_sum_tp_vol, mut vwap_sum_vol,
         mut last_day_index, mut volume_history);

    if let Some(w) = warmed {
        ema_fast = w.ema_fast;
        ema_medium = w.ema_medium;
        ema_slow = w.ema_slow;
        ema_long = w.ema_long;
        rsi_14 = w.rsi_14;
        macd = w.macd;
        adx_14 = w.adx_14;
        sqz_mom = w.sqz_mom;
        bollinger = w.bollinger;
        atr_standalone = w.atr_standalone;
        bbwp_indicator = w.bbwp_indicator;
        vwap_sum_tp_vol = w.vwap_sum_tp_vol;
        vwap_sum_vol = w.vwap_sum_vol;
        last_day_index = w.last_day_index;
        volume_history = w.volume_history;
        // Pre-populate snapshot_history from warmed state
        {
            let mut snap_hist = snapshot_history.write().await;
            for snap in &w.snapshot_history {
                snap_hist.push_back(snap.clone());
            }
        }
    } else {
        ema_fast = Ema::new(active_indicators.ema_fast);
        ema_medium = Ema::new(active_indicators.ema_medium);
        ema_slow = Ema::new(active_indicators.ema_slow);
        ema_long = Ema::new(active_indicators.ema_long);
        rsi_14 = Rsi::new(active_indicators.rsi_period);
        macd = Macd::new();
        adx_14 = Adx::new(active_indicators.adx_period);
        adx_14.set_thresholds(
            Decimal::from(active_indicators.adx_trend_threshold),
            Decimal::from(active_indicators.adx_exhaustion_threshold),
            active_indicators.adx_slope_lookback,
        );
        sqz_mom = SqueezeMomentum::new(active_indicators.squeeze_period);
        sqz_mom.set_min_duration(active_indicators.squeeze_min_duration);
        bollinger = BollingerBands::new();
        atr_standalone = Atr::new(active_indicators.atr_period);
        bbwp_indicator = Bbwp::new(active_indicators.bbwp_lookback, active_indicators.bbwp_period);
        vwap_sum_tp_vol = Decimal::ZERO;
        vwap_sum_vol = Decimal::ZERO;
        last_day_index = None;
        volume_history = VecDeque::with_capacity(20);
    }

    let mut candle_gen = CandleGenerator::new(&symbol, tf_config.candles.duration_seconds);

    let mut shadow_bid = Decimal::ZERO;
    let mut shadow_ask = Decimal::ZERO;
    #[allow(unused_assignments)]
    let mut shadow_exchange: Option<Exchange> = None;

    loop {
        let event = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                println!("🛑 Analysis Task: {} ({}) cancelled, shutting down.", symbol, timeframe_label);
                break;
            }
            result = rx.recv() => {
                match result {
                    Some(e) => e,
                    None => {
                        println!("🛑 Analysis Task: {} ({}) channel closed.", symbol, timeframe_label);
                        break;
                    }
                }
            }
        };


        {
            let mut hist = history.write().await;
            while hist.len() > HIST_BUFFER_MAX {
                hist.pop_front();
            }
        }

        match event {
            NormalizedEvent::Trade(ref trade) => {
                shadow_exchange = Some(trade.exchange);

                let (completed_opt, live_candle) = candle_gen.process_trade(trade);
                if let Some(completed) = completed_opt {
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

                    let vwap_bias = match (final_vwap, completed.close) {
                        (Some(v), cl) if cl > v * Decimal::new(1001, 3) => Some("premium".to_string()),
                        (Some(v), cl) if cl < v * Decimal::new(999, 3) => Some("discount".to_string()),
                        (Some(_), _) => Some("equilibrium".to_string()),
                        (None, _) => None,
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

                    // Divergence detection (live — potential status)
                    let _div_result = {
                        if let (Some(rsi), macd_hist) = (final_rsi, final_macd.histogram) {
                            let mut det = divergence_detector.lock().await;
                            det.update_full(completed.close, rsi, macd_hist)
                        } else {
                            shared::indicators::DivergenceResult::default_div()
                        }
                    };

                    let log_line = format!(
                        "🕯️  [{}] {} Candle Closed | Start: {} | Close: ${:.4} | Vol: {:.4} | Trades: {}",
                        symbol, timeframe_label, completed.start_time_ms, completed.close,
                        completed.volume, completed.trades_count
                    );
                    let _ = telemetry_tx.send(db::TelemetryMsg::ConsoleLog(log_line)).await;

                    volume_history.push_back(completed.volume);
                    if volume_history.len() > 20 {
                        volume_history.pop_front();
                    }
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

                    // Fibonacci retracement/extension computation
                    let fib = {
                        let hist = history.read().await;
                        let candles_high: Vec<Decimal> = hist.iter().map(|c| c.high).collect();
                        let candles_low: Vec<Decimal> = hist.iter().map(|c| c.low).collect();
                        FibonacciRange::compute_from_candles(
                            &candles_high, &candles_low,
                            fib_config.swing_lookback,
                            fib_config.swing_scan_range,
                            &fib_config.retracement_coefficients,
                            &fib_config.extension_coefficients,
                        )
                    };

                    // Chart pattern detection from pivots
                    let pattern_result = {
                        let hist = history.read().await;
                        let candles_high: Vec<Decimal> = hist.iter().map(|c| c.high).collect();
                        let candles_low: Vec<Decimal> = hist.iter().map(|c| c.low).collect();
                        let pivots = FibonacciRange::detect_pivots(
                            &candles_high, &candles_low,
                            fib_config.swing_lookback,
                            fib_config.swing_scan_range,
                        );
                        detect_pattern(&pivots)
                    };

                    let completed_snapshot = MarketSnapshot {
                        exchange: shadow_exchange,
                        timeframe_secs,
                        timestamp: candle_close_sec,
                        symbol: symbol.clone(),
                        is_completed: Some(true),
                        mid_price: completed.close,
                        bid_price: shadow_bid,
                        ask_price: shadow_ask,
                        bid_size: Some(completed.volume),
                        ask_size: Some(completed.volume),
                        funding_rate: None,
                        open: Some(completed.open),
                        high: Some(completed.high),
                        low: Some(completed.low),
                        close: Some(completed.close),
                        volume: Some(completed.volume),
                        average_volume: avg_vol,
                        rvol,
                        bb_upper: final_bb.map(|b| b.0),
                        bb_middle: final_bb.map(|b| b.1),
                        bb_lower: final_bb.map(|b| b.2),
                        atr_14: final_atr.as_ref().map(|a| a.atr_value),
                        atr_slope: final_atr.as_ref().map(|a| a.atr_slope),
                        atr_volatility_regime: final_atr.as_ref().map(|a| format!("{:?}", a.volatility_regime).to_lowercase()),
                        atr_stop_loss_level: None,
                        atr_take_profit_level: None,
                        vwap: final_vwap,
                        vwap_bias,
                        adx_14: final_adx.as_ref().map(|a| a.adx),
                        adx_plus: final_adx.as_ref().map(|a| a.plus_di),
                        adx_minus: final_adx.as_ref().map(|a| a.minus_di),
                        ema_fast: Some(final_ema_fast),
                        ema_medium: Some(final_ema_medium),
                        ema_slow: Some(final_ema_slow),
                        ema_long: Some(final_ema_long),
                        ema_stack_state,
                        rsi_14: final_rsi,
                        macd_line: Some(final_macd.macd_line),
                        macd_signal: Some(final_macd.signal_line),
                        macd_hist: Some(final_macd.histogram),
                        squeeze_on: final_sqz.as_ref().map(|s| s.squeeze_on),
                        squeeze_momentum: final_sqz.as_ref().map(|s| s.momentum_value),
                        squeeze_duration: final_sqz.as_ref().map(|s| s.squeeze_duration),
                        squeeze_release_trigger: final_sqz.as_ref().map(|s| s.squeeze_release_trigger),
                        squeeze_momentum_direction: final_sqz.as_ref().map(|s| format!("{:?}", s.momentum_direction)),
                        bbwp: final_bbwp,
                        support_levels: None,
                        resistance_levels: None,
                        sr_flip_events: None,
                        fib_golden_pocket_low: fib.golden_pocket_low,
                        fib_golden_pocket_high: fib.golden_pocket_high,
                        fib_extension_1618: fib.ext_1618,
                        fib_extension_2618: fib.ext_2618,
                        swing_high: fib.swing_high,
                        swing_low: fib.swing_low,
                        chart_pattern: if pattern_result.pattern != shared::indicators::ChartPattern::None {
                            Some(format!("{:?}", pattern_result.pattern))
                        } else {
                            None
                        },
                        chart_pattern_confidence: if pattern_result.confidence > 0.0 {
                            Some(rust_decimal::Decimal::from_f64_retain(pattern_result.confidence).unwrap_or(rust_decimal::Decimal::ZERO))
                        } else {
                            None
                        },
                        rsi_divergence_status: None,
                        rsi_divergence_coords: None,
                        macd_divergence_status: None,
                        macd_divergence_coords: None,
                        macd_histogram_peak: Some(final_macd.histogram_peak),
                        macd_trend_state: Some(format!("{:?}", final_macd.trend_state).to_lowercase()),
                        macd_crossover_detected: Some(final_macd.crossover.is_some()),
                        macd_crossover_direction: final_macd.crossover.map(|c| match c {
                            shared::indicators::CrossoverDir::Bullish => "BULLISH",
                            shared::indicators::CrossoverDir::Bearish => "BEARISH",
                        }.to_string()),
                        adx_slope: final_adx.as_ref().map(|a| a.adx_slope),
                        adx_peak: final_adx.as_ref().map(|a| a.adx_peak),
                        adx_regime: final_adx.as_ref().map(|a| format!("{:?}", a.trending_regime).to_lowercase()),
                        adx_di_crossover_detected: final_adx.as_ref().map(|a| a.di_crossover.is_some()),
                        adx_di_crossover_direction: final_adx.as_ref().and_then(|a| a.di_crossover.map(|c| match c {
                            shared::indicators::DiCrossoverDir::Bullish => "BULLISH",
                            shared::indicators::DiCrossoverDir::Bearish => "BEARISH",
                        }.to_string())),
                    };

                    let _ = telemetry_tx.send(db::TelemetryMsg::InsertSnapshot(completed_snapshot.clone())).await;

                    {
                        let mut snap = latest_snapshot.write().await;
                        *snap = Some(completed_snapshot.clone());
                    }

                    // Dedup: if the most recent history entry has the same timestamp as this
                    // completed candle (REST bootstrap/live WebSocket overlap), replace it
                    // and skip forwarding to the aggregator (already sent during bootstrap).
                    let is_duplicate = {
                        let hist = history.read().await;
                        hist.back().map_or(false, |c| c.start_time_ms == completed.start_time_ms)
                    };

                    if is_duplicate {
                        let mut hist = history.write().await;
                        hist.pop_back();
                        hist.push_back(completed.clone());
                        while hist.len() > HIST_BUFFER_MAX {
                            hist.pop_front();
                        }
                        let mut snap_hist = snapshot_history.write().await;
                        snap_hist.pop_back();
                        snap_hist.push_back(completed_snapshot.clone());
                        while snap_hist.len() > HIST_BUFFER_MAX {
                            snap_hist.pop_front();
                        }
                        // Skip candle_forward — aggregator already received the bootstrap candle
                    } else {
                        let mut hist = history.write().await;
                        hist.push_back(completed.clone());
                        while hist.len() > HIST_BUFFER_MAX {
                            hist.pop_front();
                        }
                        let mut snap_hist = snapshot_history.write().await;
                        snap_hist.push_back(completed_snapshot.clone());
                        while snap_hist.len() > HIST_BUFFER_MAX {
                            snap_hist.pop_front();
                        }
                        if let Some(ref tx) = candle_forward {
                            let _ = tx.send(completed.clone());
                        }
                    }
                }

                // BROADCAST: Flickering snapshot from live candle
                broadcast_live_snapshot(
                    &broadcast_tx, &symbol, &live_candle, shadow_exchange,
                    shadow_bid, shadow_ask,
                    &ema_fast, &ema_medium, &ema_slow, &ema_long,
                    &rsi_14, &macd, &adx_14, &sqz_mom,
                    &bollinger, &atr_standalone,
                    &vwap_sum_tp_vol, &vwap_sum_vol,
                    &volume_history,
                    timeframe_secs,
                );
            }

            NormalizedEvent::OrderBook(ref book) => {
                shadow_exchange = Some(book.exchange);
                if let (Some(best_bid), Some(best_ask)) = (book.bids.first(), book.asks.first()) {
                    shadow_bid = best_bid.0;
                    shadow_ask = best_ask.0;
                }

                if candle_gen.current_candle.is_some() {
                    let mid = (shadow_bid + shadow_ask) / Decimal::from(2);
                    let shadow_candle = NormalizedCandle {
                        symbol: symbol.clone(),
                        start_time_ms: candle_gen.current_start_ms,
                        duration_ms: candle_gen.duration_ms,
                        open: candle_gen.current_open,
                        high: candle_gen.current_high.max(mid),
                        low: candle_gen.current_low.min(mid),
                        close: mid,
                        volume: candle_gen.current_volume,
                        trades_count: candle_gen.current_trades,
                    };

                    broadcast_live_snapshot(
                        &broadcast_tx, &symbol, &shadow_candle, shadow_exchange,
                        shadow_bid, shadow_ask,
                        &ema_fast, &ema_medium, &ema_slow, &ema_long,
                        &rsi_14, &macd, &adx_14, &sqz_mom,
                        &bollinger, &atr_standalone,
                        &vwap_sum_tp_vol, &vwap_sum_vol,
                        &volume_history,
                        timeframe_secs,
                    );
                }
            }

            NormalizedEvent::Status { exchange, status, message } => {
                println!("[STATUS {}] {}: {:?} — {}", timeframe_label, exchange, status, message);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn broadcast_live_snapshot(
    broadcast_tx: &broadcast::Sender<MarketSnapshot>,
    symbol: &str,
    candle: &NormalizedCandle,
    exchange: Option<Exchange>,
    bid_price: Decimal,
    ask_price: Decimal,
    ema_fast: &Ema,
    ema_medium: &Ema,
    ema_slow: &Ema,
    ema_long: &Ema,
    rsi_14: &Rsi,
    macd: &Macd,
    adx_14: &Adx,
    sqz_mom: &SqueezeMomentum,
    bollinger: &BollingerBands,
    atr_standalone: &Atr,
    vwap_sum_tp_vol: &Decimal,
    vwap_sum_vol: &Decimal,
    volume_history: &VecDeque<Decimal>,
    timeframe_secs: u64,
) {
    let val_ema_fast = ema_fast.clone().update(candle.close);
    let val_ema_medium = ema_medium.clone().update(candle.close);
    let val_ema_slow = ema_slow.clone().update(candle.close);
    let val_ema_long = ema_long.clone().update(candle.close);
    let val_rsi = rsi_14.clone().update(candle.close);
    let val_macd = macd.clone().update(candle.close);
    let val_adx = adx_14.clone().update(candle.high, candle.low, candle.close);
    let val_sqz = sqz_mom.clone().update(candle.high, candle.low, candle.close);
    let val_bb = bollinger.clone().update(candle.close);
    let val_atr = atr_standalone.clone().update(candle.high, candle.low, candle.close);

    let typical_price = (candle.high + candle.low + candle.close) / Decimal::from(3);
    let temp_sum_tp_vol = *vwap_sum_tp_vol + typical_price * candle.volume;
    let temp_sum_vol = *vwap_sum_vol + candle.volume;
    let val_vwap = if temp_sum_vol > Decimal::ZERO {
        Some(temp_sum_tp_vol / temp_sum_vol)
    } else {
        None
    };

    let avg_vol = if !volume_history.is_empty() {
        let sum: Decimal = volume_history.iter().sum();
        Some(sum / Decimal::from(volume_history.len()))
    } else {
        None
    };

    let snapshot = MarketSnapshot {
        exchange,
        timeframe_secs,
        timestamp: candle.start_time_ms / 1000,
        symbol: symbol.to_string(),
        is_completed: Some(false),
        mid_price: candle.close,
        bid_price,
        ask_price,
        bid_size: Some(candle.volume),
        ask_size: Some(candle.volume),
        funding_rate: None,
        open: Some(candle.open),
        high: Some(candle.high),
        low: Some(candle.low),
        close: Some(candle.close),
        volume: Some(candle.volume),
        average_volume: avg_vol,
        rvol: None,
        bb_upper: val_bb.map(|b| b.0),
        bb_middle: val_bb.map(|b| b.1),
        bb_lower: val_bb.map(|b| b.2),
        atr_14: val_atr.as_ref().map(|a| a.atr_value),
        atr_slope: val_atr.as_ref().map(|a| a.atr_slope),
        atr_volatility_regime: val_atr.as_ref().map(|a| format!("{:?}", a.volatility_regime).to_lowercase()),
        atr_stop_loss_level: None,
        atr_take_profit_level: None,
        vwap: val_vwap,
        vwap_bias: None,
        adx_14: val_adx.as_ref().map(|a| a.adx),
        adx_plus: val_adx.as_ref().map(|a| a.plus_di),
        adx_minus: val_adx.as_ref().map(|a| a.minus_di),
        ema_fast: Some(val_ema_fast),
        ema_medium: Some(val_ema_medium),
        ema_slow: Some(val_ema_slow),
        ema_long: Some(val_ema_long),
        ema_stack_state: None,
        rsi_14: val_rsi,
        macd_line: Some(val_macd.macd_line),
        macd_signal: Some(val_macd.signal_line),
        macd_hist: Some(val_macd.histogram),
        squeeze_on: val_sqz.as_ref().map(|s| s.squeeze_on),
        squeeze_momentum: val_sqz.as_ref().map(|s| s.momentum_value),
        squeeze_duration: val_sqz.as_ref().map(|s| s.squeeze_duration),
        squeeze_release_trigger: val_sqz.as_ref().map(|s| s.squeeze_release_trigger),
        squeeze_momentum_direction: val_sqz.as_ref().map(|s| format!("{:?}", s.momentum_direction)),
        bbwp: None,
                        support_levels: None,
                        resistance_levels: None,
                        sr_flip_events: None,
                        fib_golden_pocket_low: None,
                        fib_golden_pocket_high: None,
                        fib_extension_1618: None,
                        fib_extension_2618: None,
                        swing_high: None,
                        swing_low: None,
                        chart_pattern: None,
                        chart_pattern_confidence: None,
                        rsi_divergence_status: None,
                        rsi_divergence_coords: None,
                        macd_divergence_status: None,
                        macd_divergence_coords: None,
        macd_histogram_peak: Some(val_macd.histogram_peak),
        macd_trend_state: Some(format!("{:?}", val_macd.trend_state).to_lowercase()),
                        macd_crossover_detected: Some(val_macd.crossover.is_some()),
                        macd_crossover_direction: val_macd.crossover.map(|c| match c {
                            shared::indicators::CrossoverDir::Bullish => "BULLISH",
                            shared::indicators::CrossoverDir::Bearish => "BEARISH",
                        }.to_string()),
                        adx_slope: val_adx.as_ref().map(|a| a.adx_slope),
                        adx_peak: val_adx.as_ref().map(|a| a.adx_peak),
                        adx_regime: val_adx.as_ref().map(|a| format!("{:?}", a.trending_regime).to_lowercase()),
                        adx_di_crossover_detected: val_adx.as_ref().map(|a| a.di_crossover.is_some()),
                        adx_di_crossover_direction: val_adx.as_ref().and_then(|a| a.di_crossover.map(|c| match c {
                            shared::indicators::DiCrossoverDir::Bullish => "BULLISH",
                            shared::indicators::DiCrossoverDir::Bearish => "BEARISH",
                        }.to_string())),
                    };

    let _ = broadcast_tx.send(snapshot);
}
