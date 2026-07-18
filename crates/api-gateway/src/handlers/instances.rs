use portfolio_supervisor::registry;
use crate::types::{
    AddInstanceRequest, InstanceConfigPayload, InstanceDetailQuery, InstanceIntervalsRequest,
    InstanceListResponse, InstanceManualRequest,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_start_instance(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    match registry::start_instance(&state.registry_context(), &instance_id).await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            format!("Instance {} started", instance_id),
        )
            .into_response(),
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
    }
}

pub async fn serve_list_instances(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InstanceDetailQuery>,
) -> impl IntoResponse {
    let all_summaries = registry::list_instances(&state.registry_context()).await;
    let summaries: Vec<_> = if let Some(ref pk) = query.pair_key {
        all_summaries
            .into_iter()
            .filter(|s| s.pair == *pk)
            .collect()
    } else {
        all_summaries
    };
    Json(InstanceListResponse {
        total_count: summaries.len(),
        instances: summaries,
    })
}

pub async fn serve_add_instance(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddInstanceRequest>,
) -> impl IntoResponse {
    let base = payload.base.trim().to_uppercase();
    let quote = payload.quote.trim().to_uppercase();

    if base.is_empty() || quote.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Base and quote currency required" })),
        )
            .into_response();
    }
    if base.len() > 10 || quote.len() > 10 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Symbol too long" })),
        )
            .into_response();
    }

    match registry::add_instance(&state.registry_context(), (base, quote)).await {
        Ok(instance) => (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({
                "id": instance.id,
                "pair": instance.pair_display(),
                "message": format!("Instance {} created", instance.pair_display()),
            })),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn serve_delete_instance(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    match registry::delete_instance(&state.registry_context(), &instance_id).await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            format!("Instance {} deleted", instance_id),
        )
            .into_response(),
        Err(e) => {
            let status = if e.contains("STOPPED") || e.contains("Cannot delete") {
                axum::http::StatusCode::CONFLICT
            } else {
                axum::http::StatusCode::NOT_FOUND
            };
            (status, e).into_response()
        }
    }
}

