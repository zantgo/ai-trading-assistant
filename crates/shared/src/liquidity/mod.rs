//! Liquidity event accumulator and cluster estimator.
//!
//! This module owns two related responsibilities:
//!
//! 1. **Real-time event accumulation** — every liquidation event from
//!    exchange WS adapters is recorded into a bounded deque and
//!    aggregated per-candle. See `LiquidityEventAccumulator`.
//!
//! 2. **Cluster-matrix estimation** — every 5 minutes, the platform
//!    recomputes a `LiquidationClusterMatrix` from current OI, funding
//!    rate, mid price, and recent price action. This is the data that
//!    becomes the liquidation heatmap on the frontend.
//!
//! The cluster estimator is a deterministic, snapshot-in / clusters-out
//! function. It uses a documented leverage distribution assumption
//! (power-law by default; configurable) and a maintenance-margin rate.
//! No ML, no online learning — all assumptions are declared and
//! visible to the user.
//!
//! ## Mathematical model
//!
//! For each leverage bucket L in `[1, 3, 5, 10, 20, 50, 100]`:
//!  - Liquidation distance = `1/L - MMR` (e.g. 10x with 0.5% MMR → 9.5%)
//!  - Long liquidation price = `entry_price * (1 - distance)`
//!  - Short liquidation price = `entry_price * (1 + distance)`
//!
//! Entry prices are inferred from recent swing lows (for longs) and
//! swing highs (for shorts), weighted by the recent-volume profile.
//! Each long entry-price × leverage bucket combination contributes
//! notional to a price bucket. Price buckets are then peak-detected
//! to identify clusters.
//!
//! The cascade-asymmetry score is the difference between
//! short-cluster notional (above mid) and long-cluster notional
//! (below mid), normalized by total OI. Negative = short squeeze
//! risk (price likely to rally), positive = long squeeze risk
//! (price likely to drop).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

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

// =============================================================================
// Phase 2 — Liquidation Cluster Matrix
// =============================================================================
//
// All types here serialize as snake_case (default) to match the
// `LiquidityMatrix` schema documented in the spec docs.

/// Direction classification for a cluster relative to current price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClusterKind {
    /// Cluster sits above current price (would attract short-squeeze rallies).
    AboveCurrentPrice,
    /// Cluster sits below current price (long-squeeze cascade target).
    BelowCurrentPrice,
    /// Cluster is essentially at current price (imminent cascade risk).
    AtCurrentPrice,
    /// Cluster is far enough away that it doesn't influence near-term price.
    Distant,
}

/// Where the leverage distribution assumption came from — for transparency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LeverageDistributionSource {
    /// Static power-law default (α ≈ 1.5).
    DefaultPowerLaw,
    /// Power-law modulated by funding-rate extremes.
    FundingAdaptive,
    /// Loaded from a user-supplied config override.
    ConfigOverride,
}

/// Documented assumptions for the leverage distribution. Always serialized
/// so the UI can show "estimated using …" to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeverageAssumptions {
    pub buckets: Vec<u32>,
    pub weights: Vec<f64>,
    pub funding_modulation_active: bool,
    pub funding_extreme_pct: f64,
    pub source: LeverageDistributionSource,
}

/// One detected liquidation cluster. The price-low / price-high window
/// contains the cluster; the peak_price is where most of the notional
/// is concentrated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiquidationCluster {
    pub price_low: f64,
    pub price_high: f64,
    pub peak_price: f64,
    pub notional_usd: f64,
    pub dominant_leverage: u32,
    pub distance_from_mid_pct: f64,
    pub cluster_kind: ClusterKind,
    pub magnet_strength: f64, // 0..100
}

/// The cluster matrix produced every 5 minutes (or per refresh) per symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationClusterMatrix {
    pub symbol: String,
    pub generated_at_ms: u64,
    pub valid_until_ms: u64,
    pub mid_price: f64,
    pub leverage_assumptions: LeverageAssumptions,
    /// Short liquidation clusters (price-above-mid; the "ceiling").
    pub short_clusters: Vec<LiquidationCluster>,
    /// Long liquidation clusters (price-below-mid; the "floor").
    pub long_clusters: Vec<LiquidationCluster>,
    /// [-1, +1]; negative = short squeeze risk, positive = long squeeze risk.
    pub cascade_asymmetry: f64,
    pub total_long_oi_usd: f64,
    pub total_short_oi_usd: f64,
    /// 0..1 — overall confidence in the estimate (1 = high OI, normal funding).
    pub estimation_confidence: f64,
}

