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

pub struct BitgetAdapter;

#[derive(Debug, Deserialize)]
struct BitgetEventMsg {
    action: String,
    arg: BitgetArg,
    data: Option<Vec<BitgetTradeData>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BitgetArg {
    #[serde(rename = "instId")]
    inst_id: String,
    channel: String,
}

#[derive(Debug, Deserialize)]
struct BitgetTradeData {
    #[serde(rename = "ts")]
    timestamp_str: String,
    price: String,
    size: String,
    side: String,
    #[serde(rename = "tradeId")]
    trade_id: String,
}

#[async_trait]
impl ExchangeAdapter for BitgetAdapter {
    fn exchange(&self) -> Exchange {
        Exchange::Bitget
    }

    async fn start(
        &self,
        symbols: Vec<String>,
        event_tx: Sender<NormalizedEvent>,
        mapper: Arc<SymbolMapper>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut subscription_args = Vec::new();

        for sym in &symbols {
            if let Some(raw_sym) = mapper.get_raw(Exchange::Bitget, sym).await {
                subscription_args.push(serde_json::json!({
                    "instType": "SPOT",
                    "channel": "trade",
                    "instId": raw_sym.to_uppercase()
                }));
            }
        }

        if subscription_args.is_empty() {
            return Err("No valid symbols mapped for Bitget".into());
        }

        let url_string = "wss://ws.bitget.com/v2/ws/public";
        println!("🔌 Bitget Adapter: Connecting to production: {}", url_string);

        let (ws_stream, _) = connect_async(url_string).await.map_err(|e| {
            format!("Bitget WS connection failed: {}", e)
        })?;
        println!("✅ Bitget Adapter: TCP/WS Handshake completed.");
        let (mut write, mut read) = ws_stream.split();

        let _ = event_tx.send(NormalizedEvent::Status {
            exchange: Exchange::Bitget,
            status: ConnectionStatus::Connected,
            message: "Production WS connection established successfully.".to_string(),
        }).await;

        let sub_request = serde_json::json!({
            "op": "subscribe",
            "args": subscription_args
        });
        write.send(Message::Text(sub_request.to_string().into())).await?;

        while let Some(msg) = read.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("⚠️ Bitget Adapter: Socket error: {}", e);
                    break;
                }
            };

            if let Message::Text(raw_text) = msg {
                if let Ok(event_msg) = serde_json::from_str::<BitgetEventMsg>(&raw_text) {
                    if event_msg.action == "snapshot" || event_msg.action == "update" {
                        if let Some(trades) = event_msg.data {
                            let internal_sym = match mapper.normalize(Exchange::Bitget, &event_msg.arg.inst_id).await {
                                Some(s) => s,
                                None => continue,
                            };

                            for trade in trades {
                                let price = Decimal::from_str(&trade.price).unwrap_or(Decimal::ZERO);
                                let size = Decimal::from_str(&trade.size).unwrap_or(Decimal::ZERO);
                                let side = if trade.side == "buy" { TradeSide::Buy } else { TradeSide::Sell };
                                let ts_ms = trade.timestamp_str.parse::<u64>().unwrap_or(0);

                                let norm_event = NormalizedEvent::Trade(NormalizedTrade {
                                    exchange: Exchange::Bitget,
                                    symbol: internal_sym.clone(),
                                    price,
                                    size,
                                    side,
                                    timestamp_ms: ts_ms,
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
        }

        Ok(())
    }
}
