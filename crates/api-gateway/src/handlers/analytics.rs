use crate::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct AnalyticsQuery {
    pub policy_id: Option<String>,
    pub limit: Option<u32>,
}

pub async fn serve_strategy_analytics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    // v7.3: the significance treatment comes from `[workspace.analytics]` —
    // the same α / Monte Carlo runs the on-demand evaluator uses.
    let analytics = {
        let ws = state.workspace.config().await;
        performance_analytics::strategy_analytics::AnalyticsParams {
            alpha: ws.analytics.alpha,
            monte_carlo_runs: ws.analytics.monte_carlo_runs,
            min_trades_for_verdict: ws.analytics.min_trades_for_verdict,
        }
    };
    let rows = if let Some(ref pid) = query.policy_id {
        database_storage::query_strategy_analytics_history(
            &state.pool,
            Some(pid),
            query.limit.unwrap_or(50).min(crate::types::API_MAX_LIMIT),
        )
        .await
    } else {
        let on_demand =
            performance_analytics::performance_evaluator::compute_strategy_on_demand(
                &state.pool,
                analytics,
            )
            .await;
        if on_demand.is_empty() {
            database_storage::query_strategy_analytics_history(
                &state.pool,
                None,
                query.limit.unwrap_or(50).min(crate::types::API_MAX_LIMIT),
            )
            .await
        } else {
            on_demand
        }
    };
    Json(rows)
}

pub async fn serve_strategy_analytics_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let rows = database_storage::query_strategy_analytics_history(
        &state.pool,
        query.policy_id.as_deref(),
        query.limit.unwrap_or(100).min(crate::types::API_MAX_LIMIT),
    )
    .await;
    Json(rows)
}

pub async fn serve_risk_analytics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let persisted = database_storage::query_risk_analytics_latest(&state.pool).await;
    if let Some(risk) = persisted {
        return Json(risk);
    }
    let risk =
        performance_analytics::performance_evaluator::compute_risk_on_demand(&state.pool).await;
    Json(risk)
}

pub async fn serve_performance_matrix(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let rows = if let Some(ref pid) = query.policy_id {
        database_storage::query_performance_matrix_latest(&state.pool, Some(pid)).await
    } else {
        let on_demand =
            performance_analytics::performance_evaluator::compute_performance_on_demand(
                &state.pool,
            )
            .await;
        if on_demand.is_empty() {
            database_storage::query_performance_matrix_latest(&state.pool, None).await
        } else {
            on_demand
        }
    };
    Json(rows)
}

pub async fn serve_optimization_report(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let persisted = database_storage::query_optimization_reports(
        &state.pool,
        query.limit.unwrap_or(10).min(crate::types::API_MAX_LIMIT),
    )
    .await;
    if !persisted.is_empty() {
        return Json(persisted);
    }

    let trades = database_storage::query_all_closed_trades(&state.pool).await;
    if trades.is_empty() {
        let report = core_domain::performance::OptimizationReport {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            total_trades: 0,
            regime_reports: vec![],
            recommendations: vec![],
        };
        return Json(vec![report]);
    }

    // Single source of truth — shared with the scheduled optimizer task.
    let report = performance_analytics::strategy_optimizer::build_optimization_report(&trades);
    Json(vec![report])
}

pub async fn serve_trade_analytics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let trades =
        performance_analytics::performance_evaluator::get_trade_analytics(&state.pool).await;
    let filtered: Vec<_> = if let Some(ref pid) = query.policy_id {
        trades
            .into_iter()
            .filter(|t| t.trigger_source == *pid)
            .take(query.limit.unwrap_or(200).min(crate::types::API_MAX_LIMIT) as usize)
            .collect()
    } else {
        trades
            .into_iter()
            .take(query.limit.unwrap_or(200).min(crate::types::API_MAX_LIMIT) as usize)
            .collect()
    };
    Json(filtered)
}

pub async fn serve_performance_summary(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let summaries = if query.policy_id.is_some() {
        let _trades =
            performance_analytics::performance_evaluator::get_trade_analytics(&state.pool).await;
        let mut all =
            performance_analytics::performance_evaluator::compute_performance_summary_on_demand(
                &state.pool,
            )
            .await;
        all.retain(|s| Some(s.setup_type.clone()) == query.policy_id);
        all
    } else {
        performance_analytics::performance_evaluator::compute_performance_summary_on_demand(
            &state.pool,
        )
        .await
    };
    Json(summaries)
}

