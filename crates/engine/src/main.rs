//! # Main Core Trading Engine Orchestrator
//!
//! This module coordinates active asynchronous system components. On startup, it initializes
//! our local SQLite database, sets up a TCP listener using `axum`, and serves our compiled
//! Svelte + TypeScript + Vite frontend statically.

mod websocket;

use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver};
use tokio::sync::broadcast;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use rust_decimal::Decimal;

// Web Server Libraries
use axum::{
    extract::{State, WebSocketUpgrade},
    extract::ws::{WebSocket, Message as AxumMessage},
    response::IntoResponse,
    routing::get,
    Router,
};
use tower_http::services::ServeDir;

use shared::models::MarketSnapshot;
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum};

/// Thread-safe sharing state accessible by incoming HTTP web routers
struct AppState {
    tx: broadcast::Sender<MarketSnapshot>,
}

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️ DeX Trading Agent Engine: Starting Live Feed Pipeline...");

    // 1. Initialize Database
    let db_options = SqliteConnectOptions::new()
        .filename("telemetry.db")
        .create_if_missing(true);

    let db_pool = SqlitePool::connect_with(db_options)
        .await
        .expect("❌ Database Setup: Failed to initialize SQLite database pool");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS market_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            symbol TEXT NOT NULL,
            mid_price TEXT NOT NULL,
            bid_price TEXT NOT NULL,
            ask_price TEXT NOT NULL,
            ema_10 TEXT,
            ema_50 TEXT,
            ema_100 TEXT,
            ema_200 TEXT,
            rsi_14 TEXT,
            macd_line TEXT,
            macd_signal TEXT,
            macd_hist TEXT,
            adx_14 TEXT,
            squeeze_on INTEGER,
            squeeze_momentum TEXT
        )"
    )
    .execute(&db_pool)
    .await
    .expect("❌ Database Setup: Failed to build schema table");

    println!("✅ Database Setup: Connected to local telemetry.db file and verified schema.");

    // 2. Setup Multi-Consumer Broadcast Channel
    let (broadcast_tx, _) = broadcast::channel::<MarketSnapshot>(100);
    let shared_state = Arc::new(AppState { tx: broadcast_tx.clone() });

    // 3. Configure HTTP Axum Router serving compiled Svelte-Vite dist assets
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new("crates/engine/frontend/dist")) // Serves our Svelte build output
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("❌ Web Server Setup: Failed to bind port 3000");
    
    println!("🌐 Web Server Setup: Visualizer Dashboard live at http://127.0.0.1:3000");

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("❌ Web Server Setup: Fatal crash running Axum HTTP server");
    });

    // 4. Instantiate local telemetry channels
    let (telemetry_tx, telemetry_rx) = channel::<MarketSnapshot>(100);

    // 5. Spawn Hyperliquid WebSocket Task
    let ws_handle = tokio::spawn(async move {
        websocket::run_hyperliquid_ws(telemetry_tx, "ETH").await;
    });

    // 6. Spawn Ingestion Analysis Task
    let db_pool_clone = db_pool.clone();
    let analysis_handle = tokio::spawn(async move {
        run_analysis(telemetry_rx, db_pool_clone, broadcast_tx).await;
    });

    // 7. Drive execution tasks concurrently
    let _ = tokio::join!(ws_handle, analysis_handle, server_handle);
}

/// Upgrade incoming HTTP connection to WebSocket protocol
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws_socket(socket, state))
}

/// Handle live WebSocket communication for browsers
async fn handle_ws_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    
    while let Ok(snapshot) = rx.recv().await {
        if let Ok(json_str) = serde_json::to_string(&snapshot) {
            if socket.send(AxumMessage::Text(json_str.into())).await.is_err() {
                break; // Break loop if browser tab closes
            }
        }
    }
}

