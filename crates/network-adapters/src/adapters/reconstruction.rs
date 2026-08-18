//! Candle reconstruction for WebSocket reconnect gaps.
//!
//! When the live trade stream drops, the engine needs to back-fill the
//! missing candles before resuming normal operation. The size of the gap
//! dictates the reconstruction strategy:
//!
//! * **Intervals `>= 1 minute`** — the REST historical endpoints
//!   (`/info` on Hyperliquid, `/api/v2/mix/market/candles` on Bitget) can
//!   fetch the missing candles directly, preserving OHLCV fidelity. The
//!   `ExchangeHistoricalFetcher` trait abstracts those exchanges behind a
//!   single interface; concrete implementations live in
//!   `adapters/hyperliquid_rest.rs` and `adapters/bitget_rest.rs`.
//!
//! * **Intervals `< 1 minute`** — exchanges either don't expose sub-minute
//!   history or limit it to the most recent window. The reconstructor
//!   therefore synthesizes flat OHLCV candles using either:
//!     - an exponential moving average of the last `ema_window` closes
//!       (`CandleReconstructor::reconstruct_ema`), or
//!     - linear interpolation / extrapolation of the last two closes
//!       (`CandleReconstructor::reconstruct_interpolation`).
//!
//! All synthesized candles carry a `ReconstructedCandle` envelope that
//! records the synthesis method and the gap window that produced them, so
//! downstream consumers can filter, down-weight, or surface them in the UI.

use async_trait::async_trait;
use core_domain::normalized::{Exchange, NormalizedCandle, ReconstructionMethod};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

/// A candle that has been synthesized to fill a WebSocket gap.
///
/// `source_gap_start_ms` and `source_gap_end_ms` define the time range
/// the reconstruction is covering (typically the same as the candle's own
/// `start_time_ms` … `start_time_ms + duration_ms`, but kept separately
/// so the orchestrator can track multi-candle gap windows).
#[derive(Debug, Clone)]
pub struct ReconstructedCandle {
    pub candle: NormalizedCandle,
    pub method: ReconstructionMethod,
    pub source_gap_start_ms: u64,
    pub source_gap_end_ms: u64,
}

/// Detects WebSocket gaps by comparing the last persisted candle
/// timestamp against the current wall clock.
pub struct GapDetector;

impl GapDetector {
    /// Returns `Some((gap_start_ms, gap_end_ms))` if `now_ms` exceeds the
    /// last persisted candle by more than `gap_threshold_secs`, otherwise
    /// `None`.
    ///
    /// `gap_start_ms` is `last_persisted_ts_ms` (the moment we last saw
    /// data); `gap_end_ms` is `now_ms` (the moment we reconnected).
    /// Saturating arithmetic is used so a degenerate input (`now_ms <
    /// last_persisted_ts_ms`) collapses to a zero-length gap rather than
    /// wrapping.
    pub fn detect_gap(
        last_persisted_ts_ms: u64,
        now_ms: u64,
        gap_threshold_secs: u64,
    ) -> Option<(u64, u64)> {
        if now_ms <= last_persisted_ts_ms {
            return None;
        }
        let elapsed_ms = now_ms - last_persisted_ts_ms;
        let threshold_ms = gap_threshold_secs.saturating_mul(1_000);
        if elapsed_ms > threshold_ms {
            Some((last_persisted_ts_ms, now_ms))
        } else {
            None
        }
    }
}

/// Synthesizes sub-1-minute candles to bridge gaps that the exchanges
/// don't preserve in their REST history.
pub struct CandleReconstructor {
    pub ema_window: usize,
    pub min_history_for_ema: usize,
    /// Volume-per-second baseline used to estimate reconstructed sub-minute
    /// candles' volume (DAT-02). When `0.0` (default), the synthesized
    /// candle carries `volume: Decimal::ZERO` — the legacy behaviour that
    /// causes flatline volume on macro candles built entirely from
    /// sub-minute reconstructions. Operators can opt into the volume
    /// heuristic by setting a positive baseline via `config.toml`
    /// `[reconstruction] volume_per_sec_baseline = <USD-per-second>`.
    pub volume_per_sec_baseline: f64,
}

impl Default for CandleReconstructor {
    fn default() -> Self {
        Self::new()
    }
}

impl CandleReconstructor {
    pub fn new() -> Self {
        Self {
            ema_window: 200,
            min_history_for_ema: 50,
            volume_per_sec_baseline: 0.0,
        }
    }

