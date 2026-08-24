//! CLI cross-folder comparison (v10.1): `--compare-folders <root>...`.
//!
//! DB-free by design: reads each folder's `ds/` NDJSON tree (backtests +
//! sessions — identical schemas everywhere) and emits ONE comparison
//! table with the PAE Comparison columns, now spanning folders. Sessions
//! and backtests are comparable because both sides carry the same
//! summary/risk/verdict vocabulary.
//!
//! Usage (multi-session experiment workflow — see
//! `scripts/multi-session-compare.sh`):
//!   execution-daemon --compare-folders experiments/exp-a experiments/exp-b

use std::path::{Path, PathBuf};

/// One comparable experiment row (backtest or paper session).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExperimentRow {
    pub folder: String,
    pub kind: String, // "backtest" | "session"
    pub id: String,
    pub exchange: String,
    pub strategy: String,
    pub symbol: String,
    pub mode: String,
    pub total_trades: i64,
    pub win_rate: Option<f64>,
    pub profit_factor: Option<f64>,
    pub expectancy: Option<f64>,
    pub max_drawdown_pct: Option<f64>,
    pub sharpe: Option<f64>,
    pub sortino: Option<f64>,
    pub calmar: Option<f64>,
    pub ulcer: Option<f64>,
    pub var95: Option<f64>,
    pub es95: Option<f64>,
    pub verdict: String,
}

impl ExperimentRow {
    fn empty(folder: &str, kind: &str, id: String) -> Self {
        Self {
            folder: folder.to_string(),
            kind: kind.to_string(),
            id,
            exchange: "—".into(),
            strategy: "—".into(),
            symbol: "—".into(),
            mode: "—".into(),
            total_trades: 0,
            win_rate: None,
            profit_factor: None,
            expectancy: None,
            max_drawdown_pct: None,
            sharpe: None,
            sortino: None,
            calmar: None,
            ulcer: None,
            var95: None,
            es95: None,
            verdict: "—".into(),
        }
    }
}

/// The folder root or its `ds/` subtree (whichever exists).
fn ds_root(root: &Path) -> PathBuf {
    if root.join("ds").is_dir() {
        root.join("ds")
    } else {
        root.to_path_buf()
    }
}

/// Parse a single NDJSON file and return the LAST row (latest snapshot).
fn latest_row(path: &Path) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut last = None;
    for line in content.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            last = Some(v);
        }
    }
    last
}

fn f64_of(v: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|k| v.get(*k).and_then(|x| x.as_f64()))
}

fn str_of(v: &serde_json::Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_str()))
        .unwrap_or("—")
        .to_string()
}

/// `equity.ndjson` → `(ts_ms, equity)` curve for the pure risk-metrics
/// recomputation (Sharpe/Sortino/Calmar/Ulcer/VaR95/ES95 live only in the
/// DB — recomputing keeps this CLI DB-free). The DS backtest equity stores
/// SECONDS; `compute_risk_metrics_from_curve` expects ms — ×1000 here.
fn equity_curve(path: &Path) -> Vec<(i64, f64)> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    content
        .lines()
        .filter_map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).ok()?;
            let ts = v.get("ts_secs").and_then(|t| t.as_i64())?;
            let eq = v.get("equity").and_then(|e| e.as_f64())?;
            Some((ts.saturating_mul(1000), eq))
        })
        .collect()
}

