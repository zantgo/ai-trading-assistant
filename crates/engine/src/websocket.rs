//! # Hyperliquid Live WebSocket Client Module
//!
//! This module connects to the Hyperliquid Testnet WebSocket node, sends a JSON
//! subscription request for the Level 2 order book channel, and processes the
//! live incoming text frames. It translates the raw order book snapshots into our
//! internal `MarketSnapshot` model, transmitting them down the async channel.

use std::str::FromStr;
use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio::sync::watch;
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
pub async fn run_hyperliquid_ws(tx: Sender<MarketSnapshot>, mut symbol_rx: watch::Receiver<String>) {
    let url = match url::Url::parse(HYPERLIQUID_TESTNET_WS) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("❌ WebSocket Task: Invalid socket URL: {}", e);
            return;
        }
    };

    loop {
        let symbol = symbol_rx.borrow().clone();
        println!("🔌 WebSocket Task: Connecting to Hyperliquid Testnet ({}) for {}...", HYPERLIQUID_TESTNET_WS, symbol);

        let (ws_stream, _) = match connect_async(url.as_str()).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("❌ WebSocket Task: Handshake connection failed: {}", e);
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
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
            continue;
        }
        println!("📡 WebSocket Task: Subscribed to {} l2Book stream successfully.", symbol);

        loop {
            tokio::select! {
                result = read.next() => {
                    let msg = match result {
                        Some(Ok(m)) => m,
                        Some(Err(e)) => {
                            eprintln!("⚠️ WebSocket Task: Connection interrupted: {}", e);
                            break;
                        }
                        None => {
                            println!("🔌 WebSocket Task: Stream ended.");
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

                                            let best_bid_sz = match Decimal::from_str(&bids[0].sz) {
                                                Ok(val) => val,
                                                Err(_) => Decimal::ZERO,
                                            };
                                            let best_ask_sz = match Decimal::from_str(&asks[0].sz) {
                                                Ok(val) => val,
                                                Err(_) => Decimal::ZERO,
                                            };

                                            let mid_price = (best_bid + best_ask) / Decimal::from(2);

                                            let snapshot = MarketSnapshot {
                                                timestamp: payload.time / 1000,
                                                symbol: payload.coin.clone(),
                                                mid_price,
                                                bid_price: best_bid,
                                                ask_price: best_ask,
                                                bid_size: Some(best_bid_sz),
                                                ask_size: Some(best_ask_sz),
                                                funding_rate: None,
                                                
                                                open: None,
                                                high: None,
                                                low: None,
                                                close: None,
                                                volume: None,
                                                
                                                bb_upper: None,
                                                bb_middle: None,
                                                bb_lower: None,
                                                
                                                atr_14: None,
                                                vwap: None,
                                                adx_14: None,
                                                adx_plus: None,
                                                adx_minus: None,
                                                
                                                ema_fast: None,
                                                ema_medium: None,
                                                ema_slow: None,
                                                ema_long: None,
                                                
                                                rsi_14: None,
                                                macd_line: None,
                                                macd_signal: None,
                                                macd_hist: None,
                                                squeeze_on: None,
                                                squeeze_momentum: None,
                                            };

                                            if tx.send(snapshot).await.is_err() {
                                                eprintln!("⚠️ WebSocket Task: Consumer disconnected. Terminating ingestion stream.");
                                                return;
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
                result = symbol_rx.changed() => {
                    match result {
                        Ok(()) => println!("🔄 WebSocket Task: Symbol changed, reconnecting..."),
                        Err(_) => return,
                    }
                    break;
                }
            }
        }
    }
}
