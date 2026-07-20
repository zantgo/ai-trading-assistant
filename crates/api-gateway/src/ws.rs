use axum::{
    extract::ws::{Message as AxumMessage, WebSocket},
    extract::{Query, State, WebSocketUpgrade},
    response::IntoResponse,
};
use core_domain::jsonrpc::JsonRpcNotification;
use core_domain::models::TimeframeSlot;
use std::sync::Arc;

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
    let pair = match state.get_active_pair(&pair_key).await {
        Some(p) => p,
        None => return,
    };
    // Prefer the slot-keyed subscription so two clients asking for the same
    // duration but on different slots never collapse onto the same broadcast.
    let rx = pair.subscribe_broadcast_by_slot(requested_slot);
    let _ = tf_secs; // keep param for logs in a future iteration; suppresses unused-but-set warning

    let mut rx_stream = rx;
    loop {
        match rx_stream.recv().await {
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
            Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                eprintln!(
                        "WS: Client fell behind by {} snapshots, resuming...",
                        missed
                    );
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
        }
    }
}
