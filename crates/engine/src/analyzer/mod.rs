use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{broadcast, RwLock};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use tokio_util::sync::CancellationToken;

use crate::config::TimeframeConfig;
use crate::config::FibonacciConfig;
use crate::db;

use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedEvent, NormalizedCandle, Exchange, CandleGenerator};
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum, BollingerBands, Atr, DivergenceDetector, SeriesDivergence, FibonacciRange, Bbwp, Stochastic, ChandeMO, Supertrend, Keltner, Donchian, Obv, Cmf, Mfi, HistoricalVolatility, Aroon, Choppiness, LinRegSlope, ZScore, detect_pattern};
use crate::sr_engine::SrRoleTracker;

pub mod normalize;
pub mod warm;
pub use warm::{HIST_BUFFER_MAX, WarmedPipelineState, warm_indicators_for_timeframe};

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
    pub fast: TimeframePipeline,
    pub slow: TimeframePipeline,
    pub r#macro: TimeframePipeline,
    pub snapshot_tx: tokio::sync::mpsc::Sender<NormalizedEvent>,
    pub cancel: CancellationToken,
}

impl ActivePair {
    fn pipeline_for(&self, timeframe_secs: u64) -> &TimeframePipeline {
        if self.fast.timeframe_secs == timeframe_secs { return &self.fast; }
        if self.slow.timeframe_secs == timeframe_secs { return &self.slow; }
        if self.r#macro.timeframe_secs == timeframe_secs { return &self.r#macro; }
        &self.micro
    }

    pub fn subscribe_broadcast(&self, timeframe_secs: u64) -> broadcast::Receiver<MarketSnapshot> {
        self.pipeline_for(timeframe_secs).broadcast_tx.subscribe()
    }

    pub async fn latest_close_str(&self) -> Option<String> {
        let hist = self.micro.history.read().await;
        hist.back().map(|c| c.close.to_string())
    }

    pub async fn latest_price(&self) -> Option<f64> {
        let snap = self.micro.latest_snapshot.read().await;
        snap.as_ref().and_then(|s| s.mid_price.to_string().parse::<f64>().ok())
    }

    pub async fn snapshot_history_vec(&self, timeframe_secs: u64) -> Vec<MarketSnapshot> {
        let hist = self.pipeline_for(timeframe_secs).snapshot_history.read().await;
        hist.iter().cloned().collect()
    }

    /// Latest completed snapshot for each of the four timeframes
    /// (micro, fast, slow, macro), for cross-timeframe synthesis.
    pub async fn latest_snapshots_all_tf(
        &self,
    ) -> (
        Option<MarketSnapshot>,
        Option<MarketSnapshot>,
        Option<MarketSnapshot>,
        Option<MarketSnapshot>,
    ) {
        (
            self.micro.latest_snapshot.read().await.clone(),
            self.fast.latest_snapshot.read().await.clone(),
            self.slow.latest_snapshot.read().await.clone(),
            self.r#macro.latest_snapshot.read().await.clone(),
        )
    }
}

