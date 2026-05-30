use std::sync::Arc;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use shared::normalized::{
    Exchange, ExchangeAdapter, NormalizedEvent, NormalizedTrade,
    SymbolMapper, TradeSide, ConnectionStatus,
};

pub struct KrakenAdapter;

#[derive(Debug, Deserialize)]
struct KrakenEventMsg {
    channel: String,
    data: Option<Vec<KrakenTradeData>>,
}

#[derive(Debug, Deserialize)]
struct KrakenTradeData {
    symbol: String,
    side: String,
    price: f64,
    qty: f64,
    trade_id: u64,
    timestamp: String,
}

#[async_trait]
impl ExchangeAdapter for KrakenAdapter {
    fn exchange(&self) -> Exchange {
        Exchange::Kraken
    }

    async fn start(
        &self,
        symbols: Vec<String>,
        event_tx: Sender<NormalizedEvent>,
        mapper: Arc<SymbolMapper>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut target_symbols = Vec::new();

        for sym in &symbols {
            if let Some(raw_sym) = mapper.get_raw(Exchange::Kraken, sym).await {
                target_symbols.push(raw_sym);
            }
        }

        if target_symbols.is_empty() {
            return Err("No valid symbols mapped for Kraken".into());
        }

        let url_string = "wss://ws.kraken.com/v2";
        println!("🔌 Kraken Adapter: Connecting to production: {}", url_string);

        let (ws_stream, _) = connect_async(url_string).await.map_err(|e| {
            format!("Kraken WS connection failed: {}", e)
        })?;
        println!("✅ Kraken Adapter: TCP/WS Handshake completed.");
        let (mut write, mut read) = ws_stream.split();

        let _ = event_tx.send(NormalizedEvent::Status {
            exchange: Exchange::Kraken,
            status: ConnectionStatus::Connected,
            message: "Production WS connection established successfully.".to_string(),
        }).await;

        let sub_request = serde_json::json!({
            "method": "subscribe",
            "params": {
                "channel": "trade",
                "symbol": target_symbols
            }
        });
        write.send(Message::Text(sub_request.to_string().into())).await?;

        while let Some(msg) = read.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("⚠️ Kraken Adapter: Socket error: {}", e);
                    break;
                }
            };

            if let Message::Text(raw_text) = msg {
                if let Ok(event_msg) = serde_json::from_str::<KrakenEventMsg>(&raw_text) {
                    if event_msg.channel == "trade" {
                        if let Some(trades) = event_msg.data {
                            for trade in trades {
                                let internal_sym = match mapper.normalize(Exchange::Kraken, &trade.symbol).await {
                                    Some(s) => s,
                                    None => continue,
                                };

                                let price = Decimal::from_f64(trade.price).unwrap_or(Decimal::ZERO);
                                let size = Decimal::from_f64(trade.qty).unwrap_or(Decimal::ZERO);
                                let side = if trade.side == "buy" { TradeSide::Buy } else { TradeSide::Sell };

                                let ts_ms = match chrono::DateTime::parse_from_rfc3339(&trade.timestamp) {
                                    Ok(dt) => dt.timestamp_millis() as u64,
                                    Err(_) => std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as u64,
                                };

                                let norm_event = NormalizedEvent::Trade(NormalizedTrade {
                                    exchange: Exchange::Kraken,
                                    symbol: internal_sym,
                                    price,
                                    size,
                                    side,
                                    timestamp_ms: ts_ms,
                                    trade_id: trade.trade_id.to_string(),
                                });

                                if event_tx.send(norm_event).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
