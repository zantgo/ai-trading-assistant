use rust_decimal::Decimal;
use serde::Deserialize;
use shared::normalized::NormalizedCandle;

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
                symbol: format!("{}-USD", cs.coin),
                start_time_ms: start,
                duration_ms: duration,
                open: parse_decimal(&cs.open)?,
                high: parse_decimal(&cs.high)?,
                low: parse_decimal(&cs.low)?,
                close: parse_decimal(&cs.close)?,
                volume: parse_decimal(&cs.volume)?,
                trades_count: cs.trades_count,
            })
        })
        .collect()
}

/// Map an internal timeframe duration in seconds to the Hyperliquid REST interval string.
pub fn timeframe_secs_to_interval(secs: u64) -> &'static str {
    match secs {
        60 => "1m",
        300 => "5m",
        900 => "15m",
        3600 => "1h",
        other => {
            if other < 3601 {
                "15m"
            } else {
                "1h"
            }
        }
    }
}