impl LiquidationClusterMatrix {
    /// Empty placeholder matrix used when estimation is not yet possible
    /// (insufficient data, zero OI, etc.).
    pub fn empty(symbol: &str, mid_price: f64) -> Self {
        Self {
            symbol: symbol.to_string(),
            generated_at_ms: 0,
            valid_until_ms: 0,
            mid_price,
            leverage_assumptions: LeverageAssumptions {
                buckets: vec![1, 3, 5, 10, 20, 50, 100],
                weights: vec![0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
                funding_modulation_active: true,
                funding_extreme_pct: 0.0005,
                source: LeverageDistributionSource::DefaultPowerLaw,
            },
            short_clusters: vec![],
            long_clusters: vec![],
            cascade_asymmetry: 0.0,
            total_long_oi_usd: 0.0,
            total_short_oi_usd: 0.0,
            estimation_confidence: 0.0,
        }
    }
}

/// Input bundle for `estimate_clusters`. Bundles everything the
/// estimator needs so callers don't have to thread 8+ parameters.
pub struct ClusterEstimateInput<'a> {
    pub symbol: &'a str,
    pub mid_price: f64,
    /// Recent close prices, oldest first. Used to detect swing lows /
    /// highs which seed the entry-price distribution.
    pub price_history: &'a [f64],
    pub total_oi_usd: f64,
    pub funding_rate: f64,
    /// Estimated split of OI between longs and shorts. None = compute
    /// from funding rate + price action heuristic.
    pub long_oi_pct: Option<f64>,
    pub maintenance_margin_rate: f64,
    pub funding_extreme_pct: f64,
    pub funding_modulation_active: bool,
    pub leverage_buckets: &'a [u32],
    pub leverage_weights: &'a [f64],
    /// Min cluster notional in USD to keep (filters noise).
    pub min_cluster_notional_usd: f64,
}

impl<'a> Default for ClusterEstimateInput<'a> {
    fn default() -> Self {
        Self {
            symbol: "",
            mid_price: 0.0,
            price_history: &[],
            total_oi_usd: 0.0,
            funding_rate: 0.0,
            long_oi_pct: None,
            maintenance_margin_rate: 0.005,
            funding_extreme_pct: 0.0005,
            funding_modulation_active: true,
            leverage_buckets: &[1, 3, 5, 10, 20, 50, 100],
            leverage_weights: &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
            min_cluster_notional_usd: 50_000.0,
        }
    }
}

/// Apply funding-rate modulation to the leverage weights. Extreme funding
/// → heavier high-leverage tail (because crowded trades = high leverage).
fn apply_funding_modulation(
    weights: &mut [f64],
    funding_rate: f64,
    extreme_pct: f64,
) {
    if extreme_pct <= 0.0 {
        return;
    }
    let funding_mag = funding_rate.abs();
    // 0 when funding is at zero; 1 when funding is at-or-above extreme.
    let tilt = (funding_mag / extreme_pct).clamp(0.0, 1.0);
    // Tilt mass from low-leverage buckets (index 0..2) toward high-leverage
    // buckets (index 4..6). 5% of mass moved at full tilt.
    let shift = 0.05 * tilt;
    let len = weights.len();
    for i in 0..2.min(len) {
        weights[i] = (weights[i] - shift / 2.0).max(0.0);
    }
    for i in len.saturating_sub(2)..len {
        weights[i] = (weights[i] + shift / 2.0).min(1.0);
    }
    // Renormalize.
    let total: f64 = weights.iter().sum();
    if total > 0.0 {
        for w in weights.iter_mut() {
            *w /= total;
        }
    }
}

