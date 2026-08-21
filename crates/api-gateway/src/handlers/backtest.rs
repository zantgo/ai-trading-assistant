//! Backtesting Engine (BTE) handlers — archive backfill, progress, and
//! the extended data-coverage surface.
//!
//! The BTE binds to one **running instance** at a time: the instance
//! provides the exchange, the internal symbol, and the TF ladder. A
//! backfill may only run when the bound instance exists and is running,
//! and only one backfill per instance at a time (409).

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use backtesting_engine::backfill::{
    run_backfill, BackfillJobConfig, PageFetcher,
};
use backtesting_engine::registry::BacktestRegistry;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// POST /api/backtest/archive/backfill — start an on-demand deep-history
/// backfill for a running instance.
#[derive(serde::Deserialize)]
pub struct BackfillRequest {
    pub instance_id: String,
    #[serde(default)]
    pub depth_days: Option<u32>,
}

pub async fn serve_backfill_start(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BackfillRequest>,
) -> impl IntoResponse {
    // Depth contract: 1..=365 (mirrors [workspace.backtest].archive_depth_days).
    let workspace_cfg = state.workspace.config().await;
    let depth_days = payload.depth_days.unwrap_or(workspace_cfg.backtest.archive_depth_days);
    if !(1..=365).contains(&depth_days) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "depth_days must be in 1..=365",
                "code": "invalid_depth",
            })),
        )
            .into_response();
    }

    // The bound instance must exist and be running.
    let instance = {
        let instances = state.workspace.list().await;
        instances.into_iter().find(|i| i.id == payload.instance_id)
    };
    let Some(instance) = instance else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("instance '{}' not found", payload.instance_id),
                "code": "instance_not_found",
            })),
        )
            .into_response();
    };
    if instance.status().await != portfolio_supervisor::instance::InstanceStatus::Running {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("instance '{}' is not running", payload.instance_id),
                "code": "instance_not_running",
            })),
        )
            .into_response();
    }

    // One backfill per instance at a time.
    if state.backtest.instance_has_active_backfill(&payload.instance_id).await {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("instance '{}' already has a running backfill", payload.instance_id),
                "code": "backfill_busy",
            })),
        )
            .into_response();
    }

    // Resolve exchange facts + TF ladder from the instance + workspace.
    let exchange = instance.exchange.as_str().to_string();
    let symbol = instance.symbol();
    let quote = match instance.pair.1.as_str() {
        "USDT" => portfolio_supervisor::session::Currency::USDT,
        _ => portfolio_supervisor::session::Currency::USDC,
    };
    let raw_symbol = instance.exchange.raw_symbol(&instance.pair.0, &quote);
    let ladder = {
        let ws = state.workspace.config().await;
        let entry = ws.instances.iter().find(|e| e.symbol == symbol);
        let micro = entry
            .map(|e| e.micro_term.candles.duration_seconds)
            .unwrap_or(60);
        let fast = entry
            .map(|e| e.fast_term.candles.duration_seconds)
            .unwrap_or(180);
        let slow = entry
            .and_then(|e| e.slow_term.as_ref())
            .map(|t| t.candles.duration_seconds)
            .unwrap_or(ws.slow_timeframe.duration_seconds);
        let macro_tf = entry
            .and_then(|e| e.macro_term.as_ref())
            .map(|t| t.candles.duration_seconds)
            .unwrap_or(ws.macro_timeframe.duration_seconds);
        vec![micro, fast, slow, macro_tf]
    };

    // Production page fetcher wired to the instance's exchange.
    let fetcher: PageFetcher = match exchange.as_str() {
        "Bitget" => {
            let raw = raw_symbol;
            let internal = symbol.clone();
            let product_type = if raw.ends_with("USDT") {
                "USDT-FUTURES"
            } else {
                "USDC-FUTURES"
            }
            .to_string();
            let rest_url = state.platform.read().await.bitget.rest_url();
            let raw_for_fetcher = raw;
            Arc::new(move |tf_secs, start_ms, end_ms| {
                let raw = raw_for_fetcher.clone();
                let internal = internal.clone();
                let product_type = product_type.clone();
                let rest_url = rest_url.clone();
                Box::pin(async move {
                    let interval = network_adapters::adapters::bitget_rest::timeframe_secs_to_interval(tf_secs);
                    network_adapters::adapters::bitget_rest::fetch_historical_candles_page(
                        &raw,
                        &internal,
                        &product_type,
                        interval,
                        start_ms,
                        end_ms,
                        200,
                        &rest_url,
                    )
                    .await
                })
            })
        }
        _ => {
            let raw = raw_symbol;
            let internal = symbol.clone();
            let rest_url = state.platform.read().await.hyperliquid.rest_url();
            Arc::new(move |tf_secs, start_ms, end_ms| {
                let raw = raw.clone();
                let internal = internal.clone();
                let rest_url = rest_url.clone();
                Box::pin(async move {
                    let interval = network_adapters::adapters::hyperliquid_rest::timeframe_secs_to_interval(tf_secs);
                    network_adapters::adapters::hyperliquid_rest::fetch_historical_candles(
                        &raw,
                        &internal,
                        interval,
                        start_ms,
                        end_ms,
                        &rest_url,
                    )
                    .await
                })
            })
        }
    };

    let job_id = database_storage::queries::archive::insert_backfill_job(
        &state.pool,
        &payload.instance_id,
        &symbol,
        &exchange,
        depth_days,
    )
    .await
    .unwrap_or_else(|| state.backtest.alloc_backfill_id());

    let progress = Arc::new(tokio::sync::Mutex::new(
        backtesting_engine::backfill::BackfillProgress::new(
            job_id,
            payload.instance_id.clone(),
            symbol.clone(),
            exchange.clone(),
            depth_days,
        ),
    ));
    let cancel = Arc::new(AtomicBool::new(false));

    {
        let mut map = state.backtest.backfills.write().await;
        map.insert(
            job_id,
            backtesting_engine::registry::TrackedBackfill {
                progress: progress.clone(),
                cancel: cancel.clone(),
            },
        );
    }

    let cfg = BackfillJobConfig {
        instance_id: payload.instance_id.clone(),
        exchange,
        symbol,
        timeframes: ladder,
        depth_days,
        backtest: workspace_cfg.backtest.clone(),
        fetcher,
    };
    let pool = state.pool.clone();
    tokio::spawn(async move {
        run_backfill(pool, cfg, progress, cancel).await;
    });

    Json(serde_json::json!({ "job_id": job_id, "depth_days": depth_days })).into_response()
}

