//! Hyperliquid derivatives telemetry poller.
//!
//! Hyperliquid does not push mark price, open interest, or funding rate
//! updates over the public WebSocket. Instead, the `/info` REST endpoint
//! with `{"type":"metaAndAssetCtxs"}` returns the per-asset context for the
//! entire universe in a single request. We poll this endpoint on a timer
//! (default 60s) per active pair and emit one `OpenInterestEvent`, one
//! `FundingRateEvent`, and one `MarkPriceEvent` per successful round-trip.
//!
//! Cancellation: the supplied `CancellationToken` cleanly stops the loop on
//! instance shutdown. Failures are logged and the loop continues with a
//! shorter retry cooldown (1s) to recover from transient REST errors.

use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

use shared::normalized::NormalizedEvent;

use crate::adapters::hyperliquid_rest::{
    derivatives_ctx_to_events, fetch_meta_and_asset_ctxs, HlDerivativesCtx,
};

/// Run the mark-price/OI/funding poller for a single Hyperliquid symbol.
pub async fn run_hl_derivatives_poller(
    raw_symbol: String,
    internal_symbol: String,
    info_url: String,
    event_tx: Sender<NormalizedEvent>,
    cancel: CancellationToken,
    poll_ms: u64,
) {
    println!(
        "💹 HL Derivatives Poller: Started for {} ({}ms cadence)",
        raw_symbol, poll_ms
    );

    let mut prev_oi: Option<Decimal> = None;
    let mut consecutive_failures: u32 = 0;
    let mut interval = tokio::time::interval(Duration::from_millis(poll_ms.max(1000)));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                println!(
                    "🛑 HL Derivatives Poller: {} cancelled, shutting down.",
                    raw_symbol
                );
                break;
            }
            _ = interval.tick() => {}
        }

        match fetch_meta_and_asset_ctxs(&info_url).await {
            Ok(map) => {
                consecutive_failures = 0;
                if let Some(ctx) = lookup_ctx(&map, &raw_symbol) {
                    let events = derivatives_ctx_to_events(&internal_symbol, ctx, prev_oi);
                    if let Some(oi) = ctx.open_interest {
                        prev_oi = Some(oi);
                    }
                    for ev in events {
                        if event_tx.send(ev).await.is_err() {
                            eprintln!("⚠️  HL Derivatives Poller: {} event_tx closed", raw_symbol);
                            return;
                        }
                    }
                } else {
                    eprintln!(
                        "⚠️  HL Derivatives Poller: {} not present in metaAndAssetCtxs response",
                        raw_symbol
                    );
                }
            }
            Err(e) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                eprintln!(
                    "⚠️  HL Derivatives Poller: {} failed ({} consecutive): {}",
                    raw_symbol, consecutive_failures, e
                );
                if consecutive_failures >= 5 {
                    interval = tokio::time::interval(Duration::from_secs(10));
                }
            }
        }
    }
}

/// Look up a coin in the metaAndAssetCtxs response map. Tries the raw name,
/// then the uppercased variant (Hyperliquid universe is case-sensitive but
/// our `SymbolMapper` may store either form depending on the coin).
pub fn lookup_ctx<'a>(
    map: &'a HashMap<String, HlDerivativesCtx>,
    raw_symbol: &str,
) -> Option<&'a HlDerivativesCtx> {
    if let Some(c) = map.get(raw_symbol) {
        return Some(c);
    }
    if let Some(c) = map.get(&raw_symbol.to_uppercase()) {
        return Some(c);
    }
    None
}

/// Spawn a poller for a Hyperliquid pair and return its JoinHandle.
///
/// Returns `None` if the pair is not Hyperliquid.
pub fn spawn_hl_derivatives_poller(
    raw_symbol: String,
    internal_symbol: String,
    info_url: String,
    event_tx: Arc<Sender<NormalizedEvent>>,
    cancel: CancellationToken,
    poll_ms: u64,
) -> Option<tokio::task::JoinHandle<()>> {
    if raw_symbol.is_empty() || info_url.is_empty() {
        return None;
    }
    Some(tokio::spawn(async move {
        run_hl_derivatives_poller(
            raw_symbol,
            internal_symbol,
            info_url,
            event_tx.as_ref().clone(),
            cancel,
            poll_ms,
        )
        .await;
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_ctx_handles_case_variants() {
        let mut m = HashMap::new();
        m.insert("BTC".to_string(), HlDerivativesCtx::default());
        assert!(lookup_ctx(&m, "BTC").is_some());
        assert!(lookup_ctx(&m, "btc").is_some());
        assert!(lookup_ctx(&m, "ETH").is_none());
    }
}