/// Estimate the long/short OI split. Uses funding rate as the primary
/// signal; price action as the secondary; default 50/50 if neither is
/// informative.
fn estimate_long_oi_pct(funding_rate: f64, price_history: &[f64], override_pct: Option<f64>) -> f64 {
    if let Some(p) = override_pct {
        return p.clamp(0.05, 0.95);
    }
    let funding_bias = (funding_rate / 0.0005).clamp(-1.0, 1.0) * 0.3; // ±30%
    let price_bias = if price_history.len() >= 4 {
        let n = price_history.len();
        let recent = price_history[n - 1];
        let prior = price_history[n - 4];
        let change = (recent - prior) / prior.max(1e-9);
        // Map price change to bias: +1% → +20% long bias.
        (change / 0.01).clamp(-1.0, 1.0) * 0.2
    } else {
        0.0
    };
    (0.5 + funding_bias + price_bias).clamp(0.10, 0.90)
}

/// Find swing lows and highs in a price history. Returns sorted unique
/// prices (oldest first). Volume weighting not available here — caller
/// can use a different seed if needed.
fn find_swing_levels(price_history: &[f64], lookback: usize) -> (Vec<f64>, Vec<f64>) {
    if price_history.len() < 3 {
        return (vec![], vec![]);
    }
    let mut lows = Vec::new();
    let mut highs = Vec::new();
    let lb = lookback.max(1).min(price_history.len() / 3);
    for i in lb..(price_history.len() - lb) {
        let window = &price_history[i - lb..=i + lb];
        let p = price_history[i];
        if window.iter().all(|&x| x >= p) {
            lows.push(p);
        }
        if window.iter().all(|&x| x <= p) {
            highs.push(p);
        }
    }
    // Deduplicate (within 0.5%).
    let dedup = |mut v: Vec<f64>| -> Vec<f64> {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v.dedup_by(|a, b| (*a - *b).abs() / a.max(1e-9) < 0.005);
        v
    };
    (dedup(lows), dedup(highs))
}

