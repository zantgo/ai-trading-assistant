# 02-12: LiquidityMatrix — Real Liquidation Flow (Phase 1)

**Version:** 6.10 (2026-08-14) — see docs/CHANGELOG.md for the canonical version history.

**Producer:** DIE L1 (NormalizedEvent::Liquidation via WS liquidation events; persisted by the telemetry logger to liquidation_events, 90-day retention) → MME L1.5 (per-candle aggregation)
**Consumer:** MME L5 (Risk) — `cascade_risk` dimension; MME L6 (Decision) — Advisory rationale; Overview — cross-symbol aggregate
**Per-bar:** yes (computed on every completed candle)
**Snapshot field:** `MarketSnapshot.liquidity: Option<LiquidityFlow>`

The LiquidityMatrix carries the **per-candle aggregate of real liquidation
events** observed on the exchange WebSocket during the current bar. It
is the ground-truth signal — every field is derived from published
exchange data, not estimated.

## 1. Why this exists

Liquidations are the **only major market microstructure event that
exchanges publish in near-real-time**. They are observable. They are
loud. They mark inflection points. The platform needs to track them
faithfully so cascade detection, risk scoring, and cluster estimation
have a real input rather than an estimated one.

## 2. Data sources

- **Hyperliquid**: subscribe to `userFills` channel. Each fill entry
  has a `liquidation` field; non-empty values are force-closed positions.
- **Bitget**: subscribe to `fill` channel. Entries with `execType == "L"`
  are liquidations.

The raw events are persisted to `liquidation_events` (90-day
retention, enforced by hourly cleanup in the telemetry logger).

## 3. Schema

```rust
pub struct LiquidityFlow {
    pub long_liquidations_usd: f64,         // sum since last completed candle
    pub short_liquidations_usd: f64,        // sum since last completed candle
    pub net_liquidation_usd: f64,           // long - short; +ve = longs dumped
    pub event_count: u32,
    pub largest_event_usd: f64,
    pub largest_event_price: Option<f64>,
    pub largest_event_side: Option<LiquidationSide>,
    pub cascade_state: CascadeState,
    pub cascade_intensity: f64,             // 0..100
}

pub enum CascadeState {
    None,
    Detected,    // 1 event in rolling window above z-score threshold
    Sustained,   // 3+ events in rolling window
    Exhausted,   // bar intensity declining after elevated state
}

pub enum LiquidationSide { Long, Short }
```

## 4. Sign convention

`net_liquidation_usd = long_liquidations_usd - short_liquidations_usd`

- **Positive** = more longs got dumped = bearish pressure (longs were
  forced sellers, adding to the sell side).
- **Negative** = more shorts got dumped = bullish pressure (short
  squeeze; shorts were forced buyers).

## 5. Cascade state machine

The accumulator runs a rolling window of recent events for event-rate context. For each completed bar, it computes a z-score from that bar's per-bar notional relative to the running mean and standard deviation of per-bar notional. A single event crossing the threshold → `Detected`. Three or more events crossing the threshold within the window → `Sustained`. Declining intensity after `Sustained` → `Exhausted`.

## Cascade Intensity Computation (`LiquidityFlow.cascade_intensity`)

The `cascade_intensity` field is the **canonical risk-feed value** consumed by `RiskMatrix.cascade_risk` (see [02-11-risk-matrix.md §4.8](../matrices/02-11-risk-matrix.md)) and surfaced on the Frontend's `LiquidityPanel` (§[07-04-ui-liquidity-panel-spec.md Flow tab](../ui-ux/07-04-ui-liquidity-panel-spec.md)). This section is the **single canonical specification** of how the value is computed. The implementation lives in `crates/core-domain/src/liquidity.rs::LiquidityEventAccumulator::update`; the equations below mirror that implementation 1:1.

### Windowing