pub async fn run_event_router(
    mut rx: Receiver<NormalizedEvent>,
    micro_tx: Sender<NormalizedEvent>,
    fast_tx: Sender<NormalizedEvent>,
    slow_tx: Sender<NormalizedEvent>,
    macro_tx: Sender<NormalizedEvent>,
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
        let _ = fast_tx.send(event.clone()).await;
        let _ = slow_tx.send(event.clone()).await;
        let _ = macro_tx.send(event).await;
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
    candle_forward: Option<tokio::sync::mpsc::Sender<NormalizedCandle>>,
    warmed: Option<WarmedPipelineState>,
    paper_pool: Option<sqlx::SqlitePool>,
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
         mut stochastic_indicator, mut chandemo_indicator,
         mut supertrend_indicator, mut keltner_indicator, mut donchian_indicator,
         mut obv_indicator, mut cmf_indicator, mut mfi_indicator, mut hv_indicator,
         mut aroon_indicator, mut choppiness_indicator, mut linreg_indicator, mut zscore_indicator,
         mut stoch_div, mut chandemo_div, mut mfi_div, mut cmf_div, mut obv_div, mut squeeze_div,
         mut vwap_sum_tp_vol, mut vwap_sum_vol,
         mut last_day_index, mut volume_history);

    // Strict chronological handover boundary: the start time of the newest
    // historical (REST/DB) candle used for pre-warming. Live candles at or
    // before this timestamp are discarded so partially-filled live wicks cannot
    // overwrite complete historical data or corrupt stateful indicators.
    // Defaults to 0 (no gate) for cold / sub-minute / non-warmed pipelines.
    let t_last_hist: u64 = warmed
        .as_ref()
        .and_then(|w| w.history.last().map(|c| c.start_time_ms))
        .unwrap_or(0);

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
        stochastic_indicator = w.stochastic_indicator;
        chandemo_indicator = w.chandemo_indicator;
        supertrend_indicator = w.supertrend_indicator;
        keltner_indicator = w.keltner_indicator;
        donchian_indicator = w.donchian_indicator;
        obv_indicator = w.obv_indicator;
        cmf_indicator = w.cmf_indicator;
        mfi_indicator = w.mfi_indicator;
        hv_indicator = w.hv_indicator;
        aroon_indicator = w.aroon_indicator;
        choppiness_indicator = w.choppiness_indicator;
        linreg_indicator = w.linreg_indicator;
        zscore_indicator = w.zscore_indicator;
        stoch_div = w.stoch_div;
        chandemo_div = w.chandemo_div;
        mfi_div = w.mfi_div;
        cmf_div = w.cmf_div;
        obv_div = w.obv_div;
        squeeze_div = w.squeeze_div;
        vwap_sum_tp_vol = w.vwap_sum_tp_vol;
        vwap_sum_vol = w.vwap_sum_vol;
        last_day_index = w.last_day_index;
        volume_history = w.volume_history;

        // Pre-populate history from warmed state
        {
            let mut hist = history.write().await;
            for c in &w.history {
                hist.push_back(c.clone());
            }
        }
        // Pre-populate latest_snapshot from warmed state
        if let Some(ref snap) = w.latest_snapshot {
            *latest_snapshot.write().await = Some(snap.clone());
        }
        // Pre-populate snapshot_history from warmed state
        {
            let mut snap_hist = snapshot_history.write().await;
            for snap in &w.snapshot_history {
                snap_hist.push_back(snap.clone());
            }
        }
        // Pre-populate divergence detector state
        {
            let mut det = divergence_detector.lock().await;
            *det = w.divergence_detector.clone();
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
        bollinger = BollingerBands::new(20);
        atr_standalone = Atr::new(active_indicators.atr_period);
        bbwp_indicator = Bbwp::new(active_indicators.bbwp_lookback, active_indicators.bbwp_period);
        stochastic_indicator = Stochastic::new(
            active_indicators.stoch_k_period,
            active_indicators.stoch_d_period,
            active_indicators.stoch_s_period,
        );
        chandemo_indicator = ChandeMO::new(active_indicators.chandemo_period);
        supertrend_indicator = Supertrend::new(
            active_indicators.supertrend_period,
            active_indicators.supertrend_multiplier,
        );
        keltner_indicator = Keltner::new(
            active_indicators.keltner_ema_period,
            active_indicators.keltner_atr_period,
            active_indicators.keltner_multiplier,
        );
        donchian_indicator = Donchian::new(active_indicators.donchian_period);
        obv_indicator = Obv::new(active_indicators.obv_smoothing);
        cmf_indicator = Cmf::new(active_indicators.cmf_period);
        mfi_indicator = Mfi::new(active_indicators.mfi_period);
        hv_indicator = HistoricalVolatility::new(active_indicators.hv_period);
        aroon_indicator = Aroon::new(active_indicators.aroon_period);
        choppiness_indicator = Choppiness::new(active_indicators.chop_period);
        linreg_indicator = LinRegSlope::new(active_indicators.linreg_period);
        zscore_indicator = ZScore::new(active_indicators.zscore_period);
        stoch_div = SeriesDivergence::new(20);
        chandemo_div = SeriesDivergence::new(20);
        mfi_div = SeriesDivergence::new(20);
        cmf_div = SeriesDivergence::new(20);
        obv_div = SeriesDivergence::new(20);
        squeeze_div = SeriesDivergence::new(20);
        vwap_sum_tp_vol = Decimal::ZERO;
        vwap_sum_vol = Decimal::ZERO;
        last_day_index = None;
        volume_history = VecDeque::with_capacity(20);
    }

    // ADX slope history for the 2-bar consecutive-deceleration hook exit.
    let mut adx_slope_history: VecDeque<Decimal> = VecDeque::with_capacity(3);

    // Signal-age tracker: maps "<indicator>:<kind>" → (first-seen bar, direction).
    // Stamps `age_bars` on each completed snapshot's signals. Live-only (resets
    // on warm handover, which is acceptable — historical bars aren't decisions).
    let mut signal_age_tracker: std::collections::HashMap<String, (u32, shared::indicators::SignalDirection)> =
        std::collections::HashMap::new();
    let mut live_bar: u32 = 0;

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
                if let Some(completed) = completed_opt.filter(|c| c.start_time_ms > t_last_hist) {
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

                    let extra_div = normalize::ExtraDivergence {
                        stochastic: final_stoch.as_ref().map(|s| normalize::series_divergence_state(&stoch_div.update(completed.close, s.k_value))).unwrap_or_default(),
                        chandemo: final_cmo.map(|v| normalize::series_divergence_state(&chandemo_div.update(completed.close, v))).unwrap_or_default(),
                        mfi: final_mfi.map(|v| normalize::series_divergence_state(&mfi_div.update(completed.close, v))).unwrap_or_default(),
                        cmf: final_cmf.map(|v| normalize::series_divergence_state(&cmf_div.update(completed.close, v))).unwrap_or_default(),
                        obv: final_obv.as_ref().map(|o| normalize::series_divergence_state(&obv_div.update(completed.close, o.obv))).unwrap_or_default(),
                        squeeze: final_sqz.as_ref().map(|s| normalize::series_divergence_state(&squeeze_div.update(completed.close, s.momentum_value))).unwrap_or_default(),
                    };

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

                    // Track ADX slope history for the 2-bar hook-exit rule.
                    if let Some(a) = final_adx.as_ref() {
                        adx_slope_history.push_back(a.adx_slope);
                        while adx_slope_history.len() > 3 {
                            adx_slope_history.pop_front();
                        }
                    }
                    let adx_consecutive_deceleration = adx_slope_history.len() >= 2
                        && adx_slope_history
                            .iter()
                            .rev()
                            .take(2)
                            .all(|s| *s < Decimal::ZERO);

                    // Active position context for direction-aware normalization.
                    let active_position: Option<i8> = if let Some(ref pool) = paper_pool {
                        match db::paper::queries::paper_get_active_position(pool, &symbol)
                            .await
                            .map(|p| p.direction)
                            .as_deref()
                        {
                            Some("LONG") => Some(1),
                            Some("SHORT") => Some(-1),
                            _ => Some(0),
                        }
                    } else {
                        Some(0)
                    };

                    let ema_stack_str = ema_stack_state.as_deref();
                    let indicators = normalize::build_indicator_map(normalize::NormalizeParams {
                        close: completed.close,
                        rsi: final_rsi,
                        rsi_divergence: normalize::rsi_divergence_state(&div_result),
                        macd_divergence: normalize::macd_divergence_state(&div_result),
                        stoch_k: final_stoch.as_ref().map(|s| s.k_value),
                        stoch_d: final_stoch.as_ref().map(|s| s.d_value),
                        chandemo: final_cmo,
                        supertrend_line: final_supertrend.as_ref().map(|s| s.line),
                        supertrend_dir: final_supertrend.as_ref().map(|s| s.direction),
                        keltner: final_keltner.as_ref().map(|k| (k.upper, k.middle, k.lower)),
                        donchian: final_donchian.as_ref().map(|d| (d.upper, d.middle, d.lower)),
                        obv: final_obv.as_ref().map(|o| o.obv),
                        obv_sma: final_obv.as_ref().map(|o| o.obv_sma),
                        cmf: final_cmf,
                        mfi: final_mfi,
                        hv: final_hv,
                        aroon_up: final_aroon.as_ref().map(|a| a.up),
                        aroon_down: final_aroon.as_ref().map(|a| a.down),
                        choppiness: final_chop,
                        linreg_slope: final_linreg,
                        zscore: final_zscore,
                        extra_div,
                        macd: &final_macd,
                        sqz: final_sqz.as_ref(),
                        adx: final_adx.as_ref(),
                        bb: final_bb,
                        atr: final_atr.as_ref(),
                        bbwp: final_bbwp,
                        vwap: final_vwap,
                        ema_stack_state: ema_stack_str,
                        ema_fast: Some(final_ema_fast),
                        ema_medium: Some(final_ema_medium),
                        ema_slow: Some(final_ema_slow),
                        ema_long: Some(final_ema_long),
                        rvol,
                        volume: Some(completed.volume),
                        average_volume: avg_vol,
                        fib: Some(&fib),
                        pattern: Some(&pattern_result),
                        support_levels: &[],
                        resistance_levels: &[],
                        active_position,
                        adx_consecutive_deceleration,
                    });

                    // Stamp signal freshness (age in completed bars).
                    let mut indicators = indicators;
                    live_bar = live_bar.wrapping_add(1);
                    stamp_signal_ages(&mut indicators, &mut signal_age_tracker, live_bar);

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
                        context: Some(shared::market_context::MarketContext::synthesize(&indicators)),
                        indicators,
                    };

                    let _ = telemetry_tx.send(db::TelemetryMsg::InsertSnapshot(completed_snapshot.clone())).await;

                    // Decisive close invalidation: check at every 1-minute candle close
                    if timeframe_secs == 60 {
                        if let Some(ref pool) = paper_pool {
                            if let Some(pos) = db::paper::queries::paper_get_active_position(pool, &symbol).await {
                                if let Some(inval_level) = pos.final_invalidation_level {
                                    let tolerance = 0.002;
                                    let close_f64 = completed.close.to_f64().unwrap_or(0.0);
                                    let invalidated = match pos.direction.as_str() {
                                        "LONG" => close_f64 < inval_level * (1.0 - tolerance),
                                        "SHORT" => close_f64 > inval_level * (1.0 + tolerance),
                                        _ => false,
                                    };
                                    if invalidated {
                                        let _ = crate::paper_trading::invalidate_position(
                                            pool,
                                            &telemetry_tx,
                                            &symbol,
                                            close_f64,
                                            "DECISIVE_CLOSE_1M",
                                        ).await;
                                        println!(
                                            "🛑 Analyzer: {} position invalidated by 1m decisive close at ${:.2}",
                                            symbol, close_f64
                                        );
                                    }
                                }
                            }
                        }
                    }

                    {
                        let mut snap = latest_snapshot.write().await;
                        *snap = Some(completed_snapshot.clone());
                    }

                    // The strict handover gate above guarantees this candle is
                    // strictly newer than any historical candle, so it is always
                    // a fresh append — no dedup/overwrite of historical data.
                    {
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
                    }
                    if let Some(ref tx) = candle_forward {
                        let _ = tx.send(completed.clone()).await;
                    }
                }

                // BROADCAST: Flickering snapshot from live candle
                broadcast_live_snapshot(
                    &broadcast_tx, &symbol, &live_candle, shadow_exchange,
                    shadow_bid, shadow_ask,
                    &ema_fast, &ema_medium, &ema_slow, &ema_long,
                    &rsi_14, &macd, &adx_14, &sqz_mom,
                    &bollinger, &atr_standalone, &bbwp_indicator,
                    &stochastic_indicator, &chandemo_indicator,
                    &supertrend_indicator, &keltner_indicator, &donchian_indicator,
                    &obv_indicator, &cmf_indicator, &mfi_indicator, &hv_indicator,
                    &aroon_indicator, &choppiness_indicator, &linreg_indicator, &zscore_indicator,
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
                        &bollinger, &atr_standalone, &bbwp_indicator,
                        &stochastic_indicator, &chandemo_indicator,
                        &supertrend_indicator, &keltner_indicator, &donchian_indicator,
                        &obv_indicator, &cmf_indicator, &mfi_indicator, &hv_indicator,
                        &aroon_indicator, &choppiness_indicator, &linreg_indicator, &zscore_indicator,
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

/// Stamp `age_bars` on every signal using a persistent tracker keyed by
/// `<indicator>:<kind>`. A signal resets to age 0 when it first appears or flips
/// direction; otherwise its age is the number of completed bars since first seen.
fn stamp_signal_ages(
    map: &mut std::collections::HashMap<String, shared::indicators::NormalizedIndicatorValue>,
    tracker: &mut std::collections::HashMap<String, (u32, shared::indicators::SignalDirection)>,
    bar: u32,
) {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (key, entry) in map.iter_mut() {
        for sig in entry.signals.iter_mut() {
            let tk = format!("{}:{:?}", key, sig.kind);
            seen.insert(tk.clone());
            match tracker.get(&tk) {
                Some((first, dir)) if *dir == sig.direction => {
                    sig.age_bars = bar.saturating_sub(*first);
                }
                _ => {
                    tracker.insert(tk, (bar, sig.direction));
                    sig.age_bars = 0;
                }
            }
        }
    }
    // Evict trackers whose signal no longer fires so a re-appearance is "fresh".
    tracker.retain(|k, _| seen.contains(k));
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
    bbwp_indicator: &Bbwp,
    stochastic_indicator: &Stochastic,
    chandemo_indicator: &ChandeMO,
    supertrend_indicator: &Supertrend,
    keltner_indicator: &Keltner,
    donchian_indicator: &Donchian,
    obv_indicator: &Obv,
    cmf_indicator: &Cmf,
    mfi_indicator: &Mfi,
    hv_indicator: &HistoricalVolatility,
    aroon_indicator: &Aroon,
    choppiness_indicator: &Choppiness,
    linreg_indicator: &LinRegSlope,
    zscore_indicator: &ZScore,
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
    let val_bbwp = bbwp_indicator.clone().update(candle.close);
    let val_stoch = stochastic_indicator.clone().update(candle.high, candle.low, candle.close);
    let val_cmo = chandemo_indicator.clone().update(candle.close);
    let val_supertrend = supertrend_indicator.clone().update(candle.high, candle.low, candle.close);
    let val_keltner = keltner_indicator.clone().update(candle.high, candle.low, candle.close);
    let val_donchian = donchian_indicator.clone().update(candle.high, candle.low);
    let val_obv = obv_indicator.clone().update(candle.close, candle.volume);
    let val_cmf = cmf_indicator.clone().update(candle.high, candle.low, candle.close, candle.volume);
    let val_mfi = mfi_indicator.clone().update(candle.high, candle.low, candle.close, candle.volume);
    let val_hv = hv_indicator.clone().update(candle.close);
    let val_aroon = aroon_indicator.clone().update(candle.high, candle.low);
    let val_chop = choppiness_indicator.clone().update(candle.high, candle.low, candle.close);
    let val_linreg = linreg_indicator.clone().update(candle.close);
    let val_zscore = zscore_indicator.clone().update(candle.close);

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

    let rvol = match (candle.volume, avg_vol) {
        (vol, Some(avg)) if avg > Decimal::ZERO => Some(vol / avg),
        _ => None,
    };

    let ema_stack_state = if val_ema_fast > val_ema_medium
        && val_ema_medium > val_ema_slow
        && val_ema_slow > val_ema_long
        && candle.close > val_ema_fast
    {
        Some("bullish")
    } else if val_ema_fast < val_ema_medium
        && val_ema_medium < val_ema_slow
        && val_ema_slow < val_ema_long
        && candle.close < val_ema_fast
    {
        Some("bearish")
    } else {
        Some("tangled")
    };

    let indicators = normalize::build_indicator_map(normalize::NormalizeParams {
        close: candle.close,
        rsi: val_rsi,
        rsi_divergence: shared::indicators::DivergenceState::None,
        macd_divergence: shared::indicators::DivergenceState::None,
        stoch_k: val_stoch.as_ref().map(|s| s.k_value),
        stoch_d: val_stoch.as_ref().map(|s| s.d_value),
        chandemo: val_cmo,
        supertrend_line: val_supertrend.as_ref().map(|s| s.line),
        supertrend_dir: val_supertrend.as_ref().map(|s| s.direction),
        keltner: val_keltner.as_ref().map(|k| (k.upper, k.middle, k.lower)),
        donchian: val_donchian.as_ref().map(|d| (d.upper, d.middle, d.lower)),
        obv: val_obv.as_ref().map(|o| o.obv),
        obv_sma: val_obv.as_ref().map(|o| o.obv_sma),
        cmf: val_cmf,
        mfi: val_mfi,
        hv: val_hv,
        aroon_up: val_aroon.as_ref().map(|a| a.up),
        aroon_down: val_aroon.as_ref().map(|a| a.down),
        choppiness: val_chop,
        linreg_slope: val_linreg,
        zscore: val_zscore,
        extra_div: normalize::ExtraDivergence::default(),
        macd: &val_macd,
        sqz: val_sqz.as_ref(),
        adx: val_adx.as_ref(),
        bb: val_bb,
        atr: val_atr.as_ref(),
        bbwp: val_bbwp,
        vwap: val_vwap,
        ema_stack_state,
        ema_fast: Some(val_ema_fast),
        ema_medium: Some(val_ema_medium),
        ema_slow: Some(val_ema_slow),
        ema_long: Some(val_ema_long),
        rvol,
        volume: Some(candle.volume),
        average_volume: avg_vol,
        fib: None,
        pattern: None,
        support_levels: &[],
        resistance_levels: &[],
        active_position: None,
        adx_consecutive_deceleration: false,
    });

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
        context: None,
        indicators,
    };

    let _ = broadcast_tx.send(snapshot);
}