pub async fn serve_delete_instance_by_pair(
    State(state): State<Arc<AppState>>,
    Path(pair_key): Path<String>,
) -> impl IntoResponse {
    let instance_id = state
        .workspace
        .get(&pair_key)
        .await
        .map(|i| i.id.clone());

    match instance_id {
        Some(id) => match registry::delete_instance(&state.registry_context(), &id).await {
            Ok(()) => (
                axum::http::StatusCode::OK,
                format!("Instance {} deleted", pair_key),
            )
                .into_response(),
            Err(e) => {
                let status = if e.contains("STOPPED") || e.contains("Cannot delete") {
                    axum::http::StatusCode::CONFLICT
                } else {
                    axum::http::StatusCode::NOT_FOUND
                };
                (status, e).into_response()
            }
        },
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

pub async fn serve_get_instance_detail(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let instance = state.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            let status = inst.config_state.read().await.status.as_str().to_string();
            let trading = inst.trading.read().await;
            let losses: u32 = inst.safety.consecutive_losses.read().await.values().sum();
            let safety_state = inst.safety.safety_state.read().await.as_str().to_string();
            Json(serde_json::json!({
                "id": inst.id,
                "pair": inst.pair_display(),
                "symbol": inst.symbol(),
                "status": status,
                "initial_capital": trading.initial_capital,
                "current_equity": trading.current_equity,
                "consecutive_losses": losses,
                "safety_state": safety_state,
            }))
            .into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

pub async fn serve_update_instance_config(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceConfigPayload>,
) -> impl IntoResponse {
    let pair_key = state
        .get_instance_by_id(&instance_id)
        .await
        .map(|inst| inst.pair_key())
        .or_else(|| Some(instance_id.clone()));

    match pair_key {
        Some(pk) => {
            let mut config = state.workspace.config().await;
            let symbol = pk.clone();

            // Locate the existing entry by symbol (workspace.instances is a
            // Vec<InstanceEntry>, not a HashMap).
            let mut existing = config
                .instances
                .iter()
                .find(|i| i.symbol == symbol)
                .cloned();
            if existing.is_none() {
                let default_indicators = config_models::IndicatorsConfig::default();
                existing = Some(config_models::InstanceEntry {
                    id: symbol.clone(),
                    symbol: symbol.clone(),
                    quote: String::new(),
                    initial_capital_usd: 1000.0,
                    status: config_models::InstanceStatus::Running,
                    micro_term: config_models::TimeframeConfig::new(60, default_indicators.clone()),
                    fast_term: config_models::TimeframeConfig::new(180, default_indicators.clone()),
                    slow_term: None,
                    macro_term: None,
                    automation: Default::default(),
                    operational_mode: Default::default(),
                    weight_overrides: None,
                    position_scaling: None,
                    activation: None,
                });
            }
            let mut entry = existing.expect("entry created above");
            entry.micro_term = payload.micro_term.unwrap_or(entry.micro_term);
            entry.fast_term = payload.fast_term.unwrap_or(entry.fast_term);
            entry.slow_term = payload.slow_term.or(entry.slow_term);
            entry.macro_term = payload.macro_term.or(entry.macro_term);
            entry.automation = payload.automation.unwrap_or(entry.automation);
             entry.operational_mode = payload
                .operational_mode
                .as_deref()
                .and_then(|s| match s {
                    "advisory" | "ManualOnly" => Some(config_models::OperationalMode::Advisory),
                    "paper_trading" | "DeterministicHeuristics" => {
                        Some(config_models::OperationalMode::PaperTrading)
                    }
                    "live_trading" => Some(config_models::OperationalMode::LiveTrading),
                    _ => None,
                })
                .unwrap_or(entry.operational_mode);
            entry.weight_overrides = payload.weight_overrides.or(entry.weight_overrides);
            entry.position_scaling = payload.position_scaling.or(entry.position_scaling);

            // Replace or insert the entry in workspace.instances.
            if let Some(slot) = config.instances.iter_mut().find(|i| i.symbol == symbol) {
                *slot = entry;
            } else {
                config.instances.push(entry);
            }
            if let Err(e) = config_models::save_workspace(&config) {
                eprintln!("⚠️  Failed to persist workspace config: {}", e);
            }
            println!(
                "Instance config saved: {} — triggering pipeline recharge",
                pk
            );

            match registry::recharge_instance(&state.registry_context(), &pk).await {
                Ok(()) => (
                    axum::http::StatusCode::OK,
                    "Instance configuration saved and pipelines recharged",
                )
                    .into_response(),
                Err(e) => {
                    eprintln!("Pipeline recharge failed for {}: {}", pk, e);
                    (
                        axum::http::StatusCode::OK,
                        format!("Config saved but pipeline recharge failed: {}", e),
                    )
                        .into_response()
                }
            }
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

pub async fn serve_pause_instance(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    match registry::pause_instance(&state.registry_context(), &instance_id).await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            format!("Instance {} paused", instance_id),
        )
            .into_response(),
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
    }
}

pub async fn serve_stop_instance(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    match registry::stop_instance(&state.registry_context(), &instance_id).await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            format!("Instance {} stopped", instance_id),
        )
            .into_response(),
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
    }
}

pub async fn serve_reset_safety(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let instance = state.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            inst.safety.reset_consecutive_losses(None).await;
            (
                axum::http::StatusCode::OK,
                format!("Safety counter reset for instance {}", instance_id),
            )
                .into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

pub async fn serve_instance_manual_open(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceManualRequest>,
) -> impl IntoResponse {
    let instance = state.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            inst.safety.reset_consecutive_losses(None).await;

            let dir = payload.direction.unwrap_or_else(|| "LONG".into());
            println!(
                "Manual Open: {} {} direction={} price={:?}",
                instance_id,
                inst.pair_display(),
                dir,
                payload.price
            );

            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Manual open recorded for {} (safety counter reset)", inst.pair_display()),
                "instance_id": instance_id,
            }))).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

