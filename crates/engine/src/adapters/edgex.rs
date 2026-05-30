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

pub struct EdgeXAdapter;

#[derive(Debug, Deserialize)]
struct EdgeXPublicEnvelope {
    #[serde(rename = "type")]
    msg_type: String,
    channel: String,
    content: Option<EdgeXQuoteContent>,
}

#[derive(Debug, Deserialize)]
struct EdgeXQuoteContent {
    #[serde(rename = "dataType")]
    data_type: String,
    data: Option<Vec<EdgeXTradeData>>,
}

#[derive(Debug, Deserialize)]
struct EdgeXTradeData {
    price: String,
    size: String,
    side: String,
    time: String,
}

#[async_trait]
impl ExchangeAdapter for EdgeXAdapter {
    fn exchange(&self) -> Exchange {
        Exchange::EdgeX
    }

    async fn start(
        &self,
        symbols: Vec<String>,
        event_tx: Sender<NormalizedEvent>,
        mapper: Arc<SymbolMapper>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut target_channels = Vec::new();

        for sym in &symbols {
            if let Some(raw_sym) = mapper.get_raw(Exchange::EdgeX, sym).await {
                target_channels.push(format!("trade.{}", raw_sym.to_uppercase()));
            }
        }

        if target_channels.is_empty() {
            return Err("No valid symbols mapped for EdgeX".into());
        }

        let url_string = "wss://quote.edgex.exchange/api/v1/public/ws";
        println!("🔌 EdgeX Adapter: Connecting to production: {}", url_string);

        let (ws_stream, _) = connect_async(url_string).await.map_err(|e| {
            format!("EdgeX WS connection failed: {}", e)
        })?;
        println!("✅ EdgeX Adapter: TCP/WS Handshake completed.");
        let (mut write, mut read) = ws_stream.split();

        let _ = event_tx.send(NormalizedEvent::Status {
            exchange: Exchange::EdgeX,
            status: ConnectionStatus::Connected,
            message: "Production WS connection established successfully.".to_string(),
        }).await;

        for channel_name in target_channels {
            let sub_request = serde_json::json!({
                "type": "subscribe",
                "channel": channel_name
            });
            write.send(Message::Text(sub_request.to_string().into())).await?;
        }

        while let Some(msg) = read.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("⚠️ EdgeX Adapter: Socket error: {}", e);
                    break;
                }
            };

            match msg {
                Message::Text(raw_text) => {
                    if let Ok(envelope) = serde_json::from_str::<EdgeXPublicEnvelope>(&raw_text) {
                        if envelope.msg_type == "quote-event" {
                            if let Some(content) = envelope.content {
                                if content.data_type.to_lowercase() == "changed" || content.data_type.to_lowercase() == "snapshot" {
                                    if let Some(trades) = content.data {
                                        let raw_symbol = envelope.channel.trim_start_matches("trade.");
                                        let internal_sym = match mapper.normalize(Exchange::EdgeX, raw_symbol).await {
                                            Some(s) => s,
                                            None => continue,
                                        };

                                        for trade in trades {
                                            let price = Decimal::from_str(&trade.price).unwrap_or(Decimal::ZERO);
                                            let size = Decimal::from_str(&trade.size).unwrap_or(Decimal::ZERO);
                                            let side = if trade.side == "buy" { TradeSide::Buy } else { TradeSide::Sell };
                                            let ts_ms = trade.time.parse::<u64>().unwrap_or(0);

                                            let norm_event = NormalizedEvent::Trade(NormalizedTrade {
                                                exchange: Exchange::EdgeX,
                                                symbol: internal_sym.clone(),
                                                price,
                                                size,
                                                side,
                                                timestamp_ms: ts_ms,
                                                trade_id: String::new(),
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
                }
                Message::Ping(ping) => {
                    let _ = write.send(Message::Pong(ping)).await;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
