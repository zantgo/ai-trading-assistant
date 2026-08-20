use crate::types::{ConfigResponse, RulesResponse, SetRulesRequest};
use crate::AppState;
use axum::{extract::State, http::header, response::IntoResponse, Json};
use config_models::WorkspaceConfig;
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
        api_failover: current_config.api_failover,
        slow_timeframe: Some(current_config.slow_timeframe.clone()),
        macro_timeframe: Some(current_config.macro_timeframe.clone()),
        liquidity: Some(current_config.liquidity.clone()),
        minimal_tae: Some(current_config.minimal_tae.clone()),
        analytics: Some(current_config.analytics.clone()),
        risk_limits: Some(current_config.risk_limits.clone()),
        safety: Some(current_config.safety.clone()),
        fees: Some(current_config.fees.clone()),
        leverage: Some(current_config.leverage.clone()),
        execution: Some(current_config.execution.clone()),
        scoring: Some(current_config.scoring.clone()),
    };
    let json = axum::Json(response_body);
    let mut response = json.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    response
}

/// A partial config update: the dashboard's `GET /api/config` response
/// round-trips through `POST /api/config` with only the operator-editable
/// fields mutated (`candles`, `indicators`, `instances`, `api_failover`).
/// The body therefore does NOT carry `WorkspaceConfig`'s mandatory
/// `id`/`name`/`default_currency`/`default_exchange` — merge the provided
/// fields into the currently loaded config rather than demanding a full
/// document (fixes the previous permanent-422 round-trip failure).
#[derive(Debug, serde::Deserialize)]
pub struct ConfigUpdateRequest {
    #[serde(default)]
    pub candles: Option<config_models::CandlesConfig>,
    #[serde(default)]
    pub indicators: Option<config_models::IndicatorsConfig>,
    #[serde(default)]
    pub instances: Option<Vec<config_models::InstanceEntry>>,
    #[serde(default)]
    pub api_failover: Option<config_models::ApiFailoverConfig>,
    // Accept-and-ignore: derived or read-only fields echoed by the GET
    // response that must not clobber the loaded config on save.
    #[serde(default)]
    pub api_key_configured: Option<bool>,
    #[serde(default)]
    pub symbols: Option<Vec<String>>,
    #[serde(default)]
    pub indicator_registry: Option<serde_json::Value>,
}

pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ConfigUpdateRequest>,
) -> impl IntoResponse {
    let mut merged = state.workspace.config().await;
    if let Some(candles) = payload.candles {
        merged.candles = candles;
    }
    if let Some(indicators) = payload.indicators {
        merged.indicators = indicators;
    }
    if let Some(instances) = payload.instances {
        merged.instances = instances;
    }
    if let Some(api_failover) = payload.api_failover {
        merged.api_failover = api_failover;
    }
    merged.config_version = merged.config_version.saturating_add(1);

    // Persist through `save_workspace` (NOT a bare `toml::to_string_pretty`
    // of the WorkspaceConfig): the on-disk shape wraps the workspace in a
    // `[workspace]` table and keeps the platform sections (`[hyperliquid]`,
    // `[bitget]`, `[clock_monitor]`, `[reconnect]`, `[candle_buffer]`,
    // `[snapshot_export]`) intact — a bare write previously produced a file
    // the daemon could not boot from (`load_platform` panics without the
    // required `OnDiskConfig.workspace`).
    if let Err(e) = config_models::save_workspace(&merged) {
        eprintln!(
            "Failed to write configuration updates to config.toml: {}",
            e
        );
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to persist configuration file",
        )
            .into_response();
    }
    state.workspace.set_config(merged).await;
    println!("Configuration Updated: successfully synchronized config.toml dynamically.");
    (
        axum::http::StatusCode::OK,
        "Configuration successfully saved.",
    )
        .into_response()
}

// ─── TOML export/import (config-sharing workflow) ───────────────────

/// Returns the raw `config.toml` file as `text/plain` so the operator can
/// download it and `scp` / upload it to another machine. This is the
/// canonical "share my config" endpoint.
pub async fn serve_workspace_toml(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let raw = std::fs::read_to_string("config.toml")
        .unwrap_or_else(|_| "# config.toml not found on disk".to_string());
    (
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"config.toml\"",
            ),
        ],
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
            )
                .into_response()
        }
    };

    // Validate workspace has required fields.
    if workspace.id.is_empty() || workspace.instances.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid workspace: missing id or instances",
        )
            .into_response();
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

/// Path of the indicators rulebook served by `GET /api/rules`. The file
/// must exist in the repo — it is the MME indicators guide spec, mirrored
/// verbatim by `03-02-09-mme-indicators-guide.md`.
pub const RULES_GUIDE_PATH: &str =
    "docs/engines/market-monitoring-engine/03-02-09-mme-indicators-guide.md";

pub async fn serve_get_rules() -> impl IntoResponse {
    match std::fs::read_to_string(RULES_GUIDE_PATH) {
        Ok(content) => Json(RulesResponse { content }).into_response(),
        Err(e) => {
            eprintln!(
                "Failed to read indicators guide ({}): {}",
                RULES_GUIDE_PATH, e
            );
            (
                axum::http::StatusCode::NOT_FOUND,
                "Indicators guide not found",
            )
                .into_response()
        }
    }
}

pub async fn serve_set_rules(Json(payload): Json<SetRulesRequest>) -> impl IntoResponse {
    // M6 (production audit): the rules endpoint serves the MME indicators
    // guide — a git-tracked spec doc. Writing attacker-controlled (or
    // operator-edited) content into it was destructive (and, combined
    // with the pre-K1 CORS posture, let any website overwrite the spec).
    // The guide is now read-only over HTTP; edit it in the repo.
    let _ = payload;
    (
        axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "The indicators guide is read-only over HTTP (edit docs/engines/market-monitoring-engine/03-02-09-mme-indicators-guide.md in the repo).",
    )
        .into_response()
}