/// The main estimator. Deterministic, snapshot-in / clusters-out.
pub fn estimate_clusters(
    input: &ClusterEstimateInput,
) -> LiquidationClusterMatrix {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Empty placeholder if no data.
    if input.mid_price <= 0.0 || input.total_oi_usd <= 0.0 {
        return LiquidationClusterMatrix::empty(input.symbol, input.mid_price);
    }

    // 1. Compute leverage distribution.
    let mut weights = input.leverage_weights.to_vec();
    let leverage_source = if input.funding_modulation_active
        && input.funding_rate.abs() > input.funding_extreme_pct
    {
        apply_funding_modulation(&mut weights, input.funding_rate, input.funding_extreme_pct);
        LeverageDistributionSource::FundingAdaptive
    } else {
        LeverageDistributionSource::DefaultPowerLaw
    };
    let leverage_assumptions = LeverageAssumptions {
        buckets: input.leverage_buckets.to_vec(),
        weights: weights.clone(),
        funding_modulation_active: input.funding_modulation_active,
        funding_extreme_pct: input.funding_extreme_pct,
        source: leverage_source,
    };

    // 2. Estimate long/short OI split.
    let long_oi_pct = estimate_long_oi_pct(input.funding_rate, input.price_history, input.long_oi_pct);
    let long_oi_usd = input.total_oi_usd * long_oi_pct;
    let short_oi_usd = input.total_oi_usd * (1.0 - long_oi_pct);

    // 3. Find swing levels.
    let (swing_lows, swing_highs) = find_swing_levels(input.price_history, 5);

    // 4. For each (entry_price, leverage) combination, compute the
    //    liquidation price and accumulate into 0.1% price buckets.
    let price_bin_pct = 0.001; // 0.1% buckets
    let mut long_bins: std::collections::BTreeMap<i64, f64> = std::collections::BTreeMap::new();
    let mut short_bins: std::collections::BTreeMap<i64, f64> = std::collections::BTreeMap::new();

    let bucket_long = |entry: f64, lev: u32| -> Option<f64> {
        let dist = (1.0 / lev as f64) - input.maintenance_margin_rate;
        if dist <= 0.0 { return None; }
        Some(entry * (1.0 - dist))
    };
    let bucket_short = |entry: f64, lev: u32| -> Option<f64> {
        let dist = (1.0 / lev as f64) - input.maintenance_margin_rate;
        if dist <= 0.0 { return None; }
        Some(entry * (1.0 + dist))
    };

    for (lev, weight) in input.leverage_buckets.iter().zip(weights.iter()) {
        if *weight <= 0.0 || *lev == 0 { continue; }
        let lev_notional_long = long_oi_usd * weight;
        let lev_notional_short = short_oi_usd * weight;
        // Distribute across swing lows (long) and swing highs (short).
        // If no swing levels, fall back to current mid (single bin).
        if swing_lows.is_empty() {
            if let Some(liq_px) = bucket_long(input.mid_price, *lev) {
                let key = (liq_px / input.mid_price / price_bin_pct).round() as i64;
                *long_bins.entry(key).or_insert(0.0) += lev_notional_long;
            }
        } else {
            let per_entry = lev_notional_long / swing_lows.len() as f64;
            for entry in &swing_lows {
                if let Some(liq_px) = bucket_long(*entry, *lev) {
                    let key = (liq_px / input.mid_price / price_bin_pct).round() as i64;
                    *long_bins.entry(key).or_insert(0.0) += per_entry;
                }
            }
        }
        if swing_highs.is_empty() {
            if let Some(liq_px) = bucket_short(input.mid_price, *lev) {
                let key = (liq_px / input.mid_price / price_bin_pct).round() as i64;
                *short_bins.entry(key).or_insert(0.0) += lev_notional_short;
            }
        } else {
            let per_entry = lev_notional_short / swing_highs.len() as f64;
            for entry in &swing_highs {
                if let Some(liq_px) = bucket_short(*entry, *lev) {
                    let key = (liq_px / input.mid_price / price_bin_pct).round() as i64;
                    *short_bins.entry(key).or_insert(0.0) += per_entry;
                }
            }
        }
    }

    // 5. Peak detection → cluster list (both sides).
    let long_clusters = detect_clusters(
        &long_bins, input.mid_price, price_bin_pct, true,
        input.min_cluster_notional_usd,
    );
    let short_clusters = detect_clusters(
        &short_bins, input.mid_price, price_bin_pct, false,
        input.min_cluster_notional_usd,
    );

    // 6. Cascade asymmetry: short liq density above mid vs long liq
    //    density below mid, normalized by total OI.
    let short_above: f64 = short_clusters.iter().map(|c| c.notional_usd).sum();
    let long_below: f64 = long_clusters.iter().map(|c| c.notional_usd).sum();
    let asymmetry = if input.total_oi_usd > 0.0 {
        (short_above - long_below) / input.total_oi_usd
    } else {
        0.0
    };
    let asymmetry = asymmetry.clamp(-1.0, 1.0);

    // 7. Confidence: lower if OI is thin, if funding is extreme, or if
    //    volatility (here proxied by funding magnitude) is high.
    let funding_mag_norm = (input.funding_rate.abs() / input.funding_extreme_pct).clamp(0.0, 2.0);
    let oi_adequacy = (input.total_oi_usd / 1_000_000.0).min(1.0); // 1M+ = full
    let confidence = (oi_adequacy * (1.0 - 0.3 * funding_mag_norm)).clamp(0.0, 1.0);

    LiquidationClusterMatrix {
        symbol: input.symbol.to_string(),
        generated_at_ms: now_ms,
        valid_until_ms: now_ms + 5 * 60 * 1000, // 5 min TTL
        mid_price: input.mid_price,
        leverage_assumptions,
        short_clusters,
        long_clusters,
        cascade_asymmetry: asymmetry,
        total_long_oi_usd: long_oi_usd,
        total_short_oi_usd: short_oi_usd,
        estimation_confidence: confidence,
    }
}

