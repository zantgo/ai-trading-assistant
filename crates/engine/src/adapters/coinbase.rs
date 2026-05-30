use std::str::FromStr;
use std::sync::Arc;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use shared::normalized::{
    Exchange, ExchangeAdapter, NormalizedEvent, NormalizedTrade,
    SymbolMapper, TradeSide, ConnectionStatus,
};

pub struct CoinbaseAdapter;

#[derive(Debug, Deserialize)]
struct CoinbaseTradeMsg {
    #[serde(rename = "type")]
    msg_type: String,
    product_id: String,
    price: String,
    size: String,
    side: String,
    trade_id: u64,
    time: String,
}

#[async_trait]
impl ExchangeAdapter for CoinbaseAdapter {
    fn exchange(&self) -> Exchange {
        Exchange::Coinbase
    }

    async fn start(
        &self,
        symbols: Vec<String>,
        event_tx: Sender<NormalizedEvent>,
        mapper: Arc<SymbolMapper>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut target_symbols = Vec::new();

        for sym in &symbols {
            if let Some(raw_sym) = mapper.get_raw(Exchange::Coinbase, sym).await {
                target_symbols.push(raw_sym);
            }
        }

        if target_symbols.is_empty() {
            return Err("No valid symbols mapped for Coinbase".into());
        }

        let url_string = "wss://ws-feed.exchange.coinbase.com";
        println!("🔌 Coinbase Adapter: Connecting to production: {}", url_string);

        let (ws_stream, _) = connect_async(url_string).await.map_err(|e| {
            format!("Coinbase WS connection failed: {}", e)
        })?;
        println!("✅ Coinbase Adapter: TCP/WS Handshake completed.");
        let (mut write, mut read) = ws_stream.split();

        let _ = event_tx.send(NormalizedEvent::Status {
            exchange: Exchange::Coinbase,
            status: ConnectionStatus::Connected,
            message: "Production WS connection established successfully.".to_string(),
        }).await;

        let sub_request = serde_json::json!({
            "type": "subscribe",
            "product_ids": target_symbols,
            "channels": ["matches"]
        });
        write.send(Message::Text(sub_request.to_string().into())).await?;

        while let Some(msg) = read.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("⚠️ Coinbase Adapter: Socket error: {}", e);
                    break;
                }
            };

            if let Message::Text(raw_text) = msg {
                if let Ok(trade) = serde_json::from_str::<CoinbaseTradeMsg>(&raw_text) {
                    if trade.msg_type == "match" || trade.msg_type == "last_match" {
                        let internal_sym = match mapper.normalize(Exchange::Coinbase, &trade.product_id).await {
                            Some(s) => s,
                            None => continue,
                        };

                        let price = Decimal::from_str(&trade.price).unwrap_or(Decimal::ZERO);
                        let size = Decimal::from_str(&trade.size).unwrap_or(Decimal::ZERO);
                        let side = if trade.side == "buy" { TradeSide::Buy } else { TradeSide::Sell };

                        let ts_ms = match chrono::DateTime::parse_from_rfc3339(&trade.time) {
                            Ok(dt) => dt.timestamp_millis() as u64,
                            Err(_) => std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        };

                        let norm_event = NormalizedEvent::Trade(NormalizedTrade {
                            exchange: Exchange::Coinbase,
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

        Ok(())
    }
}
