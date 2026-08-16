//! # Execution Daemon
//!
//! Headless orchestrator binary. Reads configuration, initializes the
//! SQLite database, builds the Axum `AppState`, spawns background tasks,
//! then runs the Axum HTTP server on `127.0.0.1:3000`.
//!
//! ## Launch modes
//!
//! - `--mode web` (default): starts the Axum server + serves the Svelte
//!   dashboard. The Welcome Gate prompts the user to select an exchange and
//!   currency before adding instances.
//! - `--mode headless`: auto-initialises the session from CLI args (or
//!   workspace config defaults), auto-spawns all instances declared in
//!   `workspace.instances[]`, then starts the Axum server for monitoring
//!   (accessible via SSH tunnel). No Welcome Gate prompt appears.
//!
//! ## CLI flags
//!
//! - `--exchange <hyperliquid|bitget>` — overrides the workspace
//!   `default_exchange`. Only meaningful in `headless` mode.
//! - `--currency <USDC|USDT>` — overrides the workspace `default_currency`.
//!   Only meaningful in `headless` mode.
//! - `--config <path>` — path to `config.toml`. Overrides the
//!   `MARKET_MONITOR_CONFIG` env var.
//!
//! ## Config sharing workflow
//!
//! 1. GUI machine: `./manage.sh run`, configure settings + instances via
//!    the dashboard, click "Download Config" → `config.toml` saved.
//! 2. Headless machine: `scp config.toml ec2-user@host:/app/`
//! 3. Start: `cargo run --bin execution-daemon -- --mode headless --exchange hyperliquid --currency USDC`
//! 4. Or point at config directly:
//!    `MARKET_MONITOR_CONFIG=/mnt/efs/config.toml cargo run --bin execution-daemon -- --mode headless`
//!
//! See `docs/conceptual-foundations/01-07-data-model-hierarchy.md` for the
//! canonical design document.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;

use api_gateway::{build_router, AppState};
use config_models::{load_platform, load_workspace, ClockMonitorBreachAction};
use database_storage::{init_db, run_telemetry_logger, verify_encryption_or_panic};
use network_adapters::{
    clock_monitor::{BreachAction, ClockMonitor, ClockMonitorConfig},
    connection_quality_tracker::ConnectionQualityRegistry,
    exchange_status_tracker::ExchangeStatusTracker,
    pipeline_reliability::ReliabilityTracker,
};
use performance_analytics::{performance_evaluator, strategy_optimizer};
use portfolio_supervisor::{
    portfolio_equity, registry, workspace_state::WorkspaceState,
    session::{Currency, ExchangeChoice},
};
use core_domain::portfolio::SafetyState;

// `snapshot_export` is owned by `lib.rs` so `api-gateway` can re-use
// its types without the daemon's CLI surface.

// ─── CLI argument parsing ────────────────────────────────────────────

struct CliArgs {
    mode: LaunchMode,
    exchange: Option<String>,
    currency: Option<String>,
    config_path: Option<String>,
    /// When `Some`, run the interactive setup flow that writes
    /// `config.toml` + (optionally) starts the daemon in headless
    /// mode. Value is the sub-mode: `"interactive"` (default), or
    /// `"status"` to print current snapshot-export state without
    /// writing anything.
    setup_mode: Option<String>,
    /// `--dry-run` for the setup subcommand — print what *would* be
    /// written but don't touch `config.toml`.
    dry_run: bool,
    /// When `true`, the setup flow auto-spawns the daemon in
    /// headless mode at the end (equivalent to answering "yes" to
    /// the "Start now?" prompt).
    auto_start: bool,
}

enum LaunchMode {
    Web,
    Headless,
    Setup,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = LaunchMode::Web; // default
    let mut exchange = None;
    let mut currency = None;
    let mut config_path = None;
    let mut setup_mode = None;
    let mut dry_run = false;
    let mut auto_start = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                i += 1;
                if i < args.len() {
                    mode = match args[i].as_str() {
                        "headless" => LaunchMode::Headless,
                        "setup" => LaunchMode::Setup,
                        _ => LaunchMode::Web,
                    };
                }
            }
            "--exchange" => {
                i += 1;
                if i < args.len() {
                    exchange = Some(args[i].clone());
                }
            }
            "--currency" => {
                i += 1;
                if i < args.len() {
                    currency = Some(args[i].clone());
                }
            }
            "--config" => {
                i += 1;
                if i < args.len() {
                    config_path = Some(args[i].clone());
                }
            }
            "--sub" => {
                i += 1;
                if i < args.len() {
                    setup_mode = Some(args[i].clone());
                }
            }
            "--dry-run" => dry_run = true,
            "--auto-start" => auto_start = true,
            "--web" | "--gui" => {
                mode = LaunchMode::Web;
            }
            "setup" => {
                // Positional shorthand: `execution-daemon setup` ≡
                // `execution-daemon --mode setup`.
                mode = LaunchMode::Setup;
            }
            _ => { /* ignore unknown args */ }
        }
        i += 1;
    }

    CliArgs {
        mode,
        exchange,
        currency,
        config_path,
        setup_mode,
        dry_run,
        auto_start,
    }
}

impl CliArgs {
    /// Resolve the exchange from CLI arg or workspace config default.
    fn resolve_exchange(&self, default_exchange: &str) -> ExchangeChoice {
        let raw = self.exchange.as_deref().unwrap_or(default_exchange);
        match raw.to_lowercase().as_str() {
            "bitget" => ExchangeChoice::Bitget,
            _ => ExchangeChoice::Hyperliquid,
        }
    }

    /// Resolve the currency from CLI arg or workspace config default.
    fn resolve_currency(&self, default_currency: &str) -> Currency {
        let raw = self.currency.as_deref().unwrap_or(default_currency);
        match raw.to_uppercase().as_str() {
            "USDT" => Currency::USDT,
            _ => Currency::USDC,
        }
    }
}

// ─── Setup CLI (interactive + status) ──────────────────────────────────
//
// Hand-rolled minimal stdin/stdout prompts. We deliberately avoid
// pulling in `inquire` or `dialoguer` for this single-purpose flow —
// the surface is small (text input + single-select + confirm) and
// staying dep-free keeps the binary slim. If more interactive flows
// are added later, consider migrating to `inquire`.
//
// The setup flow:
//   1. Loads `config.toml` (if any) to seed the prompts with the
//      existing defaults.
//   2. Asks for: exchange, settlement currency, trading pair,
//      which timeframes to enable (multi-select), per-timeframe
//      `timeframe_secs`, snapshot-export enabled + interval.
//   3. Validates: symbol existence on the chosen exchange (live
//      REST call), each timeframe against the per-TF floor/ceiling.
//   4. Writes `config.toml` (preserving platform-level sections
//      via `save_workspace` + a small platform-section rewrite).
//   5. Prints a summary + asks "Start the daemon now? [y/N]".
//      On `y` (or `--auto-start`), re-execs itself as a child
//      process in `--mode headless`.