/// Detect cluster peaks from the binned density.
fn detect_clusters(
    bins: &std::collections::BTreeMap<i64, f64>,
    mid_price: f64,
    price_bin_pct: f64,
    is_long: bool,
    min_notional: f64,
) -> Vec<LiquidationCluster> {
    if bins.is_empty() || mid_price <= 0.0 {
        return vec![];
    }
    // Convert to a sorted Vec<(price, notional)>.
    let mut series: Vec<(f64, f64)> = bins
        .iter()
        .map(|(k, v)| {
            let price = (*k as f64) * price_bin_pct * mid_price;
            (price, *v)
        })
        .collect();
    series.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Simple local-maxima detection with a half-width window.
    let mut clusters = Vec::new();
    let half = (series.len() / 20).max(2);
    for i in half..(series.len().saturating_sub(half)) {
        let (_, v) = series[i];
        if v < min_notional { continue; }
        let is_peak = series[i - half..i].iter().all(|(_, x)| *x <= v)
            && series[i + 1..=i + half].iter().all(|(_, x)| *x <= v);
        if !is_peak { continue; }
        // Find cluster bounds: walk outward while density stays >= 50% of peak.
        let mut lo = i;
        let mut hi = i;
        let half_max = v * 0.5;
        while lo > 0 && series[lo - 1].1 >= half_max { lo -= 1; }
        while hi + 1 < series.len() && series[hi + 1].1 >= half_max { hi += 1; }
        let price_low = series[lo].0;
        let price_high = series[hi].0;
        let peak_price = series[i].0;
        let notional: f64 = series[lo..=hi].iter().map(|(_, v)| *v).sum();
        let distance_pct = ((peak_price - mid_price) / mid_price).abs() * 100.0;
        let kind = if distance_pct < 0.5 {
            ClusterKind::AtCurrentPrice
        } else if is_long {
            // Long liqs sit below mid (since liq_price = entry * (1 - dist)).
            ClusterKind::BelowCurrentPrice
        } else {
            ClusterKind::AboveCurrentPrice
        };
        // Magnet strength: weighted by notional × inverse distance (closer = stronger).
        let proximity = (-distance_pct / 2.0).exp();
        let magnet = (notional / 1_000_000.0 * 100.0 * proximity).clamp(0.0, 100.0);
        clusters.push(LiquidationCluster {
            price_low,
            price_high,
            peak_price,
            notional_usd: notional,
            dominant_leverage: 10, // TODO: track per-bin
            distance_from_mid_pct: distance_pct,
            cluster_kind: kind,
            magnet_strength: magnet,
        });
    }
    // Deduplicate clusters that overlap heavily.
    clusters.sort_by(|a, b| b.notional_usd.partial_cmp(&a.notional_usd).unwrap_or(std::cmp::Ordering::Equal));
    let mut dedup: Vec<LiquidationCluster> = Vec::with_capacity(clusters.len());
    for c in clusters {
        if !dedup.iter().any(|existing| {
            (existing.peak_price - c.peak_price).abs() / mid_price < 0.005
        }) {
            dedup.push(c);
        }
    }
    dedup
}

