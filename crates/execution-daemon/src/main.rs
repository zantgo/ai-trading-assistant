//! # Execution Daemon
//!
//! Headless orchestrator binary. Reads configuration, initializes the
//! SQLite database, builds the Axum `AppState`, spawns background tasks
//! (telemetry logger, portfolio equity logger, performance evaluator,
//! strategy optimizer, clock monitor, connection-quality persistence),
//! then runs the Axum HTTP server on `127.0.0.1:3000`.

use std::sync::Arc;
use tokio::sync::{mpsc::channel, RwLock};
use tokio_util::sync::CancellationToken;

use api_gateway::{build_router, AppState};
use config_models::{load_config, load_instances, ClockMonitorBreachAction};
use database_storage::{
    init_db, run_telemetry_logger, verify_encryption_or_panic,
};
use network_adapters::{
    clock_monitor::{BreachAction, ClockMonitor, ClockMonitorConfig},
    connection_quality_tracker::ConnectionQualityTracker,
};
use performance_analytics::{performance_evaluator, strategy_optimizer};
use portfolio_supervisor::{portfolio_equity, registry_context::RegistryContext};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let _web_mode = args.is_empty() || args.iter().any(|a| a == "--web" || a == "--gui");

    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️  Market Monitor: Loading Master Configuration...");
    let mut app_config = load_config();
    app_config.instances = load_instances();
    println!(
        "✅ Configuration Loaded: Initial pairs: {:?} ({} instance-specific configs)",
        app_config.symbols,
        app_config.instances.len()
    );

    println!(
        "🚪 Session-first boot: system starts empty and inactive. Awaiting Welcome Gate session initialization before any pipelines spawn."
    );

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

    let (telemetry_tx, telemetry_rx) = channel::<database_storage::TelemetryMsg>(10000);
    let logger_handle = tokio::spawn({
        let pool = db_pool.clone();
        async move {
            run_telemetry_logger(pool, telemetry_rx).await;
        }
    });

    let symbol_mapper = Arc::new(core_domain::normalized::SymbolMapper::new());
    let connection_quality = Arc::new(ConnectionQualityTracker::new());

    let app_config = Arc::new(RwLock::new(app_config));
    let hl_ws_url = app_config.read().await.hyperliquid.ws_url.clone();
    let bg_ws_url = app_config.read().await.bitget.ws_url.clone();
    println!("📡 Hyperliquid WS endpoint: {}", hl_ws_url);
    println!("📡 Bitget WS endpoint: {}", bg_ws_url);

    let session = Arc::new(portfolio_supervisor::session::SessionState::new());
    let app_state = Arc::new(AppState {
        instances: Arc::new(RwLock::new(std::collections::HashMap::new())),
        session: session.clone(),
        config: app_config.clone(),
        pool: db_pool.clone(),
        symbol_mapper: symbol_mapper.clone(),
        telemetry_tx: telemetry_tx.clone(),
        connection_quality: connection_quality.clone(),
        ws_url: hl_ws_url.clone(),
        bitget_ws_url: bg_ws_url.clone(),
    });

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
    if let Some(clock_cfg) = app_config.read().await.clock_monitor.clone() {
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
            println!("🕒 Clock Monitor: disabled by config (enabled=false or no NTP servers)");
        }
    } else {
        println!("🕒 Clock Monitor: no [clock_monitor] section in config.toml — drift enforcement disabled");
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

// Suppress unused warnings for items used implicitly by ApiState but not in this file.
#[allow(dead_code)]
async fn _ensure_types_used() {
    let pool = database_storage::init_db().await;
    let _ctx = RegistryContext {
        instances: Arc::new(RwLock::new(Default::default())),
        session: Arc::new(portfolio_supervisor::session::SessionState::new()),
        config: Arc::new(RwLock::new(config_models::AppConfig {
            symbols: vec![],
            candles: config_models::CandlesConfig {
                duration_seconds: 60,
                analysis_limit: 500,
            },
            indicators: config_models::IndicatorsConfig::default(),
            hyperliquid: Default::default(),
            bitget: Default::default(),
            fibonacci: Default::default(),
            pivots: Default::default(),
            slow_timeframe: Default::default(),
            macro_timeframe: Default::default(),
            leverage: Default::default(),
            scoring: Default::default(),
            fees: Default::default(),
            defaults: Default::default(),
            safety: Default::default(),
            intervals: Default::default(),
            liquidity: Default::default(),
            clock_monitor: None,
            instances: Default::default(),
        })),
        pool,
        symbol_mapper: Arc::new(core_domain::normalized::SymbolMapper::new()),
        telemetry_tx: channel::<database_storage::TelemetryMsg>(1).0,
        ws_url: String::new(),
        bitget_ws_url: String::new(),
    };
}
