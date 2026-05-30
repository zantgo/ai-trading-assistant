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

pub struct BybitAdapter;

#[derive(Debug, Deserialize)]
struct BybitEventMsg {
    topic: String,
    data: Option<Vec<BybitTradeData>>,
}

#[derive(Debug, Deserialize)]
struct BybitTradeData {
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    size: String,
    #[serde(rename = "S")]
    side: String,
    #[serde(rename = "T")]
    time_ms: u64,
    #[serde(rename = "i")]
    trade_id: String,
}

#[async_trait]
impl ExchangeAdapter for BybitAdapter {
    fn exchange(&self) -> Exchange {
        Exchange::Bybit
    }

    async fn start(
        &self,
        symbols: Vec<String>,
        event_tx: Sender<NormalizedEvent>,
        mapper: Arc<SymbolMapper>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut streams = Vec::new();

        for sym in &symbols {
            if let Some(raw_sym) = mapper.get_raw(Exchange::Bybit, sym).await {
                streams.push(format!("publicTrade.{}", raw_sym.to_uppercase()));
            }
        }

        if streams.is_empty() {
            return Err("No valid symbols mapped for Bybit".into());
        }

        let url_string = "wss://stream.bybit.com/v5/public/spot";
        println!("🔌 Bybit Adapter: Connecting to production: {}", url_string);

        let (ws_stream, _) = connect_async(url_string).await.map_err(|e| {
            format!("Bybit WS connection failed: {}", e)
        })?;
        println!("✅ Bybit Adapter: TCP/WS Handshake completed.");
        let (mut write, mut read) = ws_stream.split();

        let _ = event_tx.send(NormalizedEvent::Status {
            exchange: Exchange::Bybit,
            status: ConnectionStatus::Connected,
            message: "Production WS connection established successfully.".to_string(),
        }).await;

        let sub_request = serde_json::json!({
            "op": "subscribe",
            "args": streams
        });
        write.send(Message::Text(sub_request.to_string().into())).await?;

        while let Some(msg) = read.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("⚠️ Bybit Adapter: Socket error: {}", e);
                    break;
                }
            };

            if let Message::Text(raw_text) = msg {
                if let Ok(event_msg) = serde_json::from_str::<BybitEventMsg>(&raw_text) {
                    if let Some(trades) = event_msg.data {
                        let raw_symbol = event_msg.topic.trim_start_matches("publicTrade.");
                        let internal_sym = match mapper.normalize(Exchange::Bybit, raw_symbol).await {
                            Some(s) => s,
                            None => continue,
                        };

                        for trade in trades {
                            let price = Decimal::from_str(&trade.price).unwrap_or(Decimal::ZERO);
                            let size = Decimal::from_str(&trade.size).unwrap_or(Decimal::ZERO);
                            let side = if trade.side == "Buy" { TradeSide::Buy } else { TradeSide::Sell };

                            let norm_event = NormalizedEvent::Trade(NormalizedTrade {
                                exchange: Exchange::Bybit,
                                symbol: internal_sym.clone(),
                                price,
                                size,
                                side,
                                timestamp_ms: trade.time_ms,
                                trade_id: trade.trade_id,
                            });

                            if event_tx.send(norm_event).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