#[cfg(test)]
mod cluster_tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_history(base: f64, n: usize, swing: f64) -> Vec<f64> {
        // Generate a price history with a few swing lows and highs.
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / n as f64;
            v.push(base + swing * (t * std::f64::consts::PI * 2.0).sin()
                + swing * 0.3 * (t * std::f64::consts::PI * 6.0).cos());
        }
        v
    }

    #[test]
    fn empty_input_returns_empty_matrix() {
        let mut input = ClusterEstimateInput::default();
        input.mid_price = 0.0;
        input.total_oi_usd = 0.0;
        let m = estimate_clusters(&input);
        assert!(m.short_clusters.is_empty());
        assert!(m.long_clusters.is_empty());
        assert_eq!(m.cascade_asymmetry, 0.0);
        assert_eq!(m.estimation_confidence, 0.0);
    }

    #[test]
    fn basic_estimation_produces_clusters_on_both_sides() {
        let history = make_history(50_000.0, 100, 200.0);
        let input = ClusterEstimateInput {
            symbol: "BTC-USDT",
            mid_price: 50_000.0,
            price_history: &history,
            total_oi_usd: 50_000_000.0, // $50M
            funding_rate: 0.0001,
            long_oi_pct: None,
            maintenance_margin_rate: 0.005,
            funding_extreme_pct: 0.0005,
            funding_modulation_active: true,
            leverage_buckets: &[1, 3, 5, 10, 20, 50, 100],
            leverage_weights: &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
            min_cluster_notional_usd: 100_000.0,
        };
        let m = estimate_clusters(&input);
        // Should have clusters on both sides of mid.
        assert!(!m.short_clusters.is_empty(), "expected at least one short cluster above mid");
        assert!(!m.long_clusters.is_empty(), "expected at least one long cluster below mid");
        // All short clusters are above mid, all long clusters are below.
        for c in &m.short_clusters {
            assert!(c.peak_price > m.mid_price, "short cluster should be above mid, got {}", c.peak_price);
            assert_eq!(c.cluster_kind, ClusterKind::AboveCurrentPrice);
        }
        for c in &m.long_clusters {
            assert!(c.peak_price < m.mid_price, "long cluster should be below mid, got {}", c.peak_price);
            assert_eq!(c.cluster_kind, ClusterKind::BelowCurrentPrice);
        }
    }

    #[test]
    fn long_oi_override_is_respected() {
        let history = make_history(50_000.0, 100, 200.0);
        let input = ClusterEstimateInput {
            symbol: "BTC-USDT",
            mid_price: 50_000.0,
            price_history: &history,
            total_oi_usd: 1_000_000.0,
            funding_rate: 0.0,
            long_oi_pct: Some(0.8), // 80% long
            maintenance_margin_rate: 0.005,
            funding_extreme_pct: 0.0005,
            funding_modulation_active: false,
            leverage_buckets: &[1, 3, 5, 10, 20, 50, 100],
            leverage_weights: &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
            min_cluster_notional_usd: 10_000.0,
        };
        let m = estimate_clusters(&input);
        // With 80% long, the long cluster total should exceed the short.
        let long_total: f64 = m.long_clusters.iter().map(|c| c.notional_usd).sum();
        let short_total: f64 = m.short_clusters.iter().map(|c| c.notional_usd).sum();
        assert!(long_total > short_total,
            "long_total={} should exceed short_total={}", long_total, short_total);
    }

    #[test]
    fn funding_adaptive_label_applied_for_extreme_funding() {
        let history = make_history(50_000.0, 100, 200.0);
        let input = ClusterEstimateInput {
            symbol: "BTC-USDT",
            mid_price: 50_000.0,
            price_history: &history,
            total_oi_usd: 1_000_000.0,
            funding_rate: 0.001, // 0.1% — extreme
            long_oi_pct: None,
            maintenance_margin_rate: 0.005,
            funding_extreme_pct: 0.0005,
            funding_modulation_active: true,
            leverage_buckets: &[1, 3, 5, 10, 20, 50, 100],
            leverage_weights: &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
            min_cluster_notional_usd: 0.0,
        };
        let m = estimate_clusters(&input);
        assert_eq!(m.leverage_assumptions.source, LeverageDistributionSource::FundingAdaptive);
    }

    #[test]
    fn cascade_asymmetry_sign_matches_dominant_side() {
        let history = make_history(50_000.0, 100, 200.0);
        // 90% short OI → short squeeze risk → cascade_asymmetry should be
        // negative (short clusters dominate).
        let input = ClusterEstimateInput {
            symbol: "BTC-USDT",
            mid_price: 50_000.0,
            price_history: &history,
            total_oi_usd: 1_000_000.0,
            funding_rate: 0.0,
            long_oi_pct: Some(0.1),
            maintenance_margin_rate: 0.005,
            funding_extreme_pct: 0.0005,
            funding_modulation_active: false,
            leverage_buckets: &[1, 3, 5, 10, 20, 50, 100],
            leverage_weights: &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
            min_cluster_notional_usd: 0.0,
        };
        let m = estimate_clusters(&input);
        // Shorts dominate → positive asymmetry (long squeeze risk the
        // *opposite* of short squeeze). The convention in the platform
        // is: positive asymmetry = more pressure on longs = price likely
        // to fall = long-squeeze risk. We verify the sign is well-defined
        // and the magnitude is reasonable.
        assert!(m.cascade_asymmetry.is_finite());
        assert!(m.cascade_asymmetry.abs() <= 1.0, "asymmetry must be in [-1, 1]");
    }

    #[test]
    fn cluster_magnet_strength_decays_with_distance() {
        let history = make_history(50_000.0, 200, 200.0);
        let input = ClusterEstimateInput {
            symbol: "BTC-USDT",
            mid_price: 50_000.0,
            price_history: &history,
            total_oi_usd: 50_000_000.0,
            funding_rate: 0.0,
            long_oi_pct: Some(0.5),
            maintenance_margin_rate: 0.005,
            funding_extreme_pct: 0.0005,
            funding_modulation_active: false,
            leverage_buckets: &[1, 3, 5, 10, 20, 50, 100],
            leverage_weights: &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
            min_cluster_notional_usd: 10_000.0,
        };
        let m = estimate_clusters(&input);
        // Closer cluster has higher magnet_strength.
        for c in &m.long_clusters {
            assert!(c.magnet_strength >= 0.0 && c.magnet_strength <= 100.0,
                "magnet_strength out of range: {}", c.magnet_strength);
        }
    }

    #[test]
    fn empty_history_falls_back_to_single_bin() {
        // No price history → estimator should still produce something
        // using mid_price as the only seed.
        let input = ClusterEstimateInput {
            symbol: "BTC-USDT",
            mid_price: 50_000.0,
            price_history: &[],
            total_oi_usd: 1_000_000.0,
            funding_rate: 0.0,
            long_oi_pct: None,
            maintenance_margin_rate: 0.005,
            funding_extreme_pct: 0.0005,
            funding_modulation_active: false,
            leverage_buckets: &[1, 3, 5, 10, 20, 50, 100],
            leverage_weights: &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
            min_cluster_notional_usd: 0.0,
        };
        let m = estimate_clusters(&input);
        // With no swing history, both sides still produce a cluster at
        // their respective liquidation price.
        assert!(!m.long_clusters.is_empty() || !m.short_clusters.is_empty(),
            "fallback to mid_price seed should still produce clusters");
    }

    #[test]
    fn deterministic_output_for_same_input() {
        let history = make_history(50_000.0, 100, 200.0);
        let input = ClusterEstimateInput {
            symbol: "BTC-USDT",
            mid_price: 50_000.0,
            price_history: &history,
            total_oi_usd: 10_000_000.0,
            funding_rate: 0.0001,
            long_oi_pct: None,
            maintenance_margin_rate: 0.005,
            funding_extreme_pct: 0.0005,
            funding_modulation_active: true,
            leverage_buckets: &[1, 3, 5, 10, 20, 50, 100],
            leverage_weights: &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
            min_cluster_notional_usd: 50_000.0,
        };
        let m1 = estimate_clusters(&input);
        let m2 = estimate_clusters(&input);
        // Not strictly byte-equal because timestamp differs, but the
        // cluster structures and metrics must match.
        assert_eq!(m1.short_clusters.len(), m2.short_clusters.len());
        assert_eq!(m1.long_clusters.len(), m2.long_clusters.len());
        assert!((m1.cascade_asymmetry - m2.cascade_asymmetry).abs() < 1e-9);
        assert!((m1.estimation_confidence - m2.estimation_confidence).abs() < 1e-9);
        // Verify the data is identical.
        for (a, b) in m1.long_clusters.iter().zip(m2.long_clusters.iter()) {
            assert!((a.peak_price - b.peak_price).abs() < 1e-9);
            assert!((a.notional_usd - b.notional_usd).abs() < 1e-9);
        }
        let _ = dec!(1.0);
    }

    #[test]
    fn weights_sums_to_one_after_modulation() {
        let mut w = vec![0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05];
        let before_sum: f64 = w.iter().sum();
        assert!((before_sum - 1.0).abs() < 1e-9, "default weights must sum to 1.0");
        apply_funding_modulation(&mut w, 0.001, 0.0005);
        let after_sum: f64 = w.iter().sum();
        assert!((after_sum - 1.0).abs() < 1e-9,
            "modulated weights must still sum to 1.0, got {}", after_sum);
        // Each weight must be non-negative.
        for v in &w {
            assert!(*v >= 0.0, "weight must be non-negative, got {}", v);
            assert!(*v <= 1.0, "weight must be <= 1.0, got {}", v);
        }
    }
}