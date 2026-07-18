use crate::helpers::{default_pair_key, get_active_pair};
use crate::types::{
    HistoricalIndicatorArrays, HistoryCandle, HistoryQuery, HistoryResponse, IndicatorHistoryArrays,
};
use crate::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

pub async fn serve_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let pair_key = if query.symbol.is_empty() {
        let _cfg = state.platform.read().await;
        let first = state.workspace.config().await.declared_symbols().first().cloned().unwrap_or_default();
        default_pair_key(&first)
    } else {
        query.symbol
    };

    let tf_secs = query.timeframe_secs.unwrap_or(60);
    let limit = query.limit.min(1000);

    let (prices, candles, indicator_history) = match get_active_pair(&state, &pair_key).await {
        Some(pair) => {
            let mut snap_hist = pair.snapshot_history_vec(tf_secs).await;
            snap_hist.truncate(limit);
            let count = snap_hist.len();

            // Union of all indicator keys (and their multi-line value
            // sub-keys) across the history so every per-indicator array —
            // including sub-series — stays aligned to `times`.
            let mut keys: BTreeSet<String> = BTreeSet::new();
            let mut value_keys: HashMap<String, BTreeSet<String>> = HashMap::new();
            for snap in snap_hist.iter() {
                for (k, v) in snap.indicators.iter() {
                    keys.insert(k.clone());
                    if let Some(vals) = &v.values {
                        let set = value_keys.entry(k.clone()).or_default();
                        for sub in vals.keys() {
                            set.insert(sub.clone());
                        }
                    }
                }
            }

            let empty_set: BTreeSet<String> = BTreeSet::new();
            let mut indicators: HashMap<String, HistoricalIndicatorArrays> = keys
                .iter()
                .map(|k| {
                    let vk = value_keys.get(k).unwrap_or(&empty_set);
                    (k.clone(), HistoricalIndicatorArrays::with_value_keys(vk))
                })
                .collect();

            let mut times: Vec<u64> = Vec::with_capacity(count);
            let mut candle_list: Vec<HistoryCandle> = Vec::with_capacity(count);
            let mut price_list: Vec<String> = Vec::with_capacity(count);

            for snap in snap_hist.iter() {
                times.push(snap.timestamp);

                for key in keys.iter() {
                    let arrays = indicators.get_mut(key).expect("key initialized");
                    match snap.indicators.get(key) {
                        Some(v) => arrays.push_value(v),
                        None => arrays.push_none(),
                    }
                }

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

            let indicator_history = IndicatorHistoryArrays {
                symbol: pair_key.clone(),
                timeframe_secs: tf_secs,
                times,
                indicators,
            };

            (price_list, candle_list, indicator_history)
        }
        None => (
            vec![],
            vec![],
            IndicatorHistoryArrays {
                symbol: pair_key.clone(),
                timeframe_secs: tf_secs,
                times: vec![],
                indicators: HashMap::new(),
            },
        ),
    };

    Json(HistoryResponse {
        prices,
        candles,
        indicator_history,
    })
}