/// GET /api/backtest/archive/progress/:id — live progress for a backfill.
pub async fn serve_backfill_progress(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let tracked = {
        let map = state.backtest.backfills.read().await;
        map.get(&id).map(|t| t.progress.clone())
    };
    match tracked {
        Some(progress) => {
            let p = progress.lock().await.clone();
            Json(serde_json::json!({
                "job_id": p.job_id,
                "instance_id": p.instance_id,
                "symbol": p.symbol,
                "exchange": p.exchange,
                "depth_days": p.depth_days,
                "status": p.status.as_str(),
                "pages_fetched": p.pages_fetched,
                "candles_stored": p.candles_stored,
                "cursor_ts_secs": p.cursor_ts_secs,
                "started_at": p.started_at,
                "updated_at": p.updated_at,
                "error": p.error,
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "backfill job not found" })),
        )
            .into_response(),
    }
}

/// GET /api/backtest/coverage — extended coverage for the BTE:
///
/// For every (symbol × timeframe) present in the archive:
/// `candle_count`, `earliest_secs`, `latest_secs`, and the theoretical
/// maximum lookback implied by the config (`archive_depth_days`). The
/// response also carries the recorded-snapshot coverage (the recorded
/// replay source) unchanged.
#[derive(serde::Deserialize)]
pub struct CoverageQuery {
    pub instance_id: Option<String>,
}

