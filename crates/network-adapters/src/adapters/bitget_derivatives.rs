//! Bitget derivatives telemetry helpers.
//!
//! Bitget publishes mark price on the V2 `ticker` channel and the current
//! funding rate on a dedicated `funding-rate` channel under the `mc` (mix
//! contract) `instType`. Both are pushed natively; this module exposes
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
};

#[derive(Debug, Deserialize)]
pub struct BitgetTickerData {
    #[serde(rename = "markPrice", default)]
    pub mark_price: Option<String>,
    #[serde(rename = "indexPrice", default)]
    pub index_price: Option<String>,
    #[serde(rename = "open24h", default)]
    pub open_24h: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct BitgetFundingData {
    #[serde(default, rename = "fundingRate")]
    pub fundingRate: Option<String>,
    #[serde(default, rename = "nextUpdate")]
    pub nextUpdate: Option<u64>,
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

/// Convert a parsed Bitget ticker payload to a `MarkPriceEvent`.
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

/// Convert a parsed Bitget funding payload to a `FundingRateEvent`.
/// Returns `None` if no rate is present.
pub fn funding_to_event(
    internal_symbol: &str,
    data: &BitgetFundingData,
) -> Option<NormalizedEvent> {
    let rate = data
        .fundingRate
        .as_deref()
        .and_then(|s| s.parse::<Decimal>().ok())?;
    Some(NormalizedEvent::FundingRate(FundingRateEvent {
        symbol: internal_symbol.to_string(),
        rate,
    }))
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

    #[test]
    fn ticker_to_mark_price_extracts_mark() {
        let d = BitgetTickerData {
            mark_price: Some("100.5".into()),
            index_price: Some("100.4".into()),
            open_24h: Some("99.0".into()),
        };
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
        let d = BitgetTickerData {
            mark_price: None,
            index_price: None,
            open_24h: Some("99".into()),
        };
        assert!(ticker_to_mark_price("BTC-USDT", &d).is_none());
    }

    #[test]
    fn funding_to_event_extracts_rate() {
        let d = BitgetFundingData {
            fundingRate: Some("0.0001".into()),
            nextUpdate: Some(1700000000000),
        };
        let ev = funding_to_event("BTC-USDT", &d).unwrap();
        match ev {
            NormalizedEvent::FundingRate(f) => {
                assert_eq!(f.symbol, "BTC-USDT");
                assert_eq!(f.rate.to_string(), "0.0001");
            }
            _ => panic!("expected FundingRate event"),
        }
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
                assert_eq!(
                    l.exchange,
                    core_domain::normalized::Exchange::Bitget
                );
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
