use crate::server::helpers::get_active_pair;
use crate::server::types::{
    PaperConfigRequest, PaperOrderRequest, PaperPerformanceQuery, PaperResetRequest,
    PaperScaleInRequest, PaperScaleOutRequest, PaperStatusQuery,
};
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn serve_paper_status(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaperStatusQuery>,
) -> impl IntoResponse {
    let symbol = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        crate::server::helpers::default_pair_key(&first)
    } else {
        query.symbol
    };

    let pair = get_active_pair(&state.workspace, &symbol).await;
    let current_price = if let Some(ref p) = pair {
        p.latest_price().await.unwrap_or(0.0)
    } else {
        0.0
    };

    let metrics = crate::db::paper_get_account_metrics(&state.pool, &symbol, current_price).await;

    Json(metrics)
}

pub async fn serve_paper_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperConfigRequest>,
) -> impl IntoResponse {
    let allocation = payload.allocation_pct.clamp(1.0, 100.0);
    if let Err(e) = crate::db::paper_set_advanced_config(
        &state.pool,
        &payload.symbol,
        payload.initial_usd,
        allocation,
        payload.auto_execute,
        payload.max_risk_pct,
        payload.leverage,
        payload.auto_execute_intervals,
        payload.lookback_trades,
    )
    .await
    {
        eprintln!("Paper DB: Failed to save config: {}", e);
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save paper config: {}", e),
        )
            .into_response();
    }

    println!(
        "Paper Config: {} initial=${:.2} allocation={:.1}% auto_execute={} risk={:.1}% leverage={}x interval={}m lookback={}",
        payload.symbol, payload.initial_usd, allocation, payload.auto_execute,
        payload.max_risk_pct, payload.leverage, payload.auto_execute_intervals, payload.lookback_trades
    );
    (axum::http::StatusCode::OK, "Paper trading config saved").into_response()
}

pub async fn serve_paper_reset(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperResetRequest>,
) -> impl IntoResponse {
    let position = crate::db::paper_get_active_position(&state.pool, &payload.symbol).await;
    if position.is_some() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let _ = state
            .telemetry_tx
            .send(crate::db::TelemetryMsg::PaperClosePosition {
                symbol: payload.symbol.clone(),
                exit_price: 0.0,
                exit_timestamp: now,
                trigger: "RESET".to_string(),
            })
            .await;
    }

    if let Err(e) = crate::db::paper_reset_account(&state.pool, &payload.symbol).await {
        eprintln!("Paper DB: Failed to reset account: {}", e);
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to reset paper account: {}", e),
        )
            .into_response();
    }
    (axum::http::StatusCode::OK, "Paper account reset").into_response()
}

pub async fn serve_paper_order(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperOrderRequest>,
) -> impl IntoResponse {
    let pair_arc = get_active_pair(&state.workspace, &payload.symbol).await;
    let current_price = if let Some(pair) = pair_arc {
        pair.latest_price().await.unwrap_or(0.0)
    } else {
        0.0
    };

    if current_price <= 0.0 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "No price data available for this pair",
        )
            .into_response();
    }

    if payload.action == "CLOSE" {
        let result = crate::paper_trading::close_paper_position(
            &state.pool,
            &state.telemetry_tx,
            &payload.symbol,
            current_price,
            "MANUAL",
        )
        .await;

        if result.success {
            (axum::http::StatusCode::OK, result.message).into_response()
        } else {
            (axum::http::StatusCode::BAD_REQUEST, result.message).into_response()
        }
    } else if payload.action == "OPEN" {
        let dir = payload.direction.to_uppercase();
        if dir != "LONG" && dir != "SHORT" {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Direction must be LONG or SHORT",
            )
                .into_response();
        }

        let result = crate::paper_trading::verify_margin_and_open(
            &state.pool,
            &state.telemetry_tx,
            &payload.symbol,
            &dir,
            current_price,
        )
        .await;

        if result.success {
            (axum::http::StatusCode::CREATED, result.message).into_response()
        } else {
            (axum::http::StatusCode::BAD_REQUEST, result.message).into_response()
        }
    } else {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Action must be OPEN or CLOSE",
        )
            .into_response()
    }
}

pub async fn serve_paper_scale_in(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperScaleInRequest>,
) -> impl IntoResponse {
    let pair = get_active_pair(&state.workspace, &payload.symbol).await;
    let current_price = if let Some(ref p) = pair {
        p.latest_price().await.unwrap_or(payload.entry_price)
    } else {
        payload.entry_price
    };

    let price = if payload.entry_price > 0.0 {
        payload.entry_price
    } else {
        current_price
    };

    if price <= 0.0 {
        return Json(serde_json::json!({"success": false, "message": "No price data available"}))
            .into_response();
    }

    let dir = payload.direction.to_uppercase();
    if dir != "LONG" && dir != "SHORT" {
        return Json(
            serde_json::json!({"success": false, "message": "Direction must be LONG or SHORT"}),
        )
        .into_response();
    }

    let portion = payload.portion_number.max(1).min(3);

    let result = crate::paper_trading::scale_in_portion(
        &state.pool,
        &state.telemetry_tx,
        &payload.symbol,
        &dir,
        price,
        portion,
        payload.final_invalidation_level.unwrap_or(0.0),
    )
    .await;

    Json(serde_json::json!({
        "success": result.success,
        "message": result.message,
        "new_average_entry_price": result.new_average_entry_price,
        "total_size": result.total_size,
        "portion_number": result.portion_number,
    }))
    .into_response()
}

