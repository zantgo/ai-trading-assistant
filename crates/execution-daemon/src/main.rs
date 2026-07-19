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

    println!("⚙️  Trading Platform: Loading Master Configuration...");

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
    let liq_retention_days = 7u32;
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

    let hl_ws_url = platform_arc.read().await.hyperliquid.ws_url.clone();
    let bg_ws_url = platform_arc.read().await.bitget.ws_url.clone();
    let use_hl = workspace.default_exchange.eq_ignore_ascii_case("hyperliquid");
    let use_bg = workspace.default_exchange.eq_ignore_ascii_case("bitget");
    if use_hl { exchange_status.seed_single("Hyperliquid", &hl_ws_url).await; }
    if use_bg  { exchange_status.seed_single("Bitget", &bg_ws_url).await; }
    println!("📡 Hyperliquid WS endpoint: {}", hl_ws_url);
    println!("📡 Bitget WS endpoint: {}", bg_ws_url);

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
            // In web mode keep the session technically inactive so the
            // frontend Welcome Gate is shown on first page load.  The
            // session fields (exchange, currency) are already populated
            // so that config.toml instances can be auto-spawned below.
            if matches!(cli.mode, LaunchMode::Web) {
                app_state.session.active.store(false, std::sync::atomic::Ordering::Relaxed);
                println!("   (session marked inactive for Welcome Gate)");
            }
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
            *paper_engine.equity.write().await = total_capital;
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
                let mut metas: Vec<core_domain::overview::InstanceMeta> = Vec::new();
                for inst in &instances {
                    let snapshots = inst.active_pair.latest_snapshots_all_tf().await;
                    let slow_advisory = snapshots.2.as_ref().and_then(|s| s.advisory.clone());
                    let is_active = !inst.cancel.is_cancelled();
                    if let Some(adv) = slow_advisory {
                        advisories.push(adv);
                    }
                    metas.push(core_domain::overview::InstanceMeta {
                        symbol: inst.pair.1.clone(),
                        timeframe_secs: 300,
                        timeframe_label: "slow300".into(),
                        is_active,
                    });
                }
                let overview = core_domain::overview::compute_overview(&advisories, &metas);
                *overview_ref.write().await = Some(overview);
            }
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
