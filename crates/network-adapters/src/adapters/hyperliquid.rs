use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use core_domain::normalized::{
    AssetContext, ConnectionStatus, Exchange, ExchangeAdapter, NormalizedEvent,
    NormalizedOrderBook, NormalizedTrade, SymbolMapper, TradeSide,
};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_util::sync::CancellationToken;

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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ActiveAssetCtxEnvelope {
    channel: String,
    data: Option<ActiveAssetCtxData>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct UserFillsEnvelope {
    channel: String,
    data: Option<Vec<UserFill>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct UserFill {
    coin: String,
    px: String,
    sz: String,
    side: String, // "A" (sell) or "B" (buy)
    time: u64,
    #[serde(rename = "hash", default)]
    hash: String,
    /// Hyperliquid marks liquidation fills with this field equal to the
    /// string "liquidated" or containing the literal liquidation mark.
    #[serde(default)]
    liquidation: Option<String>,
    /// Position size at the time of fill.
    #[serde(default)]
    start_position: Option<String>,
    /// Closed PnL (negative for liquidations).
    #[serde(rename = "closedPnl", default)]
    closed_pnl: Option<String>,
    /// Fee paid on this fill.
    #[serde(default)]
    fee: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ActiveAssetCtxData {
    coin: String,
    ctx: Option<AssetCtxInner>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AssetCtxInner {
    #[serde(rename = "prevDayPx")]
    prev_day_px: Option<String>,
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

        let _ = event_tx
            .send(NormalizedEvent::Status {
                exchange: Exchange::Hyperliquid,
                status: ConnectionStatus::Connected,
                message: "Testnet WS connection established.".to_string(),
            })
            .await;

        let mut subscriptions = Vec::new();
        for sym in &symbols {
            if let Some(raw_sym) = mapper.get_raw(Exchange::Hyperliquid, sym).await {
                subscriptions.push(serde_json::json!({"type": "trades", "coin": raw_sym}));
                subscriptions.push(serde_json::json!({"type": "l2Book", "coin": raw_sym}));
                subscriptions.push(serde_json::json!({"type": "activeAssetCtx", "coin": raw_sym}));
                println!(
                    "📡 Hyperliquid Adapter: Subscribed to trades + l2Book + activeAssetCtx for {} ({})",
                    sym, raw_sym
                );
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
            println!(
                "📡 Hyperliquid Adapter: Subscribing to stream: {}",
                sub_request
            );
            write
                .send(Message::Text(sub_request.to_string().into()))
                .await?;
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
                                    let side = if t.side == "A" {
                                        TradeSide::Sell
                                    } else {
                                        TradeSide::Buy
                                    };

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
                    } else if raw_text.contains("\"channel\":\"activeAssetCtx\"") {
                        if let Ok(envelope) =
                            serde_json::from_str::<ActiveAssetCtxEnvelope>(&raw_text)
                        {
                            if let Some(data) = envelope.data {
                                if let Some(ctx) = data.ctx {
                                    if let Some(px) = ctx
                                        .prev_day_px
                                        .as_deref()
                                        .and_then(|s| Decimal::from_str(s).ok())
                                    {
                                        let symbol = to_internal_symbol(&data.coin);
                                        let _ = event_tx
                                            .send(NormalizedEvent::AssetContext(AssetContext {
                                                symbol,
                                                prev_day_px: px,
                                            }))
                                            .await;
                                    }
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
    internal_symbol: String,
    event_tx: Sender<NormalizedEvent>,
    cancel: CancellationToken,
    ws_url: &str,
    user_address: &str,
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

    let _ = event_tx
        .send(NormalizedEvent::Status {
            exchange: Exchange::Hyperliquid,
            status: ConnectionStatus::Connected,
            message: format!("Dedicated WS connected for {}", symbol),
        })
        .await;

    let subscriptions = vec![
        serde_json::json!({"type": "trades", "coin": &symbol}),
        serde_json::json!({"type": "l2Book", "coin": &symbol}),
        serde_json::json!({"type": "activeAssetCtx", "coin": &symbol}),
        // Phase 1: userFills channel exposes real liquidation events.
        // Hyperliquid does NOT publish liquidations on a public channel;
        // `userFills` is the only source and is account-scoped. The
        // `user_address` parameter is passed in by the registry: an empty
        // string disables the subscription (no HL liquidations will be
        // ingested), a valid 0x-prefixed 40-hex-char address enables it
        // for that account only. Bitget, by contrast, exposes public
        // liquidations on the `fill` channel and is always active when
        // `liquidation_feed = true` in `[workspace.liquidity]`.
        user_fills_subscription(&symbol, user_address),
    ];
    for sub in &subscriptions {
        // Skip disabled subscriptions (e.g. userFills when no HL user
        // address is configured) — they would otherwise be sent to HL and
        // either fail or return no fills. `_disabled: true` is set by
        // `user_fills_subscription`.
        if sub.get("_disabled").and_then(|v| v.as_bool()).unwrap_or(false) {
            println!(
                "📡 Hyperliquid [{}]: Skipping disabled subscription (no user address configured)",
                symbol
            );
            continue;
        }
        let sub_request = serde_json::json!({
            "method": "subscribe",
            "subscription": sub
        });
        println!(
            "📡 Hyperliquid [{}]: Subscribing to stream: {}",
            symbol, sub_request
        );
        if let Err(e) = write
            .send(Message::Text(sub_request.to_string().into()))
            .await
        {
            eprintln!(
                "❌ Hyperliquid [{}]: Failed to send subscription: {}",
                symbol, e
            );
            return;
        }
    }

    // Hyperliquid keep-alive (03-01-02 §4): the client sends a JSON
    // `{"method":"ping"}` every 30 s; the server answers on the "pong"
    // channel. Two consecutive unanswered pings mark the stream as stalled
    // and force a disconnect so the caller's reconnect loop takes over.
    const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
    const MAX_MISSED_PONGS: u32 = 2;
    let mut ping_interval = tokio::time::interval_at(
        tokio::time::Instant::now() + HEARTBEAT_INTERVAL,
        HEARTBEAT_INTERVAL,
    );
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut awaiting_pongs: u32 = 0;

    loop {
        let msg = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                println!("🛑 Hyperliquid [{}]: Cancellation triggered, closing WS connection.", symbol);
                break;
            }
            _ = ping_interval.tick() => {
                if awaiting_pongs >= MAX_MISSED_PONGS {
                    eprintln!(
                        "💔 Hyperliquid [{}]: {} consecutive pings unanswered — treating stalled stream as disconnect.",
                        symbol, awaiting_pongs
                    );
                    break;
                }
                let ping_frame = serde_json::json!({"method": "ping"}).to_string();
                if let Err(e) = write.send(Message::Text(ping_frame.into())).await {
                    eprintln!("⚠️ Hyperliquid [{}]: Failed to send keep-alive ping: {}", symbol, e);
                    break;
                }
                awaiting_pongs += 1;
                continue;
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
                // Any inbound frame proves liveness; the "pong" channel is
                // the explicit keep-alive reply and carries no payload.
                awaiting_pongs = 0;
                if raw_text.contains("\"channel\":\"pong\"") {
                    continue;
                }
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
                                let side = if t.side == "A" {
                                    TradeSide::Sell
                                } else {
                                    TradeSide::Buy
                                };

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
                } else if raw_text.contains("\"channel\":\"activeAssetCtx\"") {
                    if let Ok(envelope) = serde_json::from_str::<ActiveAssetCtxEnvelope>(&raw_text)
                    {
                        if let Some(data) = envelope.data {
                            if let Some(ctx) = data.ctx {
                                if let Some(px) = ctx
                                    .prev_day_px
                                    .as_deref()
                                    .and_then(|s| Decimal::from_str(s).ok())
                                {
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
                } else if raw_text.contains("\"channel\":\"userFills\"") {
                    emit_user_fills_liquidations(&internal_symbol, &raw_text, &event_tx);
                }
            }
            Message::Ping(ping) => {
                awaiting_pongs = 0;
                let _ = write.send(Message::Pong(ping)).await;
            }
            Message::Pong(_) => {
                awaiting_pongs = 0;
            }
            Message::Close(_) => {
                println!("🔌 Hyperliquid [{}]: Connection closed by server.", symbol);
                break;
            }
            _ => {}
        }
    }

    let _ = event_tx
        .send(NormalizedEvent::Status {
            exchange: Exchange::Hyperliquid,
            status: ConnectionStatus::Disconnected,
            message: format!("Dedicated WS disconnected for {}", symbol),
        })
        .await;
}

// =============================================================================
// userFills liquidation extraction
// =============================================================================
//
// Hyperliquid's `userFills` channel includes a `liquidation` field on each
// fill entry. When set, the fill is a forced close by the liquidation
// engine. We translate it to a `NormalizedEvent::Liquidation` with the
// correct side: a "B" (buy) side on a liquidation closes a SHORT, a "A"
// (sell) side on a liquidation closes a LONG.
//
// We process the message once in `start()` and once in `run_for_symbol()`.

/// Build the `userFills` subscription request, or `None` if the operator
/// has not configured a user address (default). HL liquidations are
/// account-scoped — there is no public liquidation feed.
fn user_fills_subscription(symbol: &str, user_address: &str) -> serde_json::Value {
    let addr = user_address.trim();
    let valid = !addr.is_empty()
        && addr.starts_with("0x")
        && addr.len() == 42
        && addr[2..].chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        serde_json::json!({"type": "userFills", "coin": symbol, "user": addr})
    } else {
        // Sentinel: serialised as an object with a special `disabled` flag
        // so the dispatcher can skip it without consulting the address
        // twice. The `loop` body in `run_for_symbol` checks this and
        // never sends the frame over WS.
        serde_json::json!({
            "type": "userFills",
            "coin": symbol,
            "user": "0x0000000000000000000000000000000000000000",
            "_disabled": true
        })
    }
}

/// Public read-only flag for the dispatcher: true when the operator has
/// configured a valid HL user address and the `userFills` subscription
/// should be activated.
pub fn user_fills_enabled(user_address: &str) -> bool {
    let addr = user_address.trim();
    !addr.is_empty()
        && addr.starts_with("0x")
        && addr.len() == 42
        && addr[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn emit_user_fills_liquidations(
    internal_symbol: &str,
    raw_text: &str,
    event_tx: &tokio::sync::mpsc::Sender<NormalizedEvent>,
) {
    let envelope: UserFillsEnvelope = match serde_json::from_str(raw_text) {
        Ok(e) => e,
        Err(_) => return,
    };
    let Some(fills) = envelope.data else { return };
    for fill in fills {
        // A fill is a liquidation when the `liquidation` field is present
        // AND non-empty. Hyperliquid uses string values like "B" or "A"
        // here (the side that triggered the liquidation).
        let is_liquidation = fill
            .liquidation
            .as_deref()
            .map(|s| !s.is_empty() && s != "false")
            .unwrap_or(false);
        if !is_liquidation {
            continue;
        }
        let price = match Decimal::from_str(&fill.px) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let size = match Decimal::from_str(&fill.sz) {
            Ok(s) => s,
            Err(_) => continue,
        };
        // Side semantics for liquidations:
        //   side == "A" (aggressor was seller)  -> a long was closed
        //   side == "B" (aggressor was buyer)   -> a short was closed
        let liq_side = if fill.side == "A" {
            core_domain::normalized::LiquidationSide::Long
        } else {
            core_domain::normalized::LiquidationSide::Short
        };
        let _ = event_tx.try_send(NormalizedEvent::Liquidation(
            core_domain::normalized::LiquidationEvent {
                exchange: Exchange::Hyperliquid,
                symbol: internal_symbol.to_string(),
                side: liq_side,
                price,
                size,
                timestamp_ms: fill.time,
                venue_order_id: if fill.hash.is_empty() {
                    None
                } else {
                    Some(fill.hash.clone())
                },
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_fills_enabled_accepts_valid_address() {
        // 0x + 40 hex chars.
        let addr = "0x1234567890abcdef1234567890abcdef12345678";
        assert!(user_fills_enabled(addr));
    }

    #[test]
    fn user_fills_enabled_accepts_mixed_case() {
        let addr = "0xAbCdEf1234567890aBcDeF1234567890aBcDeF12";
        assert!(user_fills_enabled(addr));
    }

    #[test]
    fn user_fills_enabled_rejects_empty() {
        assert!(!user_fills_enabled(""));
        assert!(!user_fills_enabled("   "));
    }

    #[test]
    fn user_fills_enabled_rejects_missing_prefix() {
        let addr = "1234567890abcdef1234567890abcdef12345678";
        assert!(!user_fills_enabled(addr));
    }

    #[test]
    fn user_fills_enabled_rejects_wrong_length() {
        assert!(!user_fills_enabled("0x1234"));
        assert!(!user_fills_enabled(
            "0x1234567890abcdef1234567890abcdef12345678901234"
        ));
    }

    #[test]
    fn user_fills_enabled_rejects_non_hex() {
        let addr = "0xZZZZ567890abcdef1234567890abcdef12345678";
        assert!(!user_fills_enabled(addr));
    }

    #[test]
    fn user_fills_subscription_marks_disabled_for_empty_address() {
        let sub = user_fills_subscription("BTC", "");
        assert_eq!(
            sub.get("_disabled").and_then(|v| v.as_bool()),
            Some(true),
            "empty address must produce a disabled subscription"
        );
    }

    #[test]
    fn user_fills_subscription_marks_disabled_for_invalid_address() {
        let sub = user_fills_subscription("BTC", "not-a-real-address");
        assert_eq!(
            sub.get("_disabled").and_then(|v| v.as_bool()),
            Some(true),
            "invalid address must produce a disabled subscription"
        );
    }

    #[test]
    fn user_fills_subscription_emits_real_address_when_valid() {
        let addr = "0x1234567890abcdef1234567890abcdef12345678";
        let sub = user_fills_subscription("BTC", addr);
        assert!(sub.get("_disabled").is_none());
        assert_eq!(sub.get("user").and_then(|v| v.as_str()), Some(addr));
        assert_eq!(sub.get("coin").and_then(|v| v.as_str()), Some("BTC"));
    }
}
