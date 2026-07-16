use crate::types::{
    CommissionProjectionPayload, DecisionProfileCreate, DecisionProfileUpdate, EvaluateRequest,
    FeeTableQuery, ProfileIndicatorAdd, ProfileIndicatorUpdate, RiskCalculateRequest,
    RiskProfileCreate,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use std::sync::Arc;

// ─── Decision Profiles ───────────────────────────────────────────

pub async fn serve_decision_profiles_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let profiles = database_storage::decision_profiles_list(&state.pool).await;
    Json(profiles)
}

pub async fn serve_decision_profile_create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DecisionProfileCreate>,
) -> impl IntoResponse {
    if payload.profile_name.trim().is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Profile name required").into_response();
    }
    let id = database_storage::decision_profile_insert(
        &state.pool,
        payload.profile_name.trim(),
        payload.long_threshold,
        payload.short_threshold,
    )
    .await;
    if id > 0 {
        (
            axum::http::StatusCode::CREATED,
            format!("Profile created with id {}", id),
        )
            .into_response()
    } else {
        (
            axum::http::StatusCode::CONFLICT,
            "Profile name already exists or DB error",
        )
            .into_response()
    }
}

pub async fn serve_decision_profile_update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<DecisionProfileUpdate>,
) -> impl IntoResponse {
    let ok = database_storage::decision_profile_update(
        &state.pool,
        id,
        &payload.profile_name,
        payload.long_threshold,
        payload.short_threshold,
    )
    .await;
    if ok {
        (axum::http::StatusCode::OK, "Profile updated").into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Profile not found").into_response()
    }
}

pub async fn serve_decision_profile_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    database_storage::decision_profile_delete(&state.pool, id).await;
    (axum::http::StatusCode::OK, "Profile deleted").into_response()
}

pub async fn serve_decision_evaluate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<EvaluateRequest>,
) -> impl IntoResponse {
    let inputs = portfolio_supervisor::profile_evaluation::EvaluateInputs {
        current_price: payload.current_price,
        close: payload.close,
        rsi: payload.rsi,
        rsi_divergence_status: payload.rsi_divergence_status,
        macd_divergence_status: payload.macd_divergence_status,
        macd_line: payload.macd_line,
        macd_signal: payload.macd_signal,
        macd_hist: payload.macd_hist,
        macd_histogram_peak: payload.macd_histogram_peak,
        macd_crossover_detected: payload.macd_crossover_detected,
        macd_crossover_direction: payload.macd_crossover_direction,
        squeeze_on: payload.squeeze_on,
        squeeze_release_trigger: payload.squeeze_release_trigger,
        squeeze_momentum: payload.squeeze_momentum,
        squeeze_momentum_direction: payload.squeeze_momentum_direction,
        adx: payload.adx,
        adx_plus: payload.adx_plus,
        adx_minus: payload.adx_minus,
        adx_slope: payload.adx_slope,
        bbwp: payload.bbwp,
        rvol: payload.rvol,
        vwap: payload.vwap,
        chart_pattern: payload.chart_pattern,
        chart_pattern_confidence: payload.chart_pattern_confidence,
        atr: payload.atr,
        bb_upper: payload.bb_upper,
        bb_middle: payload.bb_middle,
        bb_lower: payload.bb_lower,
        ema_fast: payload.ema_fast,
        ema_medium: payload.ema_medium,
        ema_slow: payload.ema_slow,
        ema_long: payload.ema_long,
        ema_stack_state: payload.ema_stack_state,
    };
    let snap = portfolio_supervisor::profile_evaluation::snapshot_values_from_evaluate(&inputs);
    let score = portfolio_supervisor::profile_evaluation::evaluate_profile(
        &state.pool,
        id,
        &snap,
        payload.historical_prices.as_deref().unwrap_or(&[]),
    )
    .await;
    Json(score)
}