- **Rolling baseline window** — `W = 200` completed micro candles (configurable via `config.toml` `[liquidity.cascade_baseline_window_bars]`, default 200). The baseline statistics are computed over the last `W` completed bars' per-bar notional volume: mean `μ` (micro-window) and standard deviation `σ` (micro-window). On engine start (zero history) and when fewer than `min(window_bars, 30)` completed bars are available, the baseline is treated as `μ = 0, σ = 0` and the z-score is defined as `0` (no abnormal intensity claim is made until the warm-up threshold is reached — see "Warm-up reset behavior" below).
- **Recent-event window** — `K = 20` most recent liquidation events (the recent-events rolling buffer used for event-rate context and the state-machine classification in the next section). This window does not supply the notional units for `cascade_intensity`; intensity is computed from the most recent completed bar's aggregate notional.

### Per-bar notional definition

For each completed micro candle `b` in the baseline window, the per-bar notional is:

```
n_b = sum of long_liquidations_usd(b) + sum of short_liquidations_usd(b)
```

(where the per-side sums are themselves the per-bar aggregate published in `LiquidityFlow`.)

### Per-bar intensity formula

For the most recent completed bar, all notional values remain in per-bar units. The `K = 20` recent-event window supplies event-rate context only; it does not convert the intensity calculation to per-event units.

```
bar_notional       = long_liquidations_usd(current_bar) + short_liquidations_usd(current_bar)
baseline_mean      = mean(per-bar notionals in the W-bar baseline window)
baseline_std       = sample standard deviation of per-bar notionals in W
recent_event_count = count of liquidation events in the K=20 rolling-event window

if baseline_std > 0:
    z_score = (bar_notional - baseline_mean) / baseline_std
else:
    z_score = 0

# Map z-score to 0..100 intensity
raw_intensity = clamp(50 + z_score * 12.5, 0, 100)   # z=0 → 50; z=+4 → 100; z=-4 → 0
```

The constants in the linear map (`+50` midpoint, `12.5` scaling) are fixed at the canonical values above; `clamp` guarantees the field stays in `[0, 100]` per the schema.

### Warm-up reset behavior

- **Initial cold start** (no completed micro candles): `cascade_intensity = 0.0` and `cascade_state = None`.
- **Warm-up** (`window_bars < 30` baseline bars): the baseline is treated as `μ = 0, σ = 0`, and `cascade_intensity = raw_intensity` evaluated without z-score normalization — i.e. it is the un-normalized 0..100 scaled value, but consumers should treat it as "not yet statistically meaningful" via the `cascade_state = None` invariant (no `Detected`/`Sustained`/`Exhausted` transition can fire until the warm-up threshold is met).
- **Stable state** (`window_bars ≥ 30`): the canonical z-score formula above applies.
- **Consumer gating:** consumers gate on `cascade_state == null` and render intensity as no-data (not amber) while warm-up is in effect.

### Relationship to `cascade_state`

`cascade_state` is a discrete classification over the recent-event window (`K = 20`) using event-rate context together with the per-bar z-score computed above; see the "Cascade state machine" section above. `cascade_intensity` (continuous 0..100) is published on every candle; `cascade_state` advances only when the discrete thresholds (`cascade_detected_zscore` for `Detected`, `cascade_sustained_events` for `Sustained`) are crossed.

### Consumer contract

- **`RiskMatrix.cascade_risk` (L5)** consumes `cascade_intensity` directly as `score = max(score, flow.cascade_intensity)` (see [02-11-risk-matrix.md §4.8](../matrices/02-11-risk-matrix.md)). The discrete `cascade_state` adds a risk premium on top of the intensity (`+15` for `Detected`, `+30` for `Sustained`, `+0` for `Exhausted`).
- **`LiquidityPanel`** displays `cascade_intensity` numerically (0..100) and color-codes it relative to the per-bar thresholds (blue ≤ 30, amber ≤ 60, red > 60) for the operator's situational awareness (see [07-04-ui-liquidity-panel-spec.md Flow tab](../ui-ux/07-04-ui-liquidity-panel-spec.md)). Red = bearish cascade pressure, amber = moderate risk, blue = calm/safe (see canonical conventions at [07-06-ui-color-conventions.md](../ui-ux/07-06-ui-color-conventions.md)).