// ─── PAE L5: Backtest ────────────────────────────────────────────────

/// POST /api/backtest/run — replay recorded decisions through the executor.
///
/// Unit contract: `from_ms`/`to_ms` arrive in **milliseconds** (what
/// `Date.parse()` produces); the handler converts to Unix seconds before
/// calling the runner — `market_snapshots.timestamp` is stored in seconds.
///
/// v8 BTE: `mode` selects the replay source — `"recorded"` (recorded MME
/// decisions, default) or `"historical"` (full MME pipeline over the
/// candle archive). `instance_id` binds the run to a running instance
/// (exchange/TF ladder/config source); when omitted the legacy
/// recorded-replay path runs without instance validation.
#[derive(serde::Deserialize)]
pub struct BacktestRequest {
    pub symbol: String,
    pub timeframe_secs: u64,
    pub from_ms: i64,
    pub to_ms: i64,
    #[serde(default)]
    pub initial_capital: Option<f64>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "recorded".to_string()
}

pub async fn serve_backtest_run(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BacktestRequest>,
) -> impl IntoResponse {
    if payload.symbol.trim().is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "symbol required", "code": "invalid_symbol" })),
        )
            .into_response();
    }
    if payload.timeframe_secs == 0 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "timeframe_secs must be positive", "code": "invalid_timeframe" })),
        )
            .into_response();
    }
    if payload.to_ms <= payload.from_ms {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "to_ms must be greater than from_ms", "code": "invalid_window" })),
        )
            .into_response();
    }
    let mode = payload.mode.to_lowercase();
    if mode != "recorded" && mode != "historical" {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "mode must be 'recorded' or 'historical'", "code": "invalid_mode" })),
        )
            .into_response();
    }

    // Single-run lock: one backtest at a time (409 when busy).
    let _run_guard = match state.backtest.run_lock.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            return (
                axum::http::StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "another backtest is already running",
                    "code": "backtest_busy",
                })),
            )
                .into_response();
        }
    };

    // Instance binding (BTE): when provided, the instance must exist and
    // be running; its exchange/TF ladder/config drive the run.
    let bound_instance = match &payload.instance_id {
        Some(id) => {
            let instances = state.workspace.list().await;
            let Some(inst) = instances.into_iter().find(|i| i.id == *id) else {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("instance '{id}' not found"),
                        "code": "instance_not_found",
                    })),
                )
                    .into_response();
            };
            if inst.status().await != portfolio_supervisor::instance::InstanceStatus::Running {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("instance '{id}' is not running"),
                        "code": "instance_not_running",
                    })),
                )
                    .into_response();
            }
            Some(inst)
        }
        None => None,
    };

    // ms → s conversion (the DB stores seconds).
    let from_secs = payload.from_ms.div_euclid(1000);
    let to_secs = payload.to_ms.div_euclid(1000);

    let workspace = state.workspace.config().await;
    let fees = portfolio_supervisor::paper_trading::FeesConfig {
        maker_fee_pct: workspace.fees.maker_fee_pct,
        taker_fee_pct: workspace.fees.taker_fee_pct,
        funding_rate_8h: workspace.fees.funding_rate_8h,
        simulated_spread_pct: 0.01,
    };
    let cross_leverage = workspace.leverage.cross_leverage;
    let analytics_params = performance_analytics::strategy_analytics::AnalyticsParams {
        alpha: workspace.analytics.alpha,
        monte_carlo_runs: workspace.analytics.monte_carlo_runs,
        min_trades_for_verdict: workspace.analytics.min_trades_for_verdict,
    };

    let params = backtesting_engine::recorded::BacktestParams {
        symbol: payload.symbol,
        timeframe_secs: payload.timeframe_secs,
        from_secs,
        to_secs,
        initial_capital: payload.initial_capital.unwrap_or(1000.0),
    };

    let result = if mode == "historical" {
        // Historical mode requires an instance (exchange + ladder + config).
        let Some(inst) = &bound_instance else {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "historical mode requires an instance_id",
                    "code": "instance_required",
                })),
            )
                .into_response();
        };

        // Build the ladder + per-TF configs exactly like the registry.
        let symbol = inst.symbol();
        let entry = workspace.instances.iter().find(|e| e.symbol == symbol);
        let micro = entry.map(|e| e.micro_term.candles.duration_seconds).unwrap_or(60);
        let fast = entry.map(|e| e.fast_term.candles.duration_seconds).unwrap_or(180);
        let slow = entry
            .and_then(|e| e.slow_term.as_ref())
            .map(|t| t.candles.duration_seconds)
            .unwrap_or(workspace.slow_timeframe.duration_seconds);
        let macro_tf = entry
            .and_then(|e| e.macro_term.as_ref())
            .map(|t| t.candles.duration_seconds)
            .unwrap_or(workspace.macro_timeframe.duration_seconds);
        let ladder = vec![micro, fast, slow, macro_tf];
        if !ladder.contains(&payload.timeframe_secs) {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("timeframe_secs {} is not part of the instance ladder {:?}", payload.timeframe_secs, ladder),
                    "code": "invalid_timeframe",
                })),
            )
                .into_response();
        }

        // Coverage validation: the archive must cover the window
        // (burn-in inclusive) for every ladder TF.
        let burn_in_secs = workspace.backtest.warmup_bars as i64 * macro_tf as i64;
        let load_from = from_secs - burn_in_secs;
        let mut coverage_ok = true;
        let mut coverage_detail: Vec<serde_json::Value> = Vec::new();
        for tf in &ladder {
            let rows = database_storage::queries::archive::query_archive_window(
                &state.pool,
                &symbol,
                *tf,
                load_from,
                to_secs,
                1,
            )
            .await;
            let cov = database_storage::queries::archive::query_archive_coverage(&state.pool).await;
            let span = cov
                .iter()
                .find(|c| c.symbol == symbol && c.timeframe_secs == *tf as i64);
            if rows.is_empty() {
                coverage_ok = false;
            }
            coverage_detail.push(serde_json::json!({
                "timeframe_secs": tf,
                "has_data": !rows.is_empty(),
                "earliest_secs": span.and_then(|c| c.earliest_secs),
                "latest_secs": span.and_then(|c| c.latest_secs),
            }));
        }
        if !coverage_ok {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "not enough archived data for the requested window (burn-in included)",
                    "code": "not_enough_data",
                    "coverage": coverage_detail,
                    "hint": "The run flow fetches missing archive data automatically (see the Preparing-data step); if you see this error, the auto-prepare did not complete — retry or reduce the depth.",
                })),
            )
                .into_response();
        }

        let exchange = match inst.exchange.as_str() {
            "Bitget" => core_domain::normalized::Exchange::Bitget,
            _ => core_domain::normalized::Exchange::Hyperliquid,
        };
        // BTE v8.1 fidelity: per-slot TimeframeConfigs + the activation
        // ActiveSet — EXACTLY what the live MME builds per instance
        // (registry/mod.rs `add_instance` + registry/pipelines.rs), so the
        // historical replay uses the same indicator periods, weights, and
        // activation toggles the live pipeline uses.
        let entry = workspace.instances.iter().find(|e| e.symbol == symbol);
        let micro_cfg = entry
            .map(|e| e.micro_term.clone())
            .unwrap_or_else(|| config_models::TimeframeConfig::new(60, workspace.indicators.clone()));
        let fast_cfg = entry
            .map(|e| e.fast_term.clone())
            .unwrap_or_else(|| config_models::TimeframeConfig::new(180, workspace.indicators.clone()));
        let slow_cfg = entry
            .and_then(|e| e.slow_term.clone())
            .unwrap_or_else(|| {
                config_models::TimeframeConfig::new(
                    workspace.slow_timeframe.duration_seconds,
                    workspace.indicators.clone(),
                )
            });
        let macro_cfg = entry
            .and_then(|e| e.macro_term.clone())
            .unwrap_or_else(|| {
                config_models::TimeframeConfig::new(
                    workspace.macro_timeframe.duration_seconds,
                    workspace.indicators.clone(),
                )
            });
        let mut tf_configs = std::collections::HashMap::new();
        tf_configs.insert(micro_cfg.candles.duration_seconds, micro_cfg);
        tf_configs.insert(fast_cfg.candles.duration_seconds, fast_cfg);
        tf_configs.insert(slow_cfg.candles.duration_seconds, slow_cfg);
        tf_configs.insert(macro_cfg.candles.duration_seconds, macro_cfg);
        let active_set = market_analyzer::active_set::ActiveSet::from_config(
            &workspace.activation,
            entry.and_then(|e| e.activation.as_ref()),
            workspace.config_version,
            workspace.liquidity.enabled,
        );
        let run_cfg = backtesting_engine::historical::HistoricalRunConfig {
            symbol: symbol.clone(),
            ladder,
            tf_configs,
            fib_config: workspace.fibonacci.clone(),
            active_set,
            exchange,
            warmup_bars: workspace.backtest.warmup_bars,
            max_equity_points: workspace.backtest.max_equity_points,
        };

        backtesting_engine::historical::run_historical_backtest(
            &state.pool,
            &params,
            &workspace.minimal_tae,
            &fees,
            cross_leverage,
            analytics_params,
            &run_cfg,
        )
        .await
    } else {
        // Recorded mode: pre-flight data validation — a UI-driven run
        // over an empty window must fail loudly with coverage numbers.
        let coverage = database_storage::query_backtest_coverage(&state.pool).await;
        let cov = coverage
            .iter()
            .find(|c| c.symbol == params.symbol && c.timeframe_secs == params.timeframe_secs as i64);
        let window_count = database_storage::queries::snapshots::query_backtest_snapshots(
            &state.pool,
            &params.symbol,
            params.timeframe_secs,
            from_secs,
            to_secs,
            1,
        )
        .await
        .len();
        if window_count == 0 {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "not enough data for the requested window",
                    "code": "not_enough_data",
                    "snapshot_count": window_count,
                    "earliest_secs": cov.map(|c| c.earliest_secs),
                    "latest_secs": cov.map(|c| c.latest_secs),
                    "hint": "The requested window has no recorded snapshots. Use /api/backtest/coverage to inspect available data; recorded backtests only cover periods the daemon was running (≤ 7 days retention). For deeper windows use mode=historical with an archived instance.",
                })),
            )
                .into_response();
        }

        backtesting_engine::recorded::run_backtest(
            &state.pool,
            &params,
            &workspace.minimal_tae,
            &fees,
            cross_leverage,
            analytics_params,
        )
        .await
    };

    let params_json = serde_json::to_string(&result.params).unwrap_or_default();
    let summary_json = serde_json::to_string(&serde_json::json!({
        "total_trades": result.total_trades,
        "win_count": result.win_count,
        "loss_count": result.loss_count,
        "win_rate": result.win_rate,
        "gross_profit": result.gross_profit,
        "gross_loss": result.gross_loss,
        "profit_factor": result.profit_factor,
        "expectancy": result.expectancy,
        "max_drawdown_pct": result.max_drawdown_pct,
    }))
    .unwrap_or_default();
    let stats_json = serde_json::to_string(&result.stats).unwrap_or_default();
    let trades_json = serde_json::to_string(&result.trades).unwrap_or_default();
    let equity_curve_json = serde_json::to_string(&result.equity_curve).unwrap_or_default();

    let backtest_id = database_storage::insert_backtest_run(
        &state.pool,
        &params_json,
        &summary_json,
        &stats_json,
        &trades_json,
        &equity_curve_json,
    )
    .await;

    // BTE v8: persist the normalized DS rows (trades/equity/portfolio/
    // signals/metrics) for later data-science queries.
    let ds_trades: Vec<database_storage::queries::backtest_ds::DsTrade> = result
        .trades
        .iter()
        .map(|t| database_storage::queries::backtest_ds::DsTrade {
            ts_close_secs: t.timestamp,
            direction: t.direction.clone(),
            entry_price: t.entry_price,
            exit_price: t.exit_price,
            size: t.size,
            pnl: t.pnl,
            exit_reason: t.exit_reason.clone(),
        })
        .collect();
    let ds_metrics: Vec<database_storage::queries::backtest_ds::DsMetric> = vec![
        ("mode".to_string(), mode.clone()),
        ("total_trades".to_string(), result.total_trades.to_string()),
        ("win_rate".to_string(), format!("{:.2}", result.win_rate)),
        ("profit_factor".to_string(), result.profit_factor.map(|p| format!("{p:.4}")).unwrap_or_default()),
        ("max_drawdown_pct".to_string(), format!("{:.2}", result.max_drawdown_pct)),
        ("classification".to_string(), format!("{:?}", result.stats.classification)),
        ("p_value".to_string(), format!("{:.6}", result.stats.p_value)),
        ("p_mc".to_string(), format!("{:.6}", result.stats.p_mc)),
        ("instance_id".to_string(), payload.instance_id.clone().unwrap_or_default()),
    ]
    .into_iter()
    .map(|(key, value)| database_storage::queries::backtest_ds::DsMetric { key, value })
    .collect();
    database_storage::queries::backtest_ds::insert_backtest_ds_rows(
        &state.pool,
        backtest_id,
        &ds_trades,
        &result.equity_curve,
        &result.portfolio,
        &result.signals,
        &ds_metrics,
    )
    .await;
    database_storage::queries::backtest_ds::update_backtest_run_meta(
        &state.pool,
        backtest_id,
        payload.instance_id.as_deref(),
        &mode,
    )
    .await;

    // Historical mode: persist the exact input bars for reproducibility.
    if mode == "historical" && workspace.backtest.store_input_bars {
        if let Some(inst) = &bound_instance {
            let symbol = inst.symbol();
            let burn_in_secs =
                workspace.backtest.warmup_bars as i64 * workspace.macro_timeframe.duration_seconds as i64;
            if let Ok(mut tx) = state.pool.begin().await {
                let ladder = {
                    let entry = workspace.instances.iter().find(|e| e.symbol == symbol);
                    let micro = entry.map(|e| e.micro_term.candles.duration_seconds).unwrap_or(60);
                    let fast = entry.map(|e| e.fast_term.candles.duration_seconds).unwrap_or(180);
                    let slow = entry
                        .and_then(|e| e.slow_term.as_ref())
                        .map(|t| t.candles.duration_seconds)
                        .unwrap_or(workspace.slow_timeframe.duration_seconds);
                    let macro_tf = entry
                        .and_then(|e| e.macro_term.as_ref())
                        .map(|t| t.candles.duration_seconds)
                        .unwrap_or(workspace.macro_timeframe.duration_seconds);
                    vec![micro, fast, slow, macro_tf]
                };
                for tf in ladder {
                    let bars = database_storage::queries::archive::query_archive_window(
                        &state.pool,
                        &symbol,
                        tf,
                        from_secs - burn_in_secs,
                        to_secs,
                        100_000,
                    )
                    .await;
                    for b in bars {
                        let _ = sqlx::query(
                            "INSERT OR IGNORE INTO backtest_input_bars
                                (run_id, symbol, timeframe_secs, ts_secs, open, high, low, close, volume)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        )
                        .bind(backtest_id)
                        .bind(&symbol)
                        .bind(tf as i64)
                        .bind(b.ts_secs)
                        .bind(b.open.map(|v| v.to_string()).unwrap_or_default())
                        .bind(b.high.map(|v| v.to_string()).unwrap_or_default())
                        .bind(b.low.map(|v| v.to_string()).unwrap_or_default())
                        .bind(b.close.map(|v| v.to_string()).unwrap_or_default())
                        .bind(b.volume.map(|v| v.to_string()).unwrap_or_default())
                        .execute(&mut *tx)
                        .await;
                    }
                }
                let _ = tx.commit().await;
            }
        }
    }

    Json(serde_json::json!({
        "backtest_id": backtest_id,
        "params": result.params,
        "mode": mode,
        "summary": {
            "total_trades": result.total_trades,
            "win_count": result.win_count,
            "loss_count": result.loss_count,
            "win_rate": result.win_rate,
            "gross_profit": result.gross_profit,
            "gross_loss": result.gross_loss,
            "profit_factor": result.profit_factor,
            "expectancy": result.expectancy,
            "max_drawdown_pct": result.max_drawdown_pct,
        },
        "stats": result.stats,
        "trades": result.trades,
        "equity_curve": result.equity_curve,
    }))
    .into_response()
}

