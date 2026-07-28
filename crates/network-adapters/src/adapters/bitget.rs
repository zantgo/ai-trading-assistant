use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use core_domain::normalized::{
    AssetContext, ConnectionStatus, Exchange, NormalizedEvent, NormalizedOrderBook,
    NormalizedTrade, TradeSide,
};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Mutex};
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

/// Open Interest item from the `open-interest` WS channel.
#[derive(Debug, Deserialize)]
#[allow(non_snake_case, dead_code)]
struct OpenInterestItem {
    instId: Option<String>,
    openInterest: Option<String>,
    ts: Option<String>,
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

/// Client keep-alive cadence (Bitget V2 requires a ping at least every 30 s).
const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
/// Consecutive unanswered pings before the stream is declared stalled.
const MAX_MISSED_PONGS: u32 = 2;

pub async fn run_for_symbol(
    symbol: String,
    internal_symbol: String,
    product_type: String,
    event_tx: Sender<NormalizedEvent>,
    cancel: CancellationToken,
    ws_url: &str,
) {
    // Latest mark price seen on the `ticker` channel — used to convert
    // Bitget's base-asset-denominated `openInterest` field into a USD
    // notional so the cluster estimator downstream sees a USD figure.
    // Bitget pushes mark price on the `ticker` channel and OI on a
    // separate `open-interest` channel; the two are not in the same
    // payload, so we keep the most recent mark in a small shared cache.
    let latest_mark_px: Arc<Mutex<Option<Decimal>>> = Arc::new(Mutex::new(None));
    // Tracks whether we've already emitted a `Status` event about the
    // first-frame OI drop (race condition where the `open-interest`
    // message arrives before the first `ticker` mark price). We only
    // surface it once to avoid flooding the Exchange Status panel.
    let mut oi_drop_warned: bool = false;
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
            {"instType": &product_type, "channel": "ticker", "instId": &symbol},
            {"instType": &product_type, "channel": "funding-rate", "instId": &symbol},
            {"instType": &product_type, "channel": "open-interest", "instId": &symbol},
            // Phase 1: `fill` channel exposes real liquidation events.
            // execType == "L" marks a forced-close liquidation fill.
            {"instType": &product_type, "channel": "fill", "instId": &symbol}
        ]
    });
    println!(
        "📡 Bitget [{}]: Subscribing to trade + books5 + ticker + funding-rate + open-interest + fill streams ({})",
        symbol, product_type
    );
    if let Err(e) = write
        .send(Message::Text(sub_request.to_string().into()))
        .await
    {
        eprintln!("❌ Bitget [{}]: Failed to send subscription: {}", symbol, e);
        return;
    }

    // `symbol` is the raw contract symbol (e.g. "BTCUSDT" / "BTCUSD") used for
    // subscription. `internal_symbol` is the unified workspace symbol (e.g.
    // "BTC-USDT" / "BTC-USDC") emitted on every normalized event.

    // Bitget keep-alive (03-01-02 §4): client sends a literal "ping" text
    // frame every 30 s; the server answers "pong". Two consecutive missed
    // pongs (= a stalled stream) are treated as a disconnect so the caller's
    // reconnect loop takes over.
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
                println!("🛑 Bitget [{}]: Cancellation triggered, closing WS connection.", symbol);
                break;
            }
            _ = ping_interval.tick() => {
                if awaiting_pongs >= MAX_MISSED_PONGS {
                    eprintln!(
                        "💔 Bitget [{}]: {} consecutive pings unanswered — treating stalled stream as disconnect.",
                        symbol, awaiting_pongs
                    );
                    break;
                }
                if let Err(e) = write.send(Message::Text("ping".into())).await {
                    eprintln!("⚠️ Bitget [{}]: Failed to send keep-alive ping: {}", symbol, e);
                    break;
                }
                awaiting_pongs += 1;
                continue;
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
                // Any inbound frame proves liveness; "pong" is the explicit
                // keep-alive reply and carries no payload.
                awaiting_pongs = 0;
                if raw_text.as_str().eq_ignore_ascii_case("pong") {
                    continue;
                }
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
                        let trades: Vec<TradeItem> = match serde_json::from_value(data_val) {
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
                            let ts_ms: u64 = t.ts.parse::<u64>().unwrap_or(
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
                                // Bitget's public V2 `trade` channel doesn't
                                // emit a per-trade unique id (only the
                                // millisecond timestamp). Compose a
                                // uniqueness key from the timestamp + side +
                                // price + size so two trades in the same
                                // millisecond don't collide in any future
                                // dedupe / idempotency logic.
                                trade_id: format!(
                                    "{}:{}:{}:{}",
                                    t.ts,
                                    t.side,
                                    price,
                                    size
                                ),
                            });
                            let _ = event_tx.send(event).await;
                        }
                    }
                    "books5" => {
                        let books: Vec<BookItem> = match serde_json::from_value(data_val) {
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
                            let ts_ms: u64 = book.ts.parse::<u64>().unwrap_or(
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
                        let tickers: Vec<crate::adapters::bitget_derivatives::BitgetTickerData> =
                            match serde_json::from_value(data_val) {
                                Ok(t) => t,
                                Err(_) => continue,
                            };
                        for tk in tickers {
                            // Stash the latest mark price for the OI converter.
                            if let Some(mp) = tk
                                .mark_price
                                .as_deref()
                                .and_then(|s| Decimal::from_str(s).ok())
                            {
                                if mp > Decimal::ZERO {
                                    *latest_mark_px.lock().await = Some(mp);
                                }
                            }
                            // Emit AssetContext (prev-day price)
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
                            // Emit MarkPrice when mark/index data is present
                            if let Some(mp_ev) =
                                crate::adapters::bitget_derivatives::ticker_to_mark_price(
                                    &internal_symbol,
                                    &tk,
                                )
                            {
                                let _ = event_tx.send(mp_ev).await;
                            }
                        }
                    }
                    "funding-rate" => {
                        // Bitget funding-rate payload: array of {fundingRate, nextUpdate}.
                        let rows: Vec<crate::adapters::bitget_derivatives::BitgetFundingData> =
                            match serde_json::from_value(data_val) {
                                Ok(r) => r,
                                Err(_) => continue,
                            };
                        for row in rows {
                            if let Some(ev) = crate::adapters::bitget_derivatives::funding_to_event(
                                &internal_symbol,
                                &row,
                            ) {
                                let _ = event_tx.send(ev).await;
                            }
                        }
                    }
                    "open-interest" => {
                        let items: Vec<OpenInterestItem> =
                            match serde_json::from_value(data_val) {
                                Ok(i) => i,
                                Err(_) => continue,
                            };
                        // Bitget publishes `openInterest` in base-asset
                        // units (contracts on USDT-M perps). Convert to
                        // USD by multiplying against the latest mark
                        // price we've seen on the `ticker` channel. If
                        // mark price is missing, skip — emitting a
                        // base-asset value downstream poisons cluster
                        // confidence (the 4% symptom).
                        let mark_opt = *latest_mark_px.lock().await;
                        for item in items {
                            if let Some(oi_str) = item.openInterest.as_deref() {
                                if let Ok(raw_oi) = Decimal::from_str(oi_str) {
                                    let (oi_usd, prev_oi_usd) = match mark_opt {
                                        Some(mp) if mp > Decimal::ZERO => (raw_oi * mp, None),
                                        _ => {
                                            // First-frame race: OI arrived
                                            // before the first `ticker` mark
                                            // price. Drop silently here, but
                                            // emit a one-shot `Status` event
                                            // so the operator sees the gap in
                                            // the Exchange Status panel.
                                            if !oi_drop_warned {
                                                oi_drop_warned = true;
                                                let _ = event_tx
                                                    .send(NormalizedEvent::Status {
                                                        exchange: Exchange::Bitget,
                                                        status: ConnectionStatus::Connecting,
                                                        message: format!(
                                                            "{}: OI conversion deferred — waiting for first mark price",
                                                            internal_symbol
                                                        ),
                                                    })
                                                    .await;
                                            }
                                            continue;
                                        }
                                    };
                                    let _ = event_tx
                                        .send(NormalizedEvent::OpenInterest(
                                            core_domain::normalized::OpenInterestEvent {
                                                symbol: internal_symbol.clone(),
                                                oi: oi_usd,
                                                prev_oi: prev_oi_usd,
                                            },
                                        ))
                                        .await;
                                }
                            }
                        }
                    }
                    "fill" => {
                        // Bitget `fill` channel payload: array of fills. Each fill
                        // includes an `execType` field — "L" is a forced-close
                        // liquidation fill.
                        emit_bitget_fill_liquidations(&internal_symbol, &data_val, &event_tx).await;
                    }
                    _ => {}
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

// =============================================================================
// Bitget fill-channel liquidation extraction (Phase 1)
// =============================================================================
//
// Bitget's `fill` channel payload includes `execType` with values:
//
//   T — taker fill (normal)
//   M — maker fill
//   L — liquidation (forced close by the engine)
//
// Only the "L" type is converted to a `NormalizedEvent::Liquidation`.
// Side semantics: `side == "buy"` means the aggressor bought, which on a
// liquidation closes a SHORT (short squeeze). `side == "sell"` closes a
// LONG.

#[derive(Debug, serde::Deserialize)]
struct BitgetFillItem {
    #[serde(rename = "tradeId", default)]
    trade_id: Option<String>,
    price: Option<String>,
    size: Option<String>,
    side: Option<String>,
    ts: Option<String>,
    #[serde(rename = "execType", default)]
    exec_type: Option<String>,
}

async fn emit_bitget_fill_liquidations(
    internal_symbol: &str,
    data_val: &serde_json::Value,
    event_tx: &tokio::sync::mpsc::Sender<NormalizedEvent>,
) {
    emit_bitget_fill_liquidations_impl(internal_symbol, data_val, event_tx).await;
}

/// Test-only entry point that re-exposes the liquidation parser without
/// the production-side `Sender::send` constraint. Used by
/// `crates/network-adapters/tests/bitget_liquidation_schema.rs`.
pub async fn emit_bitget_fill_liquidations_for_test(
    internal_symbol: &str,
    data_val: &serde_json::Value,
    event_tx: &tokio::sync::mpsc::Sender<NormalizedEvent>,
) {
    emit_bitget_fill_liquidations_impl(internal_symbol, data_val, event_tx).await;
}

async fn emit_bitget_fill_liquidations_impl(
    internal_symbol: &str,
    data_val: &serde_json::Value,
    event_tx: &tokio::sync::mpsc::Sender<NormalizedEvent>,
) {
    let fills: Vec<BitgetFillItem> = match serde_json::from_value(data_val.clone()) {
        Ok(f) => f,
        Err(_) => return,
    };
    for fill in fills {
        let is_liquidation = fill
            .exec_type
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("L"))
            .unwrap_or(false);
        if !is_liquidation {
            continue;
        }
        let price = match fill
            .price
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
        {
            Some(p) => p,
            None => continue,
        };
        let size = match fill.size.as_deref().and_then(|s| Decimal::from_str(s).ok()) {
            Some(s) => s,
            None => continue,
        };
        let ts_ms: u64 = fill
            .ts
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0)
            });
        // Side semantics for liquidations:
        //   side == "sell"  -> aggressor sold  -> a long was closed
        //   side == "buy"   -> aggressor bought -> a short was closed
        let liq_side = match fill.side.as_deref() {
            Some("buy") => core_domain::normalized::LiquidationSide::Short,
            _ => core_domain::normalized::LiquidationSide::Long,
        };
        let _ = event_tx
            .send(NormalizedEvent::Liquidation(
                core_domain::normalized::LiquidationEvent {
                    exchange: Exchange::Bitget,
                    symbol: internal_symbol.to_string(),
                    side: liq_side,
                    price,
                    size,
                    timestamp_ms: ts_ms,
                    venue_order_id: fill.trade_id.clone(),
                },
            ))
            .await;
    }
}

/// Test-only helper: build a USD-converted `OpenInterestEvent` from a
/// raw base-asset OI value and a known mark price. Returns `None` when
/// the mark price is missing or non-positive (the dispatcher's policy
/// in production: skip rather than poison the downstream USD series).
pub fn open_interest_event_for_test(
    raw_oi: f64,
    mark_px: f64,
    symbol: &str,
) -> Option<NormalizedEvent> {
    let oi_dec = Decimal::from_f64_retain(raw_oi)?;
    let mark_dec = Decimal::from_f64_retain(mark_px)?;
    if oi_dec <= Decimal::ZERO || mark_dec <= Decimal::ZERO {
        return None;
    }
    Some(NormalizedEvent::OpenInterest(
        core_domain::normalized::OpenInterestEvent {
            symbol: symbol.to_string(),
            oi: oi_dec * mark_dec,
            prev_oi: None,
        },
    ))
}