/// One prompt of an interactive CLI flow — printed to stdout, the
/// answer is read from stdin.
fn prompt(label: &str, default: &str) -> String {
    if default.is_empty() {
        print!("{}", label);
        std::io::Write::flush(&mut std::io::stdout()).ok();
    } else {
        print!("{} [{}]: ", label, default);
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .unwrap_or_else(|e| {
            eprintln!("stdin read error: {}", e);
            0
        });
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Confirm prompt — returns `true` for `y`/`yes` (case-insensitive),
/// `false` for `n`/`no`/empty.
fn confirm(label: &str, default_yes: bool) -> bool {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{} {}: ", label, hint);
    std::io::Write::flush(&mut std::io::stdout()).ok();
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();
    let trimmed = buf.trim().to_lowercase();
    match trimmed.as_str() {
        "" => default_yes,
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default_yes,
    }
}

/// Multi-select prompt — reads a comma-separated list of integers
/// (1-based, per the menu shown to the user). Returns the 0-based
/// indices. Empty input returns the default set.
fn prompt_multi_select(label: &str, options: &[(&str, &str)], defaults: &[usize]) -> Vec<usize> {
    println!("\n{}", label);
    for (i, (key, desc)) in options.iter().enumerate() {
        println!("  {}. {} — {}", i + 1, key, desc);
    }
    print!("\nChoose (comma-separated, blank = default): ");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return defaults.to_vec();
    }
    let mut out = Vec::new();
    for token in trimmed.split(|c: char| c == ',' || c == ' ' || c == '\t') {
        if let Ok(n) = token.parse::<usize>() {
            if n >= 1 && n <= options.len() {
                let idx = n - 1;
                if !out.contains(&idx) {
                    out.push(idx);
                }
            }
        }
    }
    if out.is_empty() {
        defaults.to_vec()
    } else {
        out
    }
}

const TIMEFRAME_FLOOR_SECS: u64 = 10;
const TIMEFRAME_CEIL_SECS: u64 = 86_400;

fn prompt_timeframe_secs(label: &str, default_secs: u64) -> u64 {
    loop {
        let raw = prompt(label, &default_secs.to_string());
        match raw.parse::<u64>() {
            Ok(n) if n >= TIMEFRAME_FLOOR_SECS && n <= TIMEFRAME_CEIL_SECS => return n,
            Ok(n) => {
                eprintln!(
                    "  ⚠️  {}s is outside the allowed range [{}, {}].",
                    n, TIMEFRAME_FLOOR_SECS, TIMEFRAME_CEIL_SECS
                );
            }
            Err(_) => {
                eprintln!("  ⚠️  '{}' is not a number.", raw);
            }
        }
    }
}

fn prompt_snapshot_interval() -> u64 {
    loop {
        let raw = prompt("Snapshot interval in seconds", "60");
        match raw.parse::<u64>() {
            Ok(n) if (5..=3600).contains(&n) => return n,
            Ok(n) => eprintln!(
                "  ⚠️  {}s is outside the allowed range [5, 3600].",
                n
            ),
            Err(_) => eprintln!("  ⚠️  '{}' is not a number.", raw),
        }
    }
}

fn render_setup_summary(
    exchange: &str,
    currency: &str,
    pair: &str,
    selected_tfs: &[(&'static str, u64, &'static str)],
    snapshot_enabled: bool,
    snapshot_interval: u64,
    snapshot_path: &str,
) {
    println!("\n──────────────────────────────────────────────");
    println!("Trading Platform — Setup Summary");
    println!("──────────────────────────────────────────────");
    println!("  Exchange              : {}", exchange);
    println!("  Settlement currency   : {}", currency);
    println!("  Trading pair          : {}", pair);
    println!("  Timeframes            :");
    for (label, secs, slot) in selected_tfs {
        println!("    - {:<6} (slot {}) — {}s", label, slot, secs);
    }
    println!(
        "  Snapshot export       : {} (every {}s, → {})",
        if snapshot_enabled { "ENABLED" } else { "DISABLED" },
        snapshot_interval,
        snapshot_path
    );
    println!("──────────────────────────────────────────────\n");
}

/// Validate that a symbol is tradeable on the chosen exchange.
/// Mirrors the production-time validation in `registry::add_instance`
/// so a setup session can't write a `config.toml` that immediately
/// fails at boot.
async fn validate_symbol(exchange: &str, base: &str) -> Result<(), String> {
    use network_adapters::adapters;
    let cfg = config_models::load_platform().map_err(|e| format!("config load: {}", e))?;
    let _pair = format!("{}-{}", base, if exchange.eq_ignore_ascii_case("bitget") { "USDT" } else { "USDC" });
    let ex = if exchange.eq_ignore_ascii_case("bitget") {
        portfolio_supervisor::session::ExchangeChoice::Bitget
    } else {
        portfolio_supervisor::session::ExchangeChoice::Hyperliquid
    };
    let quote = if exchange.eq_ignore_ascii_case("bitget") {
        portfolio_supervisor::session::Currency::USDT
    } else {
        portfolio_supervisor::session::Currency::USDC
    };
    let raw = ex.raw_symbol(base, &quote);
    let availability = if ex == portfolio_supervisor::session::ExchangeChoice::Bitget {
        let url = cfg.bitget.ticker_url();
        let pt = ex.bitget_product_type(&quote).unwrap_or("USDT-FUTURES");
        adapters::bitget_rest::symbol_exists(&raw, pt, &url).await
    } else {
        let url = cfg.hyperliquid.rest_url();
        adapters::hyperliquid_rest::symbol_exists(base, &url).await
    };
    match availability {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "'{}' isn't available on {} perpetual futures.",
            base, exchange
        )),
        Err(e) => Err(format!("availability check failed: {}", e)),
    }
}