/// GET /api/backtest/:id — fetch a persisted run.
pub async fn serve_backtest_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    match database_storage::query_backtest_run(&state.pool, id).await {
        Some((params, summary, stats, trades, equity_curve)) => Json(serde_json::json!({
            "backtest_id": id,
            "params": serde_json::from_str::<serde_json::Value>(&params).unwrap_or_default(),
            "summary": serde_json::from_str::<serde_json::Value>(&summary).unwrap_or_default(),
            "stats": serde_json::from_str::<serde_json::Value>(&stats).unwrap_or_default(),
            "trades": serde_json::from_str::<serde_json::Value>(&trades).unwrap_or_default(),
            "equity_curve": serde_json::from_str::<serde_json::Value>(&equity_curve).unwrap_or_default(),
        }))
        .into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "Backtest run not found").into_response(),
    }
}

/// GET /api/backtest/list?limit=N — recent persisted runs (History tab).
pub async fn serve_backtest_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(20).min(200);
    let rows = database_storage::query_backtest_runs_list(&state.pool, limit).await;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "created_at": r.created_at,
                "instance_id": r.instance_id,
                "mode": r.mode,
                "params": serde_json::from_str::<serde_json::Value>(&r.params_json).unwrap_or_default(),
                "summary": serde_json::from_str::<serde_json::Value>(&r.summary_json).unwrap_or_default(),
            })
        })
        .collect();
    Json(items)
}

