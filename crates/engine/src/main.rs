//! # Main Core Trading Engine Orchestrator
//!
//! This module coordinates active asynchronous system components. On startup, it initializes
//! a local serverless SQLite database pool (`telemetry.db`), creates tables if they do not
//! exist, and passes the database pool into the consumer task for live logging.
//! 
//! Data flow topology:
//! 
//! [Hyperliquid Testnet (wss://)] ──> [websocket::run_hyperliquid_ws]
//!                                                │ (MarketSnapshot)
//!                                                ▼
//! [Database (telemetry.db)] <── [run_analysis Consumer] <── [mpsc channel]

mod websocket;

use tokio::sync::mpsc::{channel, Receiver};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

use shared::models::MarketSnapshot;
use shared::indicators::Ema;

#[tokio::main]
async fn main() {
    // 1. Install the process-wide default CryptoProvider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️ DeX Trading Agent Engine: Starting Live Feed Pipeline...");

    // 2. Initialize the SQLite Database Pool
    // This looks for a local "telemetry.db" file in your project root, creating it if missing.
    let db_options = SqliteConnectOptions::new()
        .filename("telemetry.db")
        .create_if_missing(true);

    let db_pool = SqlitePool::connect_with(db_options)
        .await
        .expect("❌ Database Setup: Failed to initialize SQLite database pool");

    // 3. Create Telemetry table if it does not exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS market_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            symbol TEXT NOT NULL,
            mid_price TEXT NOT NULL,
            bid_price TEXT NOT NULL,
            ask_price TEXT NOT NULL,
            ema_5 TEXT NOT NULL
        )"
    )
    .execute(&db_pool)
    .await
    .expect("❌ Database Setup: Failed to build schema table");

    println!("✅ Database Setup: Connected to local telemetry.db file and verified schema.");

    // 4. Instantiate the telemetry channel
    let (telemetry_tx, telemetry_rx) = channel::<MarketSnapshot>(100);

    // 5. Spawn the Live Hyperliquid WebSocket client
    let ws_handle = tokio::spawn(async move {
        websocket::run_hyperliquid_ws(telemetry_tx, "ETH").await;
    });

    // 6. Spawn the Analysis Task (Consumer) and pass the DB Pool clone down to it
    let db_pool_clone = db_pool.clone();
    let analysis_handle = tokio::spawn(async move {
        run_analysis(telemetry_rx, db_pool_clone).await;
    });

    // 7. Drive execution tasks concurrently
    let _ = tokio::join!(ws_handle, analysis_handle);
}

/// Core analytical processing engine.
/// Pulls raw snapshots, computes indicators, prints updates, and commits data to SQLite.
async fn run_analysis(mut rx: Receiver<MarketSnapshot>, pool: SqlitePool) {
    println!("📊 Analysis Task: Subscribed to telemetry channel... \n");
    
    // Initialize a 5-period Exponential Moving Average (EMA) state container
    let mut eth_ema = Ema::new(5);

    while let Some(snapshot) = rx.recv().await {
        // Feed the live mid-price from the WebSocket into the indicator state
        let current_ema_value = eth_ema.update(snapshot.mid_price);

        println!(
            "📥 [Live Ingest] Price: ${:<8} | EMA(5): ${:<8.4} | Asset: {}",
            snapshot.mid_price,
            current_ema_value,
            snapshot.symbol
        );

        // 8. Commit the snapshot and calculated EMA to the database.
        // We call `.to_string()` on decimal types to guarantee 100% mathematical precision in SQLite.
        if let Err(e) = sqlx::query(
            "INSERT INTO market_snapshots (timestamp, symbol, mid_price, bid_price, ask_price, ema_5) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )
        .bind(snapshot.timestamp as i64)
        .bind(&snapshot.symbol)
        .bind(snapshot.mid_price.to_string())
        .bind(snapshot.bid_price.to_string())
        .bind(snapshot.ask_price.to_string())
        .bind(current_ema_value.to_string())
        .execute(&pool)
        .await 
        {
            eprintln!("⚠️ Database Error: Failed to save snapshot row: {}", e);
        }
    }
    
    println!("📉 Analysis Task: Ingestion pipe closed. Exiting processing thread.");
}
