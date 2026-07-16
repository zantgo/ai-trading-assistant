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
    connection_quality_tracker::ConnectionQualityTracker,
};
use performance_analytics::{performance_evaluator, strategy_optimizer};
use portfolio_supervisor::{
    portfolio_equity, registry, workspace_state::WorkspaceState,
    session::{Currency, ExchangeChoice},
};

// ─── CLI argument parsing ────────────────────────────────────────────

struct CliArgs {
    mode: LaunchMode,
    exchange: Option<String>,
    currency: Option<String>,
    config_path: Option<String>,
}

enum LaunchMode {
    Web,
    Headless,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = LaunchMode::Web; // default
    let mut exchange = None;
    let mut currency = None;
    let mut config_path = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                i += 1;
                if i < args.len() {
                    mode = match args[i].as_str() {
                        "headless" => LaunchMode::Headless,
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
            "--web" | "--gui" => {
                mode = LaunchMode::Web;
            }
            _ => { /* ignore unknown args */ }
        }
        i += 1;
    }

    CliArgs { mode, exchange, currency, config_path }
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

// ─── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = parse_args();

    // If --config is provided, set the env var that config-models reads.
    if let Some(ref path) = cli.config_path {
        std::env::set_var("MARKET_MONITOR_CONFIG", path);
    }

    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️  Market Monitor: Loading Master Configuration...");

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
    let logger_handle = tokio::spawn({
        let pool = db_pool.clone();
        async move {
            run_telemetry_logger(pool, telemetry_rx).await;
        }
    });

    let symbol_mapper = Arc::new(core_domain::normalized::SymbolMapper::new());
    let connection_quality = Arc::new(ConnectionQualityTracker::new());

    let platform_arc = Arc::new(RwLock::new(platform));
    let workspace_state = WorkspaceState::new(workspace.clone());
    let session = Arc::new(portfolio_supervisor::session::SessionState::new());

    let hl_ws_url = platform_arc.read().await.hyperliquid.ws_url.clone();
    let bg_ws_url = platform_arc.read().await.bitget.ws_url.clone();
    println!("📡 Hyperliquid WS endpoint: {}", hl_ws_url);
    println!("📡 Bitget WS endpoint: {}", bg_ws_url);

    let app_state = Arc::new(AppState {
        workspace: workspace_state.clone(),
        session: session.clone(),
        platform: platform_arc.clone(),
        pool: db_pool.clone(),
        symbol_mapper: symbol_mapper.clone(),
        telemetry_tx: telemetry_tx.clone(),
        connection_quality: connection_quality.clone(),
        ws_url: hl_ws_url.clone(),
        bitget_ws_url: bg_ws_url.clone(),
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
    }

    let app = build_router(app_state.clone());

    let mut handles = Vec::new();
    handles.push(logger_handle);

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
    handles.push(tokio::spawn(async move {
        performance_evaluator::run_performance_evaluator(performance_evaluator::EvaluatorConfig {
            cancel: eval_cancel1,
            eval_interval_secs: 300,
        })
        .await;
    }));

    // Clock-drift monitor (NTP-based)
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
            let monitor = ClockMonitor::new(monitor_cfg);
            let clock_cancel = CancellationToken::new();
            println!(
                "🕒 Clock Monitor: starting NTP polling ({} servers, threshold={}µs)",
                clock_cfg.ntp_servers.len(),
                clock_cfg.threshold_micros
            );
            handles.push(tokio::spawn(async move {
                monitor.run_until_cancelled(clock_cancel).await;
            }));
        } else {
            println!("🕒 Clock Monitor: disabled by config");
        }
    } else {
        println!("🕒 Clock Monitor: no [clock_monitor] section — drift enforcement disabled");
    }

    let quality_pool = db_pool.clone();
    let quality_tracker = connection_quality;
    let quality_cancel = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        quality_tracker
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