    /// Construct with a non-default volume baseline. Used by `main.rs`
    /// to hydrate the reconstructor from `[reconstruction]
    /// volume_per_sec_baseline` in `config.toml`.
    pub fn with_volume_baseline(
        ema_window: usize,
        min_history_for_ema: usize,
        volume_per_sec_baseline: f64,
    ) -> Self {
        Self {
            ema_window,
            min_history_for_ema,
            volume_per_sec_baseline,
        }
    }

    /// Decide how to synthesize one candle for `interval_start_ms..
    /// interval_end_ms` (duration = `interval_end_ms - interval_start_ms`).
    ///
    /// Returns `None` when:
    ///   * the candle is `>= 1 minute` — the caller must use the
    ///     `ExchangeHistoricalFetcher` instead, or
    ///   * the candle is `< 1 minute` and there is no usable history
    ///     (fewer than 2 closes). Linear interpolation needs two points.
    ///
    /// `recent_closes` is consumed as a slice of the most recent close
    /// prices in chronological order (oldest first). The first
    /// `ema_window` entries are used for the EMA path.
    pub fn reconstruct(
        &self,
        exchange: Exchange,
        interval_start_ms: u64,
        interval_end_ms: u64,
        duration_ms: u64,
        recent_closes: &[f64],
    ) -> Option<ReconstructedCandle> {
        if duration_ms >= 60_000 {
            // 1m+ candles must be filled from exchange REST history;
            // this reconstructor only handles sub-1m synthesis.
            return None;
        }
        if recent_closes.len() >= self.min_history_for_ema {
            Some(self.reconstruct_ema(exchange, interval_start_ms, interval_end_ms, recent_closes))
        } else if recent_closes.len() >= 2 {
            Some(self.reconstruct_interpolation(
                exchange,
                interval_start_ms,
                interval_end_ms,
                recent_closes,
            ))
        } else {
            None
        }
    }

    /// EMA projection: take the last `ema_window` closes, compute the
    /// EMA, and emit a flat OHLCV candle around the EMA value.
    ///
    /// Formula (standard exponential moving average):
    ///
    /// ```text
    /// alpha = 2 / (N + 1)              // N = ema_window
    /// EMA_0 = closes[0]                // seed with first sample
    /// EMA_t = alpha * closes[t] + (1 - alpha) * EMA_{t-1}
    /// ```
    ///
    /// OHLC are all set to the final EMA value because we have no
    /// intra-candle trade information; volume is reported as zero with a
    /// flag telling downstream consumers this is an estimate.
    fn reconstruct_ema(
        &self,
        exchange: Exchange,
        interval_start_ms: u64,
        interval_end_ms: u64,
        recent_closes: &[f64],
    ) -> ReconstructedCandle {
        let window_end = recent_closes.len().min(self.ema_window);
        let window = &recent_closes[recent_closes.len() - window_end..];
        let n = window.len() as f64;
        let alpha = 2.0 / (n + 1.0);

        let mut ema = window[0];
        for &close in window.iter().skip(1) {
            ema = alpha * close + (1.0 - alpha) * ema;
        }

        let ema_dec = Decimal::from_f64_retain(ema)
            .unwrap_or_else(|| Decimal::from_f64(window[0]).unwrap_or(Decimal::ZERO));

        let duration_ms = interval_end_ms.saturating_sub(interval_start_ms);
        let volume = self.estimated_volume(duration_ms);
        let candle = NormalizedCandle {
            exchange,
            symbol: String::new(),
            start_time_ms: interval_start_ms,
            duration_ms,
            open: ema_dec,
            high: ema_dec,
            low: ema_dec,
            close: ema_dec,
            volume,
            trades_count: 0,
            reconstructed: Some(ReconstructionMethod::ExponentialMovingAverage),
        };

        ReconstructedCandle {
            candle,
            method: ReconstructionMethod::ExponentialMovingAverage,
            source_gap_start_ms: interval_start_ms,
            source_gap_end_ms: interval_end_ms,
        }
    }

