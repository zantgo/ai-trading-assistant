//! # Main Core Trading Engine Orchestrator
//!
//! This module coordinates active asynchronous system components. On startup, it parses the
//! workspace "config.toml" file, configures database tables, and spins up the web server routing.

mod websocket;

use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver};
use tokio::sync::broadcast;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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

// --- Configuration Struct Mappings ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandlesConfig {
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorsConfig {
    pub ema_fast: usize,
    pub ema_medium: usize,
    pub ema_slow: usize,
    pub ema_long: usize,
    pub rsi_period: usize,
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    pub adx_period: usize,
    pub squeeze_period: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub candles: CandlesConfig,
    pub indicators: IndicatorsConfig,
}

/// Thread-safe sharing state accessible by incoming HTTP web routers
struct AppState {
    tx: broadcast::Sender<MarketSnapshot>,
    config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️ DeX Trading Agent Engine: Loading Master Configuration...");

    // 1. Read and parse the Master config.toml file
    let config_raw = std::fs::read_to_string("config.toml")
        .expect("❌ Configuration Error: Failed to find \"config.toml\" in workspace root directory");
    
    let app_config: AppConfig = toml::from_str(&config_raw)
        .expect("❌ Configuration Error: Failed to parse fields inside config.toml");

    let shared_config = Arc::new(app_config);
    println!("✅ Configuration Loaded: System configured dynamically.");

    // 2. Initialize Database
    let db_options = SqliteConnectOptions::new()
        .filename("telemetry.db")
        .create_if_missing(true);

    let db_pool = SqlitePool::connect_with(db_options)
        .await
        .expect("❌ Database Setup: Failed to initialize SQLite database pool");

    // Dynamic schema creation containing generic EMA columns
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS market_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            symbol TEXT NOT NULL,
            mid_price TEXT NOT NULL,
            bid_price TEXT NOT NULL,
            ask_price TEXT NOT NULL,
            ema_fast TEXT,
            ema_medium TEXT,
            ema_slow TEXT,
            ema_long TEXT,
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

    // 3. Setup Multi-Consumer Broadcast Channel
    let (broadcast_tx, _) = broadcast::channel::<MarketSnapshot>(100);
    let shared_state = Arc::new(AppState { 
        tx: broadcast_tx.clone(),
        config: shared_config.clone()
    });

    // 4. Configure HTTP Axum Router
    let app = Router::new()
        .route("/api/config", get(serve_config)) // Rest API Endpoint to sync frontend
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new("crates/engine/frontend/dist"))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("❌ Web Server Setup: Failed to bind port 3000");
    
    println!("🌐 Web Server Setup: Visualizer Dashboard live at http://127.0.0.1:3000");

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("❌ Web Server Setup: Fatal crash running Axum HTTP server");
    });

    // 5. Instantiate local telemetry channels
    let (telemetry_tx, telemetry_rx) = channel::<MarketSnapshot>(100);

    // 6. Spawn Hyperliquid WebSocket Task
    let ws_handle = tokio::spawn(async move {
        websocket::run_hyperliquid_ws(telemetry_tx, "ETH").await;
    });

    // 7. Spawn Ingestion Analysis Task
    let db_pool_clone = db_pool.clone();
    let config_clone = shared_config.clone();
    let analysis_handle = tokio::spawn(async move {
        run_analysis(telemetry_rx, db_pool_clone, broadcast_tx, config_clone).await;
    });

    // 8. Drive execution tasks concurrently
    let _ = tokio::join!(ws_handle, analysis_handle, server_handle);
}

/// Serve active system configuration as JSON
async fn serve_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    axum::Json(state.config.clone())
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
                break;
            }
        }
    }
}

/// Core analytical processing engine.
async fn run_analysis(
    mut rx: Receiver<MarketSnapshot>, 
    pool: SqlitePool, 
    broadcast_tx: broadcast::Sender<MarketSnapshot>,
    config: Arc<AppConfig>
) {
    println!("📊 Analysis Task: Subscribed to telemetry channel... \n");
    
    // Dynamic indicator creation reading configuration lookback limits
    let mut ema_fast = Ema::new(config.indicators.ema_fast);
    let mut ema_medium = Ema::new(config.indicators.ema_medium);
    let mut ema_slow = Ema::new(config.indicators.ema_slow);
    let mut ema_long = Ema::new(config.indicators.ema_long);
    let mut rsi_14 = Rsi::new(config.indicators.rsi_period);
    
    let mut macd = Macd::new(); // MACD internal logic can also use config boundaries
    let mut adx_14 = Adx::new(config.indicators.adx_period);
    let mut sqz_mom = SqueezeMomentum::new(); // Squeeze Momentum can use configuration lookback limits

    while let Some(mut snapshot) = rx.recv().await {
        let price = snapshot.mid_price;
        let high = snapshot.ask_price;
        let low = snapshot.bid_price;

        // Process Indicators dynamically
        snapshot.ema_fast = Some(ema_fast.update(price));
        snapshot.ema_medium = Some(ema_medium.update(price));
        snapshot.ema_slow = Some(ema_slow.update(price));
        snapshot.ema_long = Some(ema_long.update(price));
        
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
            "   📈 EMAs:   Fast ({}): {} | Med ({}): {} | Slow ({}): {} | Long ({}): {}",
            config.indicators.ema_fast, opt_dec_str(snapshot.ema_fast),
            config.indicators.ema_medium, opt_dec_str(snapshot.ema_medium),
            config.indicators.ema_slow, opt_dec_str(snapshot.ema_slow),
            config.indicators.ema_long, opt_dec_str(snapshot.ema_long)
        );
        println!(
            "   📊 MACD:   Line: {} | Signal: {} | Histogram: {}",
            opt_dec_str(snapshot.macd_line),
            opt_dec_str(snapshot.macd_signal),
            opt_dec_str(snapshot.macd_hist)
        );
        println!(
            "   📉 Waves:  RSI({}): {} | ADX({}): {}",
            config.indicators.rsi_period, opt_dec_str(snapshot.rsi_14),
            config.indicators.adx_period, opt_dec_str(snapshot.adx_14)
        );
        println!(
            "   💥 Squeeze: Compression On: {:<5} | Momentum Value: {}",
            snapshot.squeeze_on.map(|b| b.to_string()).unwrap_or_else(|| "Uninitialized".to_string()),
            opt_dec_str(snapshot.squeeze_momentum)
        );

        // Broadcast current compiled snapshot to all connected visualizer clients
        let _ = broadcast_tx.send(snapshot.clone());

        // Save generic-named telemetry results in database
        let sqz_on_db_val = snapshot.squeeze_on.map(|b| if b { 1 } else { 0 });

        if let Err(e) = sqlx::query(
            "INSERT INTO market_snapshots (
                timestamp, symbol, mid_price, bid_price, ask_price,
                ema_fast, ema_medium, ema_slow, ema_long, rsi_14,
                macd_line, macd_signal, macd_hist, adx_14,
                squeeze_on, squeeze_momentum
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
        )
        .bind(snapshot.timestamp as i64)
        .bind(&snapshot.symbol)
        .bind(snapshot.mid_price.to_string())
        .bind(snapshot.bid_price.to_string())
        .bind(snapshot.ask_price.to_string())
        .bind(snapshot.ema_fast.map(|d| d.to_string()))
        .bind(snapshot.ema_medium.map(|d| d.to_string()))
        .bind(snapshot.ema_slow.map(|d| d.to_string()))
        .bind(snapshot.ema_long.map(|d| d.to_string()))
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
