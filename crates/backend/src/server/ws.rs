use axum::{
    extract::ws::{Message as AxumMessage, WebSocket},
    extract::{Query, State, WebSocketUpgrade},
    response::IntoResponse,
};
use shared::jsonrpc::JsonRpcNotification;
use std::sync::Arc;

use crate::server::helpers::default_pair_key;
use crate::server::types::WsQuery;
use crate::server::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pair_key = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        default_pair_key(&first)
    } else {
        query.symbol
    };
    let tf_secs = query.timeframe_secs.unwrap_or(60);
    ws.on_upgrade(move |socket| handle_ws_socket(socket, state, pair_key, tf_secs))
}

async fn handle_ws_socket(
    mut socket: WebSocket,
    state: Arc<AppState>,
    pair_key: String,
    tf_secs: u64,
) {
    let rx = match state.get_active_pair(&pair_key).await {
        Some(pair) => pair.subscribe_broadcast(tf_secs),
        None => return,
    };

    let mut rx_stream = rx;
    loop {
        match rx_stream.recv().await {
            Ok(snapshot) => {
                let symbol = snapshot.symbol.clone();
                let tf = snapshot.timeframe_secs;
                if let Ok(payload) = serde_json::to_value(&snapshot) {
                    let notif = JsonRpcNotification::new(
                        "broadcast.market_snapshot",
                        serde_json::json!({
                            "symbol": symbol,
                            "timeframe_secs": tf,
                            "snapshot": payload,
                        }),
                    );
                    if let Ok(json_str) = serde_json::to_string(&notif) {
                        if socket
                            .send(AxumMessage::Text(json_str))
                            .await
                            .is_err()
                        {
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