/// Core analytical processing engine.
async fn run_analysis(mut rx: Receiver<MarketSnapshot>, pool: SqlitePool, broadcast_tx: broadcast::Sender<MarketSnapshot>) {
    println!("📊 Analysis Task: Subscribed to telemetry channel... \n");
    
    let mut ema_10 = Ema::new(10);
    let mut ema_50 = Ema::new(50);
    let mut ema_100 = Ema::new(100);
    let mut ema_200 = Ema::new(200);
    let mut rsi_14 = Rsi::new(14);
    let mut macd = Macd::new();
    let mut adx_14 = Adx::new(14);
    let mut sqz_mom = SqueezeMomentum::new();

    while let Some(mut snapshot) = rx.recv().await {
        let price = snapshot.mid_price;
        let high = snapshot.ask_price;
        let low = snapshot.bid_price;

        // Process Indicators
        snapshot.ema_10 = Some(ema_10.update(price));
        snapshot.ema_50 = Some(ema_50.update(price));
        snapshot.ema_100 = Some(ema_100.update(price));
        snapshot.ema_200 = Some(ema_200.update(price));
        
        snapshot.rsi_14 = rsi_14.update(price);
        
        if let Some((m_line, m_sig, m_hist)) = macd.update(price) {
            snapshot.macd_line = Some(m_line);
            snapshot.macd_signal = Some(m_sig);
            snapshot.macd_hist = Some(m_hist);
        }
        
        snapshot.adx_14 = adx_14.update(high, low, price);
        
        if let Some((on, val)) = sqz_mom.update(high, low, price) {
            snapshot.squeeze_on = Some(on);
            snapshot.squeeze_momentum = Some(val);
        }

        // Print outputs to terminal console
        println!("--------------------------------------------------------------------------------");
        println!("📥 [Tick Ingested] Symbol: {} | Mid Price: ${:.4}", snapshot.symbol, price);
        println!(
            "   📈 EMAs:   10: {} | 50: {} | 100: {} | 200: {}",
            opt_dec_str(snapshot.ema_10),
            opt_dec_str(snapshot.ema_50),
            opt_dec_str(snapshot.ema_100),
            opt_dec_str(snapshot.ema_200)
        );
        println!(
            "   📊 MACD:   Line: {} | Signal: {} | Histogram: {}",
            opt_dec_str(snapshot.macd_line),
            opt_dec_str(snapshot.macd_signal),
            opt_dec_str(snapshot.macd_hist)
        );
        println!(
            "   📉 Waves:  RSI(14): {} | ADX(14): {}",
            opt_dec_str(snapshot.rsi_14),
            opt_dec_str(snapshot.adx_14)
        );
        println!(
            "   💥 Squeeze: Compression On: {:<5} | Momentum Value: {}",
            snapshot.squeeze_on.map(|b| b.to_string()).unwrap_or_else(|| "Uninitialized".to_string()),
            opt_dec_str(snapshot.squeeze_momentum)
        );

        // Broadcast current compiled snapshot to all connected visualizer clients
        let _ = broadcast_tx.send(snapshot.clone());

        // Save telemetry results in database
        let sqz_on_db_val = snapshot.squeeze_on.map(|b| if b { 1 } else { 0 });

        if let Err(e) = sqlx::query(
            "INSERT INTO market_snapshots (
                timestamp, symbol, mid_price, bid_price, ask_price,
                ema_10, ema_50, ema_100, ema_200, rsi_14,
                macd_line, macd_signal, macd_hist, adx_14,
                squeeze_on, squeeze_momentum
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
        )
        .bind(snapshot.timestamp as i64)
        .bind(&snapshot.symbol)
        .bind(snapshot.mid_price.to_string())
        .bind(snapshot.bid_price.to_string())
        .bind(snapshot.ask_price.to_string())
        .bind(snapshot.ema_10.map(|d| d.to_string()))
        .bind(snapshot.ema_50.map(|d| d.to_string()))
        .bind(snapshot.ema_100.map(|d| d.to_string()))
        .bind(snapshot.ema_200.map(|d| d.to_string()))
        .bind(snapshot.rsi_14.map(|d| d.to_string()))
        .bind(snapshot.macd_line.map(|d| d.to_string()))
        .bind(snapshot.macd_signal.map(|d| d.to_string()))
        .bind(snapshot.macd_hist.map(|d| d.to_string()))
        .bind(snapshot.adx_14.map(|d| d.to_string()))
        .bind(sqz_on_db_val)
        .bind(snapshot.squeeze_momentum.map(|d| d.to_string()))
        .execute(&pool)
        .await 
        {
            eprintln!("⚠️ Database Error: Failed to save snapshot row: {}", e);
        }
    }
}

/// Helper function to format optional indicators gracefully on startup uninitialization.
fn opt_dec_str(val: Option<Decimal>) -> String {
    match val {
        Some(d) => format!("{:.4}", d),
        None => "Uninitialized".to_string(),
    }
}
