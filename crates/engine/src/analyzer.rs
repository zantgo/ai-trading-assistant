use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{broadcast, RwLock};
use rust_decimal::Decimal;
use tokio_util::sync::CancellationToken;

use crate::config::TimeframeConfig;
use crate::config::FibonacciConfig;
use crate::risk;
use crate::db;

use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedEvent, NormalizedCandle, Exchange, CandleGenerator};
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum, BollingerBands, Atr, DivergenceDetector, FibonacciRange, Bbwp, detect_pattern};
use crate::sr_engine::SrRoleTracker;

pub struct TimeframePipeline {
    pub history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub broadcast_tx: broadcast::Sender<MarketSnapshot>,
    pub latest_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
    pub timeframe_secs: u64,
    pub timeframe_label: &'static str,
    pub divergence_detector: Arc<tokio::sync::Mutex<DivergenceDetector>>,
    pub sr_tracker: Arc<tokio::sync::Mutex<SrRoleTracker>>,
    pub fibonacci: FibonacciConfig,
}

pub struct ActivePair {
    pub symbol: String,
    pub short: TimeframePipeline,
    pub mid: TimeframePipeline,
    pub long: TimeframePipeline,
    pub r#macro: TimeframePipeline,
    pub supermacro: TimeframePipeline,
    pub snapshot_tx: tokio::sync::mpsc::Sender<NormalizedEvent>,
    pub cancel: CancellationToken,
}

pub async fn run_event_router(
    mut rx: Receiver<NormalizedEvent>,
    short_tx: Sender<NormalizedEvent>,
    mid_tx: Sender<NormalizedEvent>,
    long_tx: Sender<NormalizedEvent>,
    macro_tx: Sender<NormalizedEvent>,
    supermacro_tx: Sender<NormalizedEvent>,
    symbol: String,
    cancel: CancellationToken,
) {
    println!("🔄 Event Router: Started for {} (fanning out to 5 timeframes)...", symbol);

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

        let _ = short_tx.send(event.clone()).await;
        let _ = mid_tx.send(event.clone()).await;
        let _ = long_tx.send(event.clone()).await;
        let _ = macro_tx.send(event.clone()).await;
        let _ = supermacro_tx.send(event).await;
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
    symbol: String,
    pair_key: String,
    timeframe_secs: u64,
    timeframe_label: &'static str,
    cancel: CancellationToken,
    candle_forward: Option<tokio::sync::mpsc::UnboundedSender<NormalizedCandle>>,
) {
    println!(
        "📊 Analysis Task: Started {} ({}) — {} ({})s candles...",
        symbol, pair_key, timeframe_label, tf_config.candles.duration_seconds
    );

    let active_indicators = tf_config.indicators.clone();

    let mut ema_fast = Ema::new(active_indicators.ema_fast);
    let mut ema_medium = Ema::new(active_indicators.ema_medium);
    let mut ema_slow = Ema::new(active_indicators.ema_slow);
    let mut ema_long = Ema::new(active_indicators.ema_long);
    let mut rsi_14 = Rsi::new(active_indicators.rsi_period);

    let mut macd = Macd::new();
    let mut adx_14 = Adx::new(active_indicators.adx_period);
    adx_14.set_thresholds(
        rust_decimal::Decimal::from(active_indicators.adx_trend_threshold),
        rust_decimal::Decimal::from(active_indicators.adx_exhaustion_threshold),
        active_indicators.adx_slope_lookback,
    );
    let mut sqz_mom = SqueezeMomentum::new(active_indicators.squeeze_period);
    sqz_mom.set_min_duration(active_indicators.squeeze_min_duration);
    let mut bollinger = BollingerBands::new();
    let mut atr_standalone = Atr::new(active_indicators.atr_period);
    let mut bbwp_indicator = Bbwp::new(active_indicators.bbwp_lookback, active_indicators.bbwp_period);

    let mut candle_gen = CandleGenerator::new(&symbol, tf_config.candles.duration_seconds);

    let mut vwap_sum_tp_vol = Decimal::ZERO;
    let mut vwap_sum_vol = Decimal::ZERO;
    let mut last_day_index: Option<u64> = None;
    let mut volume_history: VecDeque<Decimal> = VecDeque::with_capacity(20);

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

        let cur_candles = &tf_config.candles;

        {
            let mut hist = history.write().await;
            while hist.len() > cur_candles.analysis_limit {
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
                    let div_result = {
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
                        *snap = Some(completed_snapshot);
                    }

                    {
                        let mut hist = history.write().await;
                        hist.push_back(completed.clone());
                        while hist.len() > cur_candles.analysis_limit {
                            hist.pop_front();
                        }
                    }

                    if let Some(ref tx) = candle_forward {
                        let _ = tx.send(completed.clone());
                    }
                }

                // FAST PATH: Risk check on trade price
                {
                    let tick = MarketSnapshot {
                        exchange: shadow_exchange,
                        timeframe_secs,
                        timestamp: trade.timestamp_ms / 1000,
                        symbol: symbol.clone(),
                        is_completed: Some(false),
                        mid_price: trade.price,
                        bid_price: shadow_bid,
                        ask_price: shadow_ask,
                        bid_size: Some(trade.size),
                        ask_size: Some(trade.size),
                        funding_rate: None,
                        open: None, high: None, low: None, close: None,
                        volume: None, average_volume: None, rvol: None,
                        bb_upper: None, bb_middle: None, bb_lower: None,
                        atr_14: None, atr_slope: None, atr_volatility_regime: None, atr_stop_loss_level: None, atr_take_profit_level: None,
                        vwap: None, vwap_bias: None,
                        adx_14: None, adx_plus: None, adx_minus: None,
                        ema_fast: None, ema_medium: None, ema_slow: None, ema_long: None, ema_stack_state: None,
                        rsi_14: None, macd_line: None, macd_signal: None, macd_hist: None,
                        squeeze_on: None, squeeze_momentum: None,
                        squeeze_duration: None, squeeze_release_trigger: None, squeeze_momentum_direction: None,
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
                        macd_histogram_peak: None,
                        macd_trend_state: None,
                        macd_crossover_detected: None,
                        macd_crossover_direction: None,
                        adx_slope: None,
                        adx_peak: None,
                        adx_regime: None,
                        adx_di_crossover_detected: None,
                        adx_di_crossover_direction: None,
                    };
                    risk::check(&tick);
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