pub async fn serve_paper_scale_out(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PaperScaleOutRequest>,
) -> impl IntoResponse {
    let pair = get_active_pair(&state.workspace, &payload.symbol).await;
    let current_price = if let Some(ref p) = pair {
        p.latest_price().await.unwrap_or(payload.exit_price)
    } else {
        payload.exit_price
    };

    let price = if payload.exit_price > 0.0 {
        payload.exit_price
    } else {
        current_price
    };
    let fraction = payload.size_fraction.clamp(0.01, 1.0);

    let result = crate::paper_trading::scale_out_portion(
        &state.pool,
        &state.telemetry_tx,
        &payload.symbol,
        price,
        fraction,
        payload.target_id,
        &payload.trigger_source.unwrap_or_else(|| "MANUAL".to_string()),
    )
    .await;

    Json(serde_json::json!({
        "success": result.success,
        "message": result.message,
        "realized_pnl": result.realized_pnl,
        "remaining_size": result.remaining_size,
    }))
}

pub async fn serve_paper_unrealized(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaperStatusQuery>,
) -> impl IntoResponse {
    let symbol = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        let first = cfg.symbols.first().cloned().unwrap_or_default();
        crate::server::helpers::default_pair_key(&first)
    } else {
        query.symbol
    };

    let pair = get_active_pair(&state.workspace, &symbol).await;
    let current_price = if let Some(ref p) = pair {
        p.latest_price().await.unwrap_or(0.0)
    } else {
        0.0
    };

    let metrics = crate::db::paper_get_account_metrics(&state.pool, &symbol, current_price).await;

    #[derive(serde::Serialize)]
    struct UnrealizedResponse {
        symbol: String,
        direction: String,
        average_entry_price: f64,
        current_price: f64,
        size: f64,
        unrealized_pnl_usd: f64,
        unrealized_roi_pct: f64,
        final_invalidation_level: f64,
        filled_portions: i32,
        active_take_profit_targets: Vec<serde_json::Value>,
    }

    let direction = metrics
        .active_position
        .as_ref()
        .map(|p| p.direction.clone())
        .unwrap_or_default();
    let avg_entry = metrics
        .active_position
        .as_ref()
        .and_then(|p| p.average_entry_price)
        .unwrap_or(0.0);
    let size = metrics
        .active_position
        .as_ref()
        .map(|p| p.size)
        .unwrap_or(0.0);
    let invalidation = metrics
        .active_position
        .as_ref()
        .and_then(|p| p.final_invalidation_level)
        .unwrap_or(0.0);
    let filled = metrics
        .active_position
        .as_ref()
        .and_then(|p| p.current_portions)
        .unwrap_or(0);

    let targets: Vec<serde_json::Value> = metrics
        .take_profit_targets
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "target_price": t.target_price,
                "size_fraction": t.size_fraction,
                "is_hit": t.is_hit,
            })
        })
        .collect();

    Json(UnrealizedResponse {
        symbol,
        direction,
        average_entry_price: avg_entry,
        current_price,
        size,
        unrealized_pnl_usd: metrics.unrealized_pnl,
        unrealized_roi_pct: metrics.unrealized_roi_pct,
        final_invalidation_level: invalidation,
        filled_portions: filled,
        active_take_profit_targets: targets,
    })
}

pub async fn serve_paper_performance(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaperPerformanceQuery>,
) -> impl IntoResponse {
    let trades = crate::db::paper_query_trades(&state.pool, query.symbol.as_deref(), 100).await;

    #[derive(Debug, serde::Serialize)]
    struct PaperPerformanceResponse {
        trades: Vec<crate::db::PaperTradeRecord>,
        total_trades: usize,
        wins: usize,
        losses: usize,
        win_rate: f64,
        profit_factor: f64,
        total_pnl: f64,
        avg_roi: f64,
        max_drawdown_pct: f64,
    }

    let total = trades.len();
    let wins = trades.iter().filter(|t| t.realized_pnl > 0.0).count();
    let losses = trades.iter().filter(|t| t.realized_pnl < 0.0).count();
    let win_rate = if total > 0 {
        wins as f64 / total as f64
    } else {
        0.0
    };

    let gross_profit: f64 = trades
        .iter()
        .filter(|t| t.realized_pnl > 0.0)
        .map(|t| t.realized_pnl)
        .sum();
    let gross_loss: f64 = trades
        .iter()
        .filter(|t| t.realized_pnl < 0.0)
        .map(|t| t.realized_pnl.abs())
        .sum();
    let profit_factor = if gross_loss > 0.0 {
        gross_profit / gross_loss
    } else if gross_profit > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    let total_pnl: f64 = trades.iter().map(|t| t.realized_pnl).sum();
    let avg_roi = if total > 0 {
        trades.iter().map(|t| t.roi_pct).sum::<f64>() / total as f64
    } else {
        0.0
    };

    let mut cumulative = 0.0;
    let mut peak = 0.0;
    let mut max_dd = 0.0;
    for t in trades.iter().rev() {
        cumulative += t.realized_pnl;
        if cumulative > peak {
            peak = cumulative;
        }
        let dd = peak - cumulative;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    let max_drawdown_pct = if peak > 0.0 {
        (max_dd / peak) * 100.0
    } else {
        0.0
    };

    Json(PaperPerformanceResponse {
        trades,
        total_trades: total,
        wins,
        losses,
        win_rate,
        profit_factor,
        total_pnl,
        avg_roi,
        max_drawdown_pct,
    })
}