pub async fn serve_backtest_coverage(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CoverageQuery>,
) -> impl IntoResponse {
    let ws = state.workspace.config().await;
    let depth_days = ws.backtest.archive_depth_days as i64;
    let depth_secs = depth_days * 86400;

    // Recorded-snapshot coverage (the recorded replay source).
    let snapshot_rows = database_storage::query_backtest_coverage(&state.pool).await;
    let snapshots: Vec<serde_json::Value> = snapshot_rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "symbol": r.symbol,
                "timeframe_secs": r.timeframe_secs,
                "snapshot_count": r.snapshot_count,
                "earliest_secs": r.earliest_secs,
                "latest_secs": r.latest_secs,
            })
        })
        .collect();

    // Candle-archive coverage (the historical replay source). Resolve the
    // bound instance's symbols + ladder up-front (the filter iterator
    // cannot await).
    let mut bound_symbols: Option<Vec<String>> = None;
    let mut bound_ladder: Option<Vec<u64>> = None;
    if let Some(id) = &query.instance_id {
        let ws = state.workspace.config().await;
        let instances = state.workspace.list().await;
        let inst = instances.iter().find(|i| i.id == *id);
        bound_symbols = Some(
            instances
                .iter()
                .filter(|i| i.id == *id)
                .map(|i| i.symbol())
                .collect(),
        );
        bound_ladder = inst.map(|i| {
            let symbol = i.symbol();
            let entry = ws.instances.iter().find(|e| e.symbol == symbol);
            let micro = entry.map(|e| e.micro_term.candles.duration_seconds).unwrap_or(60);
            let fast = entry.map(|e| e.fast_term.candles.duration_seconds).unwrap_or(180);
            let slow = entry
                .and_then(|e| e.slow_term.as_ref())
                .map(|t| t.candles.duration_seconds)
                .unwrap_or(ws.slow_timeframe.duration_seconds);
            let macro_tf = entry
                .and_then(|e| e.macro_term.as_ref())
                .map(|t| t.candles.duration_seconds)
                .unwrap_or(ws.macro_timeframe.duration_seconds);
            vec![micro, fast, slow, macro_tf]
        });
    }
    let burn_in_secs = ws.backtest.warmup_bars as i64
        * bound_ladder
            .as_ref()
            .and_then(|l| l.iter().copied().max())
            .unwrap_or(900) as i64;
    let archive_rows = database_storage::queries::archive::query_archive_coverage(&state.pool).await;
    let archive: Vec<serde_json::Value> = archive_rows
        .into_iter()
        .filter(|r| match &bound_symbols {
            Some(symbols) => symbols.iter().any(|s| s == &r.symbol),
            None => true,
        })
        .map(|r| {
            let earliest = r.earliest_secs.unwrap_or(0);
            serde_json::json!({
                "symbol": r.symbol,
                "timeframe_secs": r.timeframe_secs,
                "candle_count": r.candle_count,
                "earliest_secs": r.earliest_secs,
                "latest_secs": r.latest_secs,
                "covered_span_secs": r.latest_secs.unwrap_or(0) - earliest,
                "max_lookback_secs": depth_secs,
                "coverage_pct": if depth_secs > 0 {
                    ((r.latest_secs.unwrap_or(0) - earliest) as f64 / depth_secs as f64 * 100.0)
                        .clamp(0.0, 100.0)
                } else {
                    0.0
                },
            })
        })
        .collect();

    Json(serde_json::json!({
        "archive_depth_days": depth_days,
        "burn_in_secs": burn_in_secs,
        "ladder": bound_ladder,
        "snapshots": snapshots,
        "archive": archive,
        "backfill_jobs": database_storage::queries::archive::query_backfill_jobs(&state.pool, 10)
            .await
            .into_iter()
            .map(|j| serde_json::json!({
                "id": j.id,
                "instance_id": j.instance_id,
                "symbol": j.symbol,
                "depth_days": j.depth_days,
                "status": j.status,
                "pages_fetched": j.pages_fetched,
                "candles_stored": j.candles_stored,
                "earliest_ts_secs": j.earliest_ts_secs,
                "error": j.error,
            }))
            .collect::<Vec<_>>(),
    }))
    .into_response()
}

/// Cancel a running backfill (best-effort).
pub async fn serve_backfill_cancel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    state.backtest.cancel_backfill(id).await;
    Json(serde_json::json!({ "cancelled": id }))
}

/// Helper used by the run endpoint (P8): confirm no backfill is actively
/// writing for the bound instance (backtests read a stable archive).
pub async fn instance_backfill_running(registry: &Arc<BacktestRegistry>, instance_id: &str) -> bool {
    registry.instance_has_active_backfill(instance_id).await
}
