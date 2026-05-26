//! # Main Core Trading Engine Orchestrator
//!
//! This module coordinates active asynchronous system components. On startup, it parses the
//! master "config.toml" file, configures SQLite database tables, and spins up the web server routing.

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
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum, BollingerBands};

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

    // Dynamic schema creation containing generic EMA, volume, and Bollinger Bands columns
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS market_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            symbol TEXT NOT NULL,
            mid_price TEXT NOT NULL,
            bid_price TEXT NOT NULL,
            ask_price TEXT NOT NULL,
            open TEXT,
            high TEXT,
            low TEXT,
            close TEXT,
            volume TEXT,
            bb_upper TEXT,
            bb_middle TEXT,
            bb_lower TEXT,
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

    // 4. Configure HTTP Axum Router serving compiled Svelte-Vite dist assets
    let app = Router::new()
        .route("/api/config", get(serve_config))
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

/// High-Frequency internal risk engine.
fn run_realtime_risk_checks(tick: &MarketSnapshot) {
    let _current_price = tick.mid_price;
    if _current_price < Decimal::from(1000) {
        eprintln!("⚠️ RISK ENGINE ALERT: ETH price dropped below safety margin!");
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
    
    // Dynamic indicator state configurations
    let mut ema_fast = Ema::new(config.indicators.ema_fast);
    let mut ema_medium = Ema::new(config.indicators.ema_medium);
    let mut ema_slow = Ema::new(config.indicators.ema_slow);
    let mut ema_long = Ema::new(config.indicators.ema_long);
    let mut rsi_14 = Rsi::new(config.indicators.rsi_period);
    
    let mut macd = Macd::new();
    let mut adx_14 = Adx::new(config.indicators.adx_period);
    let mut sqz_mom = SqueezeMomentum::new();
    let mut bollinger = BollingerBands::new(); // Added

    // Active candlestick aggregation state
    let mut current_candle_time: Option<u64> = None;
    let mut o = Decimal::ZERO;
    let mut h = Decimal::ZERO;
    let mut l = Decimal::ZERO;
    let mut c = Decimal::ZERO;
    let mut accumulated_vol = Decimal::ZERO; // Cumulative candle volume tracker
    let mut last_processed_symbol = String::new();

    while let Some(tick) = rx.recv().await {
        // ----------------------------------------------------------------------
        // 1. FAST PATH: Real-Time Risk Engine (Instant, high-frequency checks)
        // ----------------------------------------------------------------------
        run_realtime_risk_checks(&tick);

        // ----------------------------------------------------------------------
        // 2. SLOW PATH: Stateful Candlestick Builder
        // ----------------------------------------------------------------------
        let rounded_time = (tick.timestamp / config.candles.duration_seconds) * config.candles.duration_seconds;
        last_processed_symbol = tick.symbol.clone();

        // Calculate tick's instant top-of-book depth size as volume proxy
        let tick_vol = tick.bid_size.unwrap_or(Decimal::ZERO) + tick.ask_size.unwrap_or(Decimal::ZERO);

        match current_candle_time {
            None => {
                // Initialize the very first candle
                current_candle_time = Some(rounded_time);
                o = tick.mid_price;
                h = tick.mid_price;
                l = tick.mid_price;
                c = tick.mid_price;
                accumulated_vol = tick_vol;
            }
            Some(curr_time) => {
                if rounded_time > curr_time {
                    // ----------------------------------------------------------
                    // A. CANDLE BOUNDARY TRIGGER: Previous Candle Has Closed!
                    // ----------------------------------------------------------
                    
                    // Commit indicators
                    let final_ema_fast = ema_fast.update(c);
                    let final_ema_medium = ema_medium.update(c);
                    let final_ema_slow = ema_slow.update(c);
                    let final_ema_long = ema_long.update(c);
                    let final_rsi = rsi_14.update(c);
                    let final_macd = macd.update(c);
                    let final_adx = adx_14.update(h, l, c);
                    let final_sqz = sqz_mom.update(h, l, c);
                    let final_bb = bollinger.update(c); // Bollinger Bands update on close

                    // Print output log of completed candle
                    println!("--------------------------------------------------------------------------------");
                    println!("🕯️  [Candle Closed] Timestamp: {} | Close: ${:.4} | Vol: {:.4}", curr_time, c, accumulated_vol);
                    println!(
                        "   📈 EMAs:   Fast ({}): {} | Med ({}): {} | Slow ({}): {} | Long ({}): {}",
                        config.indicators.ema_fast, opt_dec_str(Some(final_ema_fast)),
                        config.indicators.ema_medium, opt_dec_str(Some(final_ema_medium)),
                        config.indicators.ema_slow, opt_dec_str(Some(final_ema_slow)),
                        config.indicators.ema_long, opt_dec_str(Some(final_ema_long))
                    );
                    if let Some(bb) = final_bb {
                        println!("   🧱 BBands: Upper: {:.4} | Middle: {:.4} | Lower: {:.4}", bb.0, bb.1, bb.2);
                    }

                    // Commit the completed candle snapshot to SQLite database
                    let sqz_on_db_val = final_sqz.map(|s| if s.0 { 1 } else { 0 });
                    if let Err(e) = sqlx::query(
                        "INSERT INTO market_snapshots (
                            timestamp, symbol, mid_price, bid_price, ask_price,
                            open, high, low, close, volume,
                            bb_upper, bb_middle, bb_lower,
                            ema_fast, ema_medium, ema_slow, ema_long, rsi_14,
                            macd_line, macd_signal, macd_hist, adx_14,
                            squeeze_on, squeeze_momentum
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)"
                    )
                    .bind(curr_time as i64)
                    .bind(&tick.symbol)
                    .bind(c.to_string())
                    .bind(tick.bid_price.to_string())
                    .bind(tick.ask_price.to_string())
                    .bind(Some(o.to_string()))
                    .bind(Some(h.to_string()))
                    .bind(Some(l.to_string()))
                    .bind(Some(c.to_string()))
                    .bind(Some(accumulated_vol.to_string()))
                    .bind(final_bb.map(|b| b.0.to_string()))
                    .bind(final_bb.map(|b| b.1.to_string()))
                    .bind(final_bb.map(|b| b.2.to_string()))
                    .bind(Some(final_ema_fast.to_string()))
                    .bind(Some(final_ema_medium.to_string()))
                    .bind(Some(final_ema_slow.to_string()))
                    .bind(Some(final_ema_long.to_string()))
                    .bind(final_rsi.map(|d| d.to_string()))
                    .bind(final_macd.map(|m| m.0.to_string()))
                    .bind(final_macd.map(|m| m.1.to_string()))
                    .bind(final_macd.map(|m| m.2.to_string()))
                    .bind(final_adx.map(|d| d.to_string()))
                    .bind(sqz_on_db_val)
                    .bind(final_sqz.map(|s| s.1.to_string()))
                    .execute(&pool)
                    .await 
                    {
                        eprintln!("⚠️ Database Error: Failed to save completed snapshot: {}", e);
                    }

                    // Reset and start the new candle with current tick inputs
                    current_candle_time = Some(rounded_time);
                    o = tick.mid_price;
                    h = tick.mid_price;
                    l = tick.mid_price;
                    c = tick.mid_price;
                    accumulated_vol = tick_vol;
                } else {
                    // Same candle block: update High, Low, Close, and accumulate volume
                    h = h.max(tick.mid_price);
                    l = l.min(tick.mid_price);
                    c = tick.mid_price;
                    accumulated_vol += tick_vol;
                }
            }
        }

        // ----------------------------------------------------------------------
        // 3. REAL-TIME CANDLE FLICKERING BROADCAST
        // ----------------------------------------------------------------------
        // Clones local indicators and calculates "shadow" outputs for real-time
        // visual updates. Timestamps are locked to stop timeline scrolling.

        let mut temp_ema_fast = ema_fast.clone();
        let mut temp_ema_medium = ema_medium.clone();
        let mut temp_ema_slow = ema_slow.clone();
        let mut temp_ema_long = ema_long.clone();
        let mut temp_rsi_14 = rsi_14.clone();
        let mut temp_macd = macd.clone();
        let mut temp_adx_14 = adx_14.clone();
        let mut temp_sqz_mom = sqz_mom.clone();
        let mut temp_bollinger = bollinger.clone();

        let val_ema_fast = temp_ema_fast.update(c);
        let val_ema_medium = temp_ema_medium.update(c);
        let val_ema_slow = temp_ema_slow.update(c);
        let val_ema_long = temp_ema_long.update(c);
        let val_rsi = temp_rsi_14.update(c);
        let val_macd = temp_macd.update(c);
        let val_adx = temp_adx_14.update(h, l, c);
        let val_sqz = temp_sqz_mom.update(h, l, c);
        let val_bb = temp_bollinger.update(c);

        let broadcast_snapshot = MarketSnapshot {
            timestamp: rounded_time, // LOCKED timestamp
            symbol: tick.symbol.clone(),
            mid_price: c,
            bid_price: tick.bid_price,
            ask_price: tick.ask_price,
            bid_size: tick.bid_size,
            ask_size: tick.ask_size,
            funding_rate: tick.funding_rate,
            
            // Populated candle aggregates for live Svelte plotting
            open: Some(o),
            high: Some(h),
            low: Some(l),
            close: Some(c),
            volume: Some(accumulated_vol),
            
            // Populated Bollinger Bands
            bb_upper: val_bb.map(|b| b.0),
            bb_middle: val_bb.map(|b| b.1),
            bb_lower: val_bb.map(|b| b.2),
            
            ema_fast: Some(val_ema_fast),
            ema_medium: Some(val_ema_medium),
            ema_slow: Some(val_ema_slow),
            ema_long: Some(val_ema_long),
            
            rsi_14: val_rsi,
            macd_line: val_macd.map(|m| m.0),
            macd_signal: val_macd.map(|m| m.1),
            macd_hist: val_macd.map(|m| m.2),
            adx_14: val_adx,
            squeeze_on: val_sqz.map(|s| s.0),
            squeeze_momentum: val_sqz.map(|s| s.1),
        };

        let _ = broadcast_tx.send(broadcast_snapshot);
    }
}

/// Helper function to format optional indicators gracefully on startup uninitialization.
fn opt_dec_str(val: Option<Decimal>) -> String {
    match val {
        Some(d) => format!("{:.4}", d),
        None => "Uninitialized".to_string(),
    }
}
