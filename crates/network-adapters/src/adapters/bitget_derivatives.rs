//! Bitget derivatives telemetry helpers.
//!
//! Bitget publishes mark price on the V2 `ticker` channel and the current
//! funding rate on a dedicated `funding-rate` channel under the `mc` (mix
//! contract) `instType`. Both are pushed natively; this module exposes
//! lightweight parsers used by the per-symbol WebSocket adapter in
//! `bitget.rs`.

use rust_decimal::Decimal;
use serde::Deserialize;

use core_domain::normalized::{FundingRateEvent, MarkPriceEvent, NormalizedEvent};

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
}
