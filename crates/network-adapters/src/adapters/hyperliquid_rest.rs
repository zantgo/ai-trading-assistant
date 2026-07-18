use rust_decimal::Decimal;
use serde::Deserialize;
use core_domain::normalized::{
    Exchange, FundingRateEvent, MarkPriceEvent, NormalizedCandle, NormalizedEvent,
    OpenInterestEvent, ReconstructionMethod,
};

#[derive(Debug, Deserialize)]
struct CandleSnapshot {
    #[serde(rename = "t")]
    start_time_ms: u64,
    #[allow(dead_code)]
    #[serde(rename = "T")]
    end_time_ms: u64,
    #[allow(dead_code)]
    #[serde(rename = "s")]
    coin: String,
    #[allow(dead_code)]
    #[serde(rename = "i")]
    interval: String,
    #[serde(rename = "o")]
    open: String,
    #[serde(rename = "c")]
    close: String,
    #[serde(rename = "h")]
    high: String,
    #[serde(rename = "l")]
    low: String,
    #[serde(rename = "v")]
    volume: String,
    #[serde(rename = "n")]
    trades_count: u64,
}

fn parse_decimal(s: &str) -> Result<Decimal, String> {
    s.parse::<Decimal>()
        .map_err(|e| format!("Failed to parse decimal '{}': {}", s, e))
}

/// Fetch historical candles from the Hyperliquid Info REST API.
///
/// `rest_url` is the derived `/info` endpoint (e.g. `https://api.hyperliquid.xyz/info`).
/// `symbol` is the raw exchange coin name (e.g. `"BTC"`).
/// `interval` is the Hyperliquid interval string (e.g. `"1m"`, `"5m"`, `"15m"`, `"1h"`).
/// `start_time_ms` and `end_time_ms` bound the candle range.
pub async fn fetch_historical_candles(
    symbol: &str,
    internal_symbol: &str,
    interval: &str,
    start_time_ms: u64,
    end_time_ms: u64,
    rest_url: &str,
) -> Result<Vec<NormalizedCandle>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let request_body = serde_json::json!({
        "type": "candleSnapshot",
        "req": {
            "coin": symbol,
            "interval": interval,
            "startTime": start_time_ms,
            "endTime": end_time_ms,
        }
    });

    let response = client
        .post(rest_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("REST request failed for {} {}: {}", symbol, interval, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "REST endpoint returned HTTP {} for {} {}: {}",
            status, symbol, interval, body
        ));
    }

    let snapshots: Vec<CandleSnapshot> = response.json().await.map_err(|e| {
        format!(
            "Failed to parse candle snapshot JSON for {} {}: {}",
            symbol, interval, e
        )
    })?;

    snapshots
        .into_iter()
        .map(|cs| {
            let start = cs.start_time_ms;
            let duration = cs.end_time_ms.saturating_sub(cs.start_time_ms);
            Ok(NormalizedCandle {
                exchange: Exchange::Hyperliquid,
                symbol: internal_symbol.to_string(),
                start_time_ms: start,
                duration_ms: duration,
                open: parse_decimal(&cs.open)?,
                high: parse_decimal(&cs.high)?,
                low: parse_decimal(&cs.low)?,
                close: parse_decimal(&cs.close)?,
                volume: parse_decimal(&cs.volume)?,
                trades_count: cs.trades_count,
                reconstructed: Some(ReconstructionMethod::ExchangeHistorical),
            })
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct HlAsset {
    name: String,
}

#[derive(Debug, Deserialize)]
struct HlMeta {
    universe: Vec<HlAsset>,
}

/// Verify that a Hyperliquid perpetual coin exists by querying the `meta`
/// endpoint and checking the asset universe.
///
/// Returns `Ok(true)` if the coin is listed, `Ok(false)` if not, and `Err(..)`
/// only on transport/parse failures.
pub async fn symbol_exists(coin: &str, info_url: &str) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(info_url)
        .json(&serde_json::json!({ "type": "meta" }))
        .send()
        .await
        .map_err(|e| format!("Symbol check request failed for {}: {}", coin, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Hyperliquid meta endpoint returned HTTP {}",
            response.status()
        ));
    }

    let meta: HlMeta = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Hyperliquid meta JSON: {}", e))?;

    let target = coin.to_uppercase();
    let ok = meta
        .universe
        .iter()
        .any(|a| a.name.to_uppercase() == target);
    Ok(ok)
}

// =============================================================================
// Derivatives Telemetry — metaAndAssetCtxs polling
// =============================================================================
//
// Hyperliquid does not expose OI, mark price, or funding rate on the public
// WebSocket. Instead, the REST endpoint `/info` with
// `{"type":"metaAndAssetCtxs"}` returns the per-asset context for the entire
// universe in a single request. We poll this endpoint on a timer
// (default 60s) per active pair and emit one `OpenInterestEvent`, one
// `FundingRateEvent`, and one `MarkPriceEvent` per successful round-trip.

