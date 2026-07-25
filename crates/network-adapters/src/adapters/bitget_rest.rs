use rust_decimal::Decimal;
use serde::Deserialize;
use core_domain::normalized::{Exchange, NormalizedCandle, ReconstructionMethod};

#[derive(Debug, Deserialize)]
struct BitgetCandleResponse {
    code: String,
    #[allow(dead_code)]
    msg: Option<String>,
    data: Option<Vec<Vec<String>>>,
}

fn parse_decimal(s: &str) -> Result<Decimal, String> {
    s.parse::<Decimal>()
        .map_err(|e| format!("Failed to parse decimal '{}': {}", s, e))
}

/// Fetch historical candles from the Bitget V2 **mix (perpetual futures)**
/// market endpoint (`/api/v2/mix/market/candles`).
///
/// - `symbol` is the exchange-native contract symbol (e.g. `BTCUSDT` for
///   USDT-M, `BTCUSD` for USDC-M).
/// - `internal_symbol` is the unified workspace symbol (e.g. `BTC-USDT`,
///   `BTC-USDC`) assigned to every returned candle.
/// - `product_type` is the Bitget mix product type (`USDT-FUTURES`,
///   `USDC-FUTURES`).
/// - `interval` is the mix K-line granularity string (`1m`, `1H`, `1D`, ...).
/// - `start_time_ms` / `end_time_ms` are 13-digit millisecond timestamps.
/// - `limit` is the per-page cap (HFP-06). Bitget accepts any value but
///   empirically caps responses around 200.
pub async fn fetch_historical_candles_page(
    symbol: &str,
    internal_symbol: &str,
    product_type: &str,
    interval: &str,
    start_time_ms: u64,
    end_time_ms: u64,
    limit: u32,
    rest_url: &str,
) -> Result<Vec<NormalizedCandle>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // Bitget V2 mix market expects 13-digit millisecond timestamps — pass raw ms.
    let limit_str = limit.to_string();
    let response = client
        .get(rest_url)
        .query(&[
            ("symbol", symbol),
            ("productType", product_type),
            ("granularity", interval),
            ("startTime", &start_time_ms.to_string()),
            ("endTime", &end_time_ms.to_string()),
            ("limit", &limit_str),
        ])
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

    let candle_response: BitgetCandleResponse = response.json().await.map_err(|e| {
        format!(
            "Failed to parse candle JSON for {} {}: {}",
            symbol, interval, e
        )
    })?;

    if candle_response.code != "00000" {
        return Err(format!(
            "Bitget API error for {} {}: code={} msg={:?}",
            symbol, interval, candle_response.code, candle_response.msg
        ));
    }

    let rows = candle_response.data.unwrap_or_default();

    rows.into_iter()
        .map(|row| {
            if row.len() < 6 {
                return Err("Insufficient candle data fields".to_string());
            }
            let ts_ms_str = &row[0];
            let start_time_ms: u64 = ts_ms_str
                .parse::<u64>()
                .map_err(|e| format!("Failed to parse timestamp '{}': {}", ts_ms_str, e))?;
            let open = parse_decimal(&row[1])?;
            let high = parse_decimal(&row[2])?;
            let low = parse_decimal(&row[3])?;
            let close = parse_decimal(&row[4])?;
            let volume = parse_decimal(&row[5])?;

            let duration_ms = match interval {
                "1m" => 60_000,
                "3m" => 180_000,
                "5m" => 300_000,
                "15m" => 900_000,
                "30m" => 1_800_000,
                "1H" => 3_600_000,
                "4H" => 14_400_000,
                "6H" => 21_600_000,
                "12H" => 43_200_000,
                "1D" => 86_400_000,
                "1W" => 604_800_000,
                _ => 60_000,
            };

            Ok(NormalizedCandle {
                exchange: Exchange::Bitget,
                symbol: internal_symbol.to_string(),
                start_time_ms,
                duration_ms,
                open,
                high,
                low,
                close,
                volume,
                trades_count: 0,
                reconstructed: Some(ReconstructionMethod::ExchangeHistorical),
            })
        })
        .collect()
}

/// Backward-compatible wrapper that hardcodes `limit=200`. Use
/// [`fetch_historical_candles_page`] from new code.
pub async fn fetch_historical_candles(
    symbol: &str,
    internal_symbol: &str,
    product_type: &str,
    interval: &str,
    start_time_ms: u64,
    end_time_ms: u64,
    rest_url: &str,
) -> Result<Vec<NormalizedCandle>, String> {
    fetch_historical_candles_page(
        symbol,
        internal_symbol,
        product_type,
        interval,
        start_time_ms,
        end_time_ms,
        200,
        rest_url,
    )
    .await
}

#[derive(Debug, Deserialize)]
struct BitgetTickerResponse {
    code: String,
    data: Option<Vec<serde_json::Value>>,
}

/// Verify that a Bitget mix (perpetual futures) contract symbol exists for the
/// given product type by querying the ticker endpoint.
///
/// Returns `Ok(true)` if the contract is tradeable, `Ok(false)` if the exchange
/// reports it as unknown, and `Err(..)` only on transport/parse failures so the
/// caller can distinguish "not available" from "couldn't check".
pub async fn symbol_exists(
    symbol: &str,
    product_type: &str,
    ticker_url: &str,
) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(ticker_url)
        .query(&[("symbol", symbol), ("productType", product_type)])
        .send()
        .await
        .map_err(|e| format!("Symbol check request failed for {}: {}", symbol, e))?;

    if !response.status().is_success() {
        // A 400 here typically means the symbol/productType pair is invalid.
        return Ok(false);
    }

    let parsed: BitgetTickerResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse ticker JSON for {}: {}", symbol, e))?;

    // "00000" with a non-empty data array => the contract exists.
    let ok = parsed.code == "00000" && parsed.data.as_ref().is_some_and(|d| !d.is_empty());
    Ok(ok)
}

/// Map an internal timeframe duration in seconds to the Bitget V2 **mix**
/// K-line granularity string. Mix uses lowercase minutes (`1m`) and uppercase
/// hours/days/weeks (`1H`, `1D`, `1W`) — distinct from the spot format.
pub fn timeframe_secs_to_interval(secs: u64) -> &'static str {
    match secs {
        60 => "1m",
        180 => "3m",
        300 => "5m",
        900 => "15m",
        1800 => "30m",
        3600 => "1H",
        14400 => "4H",
        21600 => "6H",
        43200 => "12H",
        86400 => "1D",
        604800 => "1W",
        other if other < 60 => "1m",
        other if other < 180 => "1m",
        other if other < 300 => "3m",
        other if other < 900 => "5m",
        other if other < 1800 => "15m",
        other if other < 3600 => "30m",
        other if other < 14400 => "1H",
        other if other < 43200 => "4H",
        other if other < 86400 => "12H",
        _ => "1D",
    }
}
