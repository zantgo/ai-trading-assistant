use axum::{
    extract::ws::{Message as AxumMessage, WebSocket},
    extract::{Query, State, WebSocketUpgrade},
    response::IntoResponse,
};
use core_domain::jsonrpc::JsonRpcNotification;
use core_domain::models::MarketSnapshot;
use core_domain::models::TimeframeSlot;
use market_analyzer::analyzer::ActivePair;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::helpers::default_pair_key;
use crate::types::WsQuery;
use crate::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
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
    // `slot` is the authoritative wire-side identifier. New clients send
    // `?slot=micro|fast|slow|macro`; legacy clients omit it and we derive
    // a best-effort slot from the requested duration. Once the connection
    // is bound, every notification carries `timeframe_slot` so the
    // frontend never has to re-derive slot from duration.
    let slot = query
        .slot
        .as_deref()
        .map(TimeframeSlot::parse)
        .unwrap_or_else(|| TimeframeSlot::parse_from_secs(tf_secs));
    ws.on_upgrade(move |socket| handle_ws_socket(socket, state, pair_key, tf_secs, slot))
}

async fn handle_ws_socket(
    mut socket: WebSocket,
    state: Arc<AppState>,
    pair_key: String,
    tf_secs: u64,
    requested_slot: TimeframeSlot,
) {
    let _ = tf_secs; // keep param for logs in a future iteration; suppresses unused-but-set warning

    // Subscribe to the recharge notification channel BEFORE binding to the
    // initial broadcast receiver. This narrows the race window in which a
    // recharge could happen after we attach to the OLD `ActivePair` but
    // before we know to re-attach to the NEW one. After the initial bind,
    // `tokio::select!` ensures any subsequent recharges are detected
    // immediately and trigger a re-subscription onto the swapped
    // `ActivePair`. See `crates/api-gateway/src/handlers/instances.rs`
    // (`serve_update_instance_config`) for the matching emit site.
    let mut recharge_rx = state.recharge_tx.subscribe();
    let mut current_pair: Option<Arc<ActivePair>>;
    let mut rx_stream: broadcast::Receiver<MarketSnapshot>;
    loop {
        match state.get_active_pair(&pair_key).await {
            Some(p) => {
                current_pair = Some(p.clone());
                rx_stream = p.subscribe_broadcast_by_slot(requested_slot);
                break;
            }
            None => {
                // No active pair yet (e.g. session not initialised). Wait
                // for either the next recharge/insert event or a channel
                // close.
                match recharge_rx.recv().await {
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    }

    loop {
        tokio::select! {
            result = rx_stream.recv() => {
                match result {
                    Ok(snapshot) => {
                        let symbol = snapshot.symbol.clone();
                        let tf = snapshot.timeframe_secs;
                        let slot_str = snapshot
                            .timeframe_slot
                            .map(|s| s.as_str())
                            .unwrap_or(requested_slot.as_str())
                            .to_string();
                        if let Ok(payload) = serde_json::to_value(&snapshot) {
                            let notif = JsonRpcNotification::new(
                                "broadcast.market_snapshot",
                                serde_json::json!({
                                    "symbol": symbol,
                                    "timeframe_slot": slot_str,
                                    "timeframe_secs": tf,
                                    "snapshot": payload,
                                }),
                            );
                            if let Ok(json_str) = serde_json::to_string(&notif) {
                                if socket.send(AxumMessage::Text(json_str)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        eprintln!(
                            "WS: Client fell behind by {} snapshots, resuming...",
                            missed
                        );
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            notice_result = recharge_rx.recv() => {
                match notice_result {
                    Ok(notice) if notice.pair_key == pair_key => {
                        // The pipeline for this pair was just rebuilt. Drop
                        // the cached Receiver (bound to the OLD
                        // `ActivePair`'s broadcast channel) and rebind to
                        // the freshly installed one. Without this rebind,
                        // `rx_stream.recv()` would block indefinitely on a
                        // channel whose Sender is kept alive only by this
                        // very handler — the chart freezes silently.
                        if let Some(new_pair) = state.get_active_pair(&pair_key).await {
                            let same_pair = current_pair
                                .as_ref()
                                .map(|old| Arc::ptr_eq(old, &new_pair))
                                .unwrap_or(false);
                            if !same_pair {
                                current_pair = Some(new_pair.clone());
                                rx_stream =
                                    new_pair.subscribe_broadcast_by_slot(requested_slot);
                            }
                        } else {
                            // Pair was deleted; the underlying broadcast
                            // channel will eventually report `Closed` once
                            // every other Sender is dropped.
                            current_pair = None;
                        }
                    }
                    Ok(_) => {
                        // Notification for a different pair; ignore.
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // We've fallen behind on notifications. Skip to
                        // the next one — at worst we miss a recharge
                        // event and remain on the prior channel for a
                        // moment longer; the next data-frame `Closed`
                        // (or the next recharge we DO observe) will
                        // self-correct.
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Recharge channel was dropped. The data channel
                        // is unaffected — the loop keeps draining
                        // snapshots until the data channel closes too.
                    }
                }
            }
        }
    }
}
