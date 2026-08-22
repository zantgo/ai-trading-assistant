//! # CLI operations (v9) — GUI parity, headless style.
//!
//! The CLI's "visualization" is structured logging + saved JSON artifacts.
//! Strategy CRUD, account config, and instance strategy binding run fully
//! headless against the workspace config; lifecycle transitions (which act
//! on the in-memory LifecycleManager) print an explicit JSON error pointing
//! at the dashboard / `POST /api/instances/:id/lifecycle`.

use config_models::{StrategyConfig, WorkspaceConfig};

/// Print a JSON envelope and exit with 0/1.
fn emit(json: serde_json::Value) -> i32 {
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    0
}

fn fail(msg: &str) -> i32 {
    emit(serde_json::json!({ "success": false, "error": msg }))
}

fn persist(workspace: &mut WorkspaceConfig) -> Result<(), String> {
    workspace.config_version = workspace.config_version.saturating_add(1);
    config_models::save_workspace(workspace).map_err(|e| e.to_string())
}

pub fn run_strategy_op(workspace: &mut WorkspaceConfig, op: &StrategyOp) -> i32 {
    match op {
        StrategyOp::List => {
            workspace.ensure_default_strategy();
            let list: Vec<serde_json::Value> = workspace
                .strategies
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "base": s.base,
                        "description": s.description,
                        "schema_version": s.schema_version,
                    })
                })
                .collect();
            emit(serde_json::json!({ "strategies": list }))
        }
        StrategyOp::Export { name, path } => {
            let resolved = match workspace.resolve_strategy(name) {
                Ok(r) => r,
                Err(e) => return fail(&e),
            };
            let json = serde_json::to_string_pretty(&resolved).unwrap();
            match path {
                Some(p) => {
                    if let Err(e) = std::fs::write(p, &json) {
                        return fail(&format!("write {p} failed: {e}"));
                    }
                    emit(serde_json::json!({
                        "success": true,
                        "name": name,
                        "path": p,
                    }))
                }
                None => {
                    println!("{json}");
                    0
                }
            }
        }
        StrategyOp::Upsert { name, path } => {
            let raw = match std::fs::read_to_string(path) {
                Ok(r) => r,
                Err(e) => return fail(&format!("read {path} failed: {e}")),
            };
            let json: serde_json::Value = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(e) => return fail(&format!("invalid JSON: {e}")),
            };
            let resolved = match StrategyConfig::resolve(None, &json) {
                Ok(r) => r,
                Err(e) => return fail(&e),
            };
            let problems = resolved.validate();
            let mut entry = resolved;
            entry.name = name.clone();
            match workspace
                .strategies
                .iter_mut()
                .find(|s| s.name == *name)
            {
                Some(existing) => *existing = entry,
                None => workspace.strategies.push(entry),
            }
            if let Err(e) = persist(workspace) {
                return fail(&format!("persist failed: {e}"));
            }
            emit(serde_json::json!({
                "success": true,
                "name": name,
                "warnings": problems,
            }))
        }
        StrategyOp::Delete { name } => {
            if name == "default" {
                return fail("the default strategy is locked");
            }
            let before = workspace.strategies.len();
            workspace.strategies.retain(|s| s.name != *name);
            if workspace.strategies.len() == before {
                return fail(&format!("strategy '{name}' not found"));
            }
            if let Err(e) = persist(workspace) {
                return fail(&format!("persist failed: {e}"));
            }
            emit(serde_json::json!({ "success": true }))
        }
        StrategyOp::Clone { source, target } => {
            let src = match workspace.resolve_strategy(source) {
                Ok(s) => s,
                Err(e) => return fail(&e),
            };
            if workspace.strategies.iter().any(|s| s.name == *target) {
                return fail(&format!("strategy '{target}' already exists"));
            }
            let mut entry = src;
            entry.name = target.clone();
            entry.base = None;
            workspace.strategies.push(entry);
            if let Err(e) = persist(workspace) {
                return fail(&format!("persist failed: {e}"));
            }
            emit(serde_json::json!({ "success": true, "name": target }))
        }
    }
}

pub enum StrategyOp {
    List,
    Export { name: String, path: Option<String> },
    Upsert { name: String, path: String },
    Delete { name: String },
    Clone { source: String, target: String },
}

/// Account operations (config-level; the running daemon serves the same
/// data through `/api/account/*`).
pub fn run_account_op(workspace: &mut WorkspaceConfig, op: &AccountOp) -> i32 {
    match op {
        AccountOp::Summary => emit(serde_json::json!({
            "mode": "cli",
            "portfolio_capital_source": "paper_config",
            "portfolio_capital_usd": workspace.portfolio_capital_usd,
            "instance_count": workspace.instances.len(),
        })),
        AccountOp::SetCapital { usd } => {
            if !usd.is_finite() || !(100.0..=10_000_000.0).contains(usd) {
                return fail("portfolio_capital_usd must be 100–10,000,000");
            }
            workspace.portfolio_capital_usd = *usd;
            if let Err(e) = persist(workspace) {
                return fail(&format!("persist failed: {e}"));
            }
            emit(serde_json::json!({ "success": true, "portfolio_capital_usd": usd }))
        }
        AccountOp::Reset => emit(serde_json::json!({
            "success": true,
            "portfolio_capital_usd": workspace.portfolio_capital_usd,
            "note": "paper ledger reseed applies on the running daemon (POST /api/account/reset)",
        })),
    }
}

pub enum AccountOp {
    Summary,
    SetCapital { usd: f64 },
    Reset,
}

/// Bind an instance to a strategy (config-level; the running daemon
/// recharges the instance at the next candle boundary).
pub fn run_instance_bind(workspace: &mut WorkspaceConfig, instance_id: &str, strategy: &str) -> i32 {
    // Validate the strategy name resolves first.
    if let Err(e) = workspace.resolve_strategy(strategy) {
        return fail(&e);
    }
    let Some(inst) = workspace.instances.iter_mut().find(|i| i.id == instance_id) else {
        return fail(&format!("instance '{instance_id}' not found"));
    };
    inst.strategy = Some(strategy.to_string());
    if let Err(e) = persist(workspace) {
        return fail(&format!("persist failed: {e}"));
    }
    emit(serde_json::json!({
        "success": true,
        "instance_id": instance_id,
        "strategy": strategy,
        "note": "full recharge applies at the next candle boundary on the running daemon; open positions keep their entry params",
    }))
}

/// Lifecycle transitions act on the in-memory manager — headless CLI
/// cannot reach it. Honest JSON error pointing at the live surface.
pub fn run_lifecycle_op(instance_id: &str, action: &str) -> i32 {
    fail(&format!(
        "lifecycle '{action}' for '{instance_id}' requires a running daemon — use POST /api/instances/:id/lifecycle (dashboard: TAE header buttons)"
    ))
}
