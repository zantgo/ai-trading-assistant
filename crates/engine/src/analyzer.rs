use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{broadcast, RwLock};
use sqlx::SqlitePool;
use rust_decimal::Decimal;

use crate::config::AppConfig;
use crate::risk;
use crate::candle_builder::CandleBuilder;
use crate::db;

use shared::models::MarketSnapshot;
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum, BollingerBands, Atr};

pub async fn run(
    mut rx: Receiver<MarketSnapshot>,
    pool: SqlitePool,
    broadcast_tx: broadcast::Sender<MarketSnapshot>,
    config_lock: Arc<RwLock<AppConfig>>,
    history: Arc<RwLock<VecDeque<Decimal>>>,
) {
    println!("📊 Analysis Task: Subscribed to telemetry channel... \n");

    let init_config = config_lock.read().await.clone();

    let mut ema_fast = Ema::new(init_config.indicators.ema_fast);
    let mut ema_medium = Ema::new(init_config.indicators.ema_medium);
    let mut ema_slow = Ema::new(init_config.indicators.ema_slow);
    let mut ema_long = Ema::new(init_config.indicators.ema_long);
    let mut rsi_14 = Rsi::new(init_config.indicators.rsi_period);

    let mut macd = Macd::new();
    let mut adx_14 = Adx::new(init_config.indicators.adx_period);
    let mut sqz_mom = SqueezeMomentum::new(init_config.indicators.squeeze_period);
    let mut bollinger = BollingerBands::new();
    let mut atr_standalone = Atr::new(init_config.indicators.atr_period);

    let mut active_indicators = init_config.indicators.clone();

    let mut candle = CandleBuilder::new();

    let mut vwap_sum_tp_vol = Decimal::ZERO;
    let mut vwap_sum_vol = Decimal::ZERO;
    let mut last_day_index: Option<u64> = None;

    while let Some(tick) = rx.recv().await {
        // Read live config — picks up changes from update_config without restart
        let config = config_lock.read().await.clone();

        // Reinitialize indicators if periods changed
        if config.indicators != active_indicators {
            ema_fast = Ema::new(config.indicators.ema_fast);
            ema_medium = Ema::new(config.indicators.ema_medium);
            ema_slow = Ema::new(config.indicators.ema_slow);
            ema_long = Ema::new(config.indicators.ema_long);
            rsi_14 = Rsi::new(config.indicators.rsi_period);
            adx_14 = Adx::new(config.indicators.adx_period);
            sqz_mom = SqueezeMomentum::new(config.indicators.squeeze_period);
            atr_standalone = Atr::new(config.indicators.atr_period);
            active_indicators = config.indicators.clone();
        }

        // FAST PATH: Real-Time Risk Engine
        risk::check(&tick);

        // SLOW PATH: Stateful Candlestick Builder
        let rounded_time = (tick.timestamp / config.candles.duration_seconds) * config.candles.duration_seconds;

        let tick_vol = tick.bid_size.unwrap_or(Decimal::ZERO) + tick.ask_size.unwrap_or(Decimal::ZERO);

        match candle.current_time {
            None => {
                candle.initialize(rounded_time, tick.mid_price, tick_vol);
            }
            Some(curr_time) => {
                if rounded_time > curr_time {
                    // Candle boundary: previous candle closed
                    let day_index = curr_time / 86400;
                    if let Some(prev_day) = last_day_index {
                        if day_index > prev_day {
                            println!("🕒 VWAP: New UTC Day transition detected. Resetting cumulative volume weighted buffers.");
                            vwap_sum_tp_vol = Decimal::ZERO;
                            vwap_sum_vol = Decimal::ZERO;
                        }
                    }
                    last_day_index = Some(day_index);

                    let typical_price = (candle.high + candle.low + candle.close) / Decimal::from(3);
                    vwap_sum_tp_vol += typical_price * candle.volume;
                    vwap_sum_vol += candle.volume;

                    let final_vwap = if vwap_sum_vol > Decimal::ZERO {
                        Some(vwap_sum_tp_vol / vwap_sum_vol)
                    } else {
                        None
                    };

                    let final_ema_fast = ema_fast.update(candle.close);
                    let final_ema_medium = ema_medium.update(candle.close);
                    let final_ema_slow = ema_slow.update(candle.close);
                    let final_ema_long = ema_long.update(candle.close);
                    let final_rsi = rsi_14.update(candle.close);
                    let final_macd = macd.update(candle.close);
                    let final_adx = adx_14.update(candle.high, candle.low, candle.close);
                    let final_sqz = sqz_mom.update(candle.high, candle.low, candle.close);
                    let final_bb = bollinger.update(candle.close);
                    let final_atr = atr_standalone.update(candle.high, candle.low, candle.close);

                    println!("--------------------------------------------------------------------------------");
                    println!("🕯️  [Candle Closed] Timestamp: {} | Close: ${:.4} | Vol: {:.4}", curr_time, candle.close, candle.volume);
                    println!(
                        "   📈 EMAs:   Fast ({}): {} | Med ({}): {} | Slow ({}): {} | Long ({}): {}",
                        config.indicators.ema_fast, opt_dec_str(Some(final_ema_fast)),
                        config.indicators.ema_medium, opt_dec_str(Some(final_ema_medium)),
                        config.indicators.ema_slow, opt_dec_str(Some(final_ema_slow)),
                        config.indicators.ema_long, opt_dec_str(Some(final_ema_long))
                    );
                    if let Some(vw) = final_vwap {
                        println!("   📊 VWAP:   Weighted Average Equilibrium Price: ${:.4}", vw);
                    }
                    if let Some(ad) = final_adx {
                        println!("   🧭 ADX:    Trend: {:.4} | +DI: {:.4} | -DI: {:.4}", ad.0, ad.1, ad.2);
                    }

                    let closed_snapshot = MarketSnapshot {
                        timestamp: curr_time,
                        symbol: tick.symbol.clone(),
                        mid_price: candle.close,
                        bid_price: tick.bid_price,
                        ask_price: tick.ask_price,
                        bid_size: tick.bid_size,
                        ask_size: tick.ask_size,
                        funding_rate: tick.funding_rate,
                        open: Some(candle.open),
                        high: Some(candle.high),
                        low: Some(candle.low),
                        close: Some(candle.close),
                        volume: Some(candle.volume),
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

                    db::insert_snapshot(&pool, &closed_snapshot).await;

                    {
                        let mut hist = history.write().await;
                        hist.push_back(candle.close);
                        if hist.len() > 100 {
                            hist.pop_front();
                        }
                    }

                    candle.reset_to(rounded_time, tick.mid_price, tick_vol);
                } else {
                    candle.update_same_candle(tick.mid_price, tick_vol);
                }
            }
        }

        // REAL-TIME CANDLE FLICKERING BROADCAST
        let mut temp_ema_fast = ema_fast.clone();
        let mut temp_ema_medium = ema_medium.clone();
        let mut temp_ema_slow = ema_slow.clone();
        let mut temp_ema_long = ema_long.clone();
        let mut temp_rsi_14 = rsi_14.clone();
        let mut temp_macd = macd.clone();
        let mut temp_adx_14 = adx_14.clone();
        let mut temp_sqz_mom = sqz_mom.clone();
        let mut temp_bollinger = bollinger.clone();
        let mut temp_atr = atr_standalone.clone();

        let val_ema_fast = temp_ema_fast.update(candle.close);
        let val_ema_medium = temp_ema_medium.update(candle.close);
        let val_ema_slow = temp_ema_slow.update(candle.close);
        let val_ema_long = temp_ema_long.update(candle.close);
        let val_rsi = temp_rsi_14.update(candle.close);
        let val_macd = temp_macd.update(candle.close);
        let val_adx = temp_adx_14.update(candle.high, candle.low, candle.close);
        let val_sqz = temp_sqz_mom.update(candle.high, candle.low, candle.close);
        let val_bb = temp_bollinger.update(candle.close);
        let val_atr = temp_atr.update(candle.high, candle.low, candle.close);

        let temp_typical_price = (candle.high + candle.low + candle.close) / Decimal::from(3);
        let temp_sum_tp_vol = vwap_sum_tp_vol + temp_typical_price * candle.volume;
        let temp_sum_vol = vwap_sum_vol + candle.volume;
        let val_vwap = if temp_sum_vol > Decimal::ZERO {
            Some(temp_sum_tp_vol / temp_sum_vol)
        } else {
            None
        };

        let broadcast_snapshot = MarketSnapshot {
            timestamp: rounded_time,
            symbol: tick.symbol.clone(),
            mid_price: candle.close,
            bid_price: tick.bid_price,
            ask_price: tick.ask_price,
            bid_size: tick.bid_size,
            ask_size: tick.ask_size,
            funding_rate: tick.funding_rate,
            open: Some(candle.open),
            high: Some(candle.high),
            low: Some(candle.low),
            close: Some(candle.close),
            volume: Some(candle.volume),
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

        let _ = broadcast_tx.send(broadcast_snapshot);
    }
}

fn opt_dec_str(val: Option<Decimal>) -> String {
    match val {
        Some(d) => format!("{:.4}", d),
        None => "Uninitialized".to_string(),
    }
}
