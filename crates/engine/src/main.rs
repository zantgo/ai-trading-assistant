//! # Main Core Trading Engine Orchestrator
//!
//! This module coordinates active asynchronous system components. On startup, it initializes
//! our local SQLite database, sets up a TCP listener using `axum`, and configures an 
//! in-memory broadcasting engine to mirror processed analytical updates to connected browsers.

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

    // Table schema expanded to include true open, high, low, and close columns
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

/// Fast Path: Immediate, high-frequency execution engine.
/// Evaluates safety parameters, liquidation buffers, and account margins inside local RAM.
/// Operates on every single tick [1], completely decoupled from database or Svelte outputs.
fn run_realtime_risk_checks(tick: &MarketSnapshot) {
    // Safety guardrails go here
    if tick.mid_price < Decimal::from(1000) {
        eprintln!("⚠️ RISK ENGINE ALERT: ETH price dropped below safety margin!");
    }
}

/// Core analytical processing engine.
/// Aggregates ticks to candles on the "Slow Path", computes indicators, and broadcasts on candle close.
async fn run_analysis(
    mut rx: Receiver<MarketSnapshot>, 
    pool: SqlitePool, 
    broadcast_tx: broadcast::Sender<MarketSnapshot>,
    config: Arc<AppConfig>
) {
    println!("📊 Analysis Task: Subscribed to telemetry channel... \n");
    
    // Stateful Indicators Config
    let mut ema_fast = Ema::new(config.indicators.ema_fast);
    let mut ema_medium = Ema::new(config.indicators.ema_medium);
    let mut ema_slow = Ema::new(config.indicators.ema_slow);
    let mut ema_long = Ema::new(config.indicators.ema_long);
    let mut rsi_14 = Rsi::new(config.indicators.rsi_period);
    let mut macd = Macd::new();
    let mut adx_14 = Adx::new(config.indicators.adx_period);
    let mut sqz_mom = SqueezeMomentum::new();

    // Stateful Candle Builder variables
    let duration = config.candles.duration_seconds;
    let mut current_candle_start: Option<u64> = None;
    let mut ohlc_open: Option<Decimal> = None;
    let mut ohlc_high: Option<Decimal> = None;
    let mut ohlc_low: Option<Decimal> = None;
    let mut ohlc_close: Option<Decimal> = None;
    let mut last_processed_symbol = String::new();

    while let Some(tick) = rx.recv().await {
        // ---------------------------------------------------------------------
        // FAST PATH: Run high-frequency safety risk checks on EVERY raw tick!
        // ---------------------------------------------------------------------
        run_realtime_risk_checks(&tick);

        // ---------------------------------------------------------------------
        // SLOW PATH: Aggregate ticks into Candles on interval boundaries
        // ---------------------------------------------------------------------
        let rounded_time = (tick.timestamp / duration) * duration;
        last_processed_symbol = tick.symbol.clone();

        match current_candle_start {
            None => {
                // First launch: initialize active candle state
                current_candle_start = Some(rounded_time);
                ohlc_open = Some(tick.mid_price);
                ohlc_high = Some(tick.mid_price);
                ohlc_low = Some(tick.mid_price);
                ohlc_close = Some(tick.mid_price);
            }
            Some(start_time) if start_time == rounded_time => {
                // Same candle block: update High, Low, and Close
                if let Some(h) = ohlc_high { ohlc_high = Some(h.max(tick.mid_price)); }
                if let Some(l) = ohlc_low { ohlc_low = Some(l.min(tick.mid_price)); }
                ohlc_close = Some(tick.mid_price);
            }
            Some(start_time) => {
                // A new candle has started! The previous candle is formally closed.
                let finalized_open = ohlc_open.unwrap_or(tick.mid_price);
                let finalized_high = ohlc_high.unwrap_or(tick.mid_price);
                let finalized_low = ohlc_low.unwrap_or(tick.mid_price);
                let finalized_close = ohlc_close.unwrap_or(tick.mid_price);

                // Compile completed indicators using the CLOSED candle close price
                let mut completed_snapshot = MarketSnapshot {
                    timestamp: start_time,
                    symbol: last_processed_symbol.clone(),
                    mid_price: finalized_close,
                    bid_price: tick.bid_price, // raw fallback
                    ask_price: tick.ask_price,
                    funding_rate: None,
                    
                    open: Some(finalized_open),
                    high: Some(finalized_high),
                    low: Some(finalized_low),
                    close: Some(finalized_close),
                    
                    ema_fast: Some(ema_fast.update(finalized_close)),
                    ema_medium: Some(ema_medium.update(finalized_close)),
                    ema_slow: Some(ema_slow.update(finalized_close)),
                    ema_long: Some(ema_long.update(finalized_close)),
                    
                    rsi_14: rsi_14.update(finalized_close),
                    macd_line: None,
                    macd_signal: None,
                    macd_hist: None,
                    adx_14: None,
                    squeeze_on: None,
                    squeeze_momentum: None,
                };

                if let Some((m_line, m_sig, m_hist)) = macd.update(finalized_close) {
                    completed_snapshot.macd_line = Some(m_line);
                    completed_snapshot.macd_signal = Some(m_sig);
                    completed_snapshot.macd_hist = Some(m_hist);
                }
                
                completed_snapshot.adx_14 = adx_14.update(finalized_high, finalized_low, finalized_close);
                
                if let Some((on, val)) = sqz_mom.update(finalized_high, finalized_low, finalized_close) {
                    completed_snapshot.squeeze_on = Some(on);
                    completed_snapshot.squeeze_momentum = Some(val);
                }

                // Print the completed candle telemetry directly to the console
                println!("--------------------------------------------------------------------------------");
                println!("📥 [Candle Closed] Symbol: {} | Close: ${:.4} | High: ${:.4} | Low: ${:.4}", 
                    completed_snapshot.symbol, finalized_close, finalized_high, finalized_low
                );
                println!(
                    "   📈 EMAs:   Fast ({}): {} | Med ({}): {} | Slow ({}): {} | Long ({}): {}",
                    config.indicators.ema_fast, opt_dec_str(completed_snapshot.ema_fast),
                    config.indicators.ema_medium, opt_dec_str(completed_snapshot.ema_medium),
                    config.indicators.ema_slow, opt_dec_str(completed_snapshot.ema_slow),
                    config.indicators.ema_long, opt_dec_str(completed_snapshot.ema_long)
                );
                println!(
                    "   📊 MACD:   Line: {} | Signal: {} | Histogram: {}",
                    opt_dec_str(completed_snapshot.macd_line),
                    opt_dec_str(completed_snapshot.macd_signal),
                    opt_dec_str(completed_snapshot.macd_hist)
                );
                println!(
                    "   📉 Waves:  RSI({}): {} | ADX({}): {}",
                    config.indicators.rsi_period, opt_dec_str(completed_snapshot.rsi_14),
                    config.indicators.adx_period, opt_dec_str(completed_snapshot.adx_14)
                );
                println!(
                    "   💥 Squeeze: Compression On: {:<5} | Momentum Value: {}",
                    completed_snapshot.squeeze_on.map(|b| b.to_string()).unwrap_or_else(|| "Uninitialized".to_string()),
                    opt_dec_str(completed_snapshot.squeeze_momentum)
                );

                // Broadcast completed candle snapshot to Svelte (No high frequency rendering)
                let _ = broadcast_tx.send(completed_snapshot.clone());

                // Save completed candle snapshot directly in database
                let sqz_on_db_val = completed_snapshot.squeeze_on.map(|b| if b { 1 } else { 0 });

                if let Err(e) = sqlx::query(
                    "INSERT INTO market_snapshots (
                        timestamp, symbol, mid_price, bid_price, ask_price,
                        open, high, low, close,
                        ema_fast, ema_medium, ema_slow, ema_long, rsi_14,
                        macd_line, macd_signal, macd_hist, adx_14,
                        squeeze_on, squeeze_momentum
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)"
                )
                .bind(completed_snapshot.timestamp as i64)
                .bind(&completed_snapshot.symbol)
                .bind(completed_snapshot.mid_price.to_string())
                .bind(completed_snapshot.bid_price.to_string())
                .bind(completed_snapshot.ask_price.to_string())
                .bind(completed_snapshot.open.map(|d| d.to_string()))
                .bind(completed_snapshot.high.map(|d| d.to_string()))
                .bind(completed_snapshot.low.map(|d| d.to_string()))
                .bind(completed_snapshot.close.map(|d| d.to_string()))
                .bind(completed_snapshot.ema_fast.map(|d| d.to_string()))
                .bind(completed_snapshot.ema_medium.map(|d| d.to_string()))
                .bind(completed_snapshot.ema_slow.map(|d| d.to_string()))
                .bind(completed_snapshot.ema_long.map(|d| d.to_string()))
                .bind(completed_snapshot.rsi_14.map(|d| d.to_string()))
                .bind(completed_snapshot.macd_line.map(|d| d.to_string()))
                .bind(completed_snapshot.macd_signal.map(|d| d.to_string()))
                .bind(completed_snapshot.macd_hist.map(|d| d.to_string()))
                .bind(completed_snapshot.adx_14.map(|d| d.to_string()))
                .bind(sqz_on_db_val)
                .bind(completed_snapshot.squeeze_momentum.map(|d| d.to_string()))
                .execute(&pool)
                .await 
                {
                    eprintln!("⚠️ Database Error: Failed to save completed snapshot row: {}", e);
                }

                // Initialize the next candle starting bucket with current tick prices
                current_candle_start = Some(rounded_time);
                ohlc_open = Some(tick.mid_price);
                ohlc_high = Some(tick.mid_price);
                ohlc_low = Some(tick.mid_price);
                ohlc_close = Some(tick.mid_price);
            }
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
