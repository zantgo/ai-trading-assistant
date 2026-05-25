//! # Main Core Trading Engine Orchestrator
//!
//! This module sets up the asynchronous Tokio execution environment. It uses
//! an in-memory Multi-Producer Single-Consumer (`mpsc`) message channel to establish 
//! a decoupled telemetry pipeline. The data flows sequentially:
//! 
//! [Ingestion Loop (Producer)] --(MarketSnapshot)--> [mpsc channel] --> [Analysis Loop (Consumer)]
//!                                                                           |-> Computes EMA
//!                                                                           |-> Prints telemetry

use std::time::Duration;
use rust_decimal::Decimal;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::interval;

use shared::models::MarketSnapshot;
use shared::indicators::Ema;

#[tokio::main]
async fn main() {
    println!("⚙️ DeX Trading Agent Engine: Starting Pipeline...");

    // 1. Instantiate the telemetry channel. Setting a buffer of 100 snapshots
    // to prevent memory surges if the consumer falls slightly behind.
    let (telemetry_tx, telemetry_rx) = channel::<MarketSnapshot>(100);

    // 2. Spawn the Mock Ingestion Task (Producer)
    let ingestion_handle = tokio::spawn(async move {
        run_ingestion(telemetry_tx).await;
    });

    // 3. Spawn the Analysis Task (Consumer)
    let analysis_handle = tokio::spawn(async move {
        run_analysis(telemetry_rx).await;
    });

    // 4. Drive execution concurrently. If either task panics or returns, stop the program.
    let _ = tokio::join!(ingestion_handle, analysis_handle);
}

/// Simulated market telemetry data stream generator.
/// Runs as a separate task, pushing parsed snapshots into the channel.
async fn run_ingestion(tx: Sender<MarketSnapshot>) {
    println!("📡 Ingestion Task: Starting live simulation feed...");
    let mut timer = interval(Duration::from_millis(500)); // Tick twice a second
    let mut tick_counter = 0u64;

    loop {
        timer.tick().await;
        tick_counter += 1;

        // Simulate predictable price action oscillating around $3000.00
        let base_price = Decimal::from(3000);
        let tick_offset = Decimal::from(tick_counter);
        
        // Simulates price volatility rising and falling
        let mid_price = if tick_counter % 10 < 5 {
            base_price + tick_offset
        } else {
            base_price - tick_offset
        };

        let spread = Decimal::new(5, 1); // Exact $0.50 spread representation
        let funding_rate = Decimal::new(1, 4); // Exact 0.0001 (0.01%) rate representation

        let snapshot = MarketSnapshot {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            symbol: "ETH".to_string(),
            mid_price,
            bid_price: mid_price - spread,
            ask_price: mid_price + spread,
            funding_rate: Some(funding_rate),
        };

        // Try sending snapshot down the pipeline
        if tx.send(snapshot).await.is_err() {
            eprintln!("⚠️ Ingestion Task: Receiver disconnected. Shutting down stream.");
            break;
        }
    }
}

/// Core processing engine.
/// Pulls raw snapshots from the channel and feeds them to technical indicators.
async fn run_analysis(mut rx: Receiver<MarketSnapshot>) {
    println!("📊 Analysis Task: Subscribed to telemetry channel...");
    
    // Initialize a 5-period Exponential Moving Average (EMA)
    let mut eth_ema = Ema::new(5);

    while let Some(snapshot) = rx.recv().await {
        // Feed the snapshot's mid-price into the indicator state
        let current_ema_value = eth_ema.update(snapshot.mid_price);

        println!(
            "📥 [Telemetry Received] Price: ${:<8} | EMA(5): ${:<8.4} | Asset: {}",
            snapshot.mid_price,
            current_ema_value,
            snapshot.symbol
        );
    }
    
    println!("📉 Analysis Task: Ingestion pipe closed. Exiting processing thread.");
}
