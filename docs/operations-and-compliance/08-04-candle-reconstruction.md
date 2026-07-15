# Candle Reconstruction

**Status:** Implemented
**Module:** `crates/engine/src/adapters/reconstruction.rs`
**Spec version:** 1.0

## Purpose

When the WebSocket connection drops, the engine's candle stream has a gap. On reconnect, the engine must reconstruct the missing candles so downstream indicators, signals, and the MME pipeline operate on a continuous history. Reconstruction fidelity is critical: a 5-minute gap during high-volatility price action produces 5 consecutive false closes if not filled.

## Two Reconstruction Strategies

The strategy is chosen based on candle duration:

| Duration | Strategy | Source |
|----------|----------|--------|
| ≥ 1 minute | **Exchange historical fetch** | Hyperliquid `/info` candle snapshot; Bitget `/api/v2/mix/market/candles` |
| < 1 minute | **Synthesis from recent history** | EMA of last N=200 micro-candles (preferred) or linear interpolation between last 2 closes (fallback) |

## Reconstruction Method Enum

```rust
pub enum ReconstructionMethod {
    ExchangeHistorical,       // >= 1m: fetched from exchange REST
    ExponentialMovingAverage,  // < 1m, ≥50 history points: EMA projection
    LinearInterpolation,      // < 1m, ≥2 history points: linear interp
    Unavailable,              // < 1m, <2 history points: cannot reconstruct
}
```

The reconstructed candle is returned with a `ReconstructedCandle` envelope:

```rust
pub struct ReconstructedCandle {
    pub candle: NormalizedCandle,
    pub method: ReconstructionMethod,
    pub source_gap_start_ms: u64,
    pub source_gap_end_ms: u64,
}
```

## EMA Synthesis (Sub-Minute)

For sub-1m candles with sufficient history (≥ 50 recent closes), the synthesised OHLC is:

```
α   = 2 / (N + 1)                        where N = ema_window (default 200)
EMA_t = α × close_t + (1 − α) × EMA_{t−1}
OHLC  = EMA_final                         (flat candle — no intra-bar trade info)
Volume = 0
```

The flat-candle assumption is explicit: with no trade tape to reconstruct from, the most defensible estimate is that the candle traded at the EMA value. Downstream consumers can detect this via the `reconstructed: Some(ExponentialMovingAverage)` flag on the `NormalizedCandle`.

> **EMA warm-up with fewer than 200 bars.** EMA operates with `N = ema_window (default 200)` regardless of available buffer size. With only 50 closes, the smoothing factor `α = 2/(200+1) ≈ 0.00995` is applied over those 50 closes; the reconstructed candle reflects the slower EMA output (heavily weighted toward the most recent close but with substantial inertia from earlier values). Operators may lower `ema_window` via `config.json` (`adapters.ema_window`) for faster adaptation at the cost of noise sensitivity.
>
> **Volume dilution caveat.** Sub-minute reconstructed candles have `volume = 0` (no trade tape). When rolled up to higher timeframes (e.g. 15s → 1m), the aggregate volume is the sum of constituent micro-volumes; a 1m candle aggregating four 15s reconstructed candles has `volume = 0` if any constituent was reconstructed. Operators should treat volume from periods containing reconstructed sub-minute candles as informational, not authoritative.
>
> **Cold-start minimums.** The reconstruction engine requires: (a) ≥ 2 recent closes for linear interpolation fallback (sub-minute), (b) ≥ 50 recent closes for EMA synthesis (sub-minute preferred), (c) ≥ 50 closes per timeframe for indicator warm-up (any timeframe). On cold start with zero history, all indicators emit `state_label = INSUFFICIENT_DATA` and `confidence = 0.0` until the minimum buffer is reached. The minimum warm-up duration is `min_buffer × duration_seconds` — for a 1m micro timeframe with 50-bar minimum EMA warm-up, this is ~50 minutes.

## Linear Interpolation (Sub-Minute Fallback)

For sub-1m candles with minimal history (2 ≤ N < 50), the synthesised value is the linear extrapolation of the last two closes:

```
slope     = c_n − c_{n−1}
c_target  = c_n + slope = 2·c_n − c_{n−1}
OHLC      = c_target
Volume    = 0
```

> **Multi-step linear interpolation.** For a gap of N consecutive missing candles, the linear slope `slope = c_n − c_{n−1}` is applied repeatedly: `c_target_k = c_n + k × slope` for `k = 1, 2, …, N`. Each missing candle receives its own linearly-projected close; OHLC = c_target_k; Volume = 0. This produces a straight-line extrapolation that maintains the slope at the time of the gap.

This is less accurate than EMA but is the best forward-looking estimate when only two data points are available.

## Exchange Historical (≥ 1 Minute)

For 1m candles and longer, the engine delegates to a `ExchangeHistoricalFetcher` trait that exchange-specific implementations (Hyperliquid, Bitget) implement to query the REST API. The candles are returned as-is and tagged with `ReconstructionMethod::ExchangeHistorical`.

## Forwarding Through Aggregation

Aggregated macro candles (4h, 1d) built from 1m source candles **forward** the source candle's `reconstructed` flag rather than blanking it. This preserves provenance through aggregation chains: a 1d candle is marked as `reconstructed` if any of its 1,440 1m constituents is reconstructed.

## Gap Detection

The `GapDetector` decides whether reconstruction is required:

```rust
pub fn detect_gap(
    last_persisted_ts_ms: u64,
    now_ms: u64,
    gap_threshold_secs: u64,
) -> Option<(u64, u64)>
```

Returns `Some((gap_start, gap_end))` when the elapsed time since the last persisted candle exceeds the threshold; `None` otherwise. The threshold is configurable per-exchange.

**Default value.** `gap_threshold_secs = 2 × candles.duration_seconds` (twice the base micro timeframe duration, e.g. `120` for the default `micro60`). This prevents false gap detections from clock jitter while still catching real disconnects. Operators may override per-exchange via `config.json` (`adapters.<exchange>.gap_threshold_secs`). Setting the threshold below the base candle duration will trigger false gap detections.

## Serialization

`NormalizedCandle.reconstructed: Option<ReconstructionMethod>` is annotated with `#[serde(default, skip_serializing_if = "Option::is_none")]` so:
- Live (non-reconstructed) candles emit the same JSON shape as before — no `"reconstructed":null` noise on the wire
- Legacy payloads and inbound WS frames without the field deserialize cleanly as `None`

## Testing

- 8 unit tests in `reconstruction.rs`:
  - `gap_detector_returns_none_when_within_threshold`
  - `gap_detector_returns_gap_when_exceeds_threshold`
  - `reconstruct_1m_returns_none_caller_uses_exchange`
  - `reconstruct_sub_1m_with_ema_history`
  - `reconstruct_sub_1m_with_minimal_history_uses_interpolation`
  - `reconstruct_returns_none_with_no_history`
  - `ema_produces_smooth_values`
  - `interpolation_is_linear`

## Cross-References

- [Connection Resilience](08-03-connection-resilience.md) — source of `ReconnectState` events that trigger reconstruction
- [Connection Quality](08-05-connection-quality.md) — counts reconstructed candles in the `reconstructed_candles` field
- [Risk Matrix §4.8](02-11-risk-matrix.md) — `cascade_risk` reads reconstructed candle provenance