/// Collect every comparable experiment inside one folder's `ds/` tree.
pub(crate) fn collect_experiments(root: &Path) -> Vec<ExperimentRow> {
    let ds = ds_root(root);
    let folder = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());
    let mut out = Vec::new();

    // ── Backtests: ds/backtests/BTxxxx_mode/ ──
    if let Ok(entries) = std::fs::read_dir(ds.join("backtests")) {
        for e in entries.flatten() {
            let bdir = e.path();
            let run_path = bdir.join("run.json");
            let Ok(run) = serde_json::from_str::<serde_json::Value>(
                &std::fs::read_to_string(&run_path).unwrap_or_default(),
            ) else {
                continue;
            };
            let id = run
                .get("backtest_id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .to_string();
            let mode = str_of(&run, &["mode"]);
            let params = run
                .get("params")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let summary = run
                .get("summary")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let stats = run.get("stats").cloned().unwrap_or(serde_json::Value::Null);
            let mut row = ExperimentRow::empty(&folder, "backtest", id);
            row.mode = mode.clone();
            row.exchange = str_of(&params, &["exchange", "exchange_name"]);
            row.strategy = str_of(&params, &["strategy_id", "strategy"]);
            row.symbol = str_of(&params, &["symbol"]);
            row.total_trades = summary
                .get("total_trades")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            row.win_rate = f64_of(&summary, &["win_rate", "winRate"]);
            row.profit_factor = f64_of(&summary, &["profit_factor"]);
            row.expectancy = f64_of(&summary, &["expectancy"]);
            row.max_drawdown_pct = f64_of(&summary, &["max_drawdown_pct"]);
            row.verdict = str_of(&stats, &["classification"]);
            // Risk metrics recomputed from the equity curve.
            // v10.1: honor the run's bound strategy risk-free rate (DB-free: read from run.json params/stats).
            let rf_pct = params
                .get("risk_free_rate_pct")
                .and_then(|v| v.as_f64())
                .or_else(|| stats.get("risk_free_rate_pct").and_then(|v| v.as_f64()))
                .or_else(|| {
                    run.get("strategy")
                        .and_then(|s| s.get("pae"))
                        .and_then(|p| p.get("risk_math"))
                        .and_then(|r| r.get("risk_free_rate_pct"))
                        .and_then(|v| v.as_f64())
                })
                .unwrap_or(0.0);
            let curve = equity_curve(&bdir.join("equity.ndjson"));
            if curve.len() >= 2 {
                let rm =
                    performance_analytics::risk_analytics::compute_risk_metrics_from_curve_with_rf(
                        &curve, rf_pct,
                    );
                row.sharpe = rm.sharpe_ratio;
                row.sortino = rm.sortino_ratio;
                row.calmar = rm.calmar_ratio;
                row.ulcer = Some(rm.ulcer_index);
                row.var95 = Some(rm.value_at_risk_95);
                row.es95 = Some(rm.expected_shortfall_95);
            }
            out.push(row);
        }
    }

    // ── Sessions: ds/sessions/Sxxxx_mode/ ──
    if let Ok(entries) = std::fs::read_dir(ds.join("sessions")) {
        for e in entries.flatten() {
            let sdir = e.path();
            let Ok(session) = serde_json::from_str::<serde_json::Value>(
                &std::fs::read_to_string(sdir.join("session.json")).unwrap_or_default(),
            ) else {
                continue;
            };
            let id = session
                .get("session_id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .to_string();
            let mut row = ExperimentRow::empty(&folder, "session", id);
            row.mode = str_of(&session, &["mode"]);
            row.exchange = str_of(&session, &["exchange"]);
            row.total_trades = session
                .get("total_trades")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            // Latest PAE performance-matrix snapshot (I-tier).
            if let Some(perf) = latest_row(&sdir.join("trading/analytics/performance.ndjson")) {
                row.total_trades = perf
                    .get("total_trades")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(row.total_trades);
                row.profit_factor = f64_of(&perf, &["overall_profit_factor", "profit_factor"]);
                row.expectancy = f64_of(&perf, &["overall_expectancy", "expectancy"]);
                row.sharpe = f64_of(&perf, &["overall_sharpe", "sharpe_ratio"]);
                row.sortino = f64_of(&perf, &["overall_sortino", "sortino_ratio"]);
                row.max_drawdown_pct = f64_of(&perf, &["max_drawdown_pct"]);
                row.verdict = str_of(&perf, &["overall_rating", "classification"]);
            }
            // Latest NHST strategy snapshot (verdict + significance).
            if let Some(strat) = latest_row(&sdir.join("trading/analytics/strategy.ndjson")) {
                let v = str_of(&strat, &["classification", "edge_verdict"]);
                if v != "—" {
                    row.verdict = v;
                }
            }
            out.push(row);
        }
    }

    out
}

/// `--compare-folders <rootA> <rootB> ...` — scan, print JSON + table.
pub fn compare_folders(roots: &[String]) -> i32 {
    if roots.is_empty() {
        eprintln!("--compare-folders requires at least one folder root");
        return 1;
    }
    let mut rows: Vec<ExperimentRow> = Vec::new();
    for root in roots {
        let p = Path::new(root);
        if !p.is_dir() {
            eprintln!("⚠️  compare-folders: '{root}' is not a directory — skipping");
            continue;
        }
        rows.extend(collect_experiments(p));
    }
    if rows.is_empty() {
        eprintln!("compare-folders: no experiments found under the given roots");
        return 1;
    }
    // Stable output: folder, then kind, then id.
    rows.sort_by(|a, b| (&a.folder, &a.kind, &a.id).cmp(&(&b.folder, &b.kind, &b.id)));

    println!("{}", serde_json::json!({ "experiments": rows }));
    println!();
    print_table(&rows);
    0
}

fn fmt_opt(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{x:.2}"),
        None => "—".into(),
    }
}

