use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{mpsc::channel, RwLock};
use tokio_util::sync::CancellationToken;

use engine::{config, db, server, llm, performance_evaluator, strategy_optimizer, workspace, cli, instance_registry};
use shared::normalized::SymbolMapper;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let web_mode = args.iter().any(|a| a == "--web" || a == "--gui");
    let cli_mode = args.iter().any(|a| a == "--cli");

    let _ = rustls::crypto::ring::default_provider().install_default();

    match dotenvy::dotenv() {
        Ok(_) => println!("✅ Loaded .env configuration."),
        Err(e) => {
            eprintln!("⚠️  No .env file found: {}", e);
            eprintln!("   Create a .env file at the project root with: DEEPSEEK_API_KEY=sk-...");
            eprintln!("   The dashboard will run, but AI features require a valid key.");
        }
    }

    println!("⚙️  AI Trading Assistant: Loading Master Configuration...");
    let mut app_config = config::load_config();
    app_config.instances = config::load_instances();
    println!("✅ Configuration Loaded: Initial pairs: {:?} ({} instance-specific configs)", app_config.symbols, app_config.instances.len());
    let app_config = Arc::new(RwLock::new(app_config));
    let initial_symbols = app_config.read().await.symbols.clone();
    println!("✅ Configuration Loaded: Initial pairs: {:?}", initial_symbols);

    let (llm_client, key_present) = llm::LlmClient::from_env();
    let llm_client = Arc::new(RwLock::new(llm_client));
    let api_key_configured = Arc::new(AtomicBool::new(false));

    if key_present {
        println!("🔑 Validating DeepSeek API key...");
        let llm = llm_client.read().await;
        match llm.validate_key().await {
            Ok(()) => {
                println!("✅ Key validated successfully.");
                api_key_configured.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Err(e) => {
                eprintln!("⚠️  API Key Validation Failed: {}. You can configure it manually in the UI.", e);
            }
        }
    } else {
        eprintln!("⚠️  No API key found. AI analysis will fall back to local heuristics. Configure via the UI config panel.");
    }

    println!("🗄️  Initializing local SQLite telemetry database...");
    let db_pool = db::init_db().await;
    println!("✅ Database Setup: Connected to local telemetry.db file and verified schema.");

    db::check_encryption_warning(&db_pool).await;

    let (telemetry_tx, telemetry_rx) = channel::<db::TelemetryMsg>(10000);
    let logger_pool = db_pool.clone();
    let logger_llm = llm_client.clone();

    let logger_handle = tokio::spawn(async move {
        db::run_telemetry_logger(logger_pool, telemetry_rx, logger_llm).await;
    });

    let symbol_mapper = Arc::new(SymbolMapper::new());

    let hl_ws_url = app_config.read().await.hyperliquid.ws_url.clone();
    println!("📡 Hyperliquid WS endpoint: {}", hl_ws_url);

    let workspace = Arc::new(workspace::Workspace::new(
        app_config.clone(),
        db_pool.clone(),
        symbol_mapper.clone(),
        telemetry_tx.clone(),
        api_key_configured.clone(),
        hl_ws_url.clone(),
    ));

    let app_state = Arc::new(server::AppState {
        workspace: workspace.clone(),
        config: app_config.clone(),
        pool: db_pool.clone(),
        llm_client: llm_client.clone(),
        api_key_configured: api_key_configured.clone(),
        symbol_mapper: symbol_mapper.clone(),
        telemetry_tx: telemetry_tx.clone(),
        ws_url: hl_ws_url.clone(),
    });

    let app = server::build_router(app_state.clone());

    let mut handles = Vec::new();
    handles.push(logger_handle);

    if cli_mode {
        println!("🖥️  CLI Mode: Starting interactive console session...");
        println!("   Type 'help' for available commands, 'quit' to exit.");

        // Drop the web app — CLI handles all interaction
        drop(app);

        let cli_console = cli::CliConsole::new(
            workspace.clone(),
            db_pool.clone(),
            llm_client.clone(),
        );
        cli_console.run().await;

        println!("👋 CLI session ended. Shutting down...");
        return;
    }

    if web_mode {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
            .await
            .expect("❌ Web Server Setup: Failed to bind port 3000");

        println!("🌐 Web Server Setup: Visualizer Dashboard live at http://127.0.0.1:3000");

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("❌ Web Server Setup: Fatal crash running Axum HTTP server");
        });
        handles.push(server_handle);
    } else {
        println!("🖥️  CLI Mode: Running as headless daemon — Web server disabled (use --web to enable).");
        drop(app);
    }

    for item in &initial_symbols {
        let (_exchange, raw_symbol) = item.split_once(':').unwrap_or(("Hyperliquid", item));
        let base = raw_symbol.to_uppercase();
        let quote = "USDT".to_string();

        println!("🚀 Bootstrapping instance for {}-{}...", base, quote);

        match instance_registry::add_instance(
            &workspace,
            (base.clone(), quote),
            llm_client.clone(),
        ).await {
            Ok(instance) => {
                println!("✅ Instance bootstrapped: {} ({})", instance.pair_display(), instance.id);
            }
            Err(e) => {
                eprintln!("❌ Failed to bootstrap instance for {}: {}", raw_symbol, e);
            }
        }
    }

    let eval_cancel = CancellationToken::new();
    let eval_pool = db_pool.clone();
    let eval_cancel1 = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        performance_evaluator::run_performance_evaluator(
            performance_evaluator::EvaluatorConfig {
                pool: eval_pool,
                cancel: eval_cancel1,
                eval_interval_secs: 300,
            },
        ).await;
    }));

    handles.push(tokio::spawn(async move {
        strategy_optimizer::run_strategy_optimizer(
            strategy_optimizer::OptimizerConfig {
                pool: db_pool,
                cancel: eval_cancel,
                interval_secs: 3600,
            },
        ).await;
    }));

    let _ = futures_util::future::join_all(handles).await;
}
