//! # Hyperliquid Live WebSocket Client Module
//!
//! This module connects to the Hyperliquid Testnet WebSocket node, sends a JSON
//! subscription request for the Level 2 order book channel, and processes the
//! live incoming text frames. It translates the raw order book snapshots into our
//! internal `MarketSnapshot` model, transmitting them down the async channel.

use std::str::FromStr;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use shared::models::MarketSnapshot;

/// The public WebSocket server address for the Hyperliquid Testnet environment
const HYPERLIQUID_TESTNET_WS: &str = "wss://api.hyperliquid-testnet.xyz/ws";

/// Local schema to deserialize the subscription payload structure.
/// Hyperliquid wraps messages in a simple `channel` and `data` envelope.
#[derive(Debug, Deserialize)]
struct L2BookEnvelope {
    #[allow(dead_code)] // Silences compiler warnings for deserialized wrapper fields
    channel: String,
    data: Option<L2BookPayload>,
}

/// The inner payload structure for L2 Book updates.
/// Maps to the actual API design where bids and asks are grouped in a nested levels array.
#[derive(Debug, Deserialize)]
struct L2BookPayload {
    coin: String,
    time: u64, // millisecond timestamp
    /// levels[0] is bids (buys), levels[1] is asks (sells)
    levels: Vec<Vec<BookLevel>>,
}

/// Individual price levels inside bids/asks arrays.
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Silences unused variable warnings during compile
struct BookLevel {
    /// Price as a string to prevent float precision rounding issues.
    px: String,
    /// Aggregate size as a string.
    sz: String,
    /// Number of active orders placed at this price level.
    n: u64,
}

/// Establish connection, register the subscription, and stream updates continuously.
pub async fn run_hyperliquid_ws(tx: Sender<MarketSnapshot>, symbol: &str) {
    println!("🔌 WebSocket Task: Connecting to Hyperliquid Testnet ({})...", HYPERLIQUID_TESTNET_WS);
    
    let url = match url::Url::parse(HYPERLIQUID_TESTNET_WS) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("❌ WebSocket Task: Invalid socket URL: {}", e);
            return;
        }
    };

    // 1. Handshake to upgrade HTTP/TCP connection into secure WebSocket
    let (ws_stream, _) = match connect_async(url.as_str()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ WebSocket Task: Handshake connection failed: {}", e);
            return;
        }
    };
    println!("✅ WebSocket Task: TCP/WS Handshake completed.");

    // Split stream to handle reading (incoming) and writing (outgoing) concurrently
    let (mut write, mut read) = ws_stream.split();

    // 2. Format and send subscription request
    let sub_request = serde_json::json!({
        "method": "subscribe",
        "subscription": {
            "type": "l2Book",
            "coin": symbol
        }
    });

    let sub_msg = Message::Text(sub_request.to_string().into());
    if let Err(e) = write.send(sub_msg).await {
        eprintln!("❌ WebSocket Task: Failed to dispatch subscription command: {}", e);
        return;
    }
    println!("📡 WebSocket Task: Subscribed to {} l2Book stream successfully.", symbol);

    // 3. Continuous ingestion event loop
    while let Some(message_result) = read.next().await {
        let msg = match message_result {
            Ok(m) => m,
            Err(e) => {
                eprintln!("⚠️ WebSocket Task: Connection interrupted: {}", e);
                break;
            }
        };

        match msg {
            Message::Text(raw_text) => {
                // To prevent logging unrelated messages (like subscription approvals),
                // only parse and log failures for envelopes containing order book data.
                if raw_text.contains("\"channel\":\"l2Book\"") {
                    match serde_json::from_str::<L2BookEnvelope>(&raw_text) {
                        Ok(envelope) => {
                            if let Some(payload) = envelope.data {
                                // Validate that we have both bid and ask channels populated
                                if payload.levels.len() < 2 || payload.levels[0].is_empty() || payload.levels[1].is_empty() {
                                    continue;
                                }

                                let bids = &payload.levels[0];
                                let asks = &payload.levels[1];

                                // Read best bid and best ask from the first level of the book (index 0)
                                let best_bid = match Decimal::from_str(&bids[0].px) {
                                    Ok(val) => val,
                                    Err(_) => continue,
                             };
                                let best_ask = match Decimal::from_str(&asks[0].px) {
                                    Ok(val) => val,
                                    Err(_) => continue,
                                };

                                // Compute midpoint average
                                let mid_price = (best_bid + best_ask) / Decimal::from(2);

                                // Map custom Hyperliquid structures directly to our standardized model
                                let snapshot = MarketSnapshot {
                                    timestamp: payload.time / 1000, // Convert ms to standard s epoch
                                    symbol: payload.coin.clone(),
                                    mid_price,
                                    bid_price: best_bid,
                                    ask_price: best_ask,
                                    funding_rate: None,
                                };

                                // Send snapshot to internal processing queue
                                if tx.send(snapshot).await.is_err() {
                                    eprintln!("⚠️ WebSocket Task: Consumer disconnected. Terminating ingestion stream.");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            // If it is an l2Book channel but fails to parse, log the format error explicitly
                            eprintln!("❌ WebSocket Task: Failed to parse l2Book JSON: {}. Raw data: {}", e, raw_text);
                        }
                    }
                }
            }
            Message::Ping(ping) => {
                // Return incoming server pings to maintain connection longevity
                let _ = write.send(Message::Pong(ping)).await;
            }
            Message::Close(_) => {
                println!("🔌 WebSocket Task: Connection cleanly closed by server.");
                break;
            }
            _ => {} // Ignore binary and irrelevant frames
        }
    }
}