pub async fn serve_profile_indicator_add(
    State(state): State<Arc<AppState>>,
    Path(profile_id): Path<i64>,
    Json(payload): Json<ProfileIndicatorAdd>,
) -> impl IntoResponse {
    let status = if payload.override_status.is_empty() {
        "NONE"
    } else {
        &payload.override_status
    };
    let id = database_storage::profile_indicator_insert(
        &state.pool,
        profile_id,
        &payload.indicator_name,
        payload.weight,
        status,
    )
    .await;
    if id > 0 {
        (
            axum::http::StatusCode::CREATED,
            format!("Indicator added with id {}", id),
        )
            .into_response()
    } else {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Failed to add indicator",
        )
            .into_response()
    }
}

pub async fn serve_profile_indicator_update(
    State(state): State<Arc<AppState>>,
    Path((_profile_id, indicator_id)): Path<(i64, i64)>,
    Json(payload): Json<ProfileIndicatorUpdate>,
) -> impl IntoResponse {
    let status = if payload.override_status.is_empty() {
        "NONE"
    } else {
        &payload.override_status
    };
    let ok = database_storage::profile_indicator_update(&state.pool, indicator_id, payload.weight, status)
        .await;
    if ok {
        (axum::http::StatusCode::OK, "Indicator updated").into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Indicator not found").into_response()
    }
}

pub async fn serve_profile_indicator_delete(
    State(state): State<Arc<AppState>>,
    Path((_profile_id, indicator_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    database_storage::profile_indicator_delete(&state.pool, indicator_id).await;
    (axum::http::StatusCode::OK, "Indicator removed").into_response()
}

// ─── Risk Profiles ───────────────────────────────────────────────

pub async fn serve_risk_profiles_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let profiles = database_storage::risk_profiles_list(&state.pool).await;
    Json(profiles)
}

pub async fn serve_risk_profile_create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RiskProfileCreate>,
) -> impl IntoResponse {
    if payload.profile_name.trim().is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Profile name required").into_response();
    }
    let id = database_storage::risk_profile_insert(
        &state.pool,
        payload.profile_name.trim(),
        payload.capital,
        payload.max_risk_pct,
        payload.leverage,
        payload.commission_pct,
        payload.funding_rate_8h,
        payload.spread,
    )
    .await;
    if id > 0 {
        (
            axum::http::StatusCode::CREATED,
            format!("Risk profile created with id {}", id),
        )
            .into_response()
    } else {
        (
            axum::http::StatusCode::CONFLICT,
            "Profile name already exists or DB error",
        )
            .into_response()
    }
}

pub async fn serve_risk_profile_update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<RiskProfileCreate>,
) -> impl IntoResponse {
    let ok = database_storage::risk_profile_update(
        &state.pool,
        id,
        &payload.profile_name,
        payload.capital,
        payload.max_risk_pct,
        payload.leverage,
        payload.commission_pct,
        payload.funding_rate_8h,
        payload.spread,
    )
    .await;
    if ok {
        (axum::http::StatusCode::OK, "Risk profile updated").into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Risk profile not found").into_response()
    }
}

pub async fn serve_risk_profile_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    database_storage::risk_profile_delete(&state.pool, id).await;
    (axum::http::StatusCode::OK, "Risk profile deleted").into_response()
}

