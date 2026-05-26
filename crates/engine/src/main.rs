mod websocket;
mod config;
mod db;
mod server;
mod risk;
mod candle_builder;
mod analyzer;

use std::sync::Arc;
use tokio::sync::mpsc::channel;
use tokio::sync::broadcast;
use shared::models::MarketSnapshot;

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️ DeX Trading Agent Engine: Loading Master Configuration...");
    let app_config = Arc::new(config::load_config());
    println!("✅ Configuration Loaded: System configured dynamically.");

    println!("🗄️  Initializing local SQLite telemetry database...");
    let db_pool = db::init_db().await;
    println!("✅ Database Setup: Connected to local telemetry.db file and verified schema.");

    let (broadcast_tx, _) = broadcast::channel::<MarketSnapshot>(100);
    let app_state = Arc::new(server::AppState {
        tx: broadcast_tx.clone(),
        config: app_config.clone(),
    });

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
        websocket::run_hyperliquid_ws(telemetry_tx, "ETH").await;
    });

    let analysis_handle = tokio::spawn(async move {
        analyzer::run(telemetry_rx, db_pool, broadcast_tx, app_config).await;
    });

    let _ = tokio::join!(ws_handle, analysis_handle, server_handle);
}
