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
#[derive(Debug, Deserialize)]
struct L2BookEnvelope {
    #[allow(dead_code)]
    channel: String,
    data: Option<L2BookPayload>,
}

/// The inner payload structure for L2 Book updates.
#[derive(Debug, Deserialize)]
struct L2BookPayload {
    coin: String,
    time: u64,
    levels: Vec<Vec<BookLevel>>,
}

/// Individual price levels inside bids/asks arrays.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BookLevel {
    px: String,
    sz: String,
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

    let (ws_stream, _) = match connect_async(url.as_str()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ WebSocket Task: Handshake connection failed: {}", e);
            return;
        }
    };
    println!("✅ WebSocket Task: TCP/WS Handshake completed.");

    let (mut write, mut read) = ws_stream.split();

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
                if raw_text.contains("\"channel\":\"l2Book\"") {
                    match serde_json::from_str::<L2BookEnvelope>(&raw_text) {
                        Ok(envelope) => {
                            if let Some(payload) = envelope.data {
                                if payload.levels.len() < 2 || payload.levels[0].is_empty() || payload.levels[1].is_empty() {
                                    continue;
                                }

                                let bids = &payload.levels[0];
                                let asks = &payload.levels[1];

                                let best_bid = match Decimal::from_str(&bids[0].px) {
                                    Ok(val) => val,
                                    Err(_) => continue,
                                };
                                let best_ask = match Decimal::from_str(&asks[0].px) {
                                    Ok(val) => val,
                                    Err(_) => continue,
                                };

                                let mid_price = (best_bid + best_ask) / Decimal::from(2);

                                let snapshot = MarketSnapshot {
                                    timestamp: payload.time / 1000,
                                    symbol: payload.coin.clone(),
                                    mid_price,
                                    bid_price: best_bid,
                                    ask_price: best_ask,
                                    funding_rate: None,
                                    
                                    // Set dynamic open, high, low, close to None on raw ingestion
                                    open: None,
                                    high: None,
                                    low: None,
                                    close: None,
                                    
                                    // Set dynamic indicator fields to None on raw socket ingestion
                                    ema_fast: None,
                                    ema_medium: None,
                                    ema_slow: None,
                                    ema_long: None,
                                    
                                    rsi_14: None,
                                    macd_line: None,
                                    macd_signal: None,
                                    macd_hist: None,
                                    adx_14: None,
                                    squeeze_on: None,
                                    squeeze_momentum: None,
                                };

                                if tx.send(snapshot).await.is_err() {
                                    eprintln!("⚠️ WebSocket Task: Consumer disconnected. Terminating ingestion stream.");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ WebSocket Task: Failed to parse l2Book JSON: {}. Raw data: {}", e, raw_text);
                        }
                    }
                }
            }
            Message::Ping(ping) => {
                let _ = write.send(Message::Pong(ping)).await;
            }
            Message::Close(_) => {
                println!("🔌 WebSocket Task: Connection cleanly closed by server.");
                break;
            }
            _ => {}
        }
    }
}
