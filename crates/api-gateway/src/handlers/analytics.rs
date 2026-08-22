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
        // v9: the verdict bar comes from the effective default strategy's
        // `pae` section (single source of truth).
        ws.default_strategy()
            .map(|st| {
                performance_analytics::strategy_analytics::AnalyticsParams::from_strategy(&st.pae)
            })
            .unwrap_or_default()
    };
    let rows = if let Some(ref pid) = query.policy_id {
        database_storage::query_strategy_analytics_history(
            &state.pool,
            Some(pid),
            query.limit.unwrap_or(50).min(crate::types::API_MAX_LIMIT),
        )
        .await
    } else {
        let on_demand = performance_analytics::performance_evaluator::compute_strategy_on_demand(
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

/// v8 BTE: `mode` selects the replay source — `"recorded"` (recorded MME
/// decisions, default) or `"historical"` (full MME pipeline over the
/// candle archive).
///
/// v8.2 — two payload forms:
/// - **Standalone** `{ exchange, symbols: [{ symbol, timeframes,
///   allocation_pct }], from_ms, to_ms, portfolio_capital_usd, mode }` — no
///   running instance required; multi-symbol replay against one shared
///   virtual portfolio.
/// - **Bound** (v8 compat) `{ symbol, timeframe_secs, from_ms, to_ms,
///   portfolio_capital_usd, instance_id?, mode }` — `instance_id` binds the run
///   to a running instance (exchange/TF ladder/config source); when
///   omitted the legacy recorded-replay path runs without instance
///   validation.
///
/// Both forms validate synchronously and **spawn** the run: the response
/// carries `{ run_id, status: "running" }` immediately; progress is
/// polled via `GET /api/backtest/progress/:run_id`; cancel via
/// `POST /api/backtest/cancel/:run_id`.
#[derive(serde::Deserialize, Clone)]
pub struct BacktestRequest {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timeframe_secs: u64,
    #[serde(default)]
    pub from_ms: i64,
    #[serde(default)]
    pub to_ms: i64,
    #[serde(default)]
    #[serde(alias = "initial_capital")]
    pub portfolio_capital_usd: Option<f64>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default = "default_mode")]
    pub mode: String,
    /// v8.2 standalone form: exchange name ("Hyperliquid" | "Bitget").
    #[serde(default)]
    pub exchange: Option<String>,
    /// v8.2 standalone form: one or more simulated instances.
    #[serde(default)]
    pub symbols: Option<Vec<BacktestSymbolRequest>>,
    /// v9: the strategy bound to the run (default "default"). The full
    /// strategy JSON is frozen on the run.
    #[serde(default)]
    pub strategy_id: Option<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct BacktestSymbolRequest {
    pub symbol: String,
    /// The full 4-slot ladder (micro/fast/slow/macro seconds).
    pub timeframes: Vec<u64>,
    /// Per-instance allocation override (1..=100 %). `None` = global.
    #[serde(default)]
    pub allocation_pct: Option<f64>,
}

fn default_mode() -> String {
    "recorded".to_string()
}

pub async fn serve_backtest_run(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BacktestRequest>,
) -> impl IntoResponse {
    let mode = payload.mode.to_lowercase();
    if mode != "recorded" && mode != "historical" {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "mode must be 'recorded' or 'historical'", "code": "invalid_mode" })),
        )
            .into_response();
    }
    let standalone = payload.symbols.is_some();
    if standalone && mode != "historical" {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "the standalone symbols form supports mode 'historical' only",
                "code": "invalid_mode_for_standalone",
            })),
        )
            .into_response();
    }
    if standalone {
        if payload
            .symbols
            .as_ref()
            .map(|s| s.is_empty())
            .unwrap_or(true)
        {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "symbols must not be empty", "code": "invalid_symbols" })),
            )
                .into_response();
        }
        if payload
            .exchange
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "exchange required for the standalone form", "code": "exchange_required" })),
            )
                .into_response();
        }
    } else {
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
    }
    if payload.to_ms <= payload.from_ms {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "to_ms must be greater than from_ms", "code": "invalid_window" })),
        )
            .into_response();
    }

    // Single-run lock: one backtest at a time (409 when busy).
    if state.backtest.has_running_run().await {
        return (
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "another backtest is already running",
                "code": "backtest_busy",
            })),
        )
            .into_response();
    }

    let workspace = state.workspace.config().await;
    let from_secs = payload.from_ms.div_euclid(1000);
    let to_secs = payload.to_ms.div_euclid(1000);

    let fees = portfolio_supervisor::paper_trading::FeesConfig {
        maker_fee_pct: workspace.fees.maker_fee_pct,
        taker_fee_pct: workspace.fees.taker_fee_pct,
        funding_rate_8h: workspace.fees.funding_rate_8h,
        simulated_spread_pct: 0.01,
    };
    let cross_leverage = workspace.leverage.cross_leverage;
    // v9: the verdict bar comes from the RUN's bound strategy's `pae` section.
    let analytics_params = {
        let st = match &payload.strategy_id {
            Some(name) => workspace.resolve_strategy(name).unwrap_or_default(),
            None => workspace.default_strategy().unwrap_or_default(),
        };
        performance_analytics::strategy_analytics::AnalyticsParams::from_strategy(&st.pae)
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

    // ── Historical mode configuration (standalone or bound) ──
    let mut historical_cfg: Option<backtesting_engine::historical::HistoricalRunConfig> = None;
    if mode == "historical" {
        let active_set = market_analyzer::active_set::ActiveSet::from_config(
            &workspace.activation,
            None,
            workspace.config_version,
            workspace.liquidity.enabled,
        );
        let fib_config = workspace.fibonacci.clone();
        let safety = backtesting_engine::historical::SafetyParams {
            caution_threshold: workspace.safety.consecutive_loss_caution,
            dropout_threshold: workspace.safety.consecutive_loss_dropout,
            dropout_duration_hours: workspace.safety.dropout_duration_hours,
            drawdown_limit_pct: workspace.safety.drawdown_limit_pct,
            max_daily_drawdown_pct: workspace.safety.max_daily_drawdown_pct,
            systemic_risk_threshold: workspace.safety.systemic_risk_threshold,
        };

        // Resolve the per-symbol specs: (symbol, ladder, tf_configs,
        // allocation_pct) either from the standalone payload or from the
        // bound instance's entry (exactly like the live registry).
        struct ResolvedSpec {
            symbol: String,
            ladder: Vec<u64>,
            tf_configs: std::collections::HashMap<u64, config_models::TimeframeConfig>,
            allocation_pct: Option<f64>,
        }
        let mut specs: Vec<ResolvedSpec> = Vec::new();
        let exchange: core_domain::normalized::Exchange;

        if standalone {
            let name = payload.exchange.as_deref().unwrap_or("Hyperliquid");
            exchange = if name.eq_ignore_ascii_case("Bitget") {
                core_domain::normalized::Exchange::Bitget
            } else if name.eq_ignore_ascii_case("Hyperliquid") {
                core_domain::normalized::Exchange::Hyperliquid
            } else {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "exchange must be 'Hyperliquid' or 'Bitget'",
                        "code": "invalid_exchange",
                    })),
                )
                    .into_response();
            };
            let mut allocation_sum = 0.0f64;
            for sym in payload.symbols.as_ref().expect("standalone symbols") {
                let symbol = sym.symbol.trim();
                if symbol.is_empty() {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": "empty symbol in symbols[]", "code": "invalid_symbol" })),
                    )
                        .into_response();
                }
                if sym.timeframes.len() != 4
                    || sym.timeframes.windows(2).any(|w| w[0] >= w[1])
                    || sym.timeframes.iter().any(|tf| *tf < 60)
                {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "timeframes must be 4 strictly-ascending values ≥ 60s (the archive floor)",
                            "code": "invalid_timeframes",
                        })),
                    )
                        .into_response();
                }
                if let Some(pct) = sym.allocation_pct {
                    if !pct.is_finite() || !(1.0..=100.0).contains(&pct) {
                        return (
                            axum::http::StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": format!("allocation_pct for {symbol} must be in 1..=100"),
                                "code": "invalid_allocation",
                            })),
                        )
                            .into_response();
                    }
                }
                allocation_sum += sym
                    .allocation_pct
                    .unwrap_or(workspace.minimal_tae.allocation_pct);
                let mut tf_configs = std::collections::HashMap::new();
                for tf in &sym.timeframes {
                    tf_configs.insert(
                        *tf,
                        config_models::TimeframeConfig::new(*tf, workspace.indicators.clone()),
                    );
                }
                specs.push(ResolvedSpec {
                    symbol: symbol.to_string(),
                    ladder: sym.timeframes.clone(),
                    tf_configs,
                    allocation_pct: sym.allocation_pct,
                });
            }
            if specs.len() > 100 {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "at most 100 instances per backtest",
                        "code": "too_many_symbols",
                    })),
                )
                    .into_response();
            }
            if allocation_sum > 100.0 + 1e-9 {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("Σ instance allocations = {allocation_sum:.2}% (must be <= 100%)"),
                        "code": "allocation_sum_exceeded",
                    })),
                )
                    .into_response();
            }
        } else {
            // Bound form: historical mode requires an instance.
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
            let symbol = inst.symbol();
            exchange = match inst.exchange.as_str() {
                "Bitget" => core_domain::normalized::Exchange::Bitget,
                _ => core_domain::normalized::Exchange::Hyperliquid,
            };
            let entry = workspace.instances.iter().find(|e| e.symbol == symbol);
            let micro = entry
                .map(|e| e.micro_term.candles.duration_seconds)
                .unwrap_or(60);
            let fast = entry
                .map(|e| e.fast_term.candles.duration_seconds)
                .unwrap_or(180);
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
            let micro_cfg = entry.map(|e| e.micro_term.clone()).unwrap_or_else(|| {
                config_models::TimeframeConfig::new(60, workspace.indicators.clone())
            });
            let fast_cfg = entry.map(|e| e.fast_term.clone()).unwrap_or_else(|| {
                config_models::TimeframeConfig::new(180, workspace.indicators.clone())
            });
            let slow_cfg = entry.and_then(|e| e.slow_term.clone()).unwrap_or_else(|| {
                config_models::TimeframeConfig::new(
                    workspace.slow_timeframe.duration_seconds,
                    workspace.indicators.clone(),
                )
            });
            let macro_cfg = entry.and_then(|e| e.macro_term.clone()).unwrap_or_else(|| {
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
            specs.push(ResolvedSpec {
                symbol: symbol.clone(),
                ladder,
                tf_configs,
                allocation_pct: entry.and_then(|e| e.allocation_pct),
            });
        }

        // Coverage + ceiling validation for every spec × ladder TF
        // (synchronous — quick archive queries; loud failures).
        let burn_in_secs = workspace.backtest.warmup_bars as i64
            * specs
                .iter()
                .flat_map(|s| s.ladder.iter())
                .copied()
                .max()
                .unwrap_or(900) as i64;
        let exchange_name = if exchange == core_domain::normalized::Exchange::Bitget {
            "Bitget"
        } else {
            "Hyperliquid"
        };
        for spec in &specs {
            let load_from = from_secs - burn_in_secs;
            for tf in &spec.ladder {
                let max_depth_secs = backtesting_engine::backfill::exchange_max_depth_secs(
                    exchange_name,
                    *tf,
                    &workspace.backtest,
                );
                if max_depth_secs > 0 {
                    let required = (to_secs - from_secs) + burn_in_secs;
                    if required > max_depth_secs {
                        let limit_desc = if exchange == core_domain::normalized::Exchange::Bitget {
                            format!(
                                "Bitget's {tf}s history (retention ≈ {} days)",
                                max_depth_secs / 86400
                            )
                        } else {
                            format!(
                                "Hyperliquid's {}-candle ceiling (max ≈ {} days incl. burn-in)",
                                workspace.backtest.hyperliquid.max_candles_per_tf,
                                max_depth_secs / 86400
                            )
                        };
                        return (
                            axum::http::StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": format!(
                                    "depth exceeds {limit_desc} for the {tf}s timeframe of {}",
                                    spec.symbol,
                                ),
                                "code": "depth_exceeds_ceiling",
                                "limiting_timeframe_secs": tf,
                                "symbol": spec.symbol,
                            })),
                        )
                            .into_response();
                    }
                }
                let rows = database_storage::queries::archive::query_archive_window(
                    &state.pool,
                    &spec.symbol,
                    *tf,
                    load_from,
                    to_secs,
                    1,
                )
                .await;
                if rows.is_empty() {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!(
                                "not enough archived data for {} {}s (burn-in included)",
                                spec.symbol, tf
                            ),
                            "code": "not_enough_data",
                            "symbol": spec.symbol,
                            "timeframe_secs": tf,
                            "hint": "Run the launcher's auto-prepare (or POST /api/backtest/archive/backfill) to fetch the missing archive, or reduce the depth.",
                        })),
                    )
                    .into_response();
                }
            }
        }

        historical_cfg = Some(backtesting_engine::historical::HistoricalRunConfig {
            symbols: specs
                .into_iter()
                .map(|s| backtesting_engine::historical::SymbolSpec {
                    symbol: s.symbol,
                    ladder: s.ladder,
                    tf_configs: s.tf_configs,
                    allocation_pct: s.allocation_pct,
                })
                .collect(),
            fib_config,
            active_set,
            exchange,
            warmup_bars: workspace.backtest.warmup_bars,
            max_equity_points: workspace.backtest.max_equity_points,
            safety,
            // v9: the run's bound strategy (frozen on the run record).
            strategy: match &payload.strategy_id {
                Some(name) => workspace.resolve_strategy(name).unwrap_or_else(|e| {
                    eprintln!("strategy '{name}' resolution failed ({e}); using default");
                    config_models::StrategyConfig::default()
                }),
                None => workspace.default_strategy().unwrap_or_default(),
            },
        });
    } else {
        // Recorded mode: pre-flight data validation — a UI-driven run
        // over an empty window must fail loudly with coverage numbers.
        let coverage = database_storage::query_backtest_coverage(&state.pool).await;
        let cov = coverage.iter().find(|c| {
            c.symbol == payload.symbol && c.timeframe_secs == payload.timeframe_secs as i64
        });
        let window_count = database_storage::queries::snapshots::query_backtest_snapshots(
            &state.pool,
            &payload.symbol,
            payload.timeframe_secs,
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
    }

    let params = backtesting_engine::recorded::BacktestParams {
        symbol: if standalone {
            historical_cfg
                .as_ref()
                .map(|cfg| {
                    cfg.symbols
                        .iter()
                        .map(|s| s.symbol.clone())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default()
        } else {
            payload.symbol.clone()
        },
        timeframe_secs: if standalone {
            historical_cfg
                .as_ref()
                .and_then(|cfg| {
                    cfg.symbols
                        .iter()
                        .flat_map(|s| s.ladder.iter())
                        .copied()
                        .min()
                })
                .unwrap_or(payload.timeframe_secs)
        } else {
            payload.timeframe_secs
        },
        from_secs,
        to_secs,
        portfolio_capital_usd: payload.portfolio_capital_usd.unwrap_or(1000.0),
    };

    // ── v8.2 async run: register + spawn ──
    let run_id = state.backtest.alloc_run_id();
    let tracked = Arc::new(backtesting_engine::registry::TrackedRun::new());
    state
        .backtest
        .runs
        .write()
        .await
        .insert(run_id, tracked.clone());

    let task_pool = state.pool.clone();
    let task_workspace = workspace.clone();
    let task_tracked = tracked.clone();
    let task_params = params;
    let task_payload = payload.clone();
    let task_historical = historical_cfg;
    let task_bound = bound_instance.clone();
    let task_mode = mode.clone();
    let task_tae = workspace.minimal_tae.clone();
    let task_fees = fees;
    let task_analytics = analytics_params;
    let task_leverage = cross_leverage;
    let task_symbol_for_input_bars: Option<(String, Vec<u64>)> =
        task_historical.as_ref().map(|cfg| {
            (
                cfg.symbols
                    .first()
                    .map(|s| s.symbol.clone())
                    .unwrap_or_default(),
                cfg.symbols
                    .first()
                    .map(|s| s.ladder.clone())
                    .unwrap_or_default(),
            )
        });

    tokio::spawn(async move {
        // Progress callback: mirror the runner's RunProgress into the
        // tracked run for the progress endpoint.
        let tracked_cb = task_tracked.clone();
        let progress_cb: Arc<dyn Fn(backtesting_engine::historical::RunProgress) + Send + Sync> =
            Arc::new(move |p| {
                let tracked = tracked_cb.clone();
                tokio::spawn(async move {
                    *tracked.phase.lock().await = p.phase.as_str().to_string();
                    *tracked.pct.lock().await = p.pct;
                    *tracked.message.lock().await = p.message;
                });
            });
        let controls = backtesting_engine::historical::RunControls {
            progress: Some(progress_cb),
            cancel: task_tracked.cancel.clone(),
        };

        let result = if task_mode == "historical" {
            let cfg = task_historical.as_ref().expect("historical cfg");
            backtesting_engine::historical::run_historical_backtest(
                &task_pool,
                &task_params,
                &task_tae,
                &task_fees,
                task_leverage,
                task_analytics,
                cfg,
                &controls,
            )
            .await
        } else {
            *task_tracked.phase.lock().await = "replaying".to_string();
            *task_tracked.pct.lock().await = 50.0;
            backtesting_engine::recorded::run_backtest(
                &task_pool,
                &task_params,
                &task_tae,
                &task_fees,
                task_leverage,
                task_analytics,
            )
            .await
        };

        if result.cancelled {
            *task_tracked.status.lock().await = "cancelled".to_string();
            *task_tracked.phase.lock().await = "cancelled".to_string();
            *task_tracked.message.lock().await = "run cancelled".to_string();
            return;
        }
        match persist_backtest_run(
            &task_pool,
            &task_workspace,
            run_id,
            &task_mode,
            &task_params,
            &task_payload,
            &result,
            task_bound.as_ref(),
            task_symbol_for_input_bars.as_ref(),
        )
        .await
        {
            Ok(backtest_id) => {
                *task_tracked.backtest_id.lock().await = Some(backtest_id);
            }
            Err(e) => {
                *task_tracked.status.lock().await = "failed".to_string();
                *task_tracked.message.lock().await = e;
                return;
            }
        }
        *task_tracked.status.lock().await = "completed".to_string();
        *task_tracked.phase.lock().await = "analyzing".to_string();
        *task_tracked.pct.lock().await = 100.0;
    });

    Json(serde_json::json!({
        "run_id": run_id,
        "status": "running",
    }))
    .into_response()
}

/// Persist a finished run: `backtest_runs` JSON columns + the normalized
/// DS rows + input bars (historical, gated by `store_input_bars`).
///
/// v8.2: public so the CLI backtest runner reuses the identical write
/// path (the CLI has no `AppState`).
#[allow(clippy::too_many_arguments)]
pub async fn persist_backtest_run(
    pool: &sqlx::SqlitePool,
    workspace: &config_models::WorkspaceConfig,
    run_id: i64,
    mode: &str,
    params: &backtesting_engine::recorded::BacktestParams,
    payload: &BacktestRequest,
    result: &backtesting_engine::recorded::BacktestResult,
    bound_instance: Option<&Arc<portfolio_supervisor::instance::Instance>>,
    input_bars_target: Option<&(String, Vec<u64>)>,
) -> Result<i64, String> {
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
        pool,
        &params_json,
        &summary_json,
        &stats_json,
        &trades_json,
        &equity_curve_json,
    )
    .await;
    let _ = run_id;

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
        ("mode".to_string(), mode.to_string()),
        ("total_trades".to_string(), result.total_trades.to_string()),
        ("win_rate".to_string(), format!("{:.2}", result.win_rate)),
        (
            "profit_factor".to_string(),
            result
                .profit_factor
                .map(|p| format!("{p:.4}"))
                .unwrap_or_default(),
        ),
        (
            "max_drawdown_pct".to_string(),
            format!("{:.2}", result.max_drawdown_pct),
        ),
        (
            "classification".to_string(),
            format!("{:?}", result.stats.classification),
        ),
        (
            "p_value".to_string(),
            format!("{:.6}", result.stats.p_value),
        ),
        ("p_mc".to_string(), format!("{:.6}", result.stats.p_mc)),
        (
            "instance_id".to_string(),
            payload.instance_id.clone().unwrap_or_default(),
        ),
    ]
    .into_iter()
    .map(|(key, value)| database_storage::queries::backtest_ds::DsMetric { key, value })
    .collect();
    database_storage::queries::backtest_ds::insert_backtest_ds_rows(
        pool,
        backtest_id,
        &ds_trades,
        &result.equity_curve,
        &result.portfolio,
        &result.signals,
        &ds_metrics,
    )
    .await;
    database_storage::queries::backtest_ds::update_backtest_run_meta(
        pool,
        backtest_id,
        payload.instance_id.as_deref(),
        mode,
    )
    .await;

    // Historical mode: persist the exact input bars for reproducibility.
    if mode == "historical" && workspace.backtest.store_input_bars {
        let (symbol, ladder): (String, Vec<u64>) = if let Some(t) = input_bars_target {
            t.clone()
        } else if let Some(inst) = bound_instance {
            let symbol = inst.symbol();
            let entry = workspace.instances.iter().find(|e| e.symbol == symbol);
            let micro = entry
                .map(|e| e.micro_term.candles.duration_seconds)
                .unwrap_or(60);
            let fast = entry
                .map(|e| e.fast_term.candles.duration_seconds)
                .unwrap_or(180);
            let slow = entry
                .and_then(|e| e.slow_term.as_ref())
                .map(|t| t.candles.duration_seconds)
                .unwrap_or(workspace.slow_timeframe.duration_seconds);
            let macro_tf = entry
                .and_then(|e| e.macro_term.as_ref())
                .map(|t| t.candles.duration_seconds)
                .unwrap_or(workspace.macro_timeframe.duration_seconds);
            (symbol, vec![micro, fast, slow, macro_tf])
        } else {
            (String::new(), Vec::new())
        };
        if !symbol.is_empty() {
            let burn_in_secs = workspace.backtest.warmup_bars as i64
                * ladder.iter().copied().max().unwrap_or(900) as i64;
            if let Ok(mut tx) = pool.begin().await {
                for tf in ladder {
                    let bars = database_storage::queries::archive::query_archive_window(
                        pool,
                        &symbol,
                        tf,
                        params.from_secs - burn_in_secs,
                        params.to_secs,
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
    Ok(backtest_id)
}

/// GET /api/backtest/progress/:run_id — live run progress (v8.2).
pub async fn serve_backtest_progress(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(run_id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let runs = state.backtest.runs.read().await;
    match runs.get(&run_id) {
        Some(tracked) => {
            let phase = tracked.phase.lock().await.clone();
            let pct = *tracked.pct.lock().await;
            let message = tracked.message.lock().await.clone();
            let status = tracked.status.lock().await.clone();
            let backtest_id = *tracked.backtest_id.lock().await;
            Json(serde_json::json!({
                "run_id": run_id,
                "status": status,
                "phase": phase,
                "pct": pct,
                "message": message,
                "backtest_id": backtest_id,
            }))
            .into_response()
        }
        None => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "run not found", "code": "run_not_found" })),
        )
            .into_response(),
    }
}

/// POST /api/backtest/cancel/:run_id — cancel a running backtest (v8.2).
pub async fn serve_backtest_cancel(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(run_id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let runs = state.backtest.runs.read().await;
    match runs.get(&run_id) {
        Some(tracked) => {
            tracked
                .cancel
                .store(true, std::sync::atomic::Ordering::SeqCst);
            *tracked.status.lock().await = "cancelled".to_string();
            Json(serde_json::json!({
                "run_id": run_id,
                "status": "cancelled",
                "message": "cancel requested",
            }))
            .into_response()
        }
        None => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "run not found", "code": "run_not_found" })),
        )
            .into_response(),
    }
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

// GET /api/backtest/coverage — moved to `handlers::backtest` (v8 BTE):
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
    let rows =
        database_storage::queries::backtest_ds::query_backtest_trades(&state.pool, id, limit, 0)
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
    let rows =
        database_storage::queries::backtest_ds::query_backtest_signals(&state.pool, id).await;
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
    let rows =
        database_storage::queries::backtest_ds::query_backtest_metrics(&state.pool, id).await;
    Json(serde_json::json!({
        "run_id": id,
        "metrics": rows.into_iter().map(|m| serde_json::json!({
            "key": m.key,
            "value": m.value,
        })).collect::<Vec<_>>(),
    }))
}