    /// Linear interpolation / extrapolation from the last two closes.
    ///
    /// With two points `(t_{n-1}, c_{n-1})` and `(t_n, c_n)` at unit
    /// spacing (the slice index acts as the time axis), the projected
    /// close for the gap interval is:
    ///
    /// ```text
    /// slope = c_n - c_{n-1}
    /// c_target = c_n + slope = 2 * c_n - c_{n-1}
    /// ```
    ///
    /// When the target interval is between the two known points, the
    /// formula collapses to the ordinary linear-interpolation formula
    /// `c_n + slope * frac`; callers can override `frac` by passing
    /// `interval_start_ms / duration_ms` directly. The reconstructed
    /// candle carries the same flat OHLC shape as the EMA path because
    /// we cannot recover intra-candle volatility from two close prices.
    fn reconstruct_interpolation(
        &self,
        exchange: Exchange,
        interval_start_ms: u64,
        interval_end_ms: u64,
        recent_closes: &[f64],
    ) -> ReconstructedCandle {
        let n = recent_closes.len();
        let prev = recent_closes[n - 2];
        let last = recent_closes[n - 1];
        let slope = last - prev;
        let projected = last + slope;

        let proj_dec = Decimal::from_f64_retain(projected)
            .unwrap_or_else(|| Decimal::from_f64(last).unwrap_or(Decimal::ZERO));

        let duration_ms = interval_end_ms.saturating_sub(interval_start_ms);
        let volume = self.estimated_volume(duration_ms);
        let candle = NormalizedCandle {
            exchange,
            symbol: String::new(),
            start_time_ms: interval_start_ms,
            duration_ms,
            open: proj_dec,
            high: proj_dec,
            low: proj_dec,
            close: proj_dec,
            volume,
            trades_count: 0,
            reconstructed: Some(ReconstructionMethod::LinearInterpolation),
        };

        ReconstructedCandle {
            candle,
            method: ReconstructionMethod::LinearInterpolation,
            source_gap_start_ms: interval_start_ms,
            source_gap_end_ms: interval_end_ms,
        }
    }

    /// Estimated volume for a synthesized candle of `duration_ms` width
    /// (DAT-02). When the configured `volume_per_sec_baseline` is `0.0`
    /// (the default), returns `Decimal::ZERO` — the legacy conservative
    /// behaviour. When non-zero, scales the baseline by the candle duration
    /// in seconds. This bridges sub-minute reconstructed candles into
    /// macro candles with a meaningful volume footprint, preventing false
    /// "low-participation" rejections at Pre-Trade Gates 3 and 5.
    fn estimated_volume(&self, duration_ms: u64) -> Decimal {
        if self.volume_per_sec_baseline <= 0.0 {
            return Decimal::ZERO;
        }
        let duration_secs = duration_ms / 1000;
        let volume = self.volume_per_sec_baseline * (duration_secs as f64);
        Decimal::from_f64(volume).unwrap_or(Decimal::ZERO)
    }
}

/// Abstraction over the exchange-specific REST endpoints that can return
/// historical OHLCV candles.
///
/// Implemented by the Hyperliquid and Bitget REST adapters in
/// `adapters/hyperliquid_rest.rs` and `adapters/bitget_rest.rs`. The
/// reconstruction orchestrator calls this trait when a `>= 1 minute`
/// gap is detected and `CandleReconstructor::reconstruct` returns
/// `None`.
#[async_trait]
pub trait ExchangeHistoricalFetcher: Send + Sync {
    async fn fetch_candles(
        &self,
        symbol: &str,
        interval_ms: u64,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<Vec<NormalizedCandle>, ReconstructionError>;
}

/// Failures surfaced while back-filling candles from exchange REST
/// history.
#[derive(Debug, thiserror::Error)]
pub enum ReconstructionError {
    #[error("gap too large: {start_ms} to {end_ms} exceeds exchange API limit")]
    GapTooLarge { start_ms: u64, end_ms: u64 },
    #[error("exchange API error: {0}")]
    ApiError(String),
    #[error("network error: {0}")]
    Network(String),
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::ToPrimitive;

    // ---- GapDetector ---------------------------------------------------------

    #[test]
    fn gap_detector_returns_none_when_within_threshold() {
        let last = 1_700_000_000_000u64;
        let now = last + 30_000; // 30s elapsed
        let gap = GapDetector::detect_gap(last, now, 60);
        assert!(gap.is_none(), "30s gap should be below the 60s threshold");
    }

    #[test]
    fn gap_detector_returns_gap_when_exceeds_threshold() {
        let last = 1_700_000_000_000u64;
        let now = last + 120_000; // 2 minutes elapsed
        let (start, end) = GapDetector::detect_gap(last, now, 60).expect("expected gap");
        assert_eq!(start, last);
        assert_eq!(end, now);
    }

    // ---- CandleReconstructor routing ----------------------------------------

    #[test]
    fn reconstruct_1m_returns_none_caller_uses_exchange() {
        let r = CandleReconstructor::new();
        let closes: Vec<f64> = (0..200).map(|i| 100.0 + i as f64).collect();
        let result = r.reconstruct(Exchange::Hyperliquid, 1_000, 61_000, 60_000, &closes);
        assert!(
            result.is_none(),
            "1m candle must defer to ExchangeHistoricalFetcher"
        );
    }

