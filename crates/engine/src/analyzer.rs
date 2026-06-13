use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{broadcast, RwLock};
use rust_decimal::Decimal;
use tokio_util::sync::CancellationToken;

use crate::config::AppConfig;
use crate::risk;
use crate::db;

use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedEvent, NormalizedCandle, Exchange, CandleGenerator};
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum, BollingerBands, Atr};

pub struct ActivePair {
    pub symbol: String,
    pub history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub broadcast_tx: broadcast::Sender<MarketSnapshot>,
    pub snapshot_tx: tokio::sync::mpsc::Sender<NormalizedEvent>,
    pub cancel: CancellationToken,
    pub latest_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
}

pub async fn run_single(
    mut rx: Receiver<NormalizedEvent>,
    telemetry_tx: tokio::sync::mpsc::Sender<db::TelemetryMsg>,
    broadcast_tx: broadcast::Sender<MarketSnapshot>,
    config_lock: Arc<RwLock<AppConfig>>,
    history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    latest_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
    symbol: String,
    pair_key: String,
    cancel: CancellationToken,
) {
    println!("📊 Analysis Task: Started for {} ({})...", symbol, pair_key);

    let init_config = config_lock.read().await.clone();
    let init_pair_cfg = init_config.pairs.get(&pair_key);
    let init_indicators = init_pair_cfg.map(|p| &p.indicators).unwrap_or(&init_config.indicators);
    let init_candles = init_pair_cfg.map(|p| &p.candles).unwrap_or(&init_config.candles);

    let mut ema_fast = Ema::new(init_indicators.ema_fast);
    let mut ema_medium = Ema::new(init_indicators.ema_medium);
    let mut ema_slow = Ema::new(init_indicators.ema_slow);
    let mut ema_long = Ema::new(init_indicators.ema_long);
    let mut rsi_14 = Rsi::new(init_indicators.rsi_period);

    let mut macd = Macd::new();
    let mut adx_14 = Adx::new(init_indicators.adx_period);
    let mut sqz_mom = SqueezeMomentum::new(init_indicators.squeeze_period);
    let mut bollinger = BollingerBands::new();
    let mut atr_standalone = Atr::new(init_indicators.atr_period);

    let mut active_indicators = init_indicators.clone();

    let mut candle_gen = CandleGenerator::new(&symbol, init_candles.duration_seconds);

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
                println!("🛑 Analysis Task: {} cancelled, shutting down.", symbol);
                break;
            }
            result = rx.recv() => {
                match result {
                    Some(e) => e,
                    None => {
                        println!("🛑 Analysis Task: {} channel closed, shutting down.", symbol);
                        break;
                    }
                }
            }
        };

        let config = config_lock.read().await.clone();
        let pair_cfg = config.pairs.get(&pair_key);
        let cur_indicators = pair_cfg.map(|p| &p.indicators).unwrap_or(&config.indicators);
        let cur_candles = pair_cfg.map(|p| &p.candles).unwrap_or(&config.candles);

        if *cur_indicators != active_indicators {
            ema_fast = Ema::new(cur_indicators.ema_fast);
            ema_medium = Ema::new(cur_indicators.ema_medium);
            ema_slow = Ema::new(cur_indicators.ema_slow);
            ema_long = Ema::new(cur_indicators.ema_long);
            rsi_14 = Rsi::new(cur_indicators.rsi_period);
            adx_14 = Adx::new(cur_indicators.adx_period);
            sqz_mom = SqueezeMomentum::new(cur_indicators.squeeze_period);
            atr_standalone = Atr::new(cur_indicators.atr_period);
            active_indicators = cur_indicators.clone();

            history.write().await.clear();
        }

        if candle_gen.duration_ms != cur_candles.duration_seconds * 1000 {
            candle_gen = CandleGenerator::new(&symbol, cur_candles.duration_seconds);

            history.write().await.clear();
        }

        let current_limit = cur_candles.analysis_limit;
        {
            let mut hist = history.write().await;
            while hist.len() > current_limit {
                hist.pop_front();
            }
        }

        match event {
            NormalizedEvent::Trade(ref trade) => {
                shadow_exchange = Some(trade.exchange);

                // Process trade through CandleGenerator
                let (completed_opt, live_candle) = candle_gen.process_trade(trade);
                if let Some(completed) = completed_opt {
                    let candle_close_sec = completed.start_time_ms / 1000;
                    let day_index = candle_close_sec / 86400;
                    if let Some(prev_day) = last_day_index {
                        if day_index > prev_day {
                            println!("🕒 VWAP: New UTC Day transition detected. Resetting volume buffers.");
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
                    let final_rsi = rsi_14.update(completed.close);
                    let final_macd = macd.update(completed.close);
                    // ADX needs high/low/close — use trade candle values
                    let final_adx = adx_14.update(completed.high, completed.low, completed.close);
                    let final_sqz = sqz_mom.update(completed.high, completed.low, completed.close);
                    let final_bb = bollinger.update(completed.close);
                    let final_atr = atr_standalone.update(completed.high, completed.low, completed.close);

                    let mut log_lines = vec![
                        "--------------------------------------------------------------------------------".to_string(),
                        format!(
                            "🕯️  [{} Candle Closed] Start: {} | Close: ${:.4} | Vol: {:.4} | Trades: {}",
                            symbol, completed.start_time_ms, completed.close, completed.volume, completed.trades_count
                        ),
                        format!(
                            "   📈 EMAs:   Fast ({}): {} | Med ({}): {} | Slow ({}): {} | Long ({}): {}",
                            cur_indicators.ema_fast, opt_dec_str(Some(final_ema_fast)),
                            cur_indicators.ema_medium, opt_dec_str(Some(final_ema_medium)),
                            cur_indicators.ema_slow, opt_dec_str(Some(final_ema_slow)),
                            cur_indicators.ema_long, opt_dec_str(Some(final_ema_long))
                        ),
                    ];

                    if let Some(vw) = final_vwap {
                        log_lines.push(format!("   📊 VWAP:   Weighted Average Equilibrium Price: ${:.4}", vw));
                    }
                    if let Some(ad) = final_adx {
                        log_lines.push(format!("   🧭 ADX:    Trend: {:.4} | +DI: {:.4} | -DI: {:.4}", ad.0, ad.1, ad.2));
                    }

                    let display_log = log_lines.join("\n");

                    let _ = telemetry_tx.send(db::TelemetryMsg::ConsoleLog(display_log)).await;

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

                    let completed_snapshot = MarketSnapshot {
                        exchange: shadow_exchange,
                        timestamp: candle_close_sec,
                        symbol: symbol.clone(),
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
                        bb_upper: final_bb.map(|b| b.0),
                        bb_middle: final_bb.map(|b| b.1),
                        bb_lower: final_bb.map(|b| b.2),
                        atr_14: final_atr,
                        vwap: final_vwap,
                        adx_14: final_adx.map(|ad| ad.0),
                        adx_plus: final_adx.map(|ad| ad.1),
                        adx_minus: final_adx.map(|ad| ad.2),
                        ema_fast: Some(final_ema_fast),
                        ema_medium: Some(final_ema_medium),
                        ema_slow: Some(final_ema_slow),
                        ema_long: Some(final_ema_long),
                        rsi_14: final_rsi,
                        macd_line: final_macd.map(|m| m.0),
                        macd_signal: final_macd.map(|m| m.1),
                        macd_hist: final_macd.map(|m| m.2),
                        squeeze_on: final_sqz.map(|s| s.0),
                        squeeze_momentum: final_sqz.map(|s| s.1),
                    };

                    let _ = telemetry_tx.send(db::TelemetryMsg::InsertSnapshot(completed_snapshot.clone())).await;

                    {
                        let mut snap = latest_snapshot.write().await;
                        *snap = Some(completed_snapshot);
                    }

                    {
                        let mut hist = history.write().await;
                        hist.push_back(completed.clone());
                        let current_limit = cur_candles.analysis_limit;
                        if hist.len() > current_limit {
                            hist.pop_front();
                        }
                    }
                }

                // FAST PATH: Risk check on trade price
                {
                    let tick = MarketSnapshot {
                        exchange: shadow_exchange,
                        timestamp: trade.timestamp_ms / 1000,
                        symbol: symbol.clone(),
                        mid_price: trade.price,
                        bid_price: shadow_bid,
                        ask_price: shadow_ask,
                        bid_size: Some(trade.size),
                        ask_size: Some(trade.size),
                        funding_rate: None,
                        open: None, high: None, low: None, close: None,
                        volume: None, average_volume: None,
                        bb_upper: None, bb_middle: None, bb_lower: None,
                        atr_14: None, vwap: None,
                        adx_14: None, adx_plus: None, adx_minus: None,
                        ema_fast: None, ema_medium: None, ema_slow: None, ema_long: None,
                        rsi_14: None, macd_line: None, macd_signal: None, macd_hist: None,
                        squeeze_on: None, squeeze_momentum: None,
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
                    &config,
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
                        &config,
                    );
                }
            }

            NormalizedEvent::Status { exchange, status, message } => {
                println!("[STATUS] {}: {:?} — {}", exchange, status, message);
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
    _config: &AppConfig,
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
        timestamp: candle.start_time_ms / 1000,
        symbol: symbol.to_string(),
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
        bb_upper: val_bb.map(|b| b.0),
        bb_middle: val_bb.map(|b| b.1),
        bb_lower: val_bb.map(|b| b.2),
        atr_14: val_atr,
        vwap: val_vwap,
        adx_14: val_adx.map(|ad| ad.0),
        adx_plus: val_adx.map(|ad| ad.1),
        adx_minus: val_adx.map(|ad| ad.2),
        ema_fast: Some(val_ema_fast),
        ema_medium: Some(val_ema_medium),
        ema_slow: Some(val_ema_slow),
        ema_long: Some(val_ema_long),
        rsi_14: val_rsi,
        macd_line: val_macd.map(|m| m.0),
        macd_signal: val_macd.map(|m| m.1),
        macd_hist: val_macd.map(|m| m.2),
        squeeze_on: val_sqz.map(|s| s.0),
        squeeze_momentum: val_sqz.map(|s| s.1),
    };

    let _ = broadcast_tx.send(snapshot);
}

fn opt_dec_str(val: Option<Decimal>) -> String {
    match val {
        Some(d) => format!("{:.4}", d),
        None => "Uninitialized".to_string(),
    }
}
