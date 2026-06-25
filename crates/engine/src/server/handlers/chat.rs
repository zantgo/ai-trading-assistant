use crate::server::types::{ChatHistoryRequest, ChatReplResponse, InstanceChatRequest};
use crate::server::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_chat(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatHistoryRequest>,
) -> impl IntoResponse {
    match state.llm_client.chat(payload.history, None).await {
        Ok(reply) => Json(ChatReplResponse { reply }).into_response(),
        Err(e) => {
            eprintln!("LLM chat failed: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Chat request failed: {}", e),
            )
                .into_response()
        }
    }
}

pub async fn serve_instance_chat(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceChatRequest>,
) -> impl IntoResponse {
    let instance = state.workspace.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            let mut messages = payload.history.clone();
            messages.push(crate::llm::ChatMessage {
                role: "user".into(),
                content: payload.message,
            });

            match state
                .llm_client
                .chat(messages, Some(&inst.pair_key()))
                .await
            {
                Ok(reply) => Json(serde_json::json!({
                    "reply": reply,
                    "instance_id": instance_id,
                }))
                .into_response(),
                Err(e) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e})),
                )
                    .into_response(),
            }
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}
