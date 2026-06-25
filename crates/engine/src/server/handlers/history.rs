use crate::server::helpers::{default_pair_key, get_active_pair};
use crate::server::types::{HistoryCandle, HistoryQuery, HistoryResponse, IndicatorHistoryArrays};
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let pair_key = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    } else {
        query.symbol
    };

    let tf_secs = query.timeframe_secs.unwrap_or(60);

    let (prices, candles, indicator_history) =
        match get_active_pair(&state.workspace, &pair_key).await {
            Some(pair) => {
                let snap_hist = pair.snapshot_history_vec(tf_secs).await;

                let count = snap_hist.len();
                let mut indicator_history = IndicatorHistoryArrays { symbol: pair_key.clone(), timeframe_secs: tf_secs,
                    times: Vec::with_capacity(count),
                    rsi_14: Vec::with_capacity(count),
                    squeeze_on: Vec::with_capacity(count),
                    squeeze_momentum: Vec::with_capacity(count),
                    macd_line: Vec::with_capacity(count),
                    macd_signal: Vec::with_capacity(count),
                    macd_hist: Vec::with_capacity(count),
                    adx_14: Vec::with_capacity(count),
                    adx_plus: Vec::with_capacity(count),
                    adx_minus: Vec::with_capacity(count),
                    atr_14: Vec::with_capacity(count),
                    ema_fast: Vec::with_capacity(count),
                    ema_medium: Vec::with_capacity(count),
                    ema_slow: Vec::with_capacity(count),
                    ema_long: Vec::with_capacity(count),
                    bbwp: Vec::with_capacity(count),
                    vwap: Vec::with_capacity(count),
                    bb_upper: Vec::with_capacity(count),
                    bb_middle: Vec::with_capacity(count),
                    bb_lower: Vec::with_capacity(count),
                    rvol: Vec::with_capacity(count),
                };

                let mut candle_list: Vec<HistoryCandle> = Vec::with_capacity(count);
                let mut price_list: Vec<String> = Vec::with_capacity(count);

                for snap in snap_hist.iter() {
                    indicator_history.times.push(snap.timestamp);
                    indicator_history
                        .rsi_14
                        .push(snap.rsi_14.map(|v| v.to_string()));
                    indicator_history.squeeze_on.push(snap.squeeze_on);
                    indicator_history
                        .squeeze_momentum
                        .push(snap.squeeze_momentum.map(|v| v.to_string()));
                    indicator_history
                        .macd_line
                        .push(snap.macd_line.map(|v| v.to_string()));
                    indicator_history
                        .macd_signal
                        .push(snap.macd_signal.map(|v| v.to_string()));
                    indicator_history
                        .macd_hist
                        .push(snap.macd_hist.map(|v| v.to_string()));
                    indicator_history
                        .adx_14
                        .push(snap.adx_14.map(|v| v.to_string()));
                    indicator_history
                        .adx_plus
                        .push(snap.adx_plus.map(|v| v.to_string()));
                    indicator_history
                        .adx_minus
                        .push(snap.adx_minus.map(|v| v.to_string()));
                    indicator_history
                        .atr_14
                        .push(snap.atr_14.map(|v| v.to_string()));
                    indicator_history
                        .ema_fast
                        .push(snap.ema_fast.map(|v| v.to_string()));
                    indicator_history
                        .ema_medium
                        .push(snap.ema_medium.map(|v| v.to_string()));
                    indicator_history
                        .ema_slow
                        .push(snap.ema_slow.map(|v| v.to_string()));
                    indicator_history
                        .ema_long
                        .push(snap.ema_long.map(|v| v.to_string()));
                    indicator_history
                        .bbwp
                        .push(snap.bbwp.map(|v| v.to_string()));
                    indicator_history
                        .vwap
                        .push(snap.vwap.map(|v| v.to_string()));
                    indicator_history
                        .bb_upper
                        .push(snap.bb_upper.map(|v| v.to_string()));
                    indicator_history
                        .bb_middle
                        .push(snap.bb_middle.map(|v| v.to_string()));
                    indicator_history
                        .bb_lower
                        .push(snap.bb_lower.map(|v| v.to_string()));
                    indicator_history
                        .rvol
                        .push(snap.rvol.map(|v| v.to_string()));

                    candle_list.push(HistoryCandle {
                        time: snap.timestamp * 1000,
                        open: snap
                            .open
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| snap.close.unwrap_or_default().to_string()),
                        high: snap
                            .high
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| snap.close.unwrap_or_default().to_string()),
                        low: snap
                            .low
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| snap.close.unwrap_or_default().to_string()),
                        close: snap.close.map(|v| v.to_string()).unwrap_or_default(),
                        volume: snap
                            .volume
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "0".to_string()),
                    });
                    price_list.push(
                        snap.close
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "0".to_string()),
                    );
                }

                (price_list, candle_list, indicator_history)
            }
            None => (
                vec![],
                vec![],
                IndicatorHistoryArrays { symbol: pair_key.clone(), timeframe_secs: tf_secs,
                    times: vec![],
                    rsi_14: vec![],
                    squeeze_on: vec![],
                    squeeze_momentum: vec![],
                    macd_line: vec![],
                    macd_signal: vec![],
                    macd_hist: vec![],
                    adx_14: vec![],
                    adx_plus: vec![],
                    adx_minus: vec![],
                    atr_14: vec![],
                    ema_fast: vec![],
                    ema_medium: vec![],
                    ema_slow: vec![],
                    ema_long: vec![],
                    bbwp: vec![],
                    vwap: vec![],
                    bb_upper: vec![],
                    bb_middle: vec![],
                    bb_lower: vec![],
                    rvol: vec![],
                },
            ),
        };

    Json(HistoryResponse {
        prices,
        candles,
        indicator_history,
    })
}
