use config_models::WorkspaceConfig;
use crate::types::{ConfigResponse, RulesResponse, SetRulesRequest};
use crate::AppState;
use axum::{extract::State, http::header, response::IntoResponse, Json};
use std::sync::Arc;

pub async fn serve_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current_config = state.workspace.config().await;
    let response_body = ConfigResponse {
        api_key_configured: true,
        symbols: current_config.declared_symbols(),
        candles: current_config.candles.clone(),
        indicators: current_config.indicators.clone(),
        instances: current_config.instances.clone(),
        indicator_registry: market_analyzer::indicators::registry::all(),
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
    Json(payload): Json<WorkspaceConfig>,
) -> impl IntoResponse {
    match toml::to_string_pretty(&payload) {
        Ok(toml_str) => {
            if let Err(e) = std::fs::write("config.toml", toml_str) {
                eprintln!("Failed to write configuration updates to config.toml: {}", e);
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to persist configuration file",
                )
                    .into_response();
            }
            state.workspace.set_config(payload).await;
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

// ─── TOML export/import (config-sharing workflow) ───────────────────

/// Returns the raw `config.toml` file as `text/plain` so the operator can
/// download it and `scp` / upload it to another machine. This is the
/// canonical "share my config" endpoint.
pub async fn serve_workspace_toml(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let raw = std::fs::read_to_string("config.toml")
        .unwrap_or_else(|_| "# config.toml not found on disk".to_string());
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8"),
         (header::CONTENT_DISPOSITION, "attachment; filename=\"config.toml\"")],
        raw,
    )
}

/// Import a raw TOML workspace. The `[workspace]` section is parsed and
/// applied to the running state. **Platform-level fields** (`[hyperliquid]`,
/// `[bitget]`, `[clock_monitor]`) are preserved from the current machine
/// so that exchange URLs are not overwritten by a shared config from
/// another host.
pub async fn serve_workspace_toml_import(
    State(state): State<Arc<AppState>>,
    body: String,
) -> impl IntoResponse {
    let workspace: WorkspaceConfig = match toml::from_str(&body) {
        Ok(c) => c,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid TOML: {}", e),
            ).into_response()
        }
    };

    // Validate workspace has required fields.
    if workspace.id.is_empty() || workspace.instances.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid workspace: missing id or instances",
        ).into_response()
    }

    // Apply the imported workspace to memory.
    state.workspace.set_config(workspace.clone()).await;

    // Persist: read current file, swap workspace table, write back.
    let _ = std::fs::read_to_string("config.toml")
        .ok()
        .and_then(|raw| toml::from_str::<toml::Value>(&raw).ok())
        .and_then(|disk_val| {
            let mut table = disk_val.as_table()?.clone();
            let ws_toml = toml::to_string_pretty(&workspace)
                .ok()
                .and_then(|s| toml::from_str::<toml::Value>(&s).ok())
                .unwrap_or(toml::Value::Table(toml::Table::new()));
            table.insert("workspace".to_string(), ws_toml);
            toml::to_string_pretty(&toml::Value::Table(table))
                .ok()
                .map(|serialised| std::fs::write("config.toml", serialised))
        });

    println!(
        "Workspace '{}' imported from TOML ({} instances). Platform-level fields preserved.",
        workspace.name,
        workspace.instances.len(),
    );
    (
        axum::http::StatusCode::OK,
        format!(
            "Workspace '{}' imported ({} instances). Platform-level fields preserved. Restart recommended to reconcile instances.",
            workspace.name,
            workspace.instances.len(),
        ),
    ).into_response()
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

pub async fn serve_set_rules(Json(payload): Json<SetRulesRequest>) -> impl IntoResponse {
    if let Err(e) = std::fs::write("docs/indicators-guide.md", &payload.content) {
        eprintln!("Failed to write indicators guide: {}", e);
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to save rules",
        )
            .into_response();
    }

    println!("Indicators guide updated successfully.");
    (axum::http::StatusCode::OK, "Rules updated successfully.").into_response()
}
