use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use shared::normalized::{
    AssetContext, ConnectionStatus, Exchange, NormalizedEvent, NormalizedOrderBook,
    NormalizedTrade, TradeSide,
};
use std::str::FromStr;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Deserialize)]
struct BitgetArg {
    channel: String,
}

#[derive(Debug, Deserialize)]
struct TradeItem {
    ts: String,
    price: String,
    size: String,
    side: String,
}

#[derive(Debug, Deserialize)]
struct BookItem {
    asks: Vec<[String; 2]>,
    bids: Vec<[String; 2]>,
    ts: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TickerItem {
    #[serde(rename = "open24h")]
    open_24h: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FullBitgetMessage {
    #[allow(dead_code)]
    action: Option<String>,
    arg: Option<BitgetArg>,
    #[allow(dead_code)]
    event: Option<String>,
    data: Option<serde_json::Value>,
}

pub async fn run_for_symbol(
    symbol: String,
    internal_symbol: String,
    product_type: String,
    event_tx: Sender<NormalizedEvent>,
    cancel: CancellationToken,
    ws_url: &str,
) {
    let url = match url::Url::parse(ws_url) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("❌ Bitget: Invalid WS URL for {}: {}", symbol, e);
            return;
        }
    };

    let (ws_stream, _) = match tokio_tungstenite::connect_async(url.as_str()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Bitget: WS handshake failed for {}: {}", symbol, e);
            return;
        }
    };
    println!("✅ Bitget [{}]: TCP/WS Handshake completed.", symbol);
    let (mut write, mut read) = ws_stream.split();

    let _ = event_tx
        .send(NormalizedEvent::Status {
            exchange: Exchange::Bitget,
            status: ConnectionStatus::Connected,
            message: format!("Dedicated WS connected for {}", symbol),
        })
        .await;

    // Bitget V2 mix (perpetual futures) public channels: instType is the
    // productType (USDT-FUTURES / USDC-FUTURES); instId is the contract symbol
    // (e.g. BTCUSDT for USDT-M, BTCUSD for USDC-M).
    let sub_request = serde_json::json!({
        "op": "subscribe",
        "args": [
            {"instType": &product_type, "channel": "trade", "instId": &symbol},
            {"instType": &product_type, "channel": "books5", "instId": &symbol},
            {"instType": &product_type, "channel": "ticker", "instId": &symbol}
        ]
    });
    println!(
        "📡 Bitget [{}]: Subscribing to trade + books5 + ticker streams ({})",
        symbol, product_type
    );
    if let Err(e) = write
        .send(Message::Text(sub_request.to_string().into()))
        .await
    {
        eprintln!(
            "❌ Bitget [{}]: Failed to send subscription: {}",
            symbol, e
        );
        return;
    }

    // `symbol` is the raw contract symbol (e.g. "BTCUSDT" / "BTCUSD") used for
    // subscription. `internal_symbol` is the unified workspace symbol (e.g.
    // "BTC-USDT" / "BTC-USDC") emitted on every normalized event.

    loop {
        let msg = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                println!("🛑 Bitget [{}]: Cancellation triggered, closing WS connection.", symbol);
                break;
            }
            result = read.next() => {
                match result {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        eprintln!("⚠️ Bitget [{}]: Socket error: {}", symbol, e);
                        break;
                    }
                    None => {
                        println!("🔌 Bitget [{}]: Stream ended.", symbol);
                        break;
                    }
                }
            }
        };

        match msg {
            Message::Text(raw_text) => {
                let full_msg: FullBitgetMessage = match serde_json::from_str(&raw_text) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let channel = full_msg
                    .arg
                    .as_ref()
                    .map(|a| a.channel.as_str())
                    .unwrap_or("");
                let data_val = match full_msg.data {
                    Some(d) => d,
                    None => continue,
                };

                match channel {
                    "trade" => {
                        let trades: Vec<TradeItem> =
                            match serde_json::from_value(data_val) {
                                Ok(t) => t,
                                Err(_) => continue,
                            };
                        for t in trades {
                            let price = Decimal::from_str(&t.price).unwrap_or(Decimal::ZERO);
                            let size = Decimal::from_str(&t.size).unwrap_or(Decimal::ZERO);
                            let side = match t.side.as_str() {
                                "buy" => TradeSide::Buy,
                                _ => TradeSide::Sell,
                            };
                            let ts_ms: u64 =
                                t.ts.parse::<u64>().unwrap_or(
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as u64,
                                );

                            let event = NormalizedEvent::Trade(NormalizedTrade {
                                exchange: Exchange::Bitget,
                                symbol: internal_symbol.clone(),
                                price,
                                size,
                                side,
                                timestamp_ms: ts_ms,
                                trade_id: t.ts.clone(),
                            });
                            let _ = event_tx.send(event).await;
                        }
                    }
                    "books5" => {
                        let books: Vec<BookItem> =
                            match serde_json::from_value(data_val) {
                                Ok(b) => b,
                                Err(_) => continue,
                            };
                        for book in books {
                            let bids: Vec<(Decimal, Decimal)> = book
                                .bids
                                .iter()
                                .filter_map(|b| {
                                    let p = Decimal::from_str(&b[0]).ok()?;
                                    let s = Decimal::from_str(&b[1]).ok()?;
                                    Some((p, s))
                                })
                                .collect();
                            let asks: Vec<(Decimal, Decimal)> = book
                                .asks
                                .iter()
                                .filter_map(|a| {
                                    let p = Decimal::from_str(&a[0]).ok()?;
                                    let s = Decimal::from_str(&a[1]).ok()?;
                                    Some((p, s))
                                })
                                .collect();
                            let ts_ms: u64 =
                                book.ts.parse::<u64>().unwrap_or(
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as u64,
                                );

                            let event = NormalizedEvent::OrderBook(NormalizedOrderBook {
                                exchange: Exchange::Bitget,
                                symbol: internal_symbol.clone(),
                                bids,
                                asks,
                                timestamp_ms: ts_ms,
                            });
                            let _ = event_tx.send(event).await;
                        }
                    }
                    "ticker" => {
                        let tickers: Vec<TickerItem> = match serde_json::from_value(data_val) {
                            Ok(t) => t,
                            Err(_) => continue,
                        };
                        for tk in tickers {
                            if let Some(px) = tk
                                .open_24h
                                .as_deref()
                                .and_then(|s| Decimal::from_str(s).ok())
                            {
                                if px > Decimal::ZERO {
                                    let _ = event_tx
                                        .send(NormalizedEvent::AssetContext(AssetContext {
                                            symbol: internal_symbol.clone(),
                                            prev_day_px: px,
                                        }))
                                        .await;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Message::Ping(ping) => {
                let _ = write.send(Message::Pong(ping)).await;
            }
            Message::Close(_) => {
                println!("🔌 Bitget [{}]: Connection closed by server.", symbol);
                break;
            }
            _ => {}
        }
    }

    let _ = event_tx
        .send(NormalizedEvent::Status {
            exchange: Exchange::Bitget,
            status: ConnectionStatus::Disconnected,
            message: format!("Dedicated WS disconnected for {}", symbol),
        })
        .await;
}
