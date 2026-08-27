//! # Historical Fetch Policy
//!
//! Exchange-independent contract for fetching historical OHLCV candles. Replaces
//! the previous per-ad-hoc-bootstrap (`collect_candles` in
//! `portfolio-supervisor/src/registry/bootstrap.rs`) with a uniform trait that
//! both Hyperliquid and Bitget implement. Per
//! `docs/engines/data-infrastructure-engine/03-01-07-die-historical-fetch-policy.md`
//! (HFP-01 … HFP-10).
//!
//! ## Two behavior branches
//!
//! - **Sub-minute timeframes** (`timeframe_secs < 60`): the trait caller
//!   short-circuits to `Ok(vec![])` (HFP-03). No network or DB calls.
//! - **≥ 1 minute timeframes**: the implementation paginates the exchange REST
//!   endpoint until `target_count` is reached or the exchange reports no more
//!   history (HFP-04 … HFP-06). Per-exchange cursor semantics are hidden
//!   behind the trait surface.
//!
//! ## Pagination contract
//!
//! - The caller passes `target_count` (= `[candle_buffer] size`).
//! - The implementation loops until it has collected `target_count` candles,
//!   or until the exchange returns fewer than its per-page cap, or until
//!   `fetch_timeout_ms` elapses (HFP-10).
//! - The caller receives whatever was collected. A partial result is logged
//!   as a warning and the pipeline enters `LOADING` (not `FAILED`).

use async_trait::async_trait;
use core_domain::normalized::{Exchange, NormalizedCandle};
use thiserror::Error;

/// Single exchange-independent request payload.
#[derive(Debug, Clone)]
pub struct HistoricalFetchRequest {
    /// Exchange-native symbol (`"BTC"` for Hyperliquid, `"BTCUSDT"` for Bitget USDT).
    pub exchange_symbol: String,
    /// Internal unified symbol (`"BTC-USDC"`) used for the candle's `symbol` field.
    pub internal_symbol: String,
    /// Configured candle duration. Sub-minute values short-circuit (HFP-03).
    pub timeframe_secs: u64,
    /// How many candles to return. Caller's canonical number is
    /// `[candle_buffer] size` (default 500).
    pub target_count: usize,
    /// Upper bound on candle `start_time_ms`. Default: `now_ms`.
    pub end_ts: u64,
    /// Bitget only: futures product type (`"USDT-FUTURES"` / `"USDC-FUTURES"`).
    /// `None` for Hyperliquid.
    pub product_type: Option<String>,
    /// Maximum total wall-clock time (ms) the implementation may spend
    /// paginating (HFP-10).
    pub fetch_timeout_ms: u64,
}

/// Errors returned by `HistoricalFetchPolicy::fetch`. The error type is
/// intentionally rich — the caller logs the variant and surfaces a partial
/// result when applicable.
#[derive(Debug, Error)]
pub enum HistoricalFetchError {
    #[error("sub-minute timeframe {0}s bypasses historical fetch (HFP-03)")]
    SubMinuteBypassed(u64),
    #[error("HTTP {status} after {attempts} attempt(s): {body}")]
    Http {
        status: u16,
        attempts: u32,
        body: String,
    },
    #[error("decode failure: {0}")]
    Decode(String),
    #[error("fetch timeout after {0}ms (HFP-10)")]
    Timeout(u64),
}

/// Uniform contract for fetching historical candles.
#[async_trait]
pub trait HistoricalFetchPolicy: Send + Sync {
    fn exchange(&self) -> Exchange;

    /// Fetch up to `request.target_count` historical candles. The
    /// implementation is responsible for:
    ///   - HFP-03 sub-minute short-circuit (return `Ok(vec![])` for
    ///     `timeframe_secs < 60`).
    ///   - HFP-04 … HFP-06 exchange-specific pagination.
    ///   - HFP-07 filtering out currently-open candles
    ///     (`start_time_ms + duration_ms > now`).
    ///   - HFP-08 tagging each returned candle with
    ///     `ReconstructionMethod::ExchangeHistorical`.
    ///   - HFP-10 timeout enforcement.
    async fn fetch(
        &self,
        request: HistoricalFetchRequest,
    ) -> Result<Vec<NormalizedCandle>, HistoricalFetchError>;
}