#[derive(Debug, Deserialize)]
struct MetaAndAssetCtxsResponse(
    #[allow(dead_code)] serde_json::Value, // meta (asset universe)
    Vec<AssetCtxEntry>,
);

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct AssetCtxEntry {
    #[allow(dead_code)]
    coin: String,
    #[serde(default, rename = "markPx")]
    markPx: Option<String>,
    #[serde(default, rename = "oraclePx")]
    oraclePx: Option<String>,
    #[serde(default, rename = "openInterest")]
    openInterest: Option<String>,
    #[serde(default)]
    funding: Option<String>,
    #[serde(default, rename = "prevDayPx")]
    prevDayPx: Option<String>,
}

/// Per-coin parsed derivatives context. All-`None` if a field was absent.
#[derive(Debug, Clone, Default)]
pub struct HlDerivativesCtx {
    pub mark_px: Option<Decimal>,
    pub oracle_px: Option<Decimal>,
    pub open_interest: Option<Decimal>,
    pub funding: Option<Decimal>,
    pub prev_day_px: Option<Decimal>,
}

/// Fetch and parse the full asset-ctx universe. Returns a map keyed by raw
/// coin name (e.g. "BTC"). Errors propagate.
pub async fn fetch_meta_and_asset_ctxs(
    info_url: &str,
) -> Result<std::collections::HashMap<String, HlDerivativesCtx>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(info_url)
        .json(&serde_json::json!({ "type": "metaAndAssetCtxs" }))
        .send()
        .await
        .map_err(|e| format!("Hyperliquid metaAndAssetCtxs request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Hyperliquid metaAndAssetCtxs HTTP {}",
            response.status()
        ));
    }

    let parsed: MetaAndAssetCtxsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Hyperliquid metaAndAssetCtxs: {}", e))?;

    let mut map = std::collections::HashMap::new();
    for entry in parsed.1 {
        let ctx = HlDerivativesCtx {
            mark_px: entry.markPx.as_deref().and_then(|s| s.parse().ok()),
            oracle_px: entry.oraclePx.as_deref().and_then(|s| s.parse().ok()),
            open_interest: entry.openInterest.as_deref().and_then(|s| s.parse().ok()),
            funding: entry.funding.as_deref().and_then(|s| s.parse().ok()),
            prev_day_px: entry.prevDayPx.as_deref().and_then(|s| s.parse().ok()),
        };
        map.insert(entry.coin, ctx);
    }
    Ok(map)
}

/// Convert a single `HlDerivativesCtx` snapshot into the normalized events
/// the analyzer expects. Each non-`None` field yields exactly one event.
/// `internal_symbol` is the unified workspace symbol (e.g. "BTC-USDT")
/// used on every emitted event.
pub fn derivatives_ctx_to_events(
    internal_symbol: &str,
    ctx: &HlDerivativesCtx,
    prev_oi: Option<Decimal>,
) -> Vec<NormalizedEvent> {
    let mut out = Vec::with_capacity(3);
    if let Some(oi) = ctx.open_interest {
        out.push(NormalizedEvent::OpenInterest(OpenInterestEvent {
            symbol: internal_symbol.to_string(),
            oi,
            prev_oi,
        }));
    }
    if let Some(rate) = ctx.funding {
        out.push(NormalizedEvent::FundingRate(FundingRateEvent {
            symbol: internal_symbol.to_string(),
            rate,
        }));
    }
    if let Some(mark) = ctx.mark_px {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        out.push(NormalizedEvent::MarkPrice(MarkPriceEvent {
            symbol: internal_symbol.to_string(),
            mark_px: mark,
            index_px: ctx.oracle_px,
            timestamp_ms: ts,
        }));
    }
    out
}

/// Map an internal timeframe duration in seconds to the Hyperliquid REST interval string.
pub fn timeframe_secs_to_interval(secs: u64) -> &'static str {
    match secs {
        60 => "1m",
        180 => "3m",
        300 => "5m",
        900 => "15m",
        1800 => "30m",
        3600 => "1h",
        7200 => "2h",
        14400 => "4h",
        28800 => "8h",
        43200 => "12h",
        86400 => "1d",
        other if other < 60 => "1m",
        other if other < 180 => "1m",
        other if other < 300 => "3m",
        other if other < 900 => "5m",
        other if other < 1800 => "15m",
        other if other < 3600 => "30m",
        other if other < 7200 => "1h",
        other if other < 14400 => "2h",
        other if other < 28800 => "4h",
        other if other < 43200 => "8h",
        other if other < 86400 => "12h",
        _ => "1d",
    }
}