    #[test]
    fn reconstruct_sub_1m_with_ema_history() {
        let r = CandleReconstructor::new();
        // 200 closes (well above min_history_for_ema = 50)
        let closes: Vec<f64> = (0..200).map(|i| 100.0 + i as f64 * 0.1).collect();
        let result = r
            .reconstruct(Exchange::Hyperliquid, 2_000, 3_000, 1_000, &closes)
            .expect("expected reconstruction");
        assert_eq!(
            result.method,
            ReconstructionMethod::ExponentialMovingAverage
        );
        assert_eq!(result.candle.duration_ms, 1_000);
        assert_eq!(result.candle.start_time_ms, 2_000);
        assert_eq!(result.source_gap_start_ms, 2_000);
        assert_eq!(result.source_gap_end_ms, 3_000);
        assert_eq!(
            result.candle.reconstructed,
            Some(ReconstructionMethod::ExponentialMovingAverage)
        );
        // Flat OHLC around the EMA value.
        assert_eq!(result.candle.open, result.candle.close);
        assert_eq!(result.candle.high, result.candle.low);
    }

    #[test]
    fn reconstruct_sub_1m_with_minimal_history_uses_interpolation() {
        let r = CandleReconstructor::new();
        // Only 2 closes (below min_history_for_ema = 50)
        let closes = vec![100.0, 110.0];
        let result = r
            .reconstruct(Exchange::Hyperliquid, 5_000, 6_000, 1_000, &closes)
            .expect("expected interpolation");
        assert_eq!(result.method, ReconstructionMethod::LinearInterpolation);
        assert_eq!(
            result.candle.reconstructed,
            Some(ReconstructionMethod::LinearInterpolation)
        );
        // slope = 110 - 100 = 10, projection = 110 + 10 = 120
        assert_eq!(result.candle.close, Decimal::from(120));
    }

    #[test]
    fn reconstruct_returns_none_with_no_history() {
        let r = CandleReconstructor::new();
        let closes: Vec<f64> = vec![];
        assert!(r
            .reconstruct(Exchange::Hyperliquid, 0, 1_000, 500, &closes)
            .is_none());

        let one = vec![100.0];
        assert!(r
            .reconstruct(Exchange::Hyperliquid, 0, 1_000, 500, &one)
            .is_none());
    }

    // ---- EMA math -----------------------------------------------------------

    #[test]
    fn ema_produces_smooth_values() {
        let r = CandleReconstructor::new();
        // Linear ramp: closes = 0..199. EMA of a perfect ramp must
        // (a) stay finite (no NaN/Inf), (b) lag behind the most recent
        // sample by roughly N/2 (the well-known steady-state lag of an
        // N-window EMA), and (c) sit above the seed sample because the
        // ramp is strictly increasing.
        let closes: Vec<f64> = (0..200).map(|i| i as f64).collect();
        let result = r
            .reconstruct(Exchange::Hyperliquid, 0, 1_000, 500, &closes)
            .expect("expected ema reconstruction");
        let ema = result.candle.close.to_f64().unwrap();

        assert!(ema.is_finite(), "EMA must be finite, got {}", ema);
        assert!(ema > 0.0, "EMA of positive ramp must be positive");
        assert!(
            ema < 199.0,
            "EMA with smoothing must lag behind the latest sample (199), got {}",
            ema
        );
        // For N=200 and seed EMA_0 = closes[0] = 0, the steady-state lag
        // is approximately (N-1)/2 ≈ 99.5. Allow generous tolerance for
        // the early transient.
        assert!(
            (90.0..=130.0).contains(&ema),
            "EMA of ramp 0..199 should lag by ~N/2 (got {})",
            ema
        );
    }

    #[test]
    fn interpolation_is_linear() {
        // Verify the linear projection formula directly.
        let prev = 50.0_f64;
        let last = 70.0_f64;
        let slope = last - prev;
        let projected = last + slope;
        assert_eq!(projected, 90.0);

        // Same formula via the reconstructor.
        let r = CandleReconstructor::new();
        let closes = vec![prev, last];
        let result = r
            .reconstruct(Exchange::Hyperliquid, 0, 1_000, 1_000, &closes)
            .expect("expected interpolation");
        assert_eq!(result.candle.close, Decimal::from(90));
        // The candlestick is flat because we only know two close points.
        assert_eq!(result.candle.open, result.candle.close);
        assert_eq!(result.candle.high, result.candle.low);
    }
}