## 5.1 Frontend exposure

`MarketSnapshot.liquidity` rides the WebSocket frame to the frontend
under `liquidity`. The LiquidityPanel (Phase 4) renders the Flow tab
from this field.

## 6. Configuration Surface

The Liquidity Intelligence configuration is set in `config.toml` under the `[liquidity]` table. The canonical configuration surface is defined by `crates/config-models/src/models.rs::LiquidityConfig`; the TOML below mirrors that struct exactly. The legacy `config.json` reader path in `load_config()` is preserved for backward compatibility but is **scheduled for removal at v7.0**; new deploys must use `config.toml`.

```toml
[liquidity]
enabled = true
mark_price_poll_ms = 60000
funding_refresh_ms = 60000
event_retention_days = 90
bucket_retention_days = 7
cluster_refresh_secs = 300
maintenance_margin_rate = 0.005
cascade_detected_zscore = 2.5
cascade_sustained_events = 3
cascade_baseline_window_bars = 200
cascade_min_warmup_bars = 30
funding_extreme_pct = 0.0005
magnet_activation_distance_pct = 0.5
liquidity_vacuum_threshold = 0.3
oi_funding_divergence_pct = 2.0
```

| Field | Default | Description |
|------|---------|-------------|
| `enabled` | `true` | Master switch for the Liquidity Intelligence extension. |
| `mark_price_poll_ms` | `60000` | Hyperliquid mark-price / OI / funding polling cadence. |
| `funding_refresh_ms` | `60000` | Bitget funding refresh floor. |
| `event_retention_days` | `90` | Raw `liquidation_events` retention. |
| `bucket_retention_days` | `7` | Aggregated bucket retention. |
| `cluster_refresh_secs` | `300` | Cluster matrix refresh interval (5 min default). |
| `maintenance_margin_rate` | `0.005` | Industry-standard 0.5 % maintenance margin for perpetuals. |
| `cascade_detected_zscore` | `2.5` | Z-score above which a single event triggers `Detected` state. |
| `cascade_sustained_events` | `3` | Min events in the window to escalate to `Sustained`. |
| `cascade_baseline_window_bars` | `200` | Baseline rolling-window size in completed micro candles used for the `cascade_intensity` z-score computation (see §Cascade Intensity Computation above). |
| `cascade_min_warmup_bars` | `30` | Minimum completed-bar count before the z-score baseline is treated as statistically meaningful (below this threshold, `cascade_state = None` and the un-normalized intensity is published; above this threshold, the canonical z-score formula applies). |
| `funding_extreme_pct` | `0.0005` | Funding rate extreme threshold (0.05 % / 8 h). |
| `magnet_activation_distance_pct` | `0.5` | Distance from mid that activates a cluster magnet (0.5 %). |
| `liquidity_vacuum_threshold` | `0.3` | Liquidity-vacuum detection threshold. |
| `oi_funding_divergence_pct` | `2.0` | OI/funding divergence percentage. |

> **Configuration key alignment (v2.1 — canonical).** A previous version of this table used different keys (`cascade_z_score_threshold`, `cascade_sustained_min_events`, `cluster_refresh_interval_secs`, `funding_flip_threshold_pct`, `cascade_rolling_window_bars`, `oi_divergence_window_bars`) that did not match the runtime `LiquidityConfig` struct or the [01-05-liquidity-domain.md §Configuration](../conceptual-foundations/01-05-liquidity-domain.md) source-of-truth block. The corrected surface above is the single canonical configuration; the runtime enforces it via `serde` deserialization in `crates/config-models/src/models.rs`.