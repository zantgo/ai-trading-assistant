use std::sync::Arc;
use axum::{
    extract::{State, WebSocketUpgrade},
    extract::ws::{WebSocket, Message as AxumMessage},
    response::IntoResponse,
    routing::get,
    Router,
};
use tower_http::services::ServeDir;
use shared::models::MarketSnapshot;
use crate::config::AppConfig;

pub struct AppState {
    pub tx: tokio::sync::broadcast::Sender<MarketSnapshot>,
    pub config: Arc<AppConfig>,
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/config", get(serve_config))
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new("crates/engine/frontend/dist"))
        .with_state(state)
}

async fn serve_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    axum::Json(state.config.clone())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws_socket(socket, state))
}

async fn handle_ws_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();

    while let Ok(snapshot) = rx.recv().await {
        if let Ok(json_str) = serde_json::to_string(&snapshot) {
            if socket.send(AxumMessage::Text(json_str.into())).await.is_err() {
                break;
            }
        }
    }
}
