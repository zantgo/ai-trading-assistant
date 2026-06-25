use crate::config::AppConfig;
use crate::server::types::{
    BackupApiKeyRequest, ConfigResponse, RulesResponse, SetKeyRequest, SetRulesRequest,
};
use crate::server::AppState;
use axum::{extract::State, http::header, response::IntoResponse, Json};
use std::sync::Arc;

pub async fn serve_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current_config = state.config.read().await.clone();
    let api_key_configured = state
        .api_key_configured
        .load(std::sync::atomic::Ordering::Relaxed);
    let response_body = ConfigResponse {
        api_key_configured,
        symbols: current_config.symbols.clone(),
        candles: current_config.candles.clone(),
        indicators: current_config.indicators.clone(),
        instances: current_config.instances.clone(),
    };
    let json = axum::Json(response_body);
    let mut response = json.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    response
}

pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AppConfig>,
) -> impl IntoResponse {
    match toml::to_string_pretty(&payload) {
        Ok(toml_str) => {
            if let Err(e) = std::fs::write("config.toml", toml_str) {
                eprintln!("Database/Config Error: Failed to write configuration updates to config.toml: {}", e);
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to persist configuration file",
                )
                    .into_response();
            }
            *state.config.write().await = payload;
            println!("Configuration Updated: successfully synchronized config.toml dynamically.");
            (
                axum::http::StatusCode::OK,
                "Configuration successfully saved.",
            )
                .into_response()
        }
        Err(e) => {
            eprintln!("TOML Serialization Error: {}", e);
            (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid configuration object structure",
            )
                .into_response()
        }
    }
}

pub async fn serve_set_key(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetKeyRequest>,
) -> impl IntoResponse {
    let key = payload.api_key.trim().to_string();
    if key.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "API key cannot be empty",
        )
            .into_response();
    }

    state.llm_client.set_api_key(key.clone()).await;

    match state.llm_client.validate_key().await {
        Ok(()) => {
            state.llm_client.set_api_key(key.clone()).await;

            let env_entry = format!("DEEPSEEK_API_KEY={}", key);
            if let Err(e) = std::fs::write(".env", &env_entry) {
                eprintln!("Failed to persist API key to .env: {}", e);
            }

            state
                .api_key_configured
                .store(true, std::sync::atomic::Ordering::Relaxed);
            println!("API key configured and validated successfully.");
            (axum::http::StatusCode::OK, "API key validated and saved.").into_response()
        }
        Err(e) => {
            state
                .api_key_configured
                .store(false, std::sync::atomic::Ordering::Relaxed);
            eprintln!("API key validation failed: {}", e);
            (
                axum::http::StatusCode::UNAUTHORIZED,
                format!("Key validation failed: {}", e),
            )
                .into_response()
        }
    }
}

pub async fn serve_get_rules() -> impl IntoResponse {
    match std::fs::read_to_string("docs/indicators-guide.md") {
        Ok(content) => Json(RulesResponse { content }).into_response(),
        Err(e) => {
            eprintln!("Failed to read indicators guide: {}", e);
            (
                axum::http::StatusCode::NOT_FOUND,
                "Indicators guide not found",
            )
                .into_response()
        }
    }
}

pub async fn serve_set_rules(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetRulesRequest>,
) -> impl IntoResponse {
    if let Err(e) = std::fs::write("docs/indicators-guide.md", &payload.content) {
        eprintln!("Failed to write indicators guide: {}", e);
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to save rules",
        )
            .into_response();
    }

    state.llm_client.set_indicators_guide(payload.content).await;

    println!("Indicators guide updated successfully.");
    (axum::http::StatusCode::OK, "Rules updated successfully.").into_response()
}

pub async fn serve_set_backup_api_key(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BackupApiKeyRequest>,
) -> impl IntoResponse {
    let key = payload.api_key.trim().to_string();
    {
        let mut config = state.config.write().await;
        config.workspace.backup_api_key = if key.is_empty() { None } else { Some(key) };
        if let Ok(toml_str) = toml::to_string_pretty(&*config) {
            let _ = std::fs::write("config.toml", toml_str);
        }
    }
    println!("Global backup API key updated");
    (axum::http::StatusCode::OK, "Backup API key saved").into_response()
}
