//! # Main Core Trading Engine Orchestrator
//!
//! This module coordinates active asynchronous system components. Before establishing secure
//! TLS connections, it installs the default cryptographic provider to prevent provider ambiguity.
//! 
//! Data flow topology:
//! 
//! [Hyperliquid Testnet API (wss://)] 
//!                 │ 
//!                 ▼ (Incoming raw JSON Frame)
//!     [Ingestion Task (websocket::run_hyperliquid_ws)] 
//!                 │ 
//!                 ▼ (Deserialized into unified MarketSnapshot structure)
//!         [mpsc channel]
//!                 │ 
//!                 ▼ (Asynchronously read by consumer task)
//!     [Analysis Task (run_analysis)] 
//!                 │-> Computes 5-period Exponential Moving Average (EMA)
//!                 └─> Logs telemetry outputs to terminal console

mod websocket;

use tokio::sync::mpsc::{channel, Receiver};
use shared::models::MarketSnapshot;
use shared::indicators::Ema;

#[tokio::main]
async fn main() {
    // 1. Install the process-wide default CryptoProvider for rustls.
    // This resolves the runtime panic caused by dependency graph conflicts.
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("⚙️ DeX Trading Agent Engine: Starting Live Feed Pipeline...");

    // 2. Instantiate the telemetry channel. Setting a buffer limit of 100 snapshots
    // to prevent memory overhead if downstream components experience latency.
    let (telemetry_tx, telemetry_rx) = channel::<MarketSnapshot>(100);

    // 3. Spawn the Live Hyperliquid WebSocket client for Ethereum (ETH)
    let ws_handle = tokio::spawn(async move {
        websocket::run_hyperliquid_ws(telemetry_tx, "ETH").await;
    });

    // 4. Spawn the Analysis Task (Consumer)
    let analysis_handle = tokio::spawn(async move {
        run_analysis(telemetry_rx).await;
    });

    // 5. Drive execution concurrently.
    let _ = tokio::join!(ws_handle, analysis_handle);
}

/// Core analytical processing engine.
/// Pulls raw snapshots from the channel and feeds them to mathematical indicator algorithms.
async fn run_analysis(mut rx: Receiver<MarketSnapshot>) {
    println!("📊 Analysis Task: Subscribed to telemetry channel... \n");
    
    // Initialize a 5-period Exponential Moving Average (EMA) state container
    let mut eth_ema = Ema::new(5);

    while let Some(snapshot) = rx.recv().await {
        // Feed the live mid-price from the WebSocket into the indicator state
        let current_ema_value = eth_ema.update(snapshot.mid_price);

        println!(
            "📥 [Live HL Ingest] Price: ${:<8} | EMA(5): ${:<8.4} | Asset: {}",
            snapshot.mid_price,
            current_ema_value,
            snapshot.symbol
        );
    }
    
    println!("📉 Analysis Task: Ingestion pipe closed. Exiting processing thread.");
}