/// GET /api/backtest/coverage — moved to `handlers::backtest` (v8 BTE):
// the endpoint now serves the extended `{ snapshots, archive, ... }`
// shape with candle-archive coverage + theoretical lookback.

// ─── BTE DS read endpoints (v8) ────────────────────────────────────────

/// GET /api/backtest/:id/trades?limit=&offset= — normalized trade rows.
pub async fn serve_backtest_trades(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(200).min(5000);
    let rows = database_storage::queries::backtest_ds::query_backtest_trades(
        &state.pool, id, limit, 0,
    )
    .await;
    Json(serde_json::json!({
        "run_id": id,
        "count": rows.len(),
        "trades": rows.into_iter().map(|t| serde_json::json!({
            "ts_close_secs": t.ts_close_secs,
            "direction": t.direction,
            "entry_price": t.entry_price,
            "exit_price": t.exit_price,
            "size": t.size,
            "pnl": t.pnl,
            "exit_reason": t.exit_reason,
        })).collect::<Vec<_>>(),
    }))
}

/// GET /api/backtest/:id/equity — normalized equity curve rows.
pub async fn serve_backtest_equity(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let rows = database_storage::queries::backtest_ds::query_backtest_equity(&state.pool, id).await;
    Json(serde_json::json!({
        "run_id": id,
        "equity": rows,
    }))
}

