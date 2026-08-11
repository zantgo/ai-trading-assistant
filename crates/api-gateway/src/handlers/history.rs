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
use rust_decimal::Decimal;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

pub async fn serve_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let pair_key = if query.symbol.is_empty() {
        let first = state
            .workspace
            .config()
            .await
            .declared_symbols()
            .first()
            .cloned()
            .unwrap_or_default();
        default_pair_key(&state, &first).await
    } else {
        query.symbol
    };

    let tf_secs = query.timeframe_secs.unwrap_or(60);
    let limit = query.limit.min(1000);

    let (prices, candles, indicator_history) = match get_active_pair(&state, &pair_key).await {
        Some(pair) => {
            let mut snap_hist = pair.snapshot_history_vec_for_secs(tf_secs).await;
            // Sub-minute TFs: when the in-memory snapshot_history is empty
            // (e.g. fresh daemon startup, no completed candles yet), fall back
            // to the DB so the chart has OHLCV history on first mount — but
            // only when there are real persisted rows for the requested TF.
            //
            // Previously this branch also called `derive_sub_minute_candles`
            // when the DB was empty, synthesising flat-close candles from the
            // next-larger TF (typically 60s). Those synthetic candles carried
            // identical O=H=L=C=minute_close, which rendered as a horizontal
            // line spanning the entire minute. The chart couldn't distinguish
            // them from real sub-minute data, so the user saw a misleading
            // "line of about 1 minute" on every cold start.
            //
            // The derivation is gone: if neither in-memory nor DB has rows
            // for the sub-minute TF, the chart gets an empty historical
            // payload and the live WS stream fills it in within seconds.
            if snap_hist.is_empty() && tf_secs < 60 {
                let db_candles = database_storage::query_recent_candles(
                    &state.pool,
                    &pair_key,
                    tf_secs,
                    limit as u32,
                )
                .await;
                if !db_candles.is_empty() {
                    let mut db_snaps: Vec<core_domain::models::MarketSnapshot> = db_candles
                        .into_iter()
                        .rev()
                        .map(|c| core_domain::models::MarketSnapshot {
                            symbol: pair_key.clone(),
                            timeframe_secs: tf_secs,
                            timestamp: c.start_time_ms / 1000,
                            open: Some(c.open),
                            high: Some(c.high),
                            low: Some(c.low),
                            close: Some(c.close),
                            volume: Some(c.volume),
                            mid_price: c.close,
                            bid_price: Decimal::ZERO,
                            ask_price: Decimal::ZERO,
                            exchange: Some(c.exchange),
                            is_completed: Some(true),
                            ..Default::default()
                        })
                        .collect();
                    snap_hist.append(&mut db_snaps);
                }
            }
            snap_hist.truncate(limit);
            // Drop leading snapshots with no close so the first bar the UI sees
            // always has real OHLC. The first historical candle is therefore the
            // first valid bar of the response.
            let prefix = snap_hist.iter().take_while(|s| s.close.is_none()).count();
            if prefix > 0 {
                snap_hist.drain(..prefix);
            }
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

            let mut times: Vec<u64> = Vec::with_capacity(count);
            let mut candle_list: Vec<HistoryCandle> = Vec::with_capacity(count);
            let mut price_list: Vec<String> = Vec::with_capacity(count);

            for snap in snap_hist.iter() {
                times.push(snap.timestamp);
                price_list.push(
                    snap.close
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0".to_string()),
                );
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
                    reconstructed: None,
                });
            }

            // ── Gap-fill: insert flat Doji candles for any missing
            // intervals between consecutive snapshots so the chart
            // renders a continuous time axis.  Capped at 60 fill bars
            // per gap to avoid flooding the response with centuries of
            // epoch-0→now gaps on cold sub-minute starts.
            if times.len() >= 2 {
                let mut filled_times: Vec<u64> = Vec::with_capacity(times.len() + 60);
                let mut filled_candles: Vec<HistoryCandle> = Vec::with_capacity(candle_list.len() + 60);
                let mut filled_prices: Vec<String> = Vec::with_capacity(price_list.len() + 60);

                for i in 0..times.len() {
                    filled_times.push(times[i]);
                    filled_candles.push(candle_list[i].clone());
                    filled_prices.push(price_list[i].clone());

                    if i + 1 < times.len() {
                        let curr = times[i];
                        let next = times[i + 1];
                        let step = tf_secs.max(1);
                        let gap = next.saturating_sub(curr);
                        let missing = (gap.saturating_sub(step)) / step;
                        let fill_n = missing.min(60);
                        let last_close = price_list[i].clone();
                        for j in 1..=fill_n {
                            let t = curr + j * step;
                            let t_ms = t * 1000;
                            filled_times.push(t);
                            filled_candles.push(HistoryCandle {
                                time: t_ms,
                                open: last_close.clone(),
                                high: last_close.clone(),
                                low: last_close.clone(),
                                close: last_close.clone(),
                                volume: "0".to_string(),
                                // Gap-fill Dojis are also synthetic;
                                // never let the frontend cache them as
                                // if they were real historical data.
                                reconstructed: Some(
                                    core_domain::normalized::ReconstructionMethod::Synthetic,
                                ),
                            });
                            filled_prices.push(last_close.clone());
                        }
                    }
                }
                times = filled_times;
                candle_list = filled_candles;
                price_list = filled_prices;
            }

            // Rebuild the indicator arrays aligned to the (now possibly
            // gap-filled) snap_hist. Only real snapshots contribute
            // indicator values; gap-fill entries have no indicator data.
            let mut indicators: HashMap<String, HistoricalIndicatorArrays> = keys
                .iter()
                .map(|k| {
                    let vk = value_keys.get(k).unwrap_or(&empty_set);
                    (k.clone(), HistoricalIndicatorArrays::with_value_keys(vk))
                })
                .collect();
            for snap in snap_hist.iter() {
                for key in keys.iter() {
                    let arrays = indicators.get_mut(key).expect("key initialized");
                    match snap.indicators.get(key) {
                        Some(v) => arrays.push_value(v),
                        None => arrays.push_none(),
                    }
                }
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

    // v6.5: per-TF cluster matrices. Each TF pipeline owns its own
    // `cluster_matrix` handle, so we read all 4 (micro, fast, slow, macro).
    // These are the same matrices the WS broadcast already carries on each
    // snapshot — exposing them here lets the chart render overlays on
    // first-mount (before the WS delivery has happened).
    let mut clusters = std::collections::HashMap::new();
    let mut volume_profiles = std::collections::HashMap::new();
    // Phase 0-4: per-TF latest `LiquidityFlow` so the Metrics tab's
    // Flow / Cluster / Context sub-views can bootstrap immediately after
    // a daemon restart, before the WS delivers the next completed bar.
    let mut liquidity_flows: std::collections::HashMap<
        String,
        core_domain::liquidity::LiquidityFlow,
    > = std::collections::HashMap::new();
    if let Some(pair) = get_active_pair(&state, &pair_key).await {
        for (slot_label, pipe) in [
            ("micro", &pair.micro),
            ("fast", &pair.fast),
            ("slow", &pair.slow),
            ("macro", &pair.r#macro),
        ] {
            if let Ok(guard) = pipe.cluster_matrix.try_read() {
                if let Some(m) = guard.as_ref() {
                    clusters.insert(slot_label.to_string(), m.clone());
                }
            }
            // Volume profile is per-completed-candle and lives on the
            // most recent snapshot in `snapshot_history`. We take the
            // latest completed snapshot for this TF.
            let snap_hist = pipe.snapshot_history.read().await;
            if let Some(last) = snap_hist.back() {
                if let Some(vp) = last.volume_profile.as_ref() {
                    volume_profiles.insert(slot_label.to_string(), vp.clone());
                }
                if let Some(flow) = last.liquidity.as_ref() {
                    liquidity_flows.insert(slot_label.to_string(), flow.clone());
                }
            }
        }
    }

    Json(HistoryResponse {
        prices,
        candles,
        indicator_history,
        clusters,
        volume_profiles,
        liquidity_flows,
    })
}
