use std::sync::Arc;
use tokio::sync::{mpsc::channel, RwLock};
use tokio_util::sync::CancellationToken;

use engine::{
    config, db, performance_evaluator, portfolio_equity, server,
    strategy_optimizer,
};
use shared::normalized::SymbolMapper;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let _web_mode = args.is_empty()
        || args.iter().any(|a| a == "--web" || a == "--gui");

    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️  Market Monitor: Loading Master Configuration...");
    let mut app_config = config::load_config();
    app_config.instances = config::load_instances();
    println!(
        "✅ Configuration Loaded: Initial pairs: {:?} ({} instance-specific configs)",
        app_config.symbols,
        app_config.instances.len()
    );

    println!(
        "🚪 Session-first boot: system starts empty and inactive. Awaiting Welcome Gate session initialization before any pipelines spawn."
    );

    println!("🗄️  Initializing local SQLite telemetry database...");
    let db_pool = db::init_db().await;
    println!("✅ Database Setup: Connected to local telemetry.db file and verified schema.");

    if let Ok(secret) = std::env::var("EXCHANGE_SECRET_KEY") {
        let secret = secret.trim().to_string();
        if !secret.is_empty() {
            db::crypto::init_master_key(&secret);
        }
    }
    db::verify_encryption_or_panic(&db_pool).await;

    let (telemetry_tx, telemetry_rx) = channel::<db::TelemetryMsg>(10000);
    let logger_handle = tokio::spawn({
        let pool = db_pool.clone();
        async move {
            db::run_telemetry_logger(pool, telemetry_rx).await;
        }
    });

    let symbol_mapper = Arc::new(SymbolMapper::new());

    let app_config = Arc::new(RwLock::new(app_config));
    let hl_ws_url = app_config.read().await.hyperliquid.ws_url.clone();
    let bg_ws_url = app_config.read().await.bitget.ws_url.clone();
    println!("📡 Hyperliquid WS endpoint: {}", hl_ws_url);
    println!("📡 Bitget WS endpoint: {}", bg_ws_url);

    let app_state = Arc::new(server::AppState {
        instances: Arc::new(RwLock::new(std::collections::HashMap::new())),
        session: engine::session::SessionState::new(),
        config: app_config.clone(),
        pool: db_pool.clone(),
        symbol_mapper: symbol_mapper.clone(),
        telemetry_tx: telemetry_tx.clone(),
        ws_url: hl_ws_url.clone(),
        bitget_ws_url: bg_ws_url.clone(),
    });

    let app = server::build_router(app_state.clone());

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

    let eq_pool = db_pool.clone();
    let eq_state = app_state.clone();
    let eq_cancel = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        portfolio_equity::run_portfolio_equity_logger(eq_pool, eq_state, eq_cancel).await;
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
