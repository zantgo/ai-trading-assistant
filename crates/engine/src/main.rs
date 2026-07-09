use std::sync::Arc;
use tokio::sync::{mpsc::channel, RwLock};
use tokio_util::sync::CancellationToken;

use engine::{
    config, db, order_matcher, performance_evaluator, portfolio_equity, server,
    strategy_optimizer, workspace,
};
use shared::normalized::SymbolMapper;

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️  Quantitative Trading Engine: Loading Configuration...");
    let mut app_config = config::load_config();
    app_config.instances = config::load_instances();
    println!(
        "✅ Configuration Loaded: Initial pairs: {:?} ({} instance-specific configs)",
        app_config.symbols,
        app_config.instances.len()
    );
    let app_config = Arc::new(RwLock::new(app_config));

    println!(
        "🚪 Session-first boot: workspace starts empty and inactive. Awaiting Welcome Gate session initialization before any pipelines spawn."
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

    let (telemetry_tx, telemetry_rx) = channel::<db::TelemetryMsg>(20000);
    let logger_handle = tokio::spawn({
        let pool = db_pool.clone();
        async move {
            db::run_telemetry_logger(pool, telemetry_rx).await;
        }
    });

    let symbol_mapper = Arc::new(SymbolMapper::new());

    let hl_ws_url = app_config.read().await.hyperliquid.ws_url.clone();
    println!("📡 Hyperliquid WS endpoint: {}", hl_ws_url);

    let workspace = Arc::new(workspace::Workspace::new(
        app_config.clone(),
        db_pool.clone(),
        symbol_mapper.clone(),
        telemetry_tx.clone(),
        hl_ws_url.clone(),
    ));

    // Auto-restore session if profile config has persisted session data
    {
        let config = app_config.read().await;
        if let (Some(ref mode), Some(ref currency), Some(ref exchange), Some(capital)) = (
            &config.profile.session_mode,
            &config.profile.session_currency,
            &config.profile.session_exchange,
            &config.profile.initial_capital,
        ) {
            let trading_mode = match mode.to_lowercase().as_str() {
                "paper" => workspace::TradingMode::Paper,
                _ => workspace::TradingMode::Paper,
            };
            let cur = match currency.to_uppercase().as_str() {
                "USDT" => workspace::Currency::USDT,
                "USDC" => workspace::Currency::USDC,
                _ => workspace::Currency::USDC,
            };
            let exch = match exchange.to_lowercase().as_str() {
                "hyperliquid" => workspace::ExchangeChoice::Hyperliquid,
                _ => workspace::ExchangeChoice::Hyperliquid,
            };
            let name = config.profile.user_name.clone();
            match workspace.init_session(trading_mode, cur, exch, *capital, name).await {
                Ok(()) => println!("🔄 Auto-restored session from config.toml profile"),
                Err(e) => eprintln!("⚠️  Auto-restore failed: {}", e),
            }
        }
    }

    let app_state = Arc::new(server::AppState {
        workspace: workspace.clone(),
        config: app_config.clone(),
        pool: db_pool.clone(),
        symbol_mapper: symbol_mapper.clone(),
        telemetry_tx: telemetry_tx.clone(),
        ws_url: hl_ws_url.clone(),
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
    let eval_pool = db_pool.clone();
    let eval_cancel1 = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        performance_evaluator::run_performance_evaluator(performance_evaluator::EvaluatorConfig {
            pool: eval_pool,
            cancel: eval_cancel1,
            eval_interval_secs: 300,
        })
        .await;
    }));

    let eq_pool = db_pool.clone();
    let eq_workspace = workspace.clone();
    let eq_cancel = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        portfolio_equity::run_portfolio_equity_logger(eq_pool, eq_workspace, eq_cancel).await;
    }));

    let matcher_pool = db_pool.clone();
    let matcher_workspace = workspace.clone();
    let matcher_tx = telemetry_tx.clone();
    let matcher_cancel = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        order_matcher::run_order_matcher(
            matcher_workspace,
            matcher_pool,
            matcher_tx,
            matcher_cancel,
        ).await;
    }));

    let funding_pool = db_pool.clone();
    let funding_rate = app_config.read().await.fees.funding_rate_8h;
    let funding_cancel = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        engine::paper_trading::run_funding_decay_tracker(funding_pool, funding_rate, funding_cancel).await;
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
