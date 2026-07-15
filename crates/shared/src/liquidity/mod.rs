//! Liquidity event accumulator.
//!
//! Real liquidation events arrive as an event stream from exchange WS
//! adapters (Hyperliquid `userFills` filtered for liquidations, Bitget
//! `fill` channel with `execType == "L"`). This accumulator:
//!
//!  - Records every event with bounded memory (default 1,000 events per
//!    symbol; old events fall off the front).
//!  - Aggregates events into per-candle flow on demand (`flush_to_flow`).
//!  - Maintains a cascade state machine (None → Detected → Sustained →
//!    Exhausted → None) using a rolling window of recent events.
//!  - Computes per-candle liquidity metrics (long/short notional, net
//!    flow, largest single event, cascade intensity) that become the
//!    `LiquidityMatrix` for the current bar.
//!
//! All math uses `f64` for performance — the accumulator is hot-path
//! code called per WS event. The per-candle `LiquidityMatrix` is then
//! carried into the `MarketSnapshot` with `f64` fields.

use std::collections::VecDeque;

use crate::normalized::{Exchange, LiquidationEvent, LiquidationSide};

/// State machine for cascade detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CascadeState {
    /// No significant cascade activity.
    None,
    /// A single event exceeded the z-score threshold.
    Detected,
    /// Multiple events exceeded threshold within the rolling window.
    Sustained,
    /// Activity was elevated but has decayed.
    Exhausted,
}

/// One bar's aggregated liquidity flow. Attached to `MarketSnapshot` as
/// `LiquidityMatrix`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LiquidityFlow {
    pub long_liquidations_usd: f64,
    pub short_liquidations_usd: f64,
    pub net_liquidation_usd: f64, // positive = longs got dumped (bearish)
    pub event_count: u32,
    pub largest_event_usd: f64,
    pub largest_event_price: Option<f64>,
    pub largest_event_side: Option<LiquidationSide>,
    pub cascade_state: CascadeState,
    pub cascade_intensity: f64, // 0..100
}

impl Default for LiquidityFlow {
    fn default() -> Self {
        Self {
            long_liquidations_usd: 0.0,
            short_liquidations_usd: 0.0,
            net_liquidation_usd: 0.0,
            event_count: 0,
            largest_event_usd: 0.0,
            largest_event_price: None,
            largest_event_side: None,
            cascade_state: CascadeState::None,
            cascade_intensity: 0.0,
        }
    }
}

/// Per-symbol accumulator. Owns a bounded event deque and the rolling
/// cascade state.
pub struct LiquidityEventAccumulator {
    symbol: String,
    events: VecDeque<LiquidationEvent>,
    max_events: usize,
    /// Per-bar flow counters. Reset by `flush_to_flow`.
    bar_flow: LiquidityFlow,
    bar_start_ms: u64,
    /// Threshold (USD) above which a single event is considered a cascade trigger.
    cascade_event_zscore: f64,
    /// Window (number of recent candles) used to detect "Sustained" cascades.
    cascade_window_candles: usize,
    /// Rolling per-candle cascade intensity (last N completed bars).
    rolling_intensity: VecDeque<f64>,
}

impl LiquidityEventAccumulator {
    /// Create a new accumulator for `symbol`. `max_events` caps memory
    /// regardless of WS flood rate.
    pub fn new(symbol: impl Into<String>) -> Self {
        Self::with_config(symbol, 1_000, 2.5, 5)
    }