/// Apply the prompt result to `config.toml`. We rewrite only the
/// `[workspace]` table (preserving `[snapshot_export]` if present
/// already), then re-emit the file. This is the same path
/// `save_workspace` uses, extended for the snapshot section.
fn apply_setup_to_config(
    workspace: &mut config_models::WorkspaceConfig,
    exchange: &str,
    currency: &str,
    pair: &str,
    selected_tfs: &[(u64, &'static str)], // (timeframe_secs, slot_label)
    _snapshot_enabled: bool,
    _snapshot_interval: u64,
    _snapshot_path: &str,
) {
    workspace.default_exchange = exchange.to_string();
    workspace.default_currency = currency.to_string();

    // Default IndicatorsConfig — used as the base for each TF.
    let indicators = config_models::IndicatorsConfig::default();

    let tf_for = |secs: u64| config_models::TimeframeConfig::new(secs, indicators.clone());

    let find = |slot: &str| -> Option<u64> {
        selected_tfs
            .iter()
            .find(|(.., s)| *s == slot)
            .map(|(s, _)| *s)
    };

    // Replace the instances table with the new entry. This is a
    // single-pair setup flow; operators wanting many pairs should
    // hand-edit `config.toml`.
    workspace.instances = vec![config_models::InstanceEntry {
        id: pair.to_lowercase(),
        symbol: pair.to_string(),
        quote: currency.to_string(),
        initial_capital_usd: 1000.0,
        status: config_models::InstanceStatus::Running,
        micro_term: tf_for(find("micro").unwrap_or(60)),
        fast_term: tf_for(find("fast").unwrap_or(300)),
        slow_term: find("slow").map(tf_for),
        macro_term: find("macro").map(tf_for),
        automation: config_models::AutomationConfig::default(),
        operational_mode: config_models::OperationalMode::default(),
        weight_overrides: None,
        position_scaling: None,
        activation: None,
        custom_pipelines: std::collections::HashMap::new(),
    }];
}

fn apply_snapshot_to_platform(
    platform: &mut config_models::PlatformConfig,
    enabled: bool,
    interval_secs: u64,
    output_path: &str,
) {
    platform.snapshot_export = config_models::SnapshotExportConfig {
        enabled,
        output_path: output_path.to_string(),
        interval_secs,
        max_snapshots_retained: 1000,
        tabs: None,
    };
}

/// Re-serialise the full `config.toml` (preserving all
/// platform-level sections not handled by `save_workspace`).
fn write_full_config(
    platform: &config_models::PlatformConfig,
    workspace: &config_models::WorkspaceConfig,
) -> Result<(), String> {
    // Mirror the `OnDiskConfig` shape in `config-models`.
    #[derive(serde::Serialize)]
    struct OnDiskConfig<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        hyperliquid: Option<&'a config_models::HyperliquidConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bitget: Option<&'a config_models::BitgetConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        clock_monitor: Option<&'a config_models::ClockMonitorTomlConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        quality: Option<&'a config_models::QualityConfig>,
        reconnect: &'a config_models::ReconnectConfig,
        candle_buffer: &'a config_models::CandleBufferConfig,
        snapshot_export: &'a config_models::SnapshotExportConfig,
        workspace: &'a config_models::WorkspaceConfig,
    }

    let on_disk = OnDiskConfig {
        hyperliquid: Some(&platform.hyperliquid),
        bitget: Some(&platform.bitget),
        clock_monitor: platform.clock_monitor.as_ref(),
        quality: platform.quality.as_ref(),
        reconnect: &platform.reconnect,
        candle_buffer: &platform.candle_buffer,
        snapshot_export: &platform.snapshot_export,
        workspace,
    };
    let body = toml::to_string_pretty(&on_disk).map_err(|e| format!("toml: {}", e))?;
    std::fs::write("config.toml", body).map_err(|e| format!("write: {}", e))?;
    Ok(())
}

async fn run_setup_interactive(cli: &CliArgs) {
    println!("╔════════════════════════════════════════════╗");
    println!("║  Trading Platform — Interactive Setup      ║");
    println!("╚════════════════════════════════════════════╝");
    println!();
    println!("This flow writes `config.toml` and (optionally) starts the");
    println!("daemon in headless mode. Press <Enter> to accept each default.");
    println!();

    // 1. Exchange
    let exchange = {
        loop {
            let raw = prompt("Exchange (hyperliquid / bitget)", "hyperliquid");
            match raw.to_lowercase().as_str() {
                "hyperliquid" | "hl" => break "hyperliquid".to_string(),
                "bitget" | "bg" => break "bitget".to_string(),
                _ => eprintln!("  ⚠️  Expected 'hyperliquid' or 'bitget'."),
            }
        }
    };
    let currency = if exchange.eq_ignore_ascii_case("bitget") {
        "USDT".to_string()
    } else {
        "USDC".to_string()
    };
    println!("  → Settlement currency forced to {} for {}", currency, exchange);

    // 2. Pair
    let pair_base = {
        loop {
            let raw = prompt("Trading pair base symbol (e.g. BTC, ETH, SOL)", "BTC");
            let cleaned = raw.trim().to_uppercase();
            if cleaned.is_empty() || cleaned.len() > 10 {
                eprintln!("  ⚠️  Symbol must be 1–10 chars.");
                continue;
            }
            if !cli.dry_run {
                match validate_symbol(&exchange, &cleaned).await {
                    Ok(()) => break cleaned,
                    Err(e) => {
                        eprintln!("  ⚠️  {}", e);
                        if !confirm("Try a different symbol?", false) {
                            std::process::exit(1);
                        }
                    }
                }
            } else {
                break cleaned;
            }
        }
    };
    let pair = format!("{}-{}", pair_base, currency);
    println!("  → Pair: {}", pair);

    // 3. Timeframes
    let tf_choices: &[(&str, &str)] = &[
        ("micro", "1-min grain — fastest signal, most noise"),
        ("fast", "5-min grain — intraday"),
        ("slow", "15-min grain — swing (always recommended)"),
        ("macro", "60-min grain — positional / regime"),
    ];
    let defaults = vec![0usize, 1, 2, 3]; // all four by default
    let selected_indices = prompt_multi_select(
        "Timeframes to enable (slot → which candle grain feeds analysis):",
        tf_choices,
        &defaults,
    );

    let mut selected_tfs: Vec<(u64, &'static str)> = Vec::new();
    for &idx in &selected_indices {
        let (slot, _desc) = tf_choices[idx];
        let default_secs = match slot {
            "micro" => 60u64,
            "fast" => 300,
            "slow" => 900,
            "macro" => 3600,
            _ => 900,
        };
        let secs = prompt_timeframe_secs(
            &format!(
                "  timeframe_secs for {} (default {}s)",
                slot, default_secs
            ),
            default_secs,
        );
        selected_tfs.push((secs, slot));
    }
    let display_selected: Vec<(&str, u64, &str)> = selected_tfs
        .iter()
        .map(|(secs, slot)| {
            let label = match *slot {
                "micro" => "micro",
                "fast" => "fast",
                "slow" => "slow",
                "macro" => "macro",
                _ => "?",
            };
            (label, *secs, *slot)
        })
        .collect();

    // 4. Snapshot export
    println!("\nSnapshot export — periodic JSON dump for offline data science.");
    let snapshot_enabled = confirm("Enable snapshot export?", false);
    let snapshot_interval = if snapshot_enabled {
        prompt_snapshot_interval()
    } else {
        60
    };
    let snapshot_path = if snapshot_enabled {
        prompt("Output directory", "./snapshots")
    } else {
        "./snapshots".to_string()
    };

    render_setup_summary(
        &exchange,
        &currency,
        &pair,
        &display_selected,
        snapshot_enabled,
        snapshot_interval,
        &snapshot_path,
    );

    if cli.dry_run {
        println!("--dry-run: would write config.toml and exit.");
        return;
    }

    if !confirm("Apply these settings to config.toml?", true) {
        println!("Aborted — config.toml unchanged.");
        return;
    }

    // Apply to PlatformConfig + WorkspaceConfig.
    let mut platform = config_models::load_platform().unwrap_or_default();
    let mut workspace = config_models::load_workspace().unwrap_or_else(|_| config_models::WorkspaceConfig {
        id: "main".into(),
        name: "Main".into(),
        default_currency: currency.clone(),
        default_exchange: exchange.clone(),
        ..Default::default()
    });
    apply_setup_to_config(
        &mut workspace,
        &exchange,
        &currency,
        &pair,
        &selected_tfs,
        snapshot_enabled,
        snapshot_interval,
        &snapshot_path,
    );
    apply_snapshot_to_platform(&mut platform, snapshot_enabled, snapshot_interval, &snapshot_path);
    if let Err(e) = write_full_config(&platform, &workspace) {
        eprintln!("❌ Failed to write config.toml: {}", e);
        std::process::exit(1);
    }
    println!("✅ config.toml updated.");

    if cli.auto_start || confirm("Start the daemon now (headless mode)?", false) {
        println!("\n🚀 Starting daemon in headless mode...");
        let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("execution-daemon"));
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("--mode").arg("headless");
        if let Some(ref p) = cli.config_path {
            cmd.arg("--config").arg(p);
        }
        match cmd.spawn() {
            Ok(child) => {
                println!("Daemon spawned (pid {}). Logs go to engine.log.", child.id());
            }
            Err(e) => {
                eprintln!("❌ Failed to spawn daemon: {}", e);
                eprintln!("Run `./manage.sh run-silent` when ready.");
            }
        }
    } else {
        println!("\nRun `./manage.sh run-silent` (or `--mode headless`) when ready.");
    }
}

fn run_setup_status(cli: &CliArgs) {
    let platform = match config_models::load_platform() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to load platform config: {}", e);
            std::process::exit(1);
        }
    };
    let workspace = match config_models::load_workspace() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("❌ Failed to load workspace config: {}", e);
            std::process::exit(1);
        }
    };
    let rt = execution_daemon::snapshot_export::runtime_from_config(&platform.snapshot_export);

    println!("──────────────────────────────────────────────");
    println!("Trading Platform — Snapshot Export Status");
    println!("──────────────────────────────────────────────");
    println!("  Config path            : {}", cli.config_path.as_deref().unwrap_or("(default: ./config.toml)"));
    println!("  Enabled                : {}", rt.enabled);
    println!("  Output path            : {}", rt.output_path);
    println!("  Interval (s)           : {}", rt.interval_secs);
    println!("  Retention              : {}", rt.max_snapshots_retained);
    println!("  Tabs ({}):              : {}", rt.tabs.len(), rt.tabs.join(", "));
    println!("  Last snapshot          : {}", rt.last_snapshot_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| "(none yet)".into()));
    println!("  Total written          : {}", rt.total_snapshots_written);
    println!("  Last error             : {}", rt.last_error.clone().unwrap_or_else(|| "(none)".into()));
    println!("  Active instances       : {}", workspace.instances.len());
    println!("  Default exchange       : {}", workspace.default_exchange);
    println!("  Default currency       : {}", workspace.default_currency);
    println!("──────────────────────────────────────────────");
    println!("NOTE: live runtime counters (last_snapshot_at, total_written,");
    println!("       last_error, last_instance_count) update only when a");
    println!("       daemon is running. Use `./manage.sh status` to check.");
}