/// GET /api/backtest/:id/portfolio — capital/exposure/drawdown samples.
pub async fn serve_backtest_portfolio(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let rows =
        database_storage::queries::backtest_ds::query_backtest_portfolio(&state.pool, id).await;
    Json(serde_json::json!({
        "run_id": id,
        "portfolio": rows.into_iter().map(|p| serde_json::json!({
            "ts_secs": p.ts_secs,
            "equity": p.equity,
            "cash": p.cash,
            "margin_used": p.margin_used,
            "exposure_pct": p.exposure_pct,
            "drawdown_pct": p.drawdown_pct,
            "positions_open": p.positions_open,
        })).collect::<Vec<_>>(),
    }))
}

/// GET /api/backtest/:id/signals — per-tick decision snapshots.
pub async fn serve_backtest_signals(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let rows = database_storage::queries::backtest_ds::query_backtest_signals(&state.pool, id).await;
    Json(serde_json::json!({
        "run_id": id,
        "count": rows.len(),
        "signals": rows.into_iter().map(|s| serde_json::json!({
            "ts_secs": s.ts_secs,
            "timeframe_secs": s.timeframe_secs,
            "label": s.label,
            "kind": s.kind,
            "value": s.value,
        })).collect::<Vec<_>>(),
    }))
}

/// GET /api/backtest/:id/metrics — summary + NHST key/values.
pub async fn serve_backtest_metrics(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let rows = database_storage::queries::backtest_ds::query_backtest_metrics(&state.pool, id).await;
    Json(serde_json::json!({
        "run_id": id,
        "metrics": rows.into_iter().map(|m| serde_json::json!({
            "key": m.key,
            "value": m.value,
        })).collect::<Vec<_>>(),
    }))
}
