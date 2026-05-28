mod websocket;
mod config;
mod db;
mod server;
mod risk;
mod candle_builder;
mod analyzer;
mod llm;

use std::sync::Arc;
use std::collections::VecDeque;
use std::process;
use tokio::sync::mpsc::channel;
use tokio::sync::{broadcast, watch, RwLock};
use rust_decimal::Decimal;
use shared::models::MarketSnapshot;

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    if let Err(e) = dotenvy::dotenv() {
        eprintln!("❌ Failed to load .env file: {}", e);
        eprintln!("   Create a .env file at the project root with: DEEPSEEK_API_KEY=sk-...");
        eprintln!("   See .env.example for a template.");
        process::exit(1);
    }

    println!("⚙️ DeX AI Trading Assistant: Loading Master Configuration...");
    let app_config = Arc::new(config::load_config());
    println!("✅ Configuration Loaded: System configured dynamically.");

    let llm_client = match llm::LlmClient::from_dotenv() {
        Ok(client) => client,
        Err(e) => {
            eprintln!("❌ LLM Setup Error: {}", e);
            process::exit(1);
        }
    };

    println!("🔑 Validating DeepSeek API key...");
    match llm_client.validate_key().await {
        Ok(()) => println!("✅ Key validated successfully."),
        Err(e) => {
            eprintln!("❌ API Key Validation Failed: {}", e);
            process::exit(1);
        }
    }

    println!("🗄️  Initializing local SQLite telemetry database...");
    let db_pool = db::init_db().await;
    println!("✅ Database Setup: Connected to local telemetry.db file and verified schema.");

    let history_buffer: Arc<RwLock<VecDeque<Decimal>>> =
        Arc::new(RwLock::new(VecDeque::with_capacity(100)));

    let (symbol_tx, symbol_rx) = watch::channel(app_config.symbol.clone());

    let (broadcast_tx, _) = broadcast::channel::<MarketSnapshot>(100);
    let app_state = Arc::new(server::AppState {
        tx: broadcast_tx.clone(),
        config: Arc::new(RwLock::new((*app_config).clone())),
        history: history_buffer.clone(),
        pool: db_pool.clone(),
        llm_client,
        symbol_tx,
    });

    let analyzer_config = app_state.config.clone();

    let app = server::build_router(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("❌ Web Server Setup: Failed to bind port 3000");

    println!("🌐 Web Server Setup: Visualizer Dashboard live at http://127.0.0.1:3000");

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("❌ Web Server Setup: Fatal crash running Axum HTTP server");
    });

    let (telemetry_tx, telemetry_rx) = channel::<MarketSnapshot>(100);

    let ws_handle = tokio::spawn(async move {
        websocket::run_hyperliquid_ws(telemetry_tx, symbol_rx).await;
    });

    let analysis_handle = tokio::spawn(async move {
        analyzer::run(telemetry_rx, db_pool, broadcast_tx, analyzer_config, history_buffer).await;
    });

    let _ = tokio::join!(ws_handle, analysis_handle, server_handle);
}
