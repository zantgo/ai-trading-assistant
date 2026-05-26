//! # Main Core Trading Engine Orchestrator
//!
//! This module coordinates system components. On startup, it establishes the SQLite
//! database telemetry table, registers a process-wide cryptography provider, launches 
//! the live network feed, and handles analytical computing.

mod websocket;

use tokio::sync::mpsc::{channel, Receiver};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use rust_decimal::Decimal;

use shared::models::MarketSnapshot;
use shared::indicators::{Ema, Rsi, Macd, Adx, SqueezeMomentum};

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️ DeX Trading Agent Engine: Starting Live Feed Pipeline...");

    let db_options = SqliteConnectOptions::new()
        .filename("telemetry.db")
        .create_if_missing(true);

    let db_pool = SqlitePool::connect_with(db_options)
        .await
        .expect("❌ Database Setup: Failed to initialize SQLite database pool");

    // Create the expanded database table schema supporting all technical indicators
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
            squeeze_on INTEGER, -- 1 for True, 0 for False, NULL if uninitialized
            squeeze_momentum TEXT
        )"
    )
    .execute(&db_pool)
    .await
    .expect("❌ Database Setup: Failed to build schema table");

    println!("✅ Database Setup: Connected to local telemetry.db file and verified schema.");

    let (telemetry_tx, telemetry_rx) = channel::<MarketSnapshot>(100);

    let ws_handle = tokio::spawn(async move {
        websocket::run_hyperliquid_ws(telemetry_tx, "ETH").await;
    });

    let db_pool_clone = db_pool.clone();
    let analysis_handle = tokio::spawn(async move {
        run_analysis(telemetry_rx, db_pool_clone).await;
    });

    let _ = tokio::join!(ws_handle, analysis_handle);
}

/// Core analytical processing engine.
/// Computes indicators, formats console telemetry tables, and saves values to SQLite.
async fn run_analysis(mut rx: Receiver<MarketSnapshot>, pool: SqlitePool) {
    println!("📊 Analysis Task: Subscribed to telemetry channel... \n");
    
    // Initialize state containers for indicators
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
        // High/Low proxies based on immediate bid-ask levels
        let high = snapshot.ask_price;
        let low = snapshot.bid_price;

        // 1. Process and compute mathematical indicators
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

        // 2. Format and print the values cleanly to the console
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

        // 3. Map Squeeze state boolean to SQLite-compatible integer flag
        let sqz_on_db_val = snapshot.squeeze_on.map(|b| if b { 1 } else { 0 });

        // 4. Save indicator results securely to the database
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
