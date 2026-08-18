//! Bitget derivatives telemetry helpers.
//!
//! Bitget V2 (mix contract) publishes **mark price**, **open interest**, and
//! **funding rate** all on the single `ticker` channel under the
//! `USDT-FUTURES` / `USDC-FUTURES` `instType`. Earlier V1 documentation
//! described separate `open-interest` / `funding-rate` channels but those
//! were removed in V2 — the data now rides on the ticker push with field
//! names `markPrice`, `holdingAmount` (OI in base-asset units),
//! `fundingRate`, and `nextFundingTime`. This module exposes the
//! lightweight parsers used by the per-symbol WebSocket adapter in
//! `bitget.rs`.
//!
//! ## Liquidation channels — semantic inversion
//!
//! Bitget exposes liquidation flow on **two** channels, and they have
//! **inverted** side semantics. The unified `NormalizedEvent::Liquidation`
//! produced by both paths always reports the side that *got liquidated*
//! (long vs short), but the source payloads disagree on what `buy` /
//! `sell` mean:
//!
//! | Channel                 | `side == "buy"` means      | `side == "sell"` means     |
//! |-------------------------|----------------------------|----------------------------|
//! | `fill` (execType=`L`)   | short was closed (squeeze) | long was closed (dump)     |
//! | `liquidation` (public)  | **long** was closed (dump) | **short** was closed (sqz) |
//!
//! The `fill`-channel parser uses the table's first row
//! (`emit_bitget_fill_liquidations_impl`). The public-channel parser
//! here uses the second row (`pub_liquidation_to_event`). **Do not
//! confuse them** — swapping the mapping silently inverts the bands on
//! the heatmap.

use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;

use core_domain::normalized::{
    FundingRateEvent, LiquidationEvent, LiquidationSide, MarkPriceEvent, NormalizedEvent,
    OpenInterestEvent,
};

/// V2 ticker push payload (mix contract: USDT-FUTURES / USDC-FUTURES).
///
/// Bitget V2 ships mark price, open interest (`holdingAmount`, base-asset
/// units), funding rate, and next-funding time all on the single `ticker`
/// channel. Earlier V1 documentation described dedicated `open-interest`
/// and `funding-rate` channels but those no longer exist on V2 — the
/// fields are pushed together with the rest of the ticker snapshot.
#[derive(Debug, Deserialize)]
pub struct BitgetTickerData {
    #[serde(rename = "markPrice", default)]
    pub mark_price: Option<String>,
    #[serde(rename = "indexPrice", default)]
    pub index_price: Option<String>,
    #[serde(rename = "open24h", default)]
    pub open_24h: Option<String>,
    /// Open interest in **base-asset units** (contracts on USDT-M perps).
    /// Must be multiplied by the mark price to obtain the USD notional
    /// the cluster estimator expects.
    #[serde(rename = "holdingAmount", default)]
    pub holding_amount: Option<String>,
    /// Current funding rate (per-8h decimal).
    #[serde(rename = "fundingRate", default)]
    pub funding_rate: Option<String>,
    /// Next funding time as a 13-digit ms timestamp.
    #[serde(rename = "nextFundingTime", default)]
    pub next_funding_time: Option<String>,
}

/// Bitget public `liquidation` channel payload — one record per side per
/// symbol per second, carrying the **highest-quantity** forced close in
/// that window (per the docs: "only the record with the highest
/// liquidation quantity for long and short positions respectively per
/// trading pair is included").
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct BitgetPublicLiquidationData {
    #[serde(default)]
    pub symbol: Option<String>,
    /// Per the docs: `buy` ⇒ long-liquidation, `sell` ⇒ short-liquidation.
    /// This is the **opposite** of the `fill`-channel convention; see the
    /// module-level inversion table.
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub price: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
}

