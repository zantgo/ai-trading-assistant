//! # CLI data-science commands (v10)
//!
//! Headless JSON surfaces for the coding agent / Jupyter workflow:
//! `--sessions`, `--session-report <id>`, `--backtest-show <id>`.
//! Every payload is the SAME server-computed struct the GUI renders —
//! parity by construction (one producer, three sinks).

use config_models::WorkspaceConfig;
use sqlx::SqlitePool;

/// `--sessions` — list persisted sessions, newest first.
pub async fn print_sessions(pool: &SqlitePool) -> i32 {
    match database_storage::queries::sessions::list_sessions(pool).await {
        Ok(rows) => {
            let out: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "session_id": r.id,
                        "mode": r.mode,
                        "exchange": r.exchange,
                        "currency": r.currency,
                        "portfolio_capital_usd": r.portfolio_capital_usd,
                        "started_at_ms": r.started_at_ms,
                        "ended_at_ms": r.ended_at_ms,
                        "status": r.status,
                    })
                })
                .collect();
            println!("{}", serde_json::json!({ "sessions": out }));
            0
        }
        Err(e) => {
            eprintln!("sessions query failed: {e}");
            1
        }
    }
}

/// `--session-report <id>` — the PAE dashboard payloads, session-scoped.
pub async fn print_session_report(pool: &SqlitePool, session_id: i64) -> i32 {
    // I-tier artifacts the PAE tabs render (same server-computed structs).
    let stats = performance_analytics::stats_compiler::compile_dashboard_stats(pool, 1000.0).await;
    let strategy = database_storage::query_strategy_analytics_history(pool, None, 1)
        .await
        .into_iter()
        .last();
    let risk = database_storage::query_risk_analytics_latest(pool).await;
    let performance = database_storage::query_performance_matrix_latest(pool, None).await;

    // Session-scoped D-tier counts.
    let (snapshots, trades): (i64, i64) = {
        let snap: Option<(i64,)> =
            sqlx::query_as("SELECT COUNT(*) FROM market_snapshots WHERE session_id = ?1")
                .bind(session_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);
        let trades_count: Option<(i64,)> =
            sqlx::query_as("SELECT COUNT(*) FROM paper_trades WHERE session_id = ?1")
                .bind(session_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);
        (
            snap.map(|r| r.0).unwrap_or(0),
            trades_count.map(|r| r.0).unwrap_or(0),
        )
    };

    let report = serde_json::json!({
        "session_id": session_id,
        "counts": {
            "market_snapshots": snapshots,
            "trades": trades,
        },
        "stats": stats,
        "strategy_analytics": strategy,
        "risk_analytics": risk,
        "performance": performance,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    0
}

/// `--backtest-show <id>` — the full run: params, summary, NHST stats,
/// metrics, trades (enriched), equity + the ds/ file paths.
pub async fn print_backtest_show(pool: &SqlitePool, workspace: &WorkspaceConfig, id: i64) -> i32 {
    let run = database_storage::query_backtest_run(pool, id).await;
    let Some((params, summary, stats, _trades_json, equity)) = run else {
        eprintln!("backtest run {id} not found");
        return 1;
    };
    let metrics = database_storage::queries::backtest_ds::query_backtest_metrics(pool, id).await;
    let trades =
        database_storage::queries::backtest_ds::query_backtest_trades(pool, id, 5000, 0).await;
    let ds_root = std::path::PathBuf::from(&workspace.data_science.output_path);
    let ds_dir = database_storage::ds_export::backtest_dir(&ds_root, id, "historical");
    let out = serde_json::json!({
        "backtest_id": id,
        "params": serde_json::from_str::<serde_json::Value>(&params).unwrap_or(serde_json::Value::Null),
        "summary": serde_json::from_str::<serde_json::Value>(&summary).unwrap_or(serde_json::Value::Null),
        "stats": serde_json::from_str::<serde_json::Value>(&stats).unwrap_or(serde_json::Value::Null),
        "metrics": metrics.iter().map(|m| serde_json::json!({ "key": m.key, "value": m.value })).collect::<Vec<_>>(),
        "trades": trades,
        "equity": equity,
        "ds_files": {
            "run_json": format!("{}/run.json", ds_dir.display()),
            "trades_ndjson": format!("{}/trades.ndjson", ds_dir.display()),
            "equity_ndjson": format!("{}/equity.ndjson", ds_dir.display()),
            "input_bars_dir": format!("{}/input_bars/", ds_dir.display()),
        },
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
    );
    0
}