#[cfg(test)]
mod age_tests {
    use super::stamp_signal_ages;
    use shared::indicators::{
        IndicatorSignal, NormalizedIndicatorValue, SignalDirection, SignalKind, SignalStatus,
    };
    use std::collections::HashMap;

    fn entry_with_signal(dir: SignalDirection) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(0.0, 0.5, "X").push_signal(IndicatorSignal::new(
            SignalKind::Divergence,
            dir,
            SignalStatus::Potential,
            "DIV",
        ))
    }

    #[test]
    fn age_increments_while_signal_persists() {
        let mut tracker = HashMap::new();
        let mut m = HashMap::new();
        m.insert("rsi".to_string(), entry_with_signal(SignalDirection::Bullish));
        stamp_signal_ages(&mut m, &mut tracker, 1);
        assert_eq!(m["rsi"].signals[0].age_bars, 0, "fresh signal age 0");

        let mut m2 = HashMap::new();
        m2.insert("rsi".to_string(), entry_with_signal(SignalDirection::Bullish));
        stamp_signal_ages(&mut m2, &mut tracker, 4);
        assert_eq!(m2["rsi"].signals[0].age_bars, 3, "3 bars since first seen");
    }

    #[test]
    fn age_resets_on_direction_flip() {
        let mut tracker = HashMap::new();
        let mut m = HashMap::new();
        m.insert("rsi".to_string(), entry_with_signal(SignalDirection::Bullish));
        stamp_signal_ages(&mut m, &mut tracker, 1);

        let mut m2 = HashMap::new();
        m2.insert("rsi".to_string(), entry_with_signal(SignalDirection::Bearish));
        stamp_signal_ages(&mut m2, &mut tracker, 5);
        assert_eq!(m2["rsi"].signals[0].age_bars, 0, "flip resets age");
    }
}