pub async fn serve_risk_calculate(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RiskCalculateRequest>,
) -> impl IntoResponse {
    let (capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread) = {
        let pid = payload.profile_id;
        if let Some(profile) = database_storage::risk_profile_by_id(&state.pool, pid).await {
            (
                profile.capital,
                profile.max_risk_pct,
                profile.leverage,
                profile.commission_pct,
                profile.funding_rate_8h,
                profile.spread,
            )
        } else {
            (
                payload.capital.unwrap_or(dec!(1000)),
                payload.max_risk_pct.unwrap_or(dec!(2)),
                payload.leverage.unwrap_or(20),
                payload.commission_pct.unwrap_or(dec!(0.06)),
                payload.funding_rate_8h.unwrap_or(dec!(0)),
                payload.spread.unwrap_or(dec!(0)),
            )
        }
    };

    let input = portfolio_supervisor::risk_calculator::RiskCalculationInput {
        capital,
        max_risk_pct,
        leverage,
        direction: payload.direction,
        entry_price: payload.entry_price,
        stop_loss_price: payload.stop_loss_price.unwrap_or(payload.stop_loss),
        take_profit_price: payload.take_profit_price.unwrap_or(payload.take_profit),
        commission_pct,
        funding_rate_8h,
        spread,
        atr_value: payload.atr_value,
        atr_multiplier: payload.atr_multiplier,
        atr_target_rr: payload.atr_target_rr,
        use_dynamic_atr: payload.use_dynamic_atr.unwrap_or(false),
        min_tick_size: None,
    };

    let result = if payload.use_dynamic_atr.unwrap_or(false) && payload.atr_value.is_some() {
        portfolio_supervisor::risk_calculator::compute_risk_with_atr(&input)
    } else {
        portfolio_supervisor::risk_calculator::compute_risk(&input)
    };

    match result {
        Ok(calc) => Json(calc).into_response(),
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
    }
}

pub async fn serve_fee_table(
    State(state): State<Arc<AppState>>,
    Query(params): Query<FeeTableQuery>,
) -> impl IntoResponse {
    let config = state.workspace.config().await;
    let leverages = params.leverages.unwrap_or_else(|| vec![10, 20, 25, 40, 50]);
    let capitals = params
        .capitals
        .unwrap_or_else(|| vec![10.0, 50.0, 100.0, 500.0]);
    let order_type = params.order_type;
    let table = portfolio_supervisor::commission::generate_fee_table(
        &config.fees,
        &leverages.iter().map(|&l| l as u32).collect::<Vec<u32>>(),
        &capitals,
        &order_type,
    );
    Json(table).into_response()
}

pub async fn serve_commission_projection(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CommissionProjectionPayload>,
) -> impl IntoResponse {
    let (capital, leverage, max_risk_pct, commission_pct, funding_rate_8h) = {
        let pid = payload.profile_id;
        if let Some(profile) = database_storage::risk_profile_by_id(&state.pool, pid).await {
            (
                profile.capital.to_f64().unwrap_or(0.0),
                profile.leverage,
                profile.max_risk_pct.to_f64().unwrap_or(0.0),
                Some(profile.commission_pct.to_f64().unwrap_or(0.0)),
                Some(profile.funding_rate_8h.to_f64().unwrap_or(0.0)),
            )
        } else {
            (
                payload
                    .capital
                    .unwrap_or(dec!(1000))
                    .to_f64()
                    .unwrap_or(1000.0),
                payload.leverage.unwrap_or(20),
                payload
                    .max_risk_pct
                    .unwrap_or(dec!(2))
                    .to_f64()
                    .unwrap_or(2.0),
                payload.commission_pct.and_then(|d| d.to_f64()),
                payload.funding_rate_8h.and_then(|d| d.to_f64()),
            )
        }
    };

    let config = state.workspace.config().await;
    let input = portfolio_supervisor::commission::CommissionProjectionRequest {
        direction: payload.direction,
        entry_1: payload.entry_1.to_f64().unwrap_or(0.0),
        entry_2: payload.entry_2.to_f64().unwrap_or(0.0),
        stop_loss_1: payload.stop_loss_1.to_f64().unwrap_or(0.0),
        stop_loss_2: payload.stop_loss_2.to_f64().unwrap_or(0.0),
        take_profit_1: payload.take_profit_1.to_f64().unwrap_or(0.0),
        take_profit_2: payload.take_profit_2.to_f64().unwrap_or(0.0),
        capital,
        leverage,
        max_risk_pct,
        capital_entry_1_pct: payload.capital_entry_1_pct.to_f64().unwrap_or(0.0),
        order_type: payload.order_type,
        commission_pct,
        funding_rate_8h,
    };

    match portfolio_supervisor::commission::compute_commission_projection(&input, &config.fees) {
        Ok(proj) => Json(proj).into_response(),
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
    }
}
