use crate::registry;
use crate::server::types::{
    AddInstanceRequest, InstanceConfigPayload, InstanceDetailQuery,
    InstanceIntervalsRequest, InstanceListResponse, InstanceManualRequest,
};
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_list_instances(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InstanceDetailQuery>,
) -> impl IntoResponse {
    let all_summaries = registry::list_instances(&state.workspace).await;
    let summaries: Vec<_> = if let Some(ref pk) = query.pair_key {
        all_summaries
            .into_iter()
            .filter(|s| s.pair == *pk)
            .collect()
    } else {
        all_summaries
    };
    let max_count = state.workspace.max_instances().await;
    Json(InstanceListResponse {
        total_count: summaries.len(),
        max_count,
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

    match registry::add_instance(&state.workspace, (base, quote))
        .await
    {
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
    match registry::delete_instance(&state.workspace, &instance_id).await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            format!("Instance {} deleted", instance_id),
        )
            .into_response(),
        Err(e) => (axum::http::StatusCode::NOT_FOUND, e).into_response(),
    }
}

pub async fn serve_delete_instance_by_pair(
    State(state): State<Arc<AppState>>,
    Path(pair_key): Path<String>,
) -> impl IntoResponse {
    let instance_id = state
        .workspace
        .instances
        .read()
        .await
        .get(&pair_key)
        .map(|i| i.id.clone());

    match instance_id {
        Some(id) => match registry::delete_instance(&state.workspace, &id).await {
            Ok(()) => (
                axum::http::StatusCode::OK,
                format!("Instance {} deleted", pair_key),
            )
                .into_response(),
            Err(e) => (axum::http::StatusCode::NOT_FOUND, e).into_response(),
        },
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

pub async fn serve_get_instance_detail(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let instance = state.workspace.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            let status = inst.config_state.read().await.status.as_str().to_string();
            let paper =
                crate::db::paper_get_account_metrics(&state.pool, &inst.symbol(), 0.0).await;
            let trading = inst.trading.read().await;
            Json(serde_json::json!({
                "id": inst.id,
                "pair": inst.pair_display(),
                "symbol": inst.symbol(),
                "status": status,
                "initial_capital": trading.initial_capital,
                "current_equity": trading.current_equity,
                "paper_balance": paper.current_cash,
                "paper_equity": paper.total_account_value,
                "paper_unrealized_pnl": paper.unrealized_pnl,
                "consecutive_losses": inst.safety.consecutive_losses.load(std::sync::atomic::Ordering::Relaxed),
                "caution_level": inst.safety.caution_level.read().await.as_str().to_string(),
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
        .workspace
        .get_instance_by_id(&instance_id)
        .await
        .map(|inst| inst.pair_key())
        .or_else(|| Some(instance_id.clone()));

    match pair_key {
        Some(pk) => {
            let mut config = state.config.write().await;

            let existing = config.instances.get(&pk).cloned().unwrap_or_else(|| {
                let default_indicators = crate::config::IndicatorsConfig::default();
                crate::config::InstanceSpecificConfig {
                    micro_term: crate::config::TimeframeConfig::new(60, default_indicators.clone()),
                    fast_term: crate::config::TimeframeConfig::new(180, default_indicators.clone()),
                    slow_term: None,
                    macro_term: None,
                    automation: Default::default(),
                    weight_overrides: None,
                    position_scaling: None,
                }
            });

            let specific_config = crate::config::InstanceSpecificConfig {
                micro_term: payload.micro_term.unwrap_or(existing.micro_term),
                fast_term: payload.fast_term.unwrap_or(existing.fast_term),
                slow_term: payload.slow_term.or(existing.slow_term),
                macro_term: payload.macro_term.or(existing.macro_term),
                automation: payload.automation.unwrap_or(existing.automation),
                weight_overrides: payload.weight_overrides.or(existing.weight_overrides),
                position_scaling: payload.position_scaling.or(existing.position_scaling),
            };
            config.instances.insert(pk.clone(), specific_config);
            crate::config::save_instances(&config.instances).await;
            drop(config);
            println!("Instance config saved: {} — triggering pipeline recharge", pk);

            match registry::recharge_instance(
                &state.workspace,
                &pk,
            )
            .await
            {
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
    match registry::pause_instance(&state.workspace, &instance_id).await {
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
    match registry::stop_instance(&state.workspace, &instance_id).await {
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
    let instance = state.workspace.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            inst.safety.reset_consecutive_losses().await;
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
    let instance = state.workspace.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            inst.safety.reset_consecutive_losses().await;

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
    let instance = state.workspace.get_instance_by_id(&instance_id).await;

    match instance {
        Some(inst) => {
            let price = payload.price.unwrap_or(0.0);
            println!(
                "Manual Close: {} {} price={}",
                instance_id,
                inst.pair_display(),
                price
            );

            inst.safety.reset_consecutive_losses().await;

            (axum::http::StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Manual close recorded for {} (safety counter reset)", inst.pair_display()),
                "instance_id": instance_id,
            }))).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Instance not found").into_response(),
    }
}

pub async fn serve_instance_intervals(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<InstanceIntervalsRequest>,
) -> impl IntoResponse {
    let instance = state.workspace.get_instance_by_id(&instance_id).await;

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