/// Convert a parsed Bitget V2 ticker payload to a `MarkPriceEvent`.
/// Returns `None` if no mark price is present.
pub fn ticker_to_mark_price(
    internal_symbol: &str,
    data: &BitgetTickerData,
) -> Option<NormalizedEvent> {
    let mark = data.mark_price.as_deref().and_then(|s| s.parse().ok())?;
    let idx = data.index_price.as_deref().and_then(|s| s.parse().ok());
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Some(NormalizedEvent::MarkPrice(MarkPriceEvent {
        symbol: internal_symbol.to_string(),
        mark_px: mark,
        index_px: idx,
        timestamp_ms: ts,
    }))
}

/// Convert a parsed Bitget V2 ticker payload to its full derivatives event
/// triple: MarkPrice (always when mark present), OpenInterest (USD-converted
/// via `mark_px` when `holdingAmount` parses), FundingRate (when
/// `fundingRate` parses).
///
/// Mirrors Hyperliquid's `hl_derivatives_poller::derivatives_ctx_to_events`
/// in shape and downstream event variants — same `NormalizedEvent`
/// surface, same `prev_oi: None` on the first OI observation (cluster
/// estimator derives deltas from history).
///
/// `mark_px_override` is used by the adapter to pass a previously-cached
/// mark price from a prior ticker frame when the current frame lacks one
/// (first-frame race); pass `None` if no cached mark is available.
pub fn ticker_to_derivatives_events(
    internal_symbol: &str,
    data: &BitgetTickerData,
    mark_px_override: Option<Decimal>,
) -> Vec<NormalizedEvent> {
    let mut out: Vec<NormalizedEvent> = Vec::with_capacity(3);

    // 1. Mark price (always first so downstream has it for OI conversion).
    let parsed_mark = data
        .mark_price
        .as_deref()
        .and_then(|s| s.parse::<Decimal>().ok());
    if let Some(mark) = parsed_mark {
        let idx = data
            .index_price
            .as_deref()
            .and_then(|s| s.parse::<Decimal>().ok());
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        out.push(NormalizedEvent::MarkPrice(MarkPriceEvent {
            symbol: internal_symbol.to_string(),
            mark_px: mark,
            index_px: idx,
            timestamp_ms: ts,
        }));
    }

    // 2. Open interest in USD — only if we have a positive mark price.
    //    `holdingAmount` arrives in base-asset units (contracts on USDT-M
    //    perps), so we multiply by the mark price for the USD notional.
    let effective_mark = parsed_mark
        .filter(|m| *m > Decimal::ZERO)
        .or(mark_px_override.filter(|m| *m > Decimal::ZERO));
    if let (Some(mark), Some(raw_oi_str)) = (effective_mark, data.holding_amount.as_deref()) {
        if let Ok(raw_oi) = Decimal::from_str(raw_oi_str) {
            if raw_oi > Decimal::ZERO {
                let oi_usd = raw_oi * mark;
                out.push(NormalizedEvent::OpenInterest(OpenInterestEvent {
                    symbol: internal_symbol.to_string(),
                    oi: oi_usd,
                    prev_oi: None,
                }));
            }
        }
    }

    // 3. Funding rate.
    if let Some(rate_str) = data.funding_rate.as_deref() {
        if let Ok(rate) = Decimal::from_str(rate_str) {
            out.push(NormalizedEvent::FundingRate(FundingRateEvent {
                symbol: internal_symbol.to_string(),
                rate,
            }));
        }
    }

    out
}

