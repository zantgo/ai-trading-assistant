//! Bitget implementation of the [`HistoricalFetchPolicy`] trait.
//!
//! Per `docs/engines/data-infrastructure-engine/03-01-07-die-historical-fetch-policy.md`
//! (HFP-01, HFP-04, HFP-06, HFP-07, HFP-08, HFP-10).
//!
//! Pagination: forward `startTime` cursors. Each request asks for `limit=200`
//! candles. The implementation loops until either `target_count` is reached,
//! or a page returns fewer than 200 rows (exchange has no more history), or
//! `fetch_timeout_ms` elapses. The previous v6.4 behavior hardcoded
//! `limit=200` with no pagination, capping bootstrap at 200 rows from a cold
//! start — HFP-06 fixes that.

use async_trait::async_trait;
use core_domain::normalized::{Exchange, NormalizedCandle, ReconstructionMethod};
use std::time::{Duration, Instant};

use super::bitget_rest::fetch_historical_candles_page;
use super::historical_fetch::{
    HistoricalFetchError, HistoricalFetchPolicy, HistoricalFetchRequest,
};

/// Bitget hardcoded per-page limit (HFP-06).
const BITGET_PAGE_LIMIT: u32 = 200;

pub struct BitgetHistoricalFetch {
    /// Base REST URL of the V2 mix (perpetual futures) candles endpoint,
    /// e.g. `https://api.bitget.com/api/v2/mix/market/candles`.
    pub rest_url: String,
    /// Futures product type (`"USDT-FUTURES"` / `"USDC-FUTURES"`).
    pub product_type: String,
}

impl BitgetHistoricalFetch {
    pub fn new(rest_url: String, product_type: String) -> Self {
        Self {
            rest_url,
            product_type,
        }
    }
}

#[async_trait]
impl HistoricalFetchPolicy for BitgetHistoricalFetch {
    fn exchange(&self) -> Exchange {
        Exchange::Bitget
    }

    async fn fetch(
        &self,
        request: HistoricalFetchRequest,
    ) -> Result<Vec<NormalizedCandle>, HistoricalFetchError> {
        // HFP-03: sub-minute short-circuit.
        if request.timeframe_secs < 60 {
            return Err(HistoricalFetchError::SubMinuteBypassed(
                request.timeframe_secs,
            ));
        }

        let granularity = super::bitget_rest::timeframe_secs_to_interval(request.timeframe_secs);
        let started = Instant::now();
        let timeout = Duration::from_millis(request.fetch_timeout_ms);
        let duration_ms = request.timeframe_secs * 1000;

        let mut collected: Vec<NormalizedCandle> = Vec::with_capacity(request.target_count);
        let mut start_ts = request
            .end_ts
            .saturating_sub((request.target_count as u64) * duration_ms);

        loop {
            if started.elapsed() >= timeout {
                return Err(HistoricalFetchError::Timeout(
                    started.elapsed().as_millis() as u64,
                ));
            }
            if collected.len() >= request.target_count {
                break;
            }

            let page = fetch_historical_candles_page(
                &request.exchange_symbol,
                &request.internal_symbol,
                &self.product_type,
                granularity,
                start_ts,
                request.end_ts,
                BITGET_PAGE_LIMIT,
                &self.rest_url,
            )
            .await
            .map_err(|e| HistoricalFetchError::Http {
                status: 0,
                attempts: 0,
                body: e,
            })?;

            // HFP-07: drop currently-open candles.
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(request.end_ts);
            let page_len = page.len();
            let mut page = page;
            page.retain(|c| c.start_time_ms + duration_ms <= now_ms);

            if page.is_empty() {
                break;
            }

            // Advance cursor to one interval past the last (newest) candle we
            // just got. Bitget returns candles newest-first within a page.
            let last_start = page.iter().map(|c| c.start_time_ms).max().unwrap_or(start_ts);
            let next_cursor = last_start.saturating_add(duration_ms);

            // Defensive: if cursor doesn't advance, bail to avoid infinite loop.
            if next_cursor <= start_ts && !collected.is_empty() {
                break;
            }
            start_ts = next_cursor;

            collected.extend(page);

            // Short-page detection: if the exchange returned fewer than
            // `BITGET_PAGE_LIMIT` rows (after HFP-07 filtering), it has no
            // more history.
            if page_len < BITGET_PAGE_LIMIT as usize {
                break;
            }
        }

        // Sort newest-first and trim to target.
        collected.sort_by(|a, b| b.start_time_ms.cmp(&a.start_time_ms));
        collected.truncate(request.target_count);

        // HFP-08: tag every candle as ExchangeHistorical.
        for c in &mut collected {
            if c.reconstructed.is_none() {
                c.reconstructed = Some(ReconstructionMethod::ExchangeHistorical);
            }
        }

        Ok(collected)
    }
}