fn print_table(rows: &[ExperimentRow]) {
    println!("# Cross-folder comparison");
    println!();
    println!(
        "| folder | kind | id | exchange | strategy | symbol | mode | trades | WR% | PF | expectancy | Sharpe | Sortino | maxDD% | verdict |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for r in rows {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.folder,
            r.kind,
            r.id,
            r.exchange,
            r.strategy,
            r.symbol,
            r.mode,
            r.total_trades,
            r.win_rate.map(|x| format!("{x:.1}")).unwrap_or("—".into()),
            fmt_opt(r.profit_factor),
            fmt_opt(r.expectancy),
            fmt_opt(r.sharpe),
            fmt_opt(r.sortino),
            r.max_drawdown_pct
                .map(|x| format!("{x:.2}"))
                .unwrap_or("—".into()),
            r.verdict,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn collects_backtest_with_risk_metrics() {
        let tmp = std::env::temp_dir().join(format!("cmp_bt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let ds = tmp.join("ds");
        let run = serde_json::json!({
            "backtest_id": 7,
            "mode": "historical",
            "params": { "exchange": "Hyperliquid", "strategy_id": "conservative", "symbol": "BTC-USDT" },
            "summary": { "total_trades": 9, "win_rate": 55.5, "profit_factor": 1.4, "expectancy": 0.12, "max_drawdown_pct": 3.2 },
            "stats": { "classification": "Profitable", "is_significant": true, "p_value": 0.03 }
        });
        // Curve spanning 5 days with variance AND a mid drawdown so the
        // risk-metrics recomputation yields real Sharpe/Sortino/Calmar.
        let day = 86_400i64;
        let mut equity_lines = Vec::new();
        for d in 0..5i64 {
            for k in 0..4i64 {
                let dip = if d == 3 { -4.0 } else { 0.0 };
                let v = 1000.0 + d as f64 * 2.0 + k as f64 * 0.4 + dip + (k % 2) as f64 * 0.2;
                equity_lines.push(format!(
                    "{{\"ts_secs\": {}, \"equity\": {v}}}",
                    1_700_000_000 + d * day + k * 60
                ));
            }
        }
        let equity = equity_lines.join("\n");
        write(
            &ds,
            "backtests/BT0007_historical/run.json",
            &serde_json::to_string(&run).unwrap(),
        );
        write(&ds, "backtests/BT0007_historical/equity.ndjson", &equity);

        let rows = collect_experiments(&tmp);
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.kind, "backtest");
        assert_eq!(r.exchange, "Hyperliquid");
        assert_eq!(r.strategy, "conservative");
        assert_eq!(r.total_trades, 9);
        assert_eq!(r.profit_factor, Some(1.4));
        assert_eq!(r.verdict, "Profitable");
        assert!(
            r.sharpe.unwrap_or(0.0) > 0.0,
            "sharpe recomputed from the equity curve"
        );
        assert!(r.calmar.unwrap_or(0.0) > 0.0);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn collects_session_from_analytics_ndjson() {
        let tmp = std::env::temp_dir().join(format!("cmp_ses_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let ds = tmp.join("ds");
        write(
            &ds,
            "sessions/S0003_paper/session.json",
            &serde_json::json!({
                "session_id": 3,
                "mode": "paper",
                "exchange": "Bitget",
                "portfolio_capital_usd": 1000.0,
            })
            .to_string(),
        );
        write(
            &ds,
            "sessions/S0003_paper/trading/analytics/performance.ndjson",
            &serde_json::json!({
                "total_trades": 4,
                "overall_profit_factor": 2.1,
                "overall_expectancy": 0.5,
                "overall_sharpe": 1.7,
                "overall_sortino": 2.2,
                "max_drawdown_pct": 1.5,
                "overall_rating": "Good",
            })
            .to_string(),
        );
        write(
            &ds,
            "sessions/S0003_paper/trading/analytics/strategy.ndjson",
            &serde_json::json!({ "classification": "Profitable", "p_value": 0.02 }).to_string(),
        );

        let rows = collect_experiments(&tmp);
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.kind, "session");
        assert_eq!(r.exchange, "Bitget");
        assert_eq!(r.total_trades, 4);
        assert_eq!(r.profit_factor, Some(2.1));
        assert_eq!(r.sharpe, Some(1.7));
        assert_eq!(r.verdict, "Profitable");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn missing_ds_yields_no_rows() {
        let tmp = std::env::temp_dir().join(format!("cmp_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(collect_experiments(&tmp).is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