/// Convert a parsed Bitget public `liquidation` payload to a
/// `NormalizedEvent::Liquidation`.
///
/// Side semantics (OPPOSITE of the `fill` channel):
///   - `side == "buy"`  → the position that got closed was a **Long**
///   - `side == "sell"` → the position that got closed was a **Short**
///
/// Returns `None` if price or size are missing/non-parseable. The
/// `amount` field is the base-asset quantity; we emit it as `size`
/// directly — the per-event USD notional is computed downstream by
/// `LiquidityEventAccumulator` as `price * size`.
pub fn pub_liquidation_to_event(
    internal_symbol: &str,
    data: &BitgetPublicLiquidationData,
) -> Option<NormalizedEvent> {
    let price = data
        .price
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok())?;
    let size = data
        .amount
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok())?;
    let ts_ms: u64 = data
        .ts
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        });
    // Note the opposite mapping relative to `emit_bitget_fill_liquidations_impl`.
    let side = match data.side.as_deref() {
        Some("buy") => LiquidationSide::Long,
        Some("sell") => LiquidationSide::Short,
        // Unknown / missing side is dropped — better to skip than
        // misclassify on the heatmap, which has no notion of "unknown".
        _ => return None,
    };
    Some(NormalizedEvent::Liquidation(LiquidationEvent {
        exchange: core_domain::normalized::Exchange::Bitget,
        symbol: internal_symbol.to_string(),
        side,
        price,
        size,
        timestamp_ms: ts_ms,
        venue_order_id: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_ticker() -> BitgetTickerData {
        BitgetTickerData {
            mark_price: None,
            index_price: None,
            open_24h: None,
            holding_amount: None,
            funding_rate: None,
            next_funding_time: None,
        }
    }

    #[test]
    fn ticker_to_mark_price_extracts_mark() {
        let mut d = mk_ticker();
        d.mark_price = Some("100.5".into());
        d.index_price = Some("100.4".into());
        d.open_24h = Some("99.0".into());
        let ev = ticker_to_mark_price("BTC-USDT", &d).unwrap();
        match ev {
            NormalizedEvent::MarkPrice(m) => {
                assert_eq!(m.symbol, "BTC-USDT");
                assert_eq!(m.mark_px.to_string(), "100.5");
                assert_eq!(m.index_px.unwrap().to_string(), "100.4");
            }
            _ => panic!("expected MarkPrice event"),
        }
    }

    #[test]
    fn ticker_to_mark_price_returns_none_when_absent() {
        let d = mk_ticker();
        assert!(ticker_to_mark_price("BTC-USDT", &d).is_none());
    }

    #[test]
    fn ticker_funding_emits_funding_rate() {
        let d = BitgetTickerData {
            mark_price: Some("50100.5".into()),
            index_price: None,
            open_24h: None,
            holding_amount: None,
            funding_rate: Some("0.0001".into()),
            next_funding_time: None,
        };
        let events = ticker_to_derivatives_events("BTC-USDT", &d, None);
        let fr = events
            .iter()
            .find_map(|e| match e {
                NormalizedEvent::FundingRate(f) => Some(f),
                _ => None,
            })
            .expect("ticker funding must emit a FundingRate event");
        assert_eq!(fr.symbol, "BTC-USDT");
        assert_eq!(fr.rate.to_string(), "0.0001");
    }

    /// V2 ticker: only mark_price present — emit MarkPrice only.
    #[test]
    fn ticker_to_derivatives_emits_mark_only_when_only_mark_present() {
        let mut d = mk_ticker();
        d.mark_price = Some("65000".into());
        let evs = ticker_to_derivatives_events("BTC-USDT", &d, None);
        assert_eq!(evs.len(), 1);
        assert!(matches!(evs[0], NormalizedEvent::MarkPrice(_)));
    }

    /// V2 ticker: full payload — emit all three events in order.
    #[test]
    fn ticker_to_derivatives_emits_all_three_when_payload_full() {
        let mut d = mk_ticker();
        d.mark_price = Some("65000".into());
        d.holding_amount = Some("1000".into());
        d.funding_rate = Some("0.0001".into());
        let evs = ticker_to_derivatives_events("BTC-USDT", &d, None);
        assert_eq!(
            evs.len(),
            3,
            "expected MarkPrice + OpenInterest + FundingRate"
        );
        // Order: MarkPrice, OpenInterest, FundingRate.
        let NormalizedEvent::MarkPrice(mp) = &evs[0] else {
            panic!("expected MarkPrice at index 0");
        };
        assert_eq!(mp.mark_px.to_string(), "65000");
        let NormalizedEvent::OpenInterest(oi) = &evs[1] else {
            panic!("expected OpenInterest at index 1");
        };
        // 1000 contracts * $65_000 = $65_000_000 USD notional.
        assert_eq!(oi.oi.to_string(), "65000000");
        assert!(oi.prev_oi.is_none());
        let NormalizedEvent::FundingRate(fr) = &evs[2] else {
            panic!("expected FundingRate at index 2");
        };
        assert_eq!(fr.rate.to_string(), "0.0001");
    }

    /// V2 ticker: holdingAmount present but mark_price absent — OI is
    /// dropped (first-frame race); mark_px_override can rescue it.
    #[test]
    fn ticker_to_derivatives_emits_oi_with_cached_mark_when_frame_lacks_mark() {
        let mut d = mk_ticker();
        d.holding_amount = Some("500".into());
        d.funding_rate = Some("0.00005".into());
        // Override mark as 60_000.
        let evs =
            ticker_to_derivatives_events("BTC-USDT", &d, Some(Decimal::from_str("60000").unwrap()));
        // Funding rate emits without needing mark.
        // OI emits using override mark: 500 * 60000 = 30_000_000.
        // MarkPrice is NOT emitted (frame didn't carry one).
        assert_eq!(evs.len(), 2);
        let NormalizedEvent::OpenInterest(oi) = &evs[0] else {
            panic!("expected OpenInterest first");
        };
        assert_eq!(oi.oi.to_string(), "30000000");
        let NormalizedEvent::FundingRate(fr) = &evs[1] else {
            panic!("expected FundingRate second");
        };
        assert_eq!(fr.rate.to_string(), "0.00005");
    }

    /// V2 ticker: holdingAmount present, mark_price absent, no override —
    /// OI must be DROPPED (cannot convert base-asset to USD safely).
    #[test]
    fn ticker_to_derivatives_drops_oi_when_no_mark_available() {
        let mut d = mk_ticker();
        d.holding_amount = Some("500".into());
        d.funding_rate = Some("0.00005".into());
        let evs = ticker_to_derivatives_events("BTC-USDT", &d, None);
        // No MarkPrice event because frame lacks markPrice.
        // OI is dropped (no mark to convert).
        // FundingRate still emits.
        assert_eq!(evs.len(), 1);
        assert!(matches!(evs[0], NormalizedEvent::FundingRate(_)));
    }

    /// V2 ticker: funding_rate absent — OI + Mark still emit.
    #[test]
    fn ticker_to_derivatives_emits_oi_without_funding() {
        let mut d = mk_ticker();
        d.mark_price = Some("65000".into());
        d.holding_amount = Some("1000".into());
        let evs = ticker_to_derivatives_events("BTC-USDT", &d, None);
        assert_eq!(evs.len(), 2);
        assert!(matches!(evs[0], NormalizedEvent::MarkPrice(_)));
        assert!(matches!(evs[1], NormalizedEvent::OpenInterest(_)));
    }

    /// V2 ticker: holdingAmount absent — only mark + funding emit.
    #[test]
    fn ticker_to_derivatives_emits_funding_without_oi() {
        let mut d = mk_ticker();
        d.mark_price = Some("65000".into());
        d.funding_rate = Some("0.0001".into());
        let evs = ticker_to_derivatives_events("BTC-USDT", &d, None);
        assert_eq!(evs.len(), 2);
        assert!(matches!(evs[0], NormalizedEvent::MarkPrice(_)));
        assert!(matches!(evs[1], NormalizedEvent::FundingRate(_)));
    }

    /// V2 ticker: completely empty payload — no events emitted.
    #[test]
    fn ticker_to_derivatives_emits_nothing_when_empty() {
        let d = mk_ticker();
        let evs = ticker_to_derivatives_events("BTC-USDT", &d, None);
        assert!(evs.is_empty());
    }

    /// V2 ticker: holdingAmount is "0" — dropped (zero OI is not a
    /// meaningful reading, and would confuse the cluster estimator).
    #[test]
    fn ticker_to_derivatives_drops_zero_oi() {
        let mut d = mk_ticker();
        d.mark_price = Some("65000".into());
        d.holding_amount = Some("0".into());
        let evs = ticker_to_derivatives_events("BTC-USDT", &d, None);
        // Only MarkPrice emits; OI dropped.
        assert_eq!(evs.len(), 1);
        assert!(matches!(evs[0], NormalizedEvent::MarkPrice(_)));
    }

    /// CRITICAL: Bitget's public `liquidation` channel reports `side == "buy"`
    /// for long liquidations — the **opposite** of the `fill`-channel convention.
    /// Swapping this mapping silently inverts the heatmap colors.
    #[test]
    fn pub_liquidation_side_buy_is_long_liquidation() {
        let d = BitgetPublicLiquidationData {
            symbol: Some("BTCUSDT".into()),
            side: Some("buy".into()),
            price: Some("65000.50".into()),
            amount: Some("1.5".into()),
            ts: Some("1700000000000".into()),
        };
        let ev = pub_liquidation_to_event("BTC-USDT", &d).unwrap();
        match ev {
            NormalizedEvent::Liquidation(l) => {
                assert_eq!(l.symbol, "BTC-USDT");
                assert_eq!(l.side, LiquidationSide::Long);
                assert_eq!(l.price.to_string(), "65000.50");
                assert_eq!(l.size.to_string(), "1.5");
                assert_eq!(l.timestamp_ms, 1_700_000_000_000);
                assert_eq!(l.exchange, core_domain::normalized::Exchange::Bitget);
            }
            _ => panic!("expected Liquidation event"),
        }
    }

    #[test]
    fn pub_liquidation_side_sell_is_short_liquidation() {
        let d = BitgetPublicLiquidationData {
            symbol: Some("BTCUSDT".into()),
            side: Some("sell".into()),
            price: Some("65000.50".into()),
            amount: Some("2.0".into()),
            ts: Some("1700000000000".into()),
        };
        let ev = pub_liquidation_to_event("BTC-USDT", &d).unwrap();
        match ev {
            NormalizedEvent::Liquidation(l) => {
                assert_eq!(l.side, LiquidationSide::Short);
                assert_eq!(l.size.to_string(), "2.0");
            }
            _ => panic!("expected Liquidation event"),
        }
    }

    #[test]
    fn pub_liquidation_missing_side_is_dropped() {
        // Defensive: unknown / missing side is dropped rather than guessed.
        let d = BitgetPublicLiquidationData {
            symbol: Some("BTCUSDT".into()),
            side: None,
            price: Some("65000".into()),
            amount: Some("1".into()),
            ts: Some("1700000000000".into()),
        };
        assert!(pub_liquidation_to_event("BTC-USDT", &d).is_none());
    }

    #[test]
    fn pub_liquidation_unknown_side_is_dropped() {
        let d = BitgetPublicLiquidationData {
            symbol: Some("BTCUSDT".into()),
            side: Some("mystery".into()),
            price: Some("65000".into()),
            amount: Some("1".into()),
            ts: Some("1700000000000".into()),
        };
        assert!(pub_liquidation_to_event("BTC-USDT", &d).is_none());
    }

    #[test]
    fn pub_liquidation_missing_price_is_dropped() {
        let d = BitgetPublicLiquidationData {
            symbol: Some("BTCUSDT".into()),
            side: Some("buy".into()),
            price: None,
            amount: Some("1".into()),
            ts: Some("1700000000000".into()),
        };
        assert!(pub_liquidation_to_event("BTC-USDT", &d).is_none());
    }
}