    /// Full-configuration constructor.
    pub fn with_config(
        symbol: impl Into<String>,
        max_events: usize,
        cascade_event_zscore: f64,
        cascade_window_candles: usize,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            events: VecDeque::with_capacity(max_events.min(8_000)),
            max_events,
            bar_flow: LiquidityFlow::default(),
            bar_start_ms: 0,
            cascade_event_zscore,
            cascade_window_candles: cascade_window_candles.max(2),
            rolling_intensity: VecDeque::with_capacity(cascade_window_candles.max(2) * 2),
        }
    }

    /// Symbol this accumulator tracks.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Record one event. Updates bar-level counters immediately. The
    /// rolling event deque is bounded — when full, the oldest event is
    /// dropped from the back (most recent events retained).
    pub fn record_event(&mut self, ev: LiquidationEvent) {
        let notional = (ev.price.to_string().parse::<f64>().unwrap_or(0.0))
            * (ev.size.to_string().parse::<f64>().unwrap_or(0.0));

        // Per-bar accumulation.
        match ev.side {
            LiquidationSide::Long => {
                self.bar_flow.long_liquidations_usd += notional;
            }
            LiquidationSide::Short => {
                self.bar_flow.short_liquidations_usd += notional;
            }
        }
        if notional > self.bar_flow.largest_event_usd {
            self.bar_flow.largest_event_usd = notional;
            self.bar_flow.largest_event_price = ev.price.to_string().parse().ok();
            self.bar_flow.largest_event_side = Some(ev.side);
        }
        self.bar_flow.event_count += 1;

        // Bounded event history (newest at back).
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(ev);
    }

    /// Flush the per-bar counters and return the aggregated `LiquidityFlow`.
    /// After this call, the per-bar counters are reset to zero. The rolling
    /// intensity deque is updated with the bar's intensity.
    pub fn flush_to_flow(&mut self) -> LiquidityFlow {
        // Net flow: positive = longs got dumped = bearish pressure.
        self.bar_flow.net_liquidation_usd =
            self.bar_flow.long_liquidations_usd - self.bar_flow.short_liquidations_usd;

        // Cascade state: compute from the rolling intensity window.
        self.bar_flow.cascade_state = self.derive_cascade_state();
        self.bar_flow.cascade_intensity = self.compute_intensity();

        // Stash this bar's intensity in the rolling window.
        if self.rolling_intensity.len() >= self.cascade_window_candles {
            self.rolling_intensity.pop_front();
        }
        self.rolling_intensity.push_back(self.bar_flow.cascade_intensity);

        let out = self.bar_flow.clone();
        self.bar_flow = LiquidityFlow::default();
        out
    }

    fn compute_intensity(&self) -> f64 {
        // Single-event trigger: large liquidation relative to typical.
        // We use the largest event in this bar vs. the running mean.
        if self.events.is_empty() {
            return 0.0;
        }
        // Sum of all notional in current bar vs. mean per-bar in window.
        let total = self.bar_flow.long_liquidations_usd + self.bar_flow.short_liquidations_usd;
        let baseline: f64 = if self.rolling_intensity.is_empty() {
            1000.0 // $1,000 baseline when no history — single event is significant
        } else {
            // Map mean rolling intensity back to USD for comparison.
            self.rolling_intensity.iter().sum::<f64>()
                / self.rolling_intensity.len() as f64
                * 1000.0
                + 1.0
        };
        let ratio = total / baseline;
        // log-scaled, clamped 0..100.
        (ratio.ln().max(0.0) * 20.0).min(100.0)
    }

    fn derive_cascade_state(&self) -> CascadeState {
        // Count events within the rolling window that exceed the
        // per-event z-score threshold (heuristic: largest event_usd >
        // baseline × zscore).
        let mut significant_events: u32 = 0;
        let baseline_event_usd: f64 = if self.rolling_intensity.is_empty() {
            500.0
        } else {
            (self.rolling_intensity.iter().sum::<f64>()
                / self.rolling_intensity.len() as f64)
                * 100.0
                + 1.0
        };
        let threshold_usd = baseline_event_usd * self.cascade_event_zscore;
        for ev in self.events.iter().rev().take(50) {
            let notional = (ev.price.to_string().parse::<f64>().unwrap_or(0.0))
                * (ev.size.to_string().parse::<f64>().unwrap_or(0.0));
            if notional >= threshold_usd {
                significant_events += 1;
            }
        }
        // >= 3 in the last 50 events = Sustained; 1-2 = Detected.
        if significant_events >= 3 {
            CascadeState::Sustained
        } else if significant_events >= 1 {
            CascadeState::Detected
        } else if self.bar_flow.cascade_intensity > 30.0
            && !self.rolling_intensity.is_empty()
        {
            // Decayed: bar was hot, window shows decline.
            CascadeState::Exhausted
        } else {
            CascadeState::None
        }
    }

    /// Number of events currently buffered.
    pub fn buffered_event_count(&self) -> usize {
        self.events.len()
    }

    /// Most recent events, newest first.
    pub fn recent_events(&self, limit: usize) -> Vec<&LiquidationEvent> {
        self.events.iter().rev().take(limit).collect()
    }

    /// Reset the bar-level counters without touching the event history.
    pub fn reset_bar(&mut self) {
        self.bar_flow = LiquidityFlow::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn make_event(side: LiquidationSide, price: f64, size: f64, ts_ms: u64) -> LiquidationEvent {
        LiquidationEvent {
            exchange: Exchange::Hyperliquid,
            symbol: "BTC-USDT".to_string(),
            side,
            price: Decimal::from_f64_retain(price).unwrap_or(Decimal::ZERO),
            size: Decimal::from_f64_retain(size).unwrap_or(Decimal::ZERO),
            timestamp_ms: ts_ms,
            venue_order_id: None,
        }
    }

    #[test]
    fn empty_accumulator_returns_zero_flow() {
        let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
        let flow = acc.flush_to_flow();
        assert_eq!(flow.event_count, 0);
        assert_eq!(flow.long_liquidations_usd, 0.0);
        assert_eq!(flow.short_liquidations_usd, 0.0);
        assert_eq!(flow.cascade_state, CascadeState::None);
        assert_eq!(flow.cascade_intensity, 0.0);
    }

    #[test]
    fn long_liquidation_increments_long_bucket() {
        let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
        // 1 BTC at $50,000 = $50,000 notional.
        acc.record_event(make_event(LiquidationSide::Long, 50_000.0, 1.0, 1));
        let flow = acc.flush_to_flow();
        assert_eq!(flow.event_count, 1);
        assert!((flow.long_liquidations_usd - 50_000.0).abs() < 0.01);
        assert_eq!(flow.short_liquidations_usd, 0.0);
        assert!(flow.net_liquidation_usd > 0.0, "net should be positive for longs");
    }

    #[test]
    fn short_liquidation_increments_short_bucket() {
        let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
        acc.record_event(make_event(LiquidationSide::Short, 50_000.0, 2.0, 1));
        let flow = acc.flush_to_flow();
        assert_eq!(flow.event_count, 1);
        assert_eq!(flow.long_liquidations_usd, 0.0);
        assert!((flow.short_liquidations_usd - 100_000.0).abs() < 0.01);
        assert!(flow.net_liquidation_usd < 0.0, "net should be negative for shorts");
    }

    #[test]
    fn largest_event_tracking() {
        let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
        acc.record_event(make_event(LiquidationSide::Long, 50_000.0, 0.5, 1));
        acc.record_event(make_event(LiquidationSide::Long, 51_000.0, 2.0, 2));
        acc.record_event(make_event(LiquidationSide::Short, 49_000.0, 0.1, 3));
        let flow = acc.flush_to_flow();
        // Largest: 51000 * 2 = 102000.
        assert!((flow.largest_event_usd - 102_000.0).abs() < 0.01);
        assert_eq!(flow.largest_event_price, Some(51_000.0));
        assert_eq!(flow.largest_event_side, Some(LiquidationSide::Long));
    }

    #[test]
    fn bounded_event_history() {
        let mut acc = LiquidityEventAccumulator::with_config("BTC-USDT", 5, 2.5, 3);
        for i in 0..20 {
            acc.record_event(make_event(LiquidationSide::Long, 50_000.0, 0.1, i));
        }
        // History must be capped.
        assert_eq!(acc.buffered_event_count(), 5);
    }

    #[test]
    fn flush_resets_per_bar_counters() {
        let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
        acc.record_event(make_event(LiquidationSide::Long, 50_000.0, 1.0, 1));
        let first = acc.flush_to_flow();
        assert_eq!(first.event_count, 1);
        // Second flush should be empty.
        let second = acc.flush_to_flow();
        assert_eq!(second.event_count, 0);
        assert_eq!(second.long_liquidations_usd, 0.0);
    }

    #[test]
    fn cascade_state_progression_with_large_events() {
        let mut acc = LiquidityEventAccumulator::with_config("BTC-USDT", 100, 1.5, 5);
        // Build up baseline with small events.
        for i in 0..5 {
            acc.record_event(make_event(LiquidationSide::Long, 50_000.0, 0.01, i * 1000));
            let _ = acc.flush_to_flow();
        }
        // Now produce a big event.
        acc.record_event(make_event(LiquidationSide::Long, 50_000.0, 5.0, 9999));
        let flow = acc.flush_to_flow();
        assert!(
            matches!(flow.cascade_state, CascadeState::Detected | CascadeState::Sustained),
            "expected Detected or Sustained, got {:?}",
            flow.cascade_state
        );
    }
}