// ─── Main ────────────────────────────────────────────────────────────

/// v6.10.19a: the canonical per-pair risk feeding the L7 risk mean is the
/// MICRO window's L5 score — the same number the Risk panel, the dashboard
/// KPI, and the asset rows show. The previous TF-window MEAN drifted
/// upward whenever the macro window scored high (MAX_COMPRESSION / thin
/// participation) and rendered "HIGH_RISK" environments next to a visible
/// avg-risk of 41–46. Falls back to the window mean during warmup (micro
/// risk matrix not present yet), then 50 (moderate).
fn canonical_overall_risk(micro_risk: Option<f64>, risk_count: u32, risk_sum: f64) -> f64 {
    micro_risk.unwrap_or_else(|| {
        if risk_count > 0 {
            risk_sum / risk_count as f64
        } else {
            50.0
        }
    })
}

#[tokio::main]
async fn main() {
    let cli = parse_args();

    // If --config is provided, set the env var that config-models reads.
    if let Some(ref path) = cli.config_path {
        std::env::set_var("MARKET_MONITOR_CONFIG", path);
    }

    let _ = rustls::crypto::ring::default_provider().install_default();

    // ── Setup subcommand early-exit ─────────────────────────────────
    //
    // The setup flow is interactive (or `--dry-run`) and does its
    // own config.toml I/O + (optionally) spawns the daemon as a
    // child process. It must short-circuit BEFORE the heavy daemon
    // bootstrap (DB pool, telemetry logger, workspace state, …)
    // fires.
    if matches!(cli.mode, LaunchMode::Setup) {
        let sub = cli.setup_mode.as_deref().unwrap_or("interactive");
        match sub {
            "status" => {
                run_setup_status(&cli);
                std::process::exit(0);
            }
            "interactive" | "setup" => {
                run_setup_interactive(&cli).await;
                std::process::exit(0);
            }
            other => {
                eprintln!(
                    "Unknown --sub value: '{}' (expected 'interactive' or 'status')",
                    other
                );
                std::process::exit(2);
            }
        }
    }

    println!("⚙️  Trading Platform: Loading Master Configuration...");

    // AUDIT-V7-300 (08-08 §CB-01): one-shot startup warning for stale
    // legacy `analysis_limit` keys — the canonical number is
    // `candle_buffer.size` and stale keys are silently ignored.
    if let Ok(raw_toml) = std::fs::read_to_string("config.toml") {
        if config_models::detect_legacy_analysis_limit_keys(&raw_toml) {
            eprintln!(
                "⚠️  config.toml contains legacy `analysis_limit` key(s) — \
                 ignored; the canonical candle-buffer number is `[candle_buffer] size` \
                 (see docs/operations-and-compliance/08-08-candle-buffer-spec.md)"
            );
        }
    }

    let platform = load_platform().expect(
        "❌ Configuration Error: failed to parse platform config from config.toml",
    );
    let workspace = load_workspace().expect(
        "❌ Configuration Error: failed to parse workspace config from config.toml",
    );
    println!(
        "✅ Configuration Loaded: platform + workspace ({} instance{})",
        workspace.instances.len(),
        if workspace.instances.len() == 1 { "" } else { "s" }
    );

    match cli.mode {
        LaunchMode::Headless => println!("🤖 Launch mode: HEADLESS (auto-spawn, no Welcome Gate)"),
        LaunchMode::Web => println!("🖥️  Launch mode: WEB (Welcome Gate will prompt for exchange/currency)"),
        // Setup is short-circuited at the top of main(); this arm
        // exists for exhaustiveness only.
        LaunchMode::Setup => unreachable!("Setup is short-circuited at the top of main()"),
    }

    println!("🗄️  Initializing local SQLite telemetry database...");
    let db_pool = init_db().await;
    println!("✅ Database Setup: Connected to local telemetry.db file and verified schema.");

    if let Ok(secret) = std::env::var("EXCHANGE_SECRET_KEY") {
        let secret = secret.trim().to_string();
        if !secret.is_empty() {
            database_storage::crypto::init_master_key(&secret);
        }
    }
    verify_encryption_or_panic(&db_pool).await;

    let (telemetry_tx, telemetry_rx) = mpsc::channel::<database_storage::TelemetryMsg>(10000);
    // Read the liquidation-event retention window from the user's
    // `[workspace.liquidity]` config. The legacy hardcoded `7u32` was
    // 5x shorter than the configured 90 days and prematurely aged out
    // event rows that operators still wanted to query.
    let liq_retention_days = workspace.liquidity.event_retention_days.max(1);
    let logger_handle = tokio::spawn({
        let pool = db_pool.clone();
        async move {
            run_telemetry_logger(pool, telemetry_rx, liq_retention_days).await;
        }
    });

    let symbol_mapper = Arc::new(core_domain::normalized::SymbolMapper::new());
    let connection_quality = Arc::new(ConnectionQualityRegistry::new());
    let reliability = Arc::new(ReliabilityTracker::new());
    let exchange_status = Arc::new(ExchangeStatusTracker::new());
    let latency_tracker = Arc::new(core_domain::LatencyTracker::default());

    let execution_engine = Arc::new({
        let mut engine = portfolio_supervisor::execution::engine::ExecutionEngine::new();
        engine.set_db(Arc::new(db_pool.clone()));
        engine
    });
    *execution_engine.slippage_ceiling_pct.write().await = workspace.execution.slippage_ceiling_pct;
    execution_engine.set_fee_config(
        workspace.fees.maker_fee_pct,
        workspace.fees.taker_fee_pct,
        workspace.leverage.cross_leverage,
    ).await;

    let platform_arc = Arc::new(RwLock::new(platform));
    let workspace_state = WorkspaceState::new(workspace.clone());
    let session = Arc::new(portfolio_supervisor::session::SessionState::new());
    let (recharge_tx, _) =
        tokio::sync::broadcast::channel::<api_gateway::RechargeNotice>(64);

    let hl_ws_url = platform_arc.read().await.hyperliquid.ws_url.clone();
    let bg_ws_url = platform_arc.read().await.bitget.ws_url.clone();
    let use_hl = workspace.default_exchange.eq_ignore_ascii_case("hyperliquid");
    let use_bg = workspace.default_exchange.eq_ignore_ascii_case("bitget");
    if use_hl { exchange_status.seed_single("Hyperliquid", &hl_ws_url).await; }
    if use_bg  { exchange_status.seed_single("Bitget", &bg_ws_url).await; }
    println!("📡 Hyperliquid WS endpoint: {}", hl_ws_url);
    println!("📡 Bitget WS endpoint: {}", bg_ws_url);

    // ── Snapshot Export runtime (hydrated from `[snapshot_export]`) ─
    let snapshot_export_cfg = platform_arc.read().await.snapshot_export.clone();
    let snapshot_export_runtime = Arc::new(RwLock::new(
        execution_daemon::snapshot_export::runtime_from_config(&snapshot_export_cfg),
    ));
    let snapshot_export_manual_tick = Arc::new(tokio::sync::Notify::new());

    let mut app_state = Arc::new(AppState {
        workspace: workspace_state.clone(),
        session: session.clone(),
        platform: platform_arc.clone(),
        pool: db_pool.clone(),
        symbol_mapper: symbol_mapper.clone(),
        telemetry_tx: telemetry_tx.clone(),
        connection_quality: connection_quality.clone(),
        clock_monitor: None,
        reliability: reliability.clone(),
        exchange_status: exchange_status.clone(),
        latency_tracker: latency_tracker.clone(),
        ws_url: hl_ws_url.clone(),
        bitget_ws_url: bg_ws_url.clone(),
        overview: Arc::new(RwLock::new(None)),
        execution_engine: execution_engine.clone(),
        recharge_tx: recharge_tx.clone(),
        snapshot_export: snapshot_export_runtime.clone(),
        snapshot_export_manual_tick: snapshot_export_manual_tick.clone(),
    });

    // ── Session auto-init (headless and web mode) ──────────────────
    //
    // In both modes we initialise the session so that instances can be
    // spawned. In web mode the user may re-select the exchange via the
    // Welcome Gate; the gate handler will overwrite the session fields.
    {
        let exchange = cli.resolve_exchange(&workspace.default_exchange);
        let currency = cli.resolve_currency(&workspace.default_currency);
        if let Err(e) = app_state.init_session(currency, exchange).await {
            eprintln!("⚠️  Session auto-init failed: {}", e);
        } else {
            println!(
                "✅ Session auto-initialised: {} on {}",
                currency.as_str(),
                exchange.as_str(),
            );
        }
    }

    // ── Instance auto-spawn (both modes) ───────────────────────────
    //
    // Every entry in workspace.instances[] is spawned automatically.
    // In web mode the user may additionally add more pairs via the GUI.
    // In headless mode this is the ONLY way instances are created.
    //
    // **v6.5 fix (AUDIT-V7-306):** the session is left `active = true`
    // through the auto-spawn loop. The Welcome Gate only needs the
    // session fields (exchange, currency), not the `active` flag — and
    // `add_instance` rejects inactive sessions. We flip to inactive
    // **after** the loop completes, so configured instances bootstrap on
    // cold start in `--web` mode just like `--headless`.
    {
        let ctx = app_state.registry_context();
        for entry in &workspace.instances {
            if entry.symbol.is_empty() {
                continue;
            }
            let (base, quote) = match entry.symbol.split_once('-') {
                Some((b, q)) => (b.to_string(), q.to_string()),
                None => {
                    eprintln!("⚠️  Skipping malformed symbol: {}", entry.symbol);
                    continue;
                }
            };
            match registry::add_instance(&ctx, (base, quote)).await {
                Ok(_inst) => {
                    println!("✅ Instance spawned: {}", entry.symbol);
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to spawn instance {}: {}", entry.symbol, e);
                }
            }
        }

        // v6.5 (AUDIT-V7-306): in web mode, mark session inactive AFTER
        // auto-spawn so the Welcome Gate still appears on first page load
        // but cold-start bootstrap is no longer skipped.
        if matches!(cli.mode, LaunchMode::Web) {
            app_state.session.active.store(false, std::sync::atomic::Ordering::Relaxed);
            println!("   (session marked inactive for Welcome Gate)");
        }
    }

    // ── Safety pool initialization ─────────────────────────────────
    {
        let instances = workspace_state.list().await;
        let pool_arc = Arc::new(db_pool.clone());
        for inst in &instances {
            inst.safety.set_db_pool(Arc::clone(&pool_arc)).await;
        }
    }

    // ── TAE: Policy & Execution Engines ─────────────────────────────
    let policy_engine = Arc::new(RwLock::new(
        portfolio_supervisor::policy::engine::PolicyEngine::new(
            workspace.execution_policies.clone(),
        ),
    ));
    let paper_engine = Arc::new({
        let mut engine =
            portfolio_supervisor::paper_trading::PaperTradingEngine::new(
                portfolio_supervisor::paper_trading::FeesConfig {
                    maker_fee_pct: workspace.fees.maker_fee_pct,
                    taker_fee_pct: workspace.fees.taker_fee_pct,
                    funding_rate_8h: workspace.fees.funding_rate_8h,
                    simulated_spread_pct: 0.01,
                },
            );
        engine.set_db(Arc::new(db_pool.clone()));
        engine
    });
    {
        let total_capital: f64 = workspace
            .instances
            .iter()
            .map(|e| e.initial_capital_usd)
            .sum();
        if total_capital > 0.0 {
            *paper_engine.equity.write().await = rust_decimal::Decimal::from_f64_retain(total_capital).unwrap_or(rust_decimal_macros::dec!(10000));
        }
    }

    if !workspace.execution_policies.is_empty() {
        println!(
            "⚡ TAE: Initialized with {} execution policies",
            workspace.execution_policies.len()
        );
    }

    // ── Clock-drift monitor (NTP-based) — must run BEFORE build_router
    //    so the Arc is stored in AppState before the router clones it.
    if let Some(clock_cfg) = platform_arc.read().await.clock_monitor.clone() {
        if clock_cfg.is_active() {
            let monitor_cfg = ClockMonitorConfig {
                ntp_servers: clock_cfg.ntp_servers.clone(),
                poll_interval: std::time::Duration::from_secs(clock_cfg.poll_interval_secs),
                threshold: std::time::Duration::from_micros(
                    clock_cfg.threshold_micros.max(0) as u64,
                ),
                breach_action: match clock_cfg.breach_action {
                    ClockMonitorBreachAction::Warn => BreachAction::Warn,
                    ClockMonitorBreachAction::Panic => BreachAction::Panic,
                },
                warn_on_breach: clock_cfg.warn_on_breach,
                jitter_window_size: clock_cfg.jitter_window_size,
                query_timeout: std::time::Duration::from_secs(clock_cfg.query_timeout_secs),
            };
            let monitor = Arc::new(ClockMonitor::new(monitor_cfg));
            Arc::get_mut(&mut app_state).unwrap().clock_monitor = Some(monitor.clone());
            println!(
                "🕒 Clock Monitor: starting NTP polling ({} servers, threshold={}µs)",
                clock_cfg.ntp_servers.len(),
                clock_cfg.threshold_micros
            );
        } else {
            println!("🕒 Clock Monitor: disabled by config");
        }
    } else {
        println!("🕒 Clock Monitor: no [clock_monitor] section — drift enforcement disabled");
    }

    let app = build_router(app_state.clone());

    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    handles.push(logger_handle);

    // ── PME Veto Loop ────────────────────────────────────────────────
    let (veto_tx, mut veto_rx) = tokio::sync::mpsc::channel::<portfolio_supervisor::veto_loop::VetoEvent>(64);
    {
        let veto_workspace = workspace_state.clone();
        let veto_paper = paper_engine.clone();
        let veto_overview = app_state.overview.clone();
        handles.push(portfolio_supervisor::veto_loop::spawn_veto_loop(
            veto_workspace,
            veto_paper,
            veto_overview,
            veto_tx,
        ));
    }
    println!("🛡️  PME: Veto safety loop started (5s cadence)");

    // ── TAE event loop ──────────────────────────────────────────────
    if !workspace.execution_policies.is_empty() {
        let tae_policy = policy_engine.clone();
        let tae_exec = execution_engine.clone();
        let tae_paper = paper_engine.clone();
        let tae_workspace = workspace_state.clone();
        let tae_cancel = CancellationToken::new();
        let tae_pool = db_pool.clone();
        handles.push(tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(5));
            let mut funding_tick: u64 = 0;
            let funding_interval_secs: u64 = 8 * 3600; // 8h funding settlement

            // Trigger engine: track candle completions per timeframe
            let candle_counters: std::sync::Arc<tokio::sync::RwLock<
                std::collections::HashMap<String, u32>,
            >> = std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            ));

            // Per-timeframe last-candle counters for CandleClose triggers
            let mut last_seen_candles: std::collections::HashMap<String, u32> =
                std::collections::HashMap::new();

            loop {
                tokio::select! {
                    _ = tae_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        let now_secs = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                        // Periodic funding rate settlement
                        funding_tick += 5;
                        if funding_tick >= funding_interval_secs {
                            tae_paper.settle_funding().await;
                            funding_tick = 0;
                        }

                        let instances = tae_workspace.list().await;
                        for inst in instances {
                            let snapshot_guard = inst.micro.latest.read().await;
                            let mut lifecycle = inst.lifecycle.write().await;

                            // Evaluate automation conditions
                            let current_price = snapshot_guard.as_ref()
                                .and_then(|s| s.close.as_ref())
                                .and_then(|c| c.to_string().parse::<f64>().ok());
                            let auto_actions = lifecycle.evaluate_automation(current_price);
                            for action in auto_actions {
                                match action {
                                    portfolio_supervisor::lifecycle::AutomationAction::Start => {
                                        let _ = lifecycle.start("automation", Some("Auto start condition".into()));
                                        eprintln!("🤖 LIFECYCLE: Auto-started instance {}", inst.id);
                                    }
                                    portfolio_supervisor::lifecycle::AutomationAction::Pause => {
                                        let _ = lifecycle.pause("automation", Some("Auto pause condition".into()));
                                        eprintln!("🤖 LIFECYCLE: Auto-paused instance {}", inst.id);
                                    }
                                    portfolio_supervisor::lifecycle::AutomationAction::Stop => {
                                        let _ = lifecycle.stop("automation", Some("Auto stop condition".into()));
                                        eprintln!("🤖 LIFECYCLE: Auto-stopped instance {}", inst.id);
                                    }
                                }
                            }

                            let lifecycle_state = lifecycle.state;

                            // ── STOPPING → STOPPED auto-transition ──
                            if lifecycle_state == config_models::LifecycleState::Stopping {
                                let paper_positions = tae_paper.positions.read().await;
                                let has_positions = paper_positions.contains_key(&inst.symbol());
                                drop(paper_positions);
                                let orders = tae_exec.orders.read().await;
                                let has_open_orders = orders.iter().any(
                                    |(_, o)| o.packet.symbol == inst.symbol()
                                        && o.status != config_models::OrderStatus::Closed
                                        && o.status != config_models::OrderStatus::Cancelled
                                        && o.status != config_models::OrderStatus::Rejected
                                );
                                drop(orders);
                                if !has_positions && !has_open_orders {
                                    let _ = lifecycle.complete_stop().await;
                                    eprintln!("🛑 LIFECYCLE: Instance {} → STOPPED (flatten confirmed)", inst.id);
                                    drop(lifecycle);
                                    continue;
                                }
                                drop(lifecycle);
                                continue;
                            }

                            drop(lifecycle);

                            // Skip if STOPPED (no new triggers)
                            if lifecycle_state == config_models::LifecycleState::Stopped {
                                continue;
                            }

                            // ── Process PME veto events (VetoHandler-driven) ──
                            let mut veto_handlers: Vec<portfolio_supervisor::policy::veto::VetoHandler> = Vec::new();
                            while let Ok(veto) = veto_rx.try_recv() {
                                if veto.instance_id != inst.id {
                                    continue;
                                }
                                eprintln!(
                                    "⚡ TAE: Processing veto for {} — stance={:?} hard_exit={}",
                                    veto.symbol, veto.target_stance, veto.hard_exit
                                );

                                let prev_stance = inst.stances.read().await
                                    .get(&veto.symbol).copied().unwrap_or(config_models::Stance::Active);

                                let mut handler = portfolio_supervisor::policy::veto::VetoHandler::new(2000);
                                handler.initiate(portfolio_supervisor::policy::veto::VetoEvent {
                                    symbol: veto.symbol.clone(),
                                    target_stance: veto.target_stance,
                                    reason: veto.reason.clone(),
                                    timestamp_ms: veto.timestamp_ms,
                                });

                                // Step 2a: Hard Exit (AVOID only) — BEFORE stance change
                                if handler.needs_hard_exit() {
                                    tae_exec.hard_exit_for_symbol(&veto.symbol).await;
                                    eprintln!("🔥 VETO: Hard Exit dispatched for {}", veto.symbol);
                                }
                                handler.advance_phase(); // → DiscardPending
                                handler.advance_phase(); // → CommitStance

                                // Step 2c: Commit stance change
                                let mut stances = inst.stances.write().await;
                                stances.insert(veto.symbol.clone(), veto.target_stance);
                                drop(stances);
                                handler.advance_phase(); // → CancelRemaining

                                // Step 2d: Cancel remaining orders
                                tae_exec.cancel_all_orders(&veto.symbol).await;
                                handler.advance_phase(); // → NullifyEntry
                                handler.advance_phase(); // → Audit

                                // Step 2f: Audit log
                                let log = portfolio_supervisor::policy::veto::VetoLog::new(
                                    &portfolio_supervisor::policy::veto::VetoEvent {
                                        symbol: veto.symbol.clone(),
                                        target_stance: veto.target_stance,
                                        reason: veto.reason.clone(),
                                        timestamp_ms: veto.timestamp_ms,
                                    },
                                    prev_stance,
                                    veto.hard_exit,
                                );
                                eprintln!(
                                    "📋 VETO LOG: symbol={} from={:?} to={:?} reason={} hard_exit={}",
                                    log.symbol, log.from_stance, log.to_stance, log.reason, log.hard_exit_dispatched
                                );

                                let _ = sqlx::query(
                                    "INSERT INTO risk_control_events (instance_id, symbol, gate_id, decision, reason, timestamp_ms) VALUES (?, ?, ?, ?, ?, ?)"
                                )
                                .bind(&inst.id)
                                .bind(&veto.symbol)
                                .bind(7i64)
                                .bind("VETO_BLOCK")
                                .bind(&veto.reason)
                                .bind(veto.timestamp_ms as i64)
                                .execute(&tae_pool)
                                .await;

                                veto_handlers.push(handler);
                            }

                            if let Some(ref snap) = *snapshot_guard {
                                if snap.is_completed != Some(true) {
                                    continue;
                                }

                                // Track candle completions for CandleClose triggers
                                let timeframe_label = &snap.symbol;
                                {
                                    let mut counters = candle_counters.write().await;
                                    let count = counters.entry(timeframe_label.clone())
                                        .and_modify(|c| *c += 1)
                                        .or_insert(1);
                                    last_seen_candles.insert(timeframe_label.clone(), *count);
                                }

                                let stances = inst.stances.read().await;
                                let symbol = snap.symbol.clone();
                                let stance = stances.get(&symbol).copied()
                                    .unwrap_or(config_models::Stance::Active);

                                // Sync safety state from instance to execution engine
                                {
                                    let safety_state = *inst.safety.safety_state.read().await;
                                    let safety_str = format!("{:?}", safety_state);
                                    tae_exec.set_safety_state(&safety_str).await;
                                }

                                // Auto-pause policies for symbols in Cautious/Suspended safety states
                                {
                                    let safety_state = *inst.safety.safety_state.read().await;
                                    let should_auto_pause = matches!(
                                        safety_state,
                                        SafetyState::Cautious
                                            | SafetyState::Suspended
                                    );
                                    let policy_ids: Vec<String> = {
                                        let policy = tae_policy.read().await;
                                        policy.get_active_stance_policies()
                                            .iter()
                                            .filter(|p| p.symbol == symbol)
                                            .map(|p| p.policy_id.clone())
                                            .collect()
                                    };
                                    if !policy_ids.is_empty() {
                                        let mut policy = tae_policy.write().await;
                                        for pid in &policy_ids {
                                            policy.set_policy_auto_paused(pid, should_auto_pause);
                                            if should_auto_pause {
                                                eprintln!("⏸️  POLICY AUTO_PAUSED: policy={} symbol={} safety={:?}", pid, symbol, safety_state);
                                            }
                                        }
                                    }
                                }

                                // Check which policies are due per their TriggerMode
                                let candles_completed = last_seen_candles
                                    .get(timeframe_label).copied().unwrap_or(0);
                                let pending_events: Vec<String> = vec![]; // EventDriven events would come from MME

                                let policies_due: Vec<config_models::ExecutionPolicy> = {
                                    let policy = tae_policy.read().await;
                                    policy.get_active_stance_policies()
                                        .iter()
                                        .filter(|p| {
                                            policy.is_policy_due(
                                                &p.policy_id,
                                                &p.trigger_mode,
                                                now_secs,
                                                candles_completed,
                                                &pending_events,
                                            )
                                        })
                                        .map(|&p| p.clone())
                                        .collect()
                                };

                                if !policies_due.is_empty() {
                                    let triggers = {
                                        let mut policy = tae_policy.write().await;
                                        let triggers = policy.evaluate_policies(snap, now_secs);
                                        for due_policy in &policies_due {
                                            if matches!(due_policy.trigger_mode, config_models::TriggerMode::Interval { .. }) {
                                                policy.mark_interval_evaluated(&due_policy.policy_id, now_secs);
                                            }
                                        }
                                        triggers
                                    };

                                    for trigger in &triggers {
                                        match tae_exec.process_trigger(
                                            trigger, snap,
                                            lifecycle_state, stance,
                                        ).await {
                                            Ok(Some(order_id)) => {
                                                let lifecycle = tae_exec.orders.read().await
                                                    .get(&order_id)
                                                    .map(|o| o.status);
                                                let is_pre_dispatch = lifecycle == Some(config_models::OrderStatus::PreDispatch);
                                                if !is_pre_dispatch {
                                                    let packet = tae_exec.orders.read().await
                                                        .get(&order_id)
                                                        .map(|o| o.packet.clone());
                                                    if let Some(pkt) = packet {
                                                        let _ = tae_paper.submit_order(
                                                            pkt,
                                                            snap.mid_price,
                                                        ).await;
                                                    }
                                                }
                                            }
                                            Ok(None) => {}
                                            Err(e) => {
                                                eprintln!("TAE: trigger error: {}", e);
                                            }
                                        }
                                    }
                                }
                            }

                            // Evaluate pending paper order fills
                            if let Some(ref snap) = *snapshot_guard {
                                let _fills = tae_paper.evaluate_order_fills(snap.mid_price).await;
                            }

                            // Sync paper engine equity back to instance trading state
                            let current_equity = tae_paper.get_equity().await;
                            inst.set_current_equity(current_equity).await;
                            inst.safety.set_current_equity(
                                rust_decimal::Decimal::from_f64_retain(current_equity)
                                    .unwrap_or_default(),
                            ).await;
                        }
                    }
                }
            }
        }));
    }

    // ── L7 Overview aggregation task ──────────────────────────
    {
        let overview_ref = app_state.overview.clone();
        let workspace = workspace_state.clone();
        let overview_cancel = CancellationToken::new();
        handles.push(tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = overview_cancel.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                }
                let instances = workspace.list().await;
                let mut advisories: Vec<core_domain::advisory::AdvisoryMatrix> = Vec::new();
                let mut alignments: Vec<core_domain::alignment::AlignmentMatrix> = Vec::new();
                let mut metas: Vec<core_domain::overview::InstanceMeta> = Vec::new();
                for inst in &instances {
                    let snapshots = inst.active_pair.latest_snapshots_all_tf().await;
                    // v6.10.18 (I-2): the L7 aggregates ALL FOUR timeframe
                    // windows (micro / fast / slow / macro) — per-window
                    // advisories feed the breadth / bias / opportunity /
                    // regime tallies. The previous slow-300s-only basis
                    // made the dashboard headline contradict every panel
                    // (e.g. HIGH_RISK next to an avg-risk of 41, or a
                    // stale "Pullback" while the Opportunity tab shows
                    // Scalp).
                    // v6.10.19a: the canonical per-pair risk is the MICRO
                    // window's L5 score (the same number the Risk panel,
                    // the dashboard KPI, and the asset rows show) — the
                    // TF-window MEAN drifted upward whenever the macro
                    // window scored high (MAX_COMPRESSION / thin
                    // participation) and rendered "HIGH_RISK" next to a
                    // visible avg-risk of 41–46. The window mean remains
                    // the warmup fallback when micro has no risk matrix
                    // yet.
                    let snaps = [
                        snapshots.0.as_ref(),
                        snapshots.1.as_ref(),
                        snapshots.2.as_ref(),
                        snapshots.3.as_ref(),
                    ];
                    let is_active = !inst.cancel.is_cancelled();
                    // v6.10.19 (P7): per-TF-window risk pairs with decay
                    // weights (micro 0.1 / fast 0.2 / slow 0.3 / macro
                    // 0.4) — the L7 SYSTEMIC path must stay anchored to
                    // macro stability so a transient micro risk spike can
                    // never fire the PME safety veto.
                    let tf_weights = [0.1_f64, 0.2, 0.3, 0.4];
                    let mut risk_windows: Vec<(f64, f64)> = Vec::new();
                    let mut risk_sum = 0.0;
                    let mut risk_count = 0u32;
                    let mut alignment_pushed = false;
                    for (i, snap) in snaps.iter().enumerate() {
                        if let Some(snap) = snap {
                            if let Some(adv) = snap.advisory.clone() {
                                advisories.push(adv);
                            }
                            if let Some(r) = snap.risk.as_ref() {
                                risk_windows.push((tf_weights[i], r.overall_risk.score));
                                risk_sum += r.overall_risk.score;
                                risk_count += 1;
                            }
                            // The MTF AlignmentMatrix is one per symbol — push
                            // it once, from the fastest present window.
                            if !alignment_pushed {
                                if let Some(aln) = snap.alignment.clone() {
                                    alignments.push(aln);
                                    alignment_pushed = true;
                                }
                            }
                        }
                    }
                    metas.push(core_domain::overview::InstanceMeta {
                        symbol: inst.pair.1.clone(),
                        timeframe_secs: 300,
                        timeframe_label: "tf-average".into(),
                        is_active,
                        // L7-A (v6.10.13): the per-symbol L5 overall risk —
                        // the canonical aggregate the L7 risk distribution
                        // bins on. v6.10.19a: the MICRO window's L5 score
                        // (operator-visible); falls back to the window mean,
                        // then 50 (moderate) during warmup.
                        overall_risk: canonical_overall_risk(
                            snaps[0]
                                .and_then(|s| s.risk.as_ref())
                                .map(|r| r.overall_risk.score),
                            risk_count,
                            risk_sum,
                        ),
                        risk_windows,
                    });
                }
                let overview =
                    core_domain::overview::compute_overview(&advisories, &metas, &alignments);
                *overview_ref.write().await = Some(overview);
            }
        }));
    }

    // ── Snapshot Export periodic task ─────────────────────────────
    {
        let runtime = snapshot_export_runtime.clone();
        let workspace = workspace_state.clone();
        let manual_tick = snapshot_export_manual_tick.clone();
        handles.push(tokio::spawn(async move {
            execution_daemon::snapshot_export::run_snapshot_exporter(
                runtime,
                workspace,
                CancellationToken::new(),
                manual_tick,
            )
            .await;
        }));
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("❌ Web Server Setup: Failed to bind port 3000");

    println!("🌐 Web Server Setup: Dashboard live at http://127.0.0.1:3000");

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("❌ Web Server Setup: Fatal crash running Axum HTTP server");
    });
    handles.push(server_handle);

    let eval_cancel = CancellationToken::new();
    let eval_cancel1 = eval_cancel.clone();
    let pae_pool = db_pool.clone();
    handles.push(tokio::spawn(async move {
        performance_evaluator::run_performance_evaluator(performance_evaluator::EvaluatorConfig {
            pool: pae_pool,
            cancel: eval_cancel1,
            eval_interval_secs: 300,
        })
        .await;
    }));

    // Spawn clock monitor background task NOW (after storing its Arc).
    if let Some(clock_mon) = &app_state.clock_monitor {
        let monitor_clone = clock_mon.clone();
        let clock_cancel = CancellationToken::new();
        handles.push(tokio::spawn(async move {
            monitor_clone.run_until_cancelled(clock_cancel).await;
        }));
    }

    let quality_pool = db_pool.clone();
    let quality_registry = (*connection_quality).clone();
    let quality_cancel = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        quality_registry
            .run_persistence_loop(quality_pool, quality_cancel)
            .await;
    }));

    let eq_pool = db_pool.clone();
    let eq_cancel = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        portfolio_equity::run_portfolio_equity_logger(eq_pool, eq_cancel).await;
    }));

    handles.push(tokio::spawn(async move {
        strategy_optimizer::run_strategy_optimizer(strategy_optimizer::OptimizerConfig {
            pool: db_pool,
            cancel: eval_cancel,
            interval_secs: 3600,
        })
        .await;
    }));

    let _ = futures_util::future::join_all(handles).await;
}

#[cfg(test)]
mod tests {
    use super::canonical_overall_risk;

    // v6.10.19a: the L7 canonical risk is the MICRO window's L5 score —
    // the visible number. The window-mean drift (macro MAX_COMPRESSION
    // windows) produced "HIGH_RISK" environments next to an avg-risk of
    // 41–46; regression tests for all three input shapes.
    #[test]
    fn micro_risk_wins_over_window_mean() {
        // Micro 41.45 vs windows averaging 58.6 (macro-heavy) — the old
        // mean would have crossed the ≥50 HIGH_RISK line.
        assert!((canonical_overall_risk(Some(41.45), 4, 234.4) - 41.45).abs() < 1e-9);
    }

    #[test]
    fn window_mean_is_the_warmup_fallback() {
        assert!((canonical_overall_risk(None, 4, 234.4) - 58.6).abs() < 1e-9);
        assert!((canonical_overall_risk(None, 2, 90.0) - 45.0).abs() < 1e-9);
    }

    #[test]
    fn no_risk_matrices_yet_falls_back_to_moderate() {
        assert_eq!(canonical_overall_risk(None, 0, 0.0), 50.0);
    }
}
