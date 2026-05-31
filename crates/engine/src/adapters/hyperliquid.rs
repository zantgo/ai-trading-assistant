use std::str::FromStr;
use std::sync::Arc;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_util::sync::CancellationToken;
use shared::normalized::{
    Exchange, ExchangeAdapter, NormalizedEvent, NormalizedTrade, NormalizedOrderBook,
    SymbolMapper, TradeSide, ConnectionStatus,
};

pub struct HyperliquidAdapter {
    pub ws_url: String,
}

impl HyperliquidAdapter {
    pub fn new(ws_url: String) -> Self {
        Self { ws_url }
    }
}

#[derive(Debug, Deserialize)]
struct L2BookEnvelope {
    #[allow(dead_code)]
    channel: String,
    data: Option<L2BookPayload>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct L2BookPayload {
    coin: String,
    time: u64,
    levels: Vec<Vec<BookLevel>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BookLevel {
    px: String,
    sz: String,
    n: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TradesEnvelope {
    channel: String,
    data: Option<Vec<TradePayload>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TradePayload {
    coin: String,
    side: String,
    px: String,
    sz: String,
    hash: String,
    tid: u64,
    time: u64,
}

fn to_internal_symbol(raw: &str) -> String {
    format!("{}-USD", raw)
}

#[allow(dead_code)]
#[async_trait]
impl ExchangeAdapter for HyperliquidAdapter {
    fn exchange(&self) -> Exchange {
        Exchange::Hyperliquid
    }

    async fn start(
        &self,
        symbols: Vec<String>,
        event_tx: Sender<NormalizedEvent>,
        mapper: Arc<SymbolMapper>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = url::Url::parse(&self.ws_url)?;

        let (ws_stream, _) = connect_async(url.as_str()).await?;
        println!("✅ Hyperliquid Adapter: TCP/WS Handshake completed.");
        let (mut write, mut read) = ws_stream.split();

        let _ = event_tx.send(NormalizedEvent::Status {
            exchange: Exchange::Hyperliquid,
            status: ConnectionStatus::Connected,
            message: "Testnet WS connection established.".to_string(),
        }).await;

        let mut subscriptions = Vec::new();
        for sym in &symbols {
            if let Some(raw_sym) = mapper.get_raw(Exchange::Hyperliquid, sym).await {
                subscriptions.push(serde_json::json!({"type": "trades", "coin": raw_sym}));
                subscriptions.push(serde_json::json!({"type": "l2Book", "coin": raw_sym}));
                println!("📡 Hyperliquid Adapter: Subscribed to trades + l2Book for {} ({})", sym, raw_sym);
            }
        }

        if subscriptions.is_empty() {
            return Err("No valid symbols mapped for Hyperliquid".into());
        }

        for sub in subscriptions {
            let sub_request = serde_json::json!({
                "method": "subscribe",
                "subscription": sub
            });
            println!("📡 Hyperliquid Adapter: Subscribing to stream: {}", sub_request);
            write.send(Message::Text(sub_request.to_string().into())).await?;
        }

        while let Some(msg) = read.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("⚠️ Hyperliquid Adapter: Socket error: {}", e);
                    break;
                }
            };

            match msg {
                Message::Text(raw_text) => {
                    if raw_text.contains("\"channel\":\"l2Book\"") {
                        if let Ok(envelope) = serde_json::from_str::<L2BookEnvelope>(&raw_text) {
                            if let Some(payload) = envelope.data {
                                if payload.levels.len() >= 2
                                    && !payload.levels[0].is_empty()
                                    && !payload.levels[1].is_empty()
                                {
                                    let symbol = to_internal_symbol(&payload.coin);
                                    let bids: Vec<(Decimal, Decimal)> = payload.levels[0]
                                        .iter()
                                        .filter_map(|l| {
                                            let p = Decimal::from_str(&l.px).ok()?;
                                            let s = Decimal::from_str(&l.sz).ok()?;
                                            Some((p, s))
                                        })
                                        .collect();
                                    let asks: Vec<(Decimal, Decimal)> = payload.levels[1]
                                        .iter()
                                        .filter_map(|l| {
                                            let p = Decimal::from_str(&l.px).ok()?;
                                            let s = Decimal::from_str(&l.sz).ok()?;
                                            Some((p, s))
                                        })
                                        .collect();

                                    let event = NormalizedEvent::OrderBook(NormalizedOrderBook {
                                        exchange: Exchange::Hyperliquid,
                                        symbol,
                                        bids,
                                        asks,
                                        timestamp_ms: payload.time,
                                    });
                                    let _ = event_tx.send(event).await;
                                }
                            }
                        }
                    } else if raw_text.contains("\"channel\":\"trades\"") {
                        if let Ok(envelope) = serde_json::from_str::<TradesEnvelope>(&raw_text) {
                            if let Some(trades) = envelope.data {
                                for t in trades {
                                    let symbol = to_internal_symbol(&t.coin);
                                    let price = Decimal::from_str(&t.px).unwrap_or(Decimal::ZERO);
                                    let size = Decimal::from_str(&t.sz).unwrap_or(Decimal::ZERO);
                                    let side = if t.side == "A" { TradeSide::Sell } else { TradeSide::Buy };

                                    let event = NormalizedEvent::Trade(NormalizedTrade {
                                        exchange: Exchange::Hyperliquid,
                                        symbol,
                                        price,
                                        size,
                                        side,
                                        timestamp_ms: t.time,
                                        trade_id: t.tid.to_string(),
                                    });
                                    let _ = event_tx.send(event).await;
                                }
                            }
                        }
                    }
                }
                Message::Ping(ping) => {
                    let _ = write.send(Message::Pong(ping)).await;
                }
                Message::Close(_) => {
                    println!("🔌 Hyperliquid Adapter: Connection closed by server.");
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

pub async fn run_for_symbol(
    symbol: String,
    event_tx: Sender<NormalizedEvent>,
    cancel: CancellationToken,
    ws_url: &str,
) {
    let url = match url::Url::parse(ws_url) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("❌ Hyperliquid: Invalid WS URL for {}: {}", symbol, e);
            return;
        }
    };

    let (ws_stream, _) = match connect_async(url.as_str()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Hyperliquid: WS handshake failed for {}: {}", symbol, e);
            return;
        }
    };
    println!("✅ Hyperliquid [{}]: TCP/WS Handshake completed.", symbol);
    let (mut write, mut read) = ws_stream.split();

    let _ = event_tx.send(NormalizedEvent::Status {
        exchange: Exchange::Hyperliquid,
        status: ConnectionStatus::Connected,
        message: format!("Dedicated WS connected for {}", symbol),
    }).await;

    let subscriptions = vec![
        serde_json::json!({"type": "trades", "coin": &symbol}),
        serde_json::json!({"type": "l2Book", "coin": &symbol}),
    ];
    for sub in &subscriptions {
        let sub_request = serde_json::json!({
            "method": "subscribe",
            "subscription": sub
        });
        println!("📡 Hyperliquid [{}]: Subscribing to stream: {}", symbol, sub_request);
        if let Err(e) = write.send(Message::Text(sub_request.to_string().into())).await {
            eprintln!("❌ Hyperliquid [{}]: Failed to send subscription: {}", symbol, e);
            return;
        }
    }

    let internal_symbol = to_internal_symbol(&symbol);

    loop {
        let msg = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                println!("🛑 Hyperliquid [{}]: Cancellation triggered, closing WS connection.", symbol);
                break;
            }
            result = read.next() => {
                match result {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        eprintln!("⚠️ Hyperliquid [{}]: Socket error: {}", symbol, e);
                        break;
                    }
                    None => {
                        println!("🔌 Hyperliquid [{}]: Stream ended.", symbol);
                        break;
                    }
                }
            }
        };

        match msg {
            Message::Text(raw_text) => {
                if raw_text.contains("\"channel\":\"l2Book\"") {
                    if let Ok(envelope) = serde_json::from_str::<L2BookEnvelope>(&raw_text) {
                        if let Some(payload) = envelope.data {
                            if payload.levels.len() >= 2
                                && !payload.levels[0].is_empty()
                                && !payload.levels[1].is_empty()
                            {
                                let bids: Vec<(Decimal, Decimal)> = payload.levels[0]
                                    .iter()
                                    .filter_map(|l| {
                                        let p = Decimal::from_str(&l.px).ok()?;
                                        let s = Decimal::from_str(&l.sz).ok()?;
                                        Some((p, s))
                                    })
                                    .collect();
                                let asks: Vec<(Decimal, Decimal)> = payload.levels[1]
                                    .iter()
                                    .filter_map(|l| {
                                        let p = Decimal::from_str(&l.px).ok()?;
                                        let s = Decimal::from_str(&l.sz).ok()?;
                                        Some((p, s))
                                    })
                                    .collect();

                                let event = NormalizedEvent::OrderBook(NormalizedOrderBook {
                                    exchange: Exchange::Hyperliquid,
                                    symbol: internal_symbol.clone(),
                                    bids,
                                    asks,
                                    timestamp_ms: payload.time,
                                });
                                let _ = event_tx.send(event).await;
                            }
                        }
                    }
                } else if raw_text.contains("\"channel\":\"trades\"") {
                    if let Ok(envelope) = serde_json::from_str::<TradesEnvelope>(&raw_text) {
                        if let Some(trades) = envelope.data {
                            for t in trades {
                                let price = Decimal::from_str(&t.px).unwrap_or(Decimal::ZERO);
                                let size = Decimal::from_str(&t.sz).unwrap_or(Decimal::ZERO);
                                let side = if t.side == "A" { TradeSide::Sell } else { TradeSide::Buy };

                                let event = NormalizedEvent::Trade(NormalizedTrade {
                                    exchange: Exchange::Hyperliquid,
                                    symbol: internal_symbol.clone(),
                                    price,
                                    size,
                                    side,
                                    timestamp_ms: t.time,
                                    trade_id: t.tid.to_string(),
                                });
                                let _ = event_tx.send(event).await;
                            }
                        }
                    }
                }
            }
            Message::Ping(ping) => {
                let _ = write.send(Message::Pong(ping)).await;
            }
            Message::Close(_) => {
                println!("🔌 Hyperliquid [{}]: Connection closed by server.", symbol);
                break;
            }
            _ => {}
        }
    }

    let _ = event_tx.send(NormalizedEvent::Status {
        exchange: Exchange::Hyperliquid,
        status: ConnectionStatus::Disconnected,
        message: format!("Dedicated WS disconnected for {}", symbol),
    }).await;
}