pub async fn serve_instance_manual_close(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceManualRequest>,
) -> impl IntoResponse {
    let instance = state.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            let price = payload.price.unwrap_or(0.0);
            println!(
                "Manual Close: {} {} price={}",
                instance_id,
                inst.pair_display(),
                price
            );

            inst.safety.reset_consecutive_losses(None).await;

            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Manual close recorded for {} (safety counter reset)", inst.pair_display()),
                "instance_id": instance_id,
            }))).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

// ─── Instance Intervals ───────────────────────────────────────────

pub async fn serve_instance_intervals(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceIntervalsRequest>,
) -> impl IntoResponse {
    let instance = state.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            let mut config_state = inst.config_state.write().await;
            config_state.intervals.slow_seconds = payload.slow_seconds as u64;
            config_state.intervals.normal_seconds = payload.normal_seconds as u64;
            config_state.intervals.fast_seconds = payload.fast_seconds as u64;

            println!(
                "Instance intervals updated for {} ({}) slow={}s normal={}s fast={}s",
                inst.pair_display(),
                instance_id,
                payload.slow_seconds as u64,
                payload.normal_seconds as u64,
                payload.fast_seconds as u64
            );

            (
                axum::http::StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "instance_id": instance_id,
                    "intervals": {
                        "slow_seconds": payload.slow_seconds as u64,
                        "normal_seconds": payload.normal_seconds as u64,
                        "fast_seconds": payload.fast_seconds as u64,
                    },
                })),
            )
                .into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct ReleaseVetoBody {
    #[serde(default)]
    reset_peak: bool,
}

pub async fn serve_release_veto(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(body): Json<ReleaseVetoBody>,
) -> impl IntoResponse {
    let instance = state.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            match inst.safety.release_veto(body.reset_peak).await {
                Ok(()) => {
                    let mut stances = inst.stances.write().await;
                    for (_sym, stance) in stances.iter_mut() {
                        *stance = config_models::Stance::Active;
                    }
                    drop(stances);

                    (axum::http::StatusCode::OK, Json(serde_json::json!({
                        "success": true,
                        "message": format!("Safety veto released for instance {}", instance_id),
                        "instance_id": instance_id,
                        "reset_peak": body.reset_peak,
                        "stances_restored": true,
                    }))).into_response()
                }
                Err(e) => {
                    (axum::http::StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({
                        "success": false,
                        "error": e,
                        "message": "Veto condition still active — cannot release"
                    }))).into_response()
                }
            }
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

pub async fn serve_get_safety(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let instance = state.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            let safety_state = inst.safety.safety_state.read().await.as_str().to_string();
            let losses_map = inst.safety.consecutive_losses.read().await.clone();
            let peak_eq = inst.safety.peak_equity.read().await.to_string();
            let current_eq = inst.trading.read().await.current_equity;
            let initial_cap = inst.trading.read().await.initial_capital;
            let context = inst.safety.get_safety_context().await;

            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "instance_id": instance_id,
                "safety_state": safety_state,
                "consecutive_losses": losses_map,
                "peak_equity": peak_eq,
                "current_equity": current_eq,
                "initial_capital": initial_cap,
                "context": context,
            }))).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

pub async fn serve_get_portfolio(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let instance = state.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            let trading = inst.trading.read().await;
            let safety_state = *inst.safety.safety_state.read().await;
            let losses_map = inst.safety.consecutive_losses.read().await.clone();
            let peak_eq = inst.safety.peak_equity.read().await.clone();
            let stances: std::collections::HashMap<String, String> = inst
                .stances
                .read()
                .await
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().to_string()))
                .collect();

            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "instance_id": instance_id,
                "symbol": inst.symbol(),
                "current_equity": trading.current_equity,
                "initial_capital": trading.initial_capital,
                "peak_equity": peak_eq.to_string(),
                "safety_state": safety_state.as_str(),
                "consecutive_losses": losses_map,
                "active_stances": stances,
                "position_count": 0u32,
            }))).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}
