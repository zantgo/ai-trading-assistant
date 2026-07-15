# 02-12: LiquidityMatrix — Real Liquidation Flow (Phase 1)

**Producer:** DIE L2 (WS liquidation events) → MME L1.5 (per-candle aggregation)
**Consumer:** MME L5 (Risk) — `cascade_risk` dimension; MME L6 (Decision) — Advisory rationale; Overview — cross-symbol aggregate
**Per-bar:** yes (computed on every completed candle)
**Snapshot field:** `MarketSnapshot.liquidity: Option<LiquidityFlow>`

The LiquidityMatrix carries the **per-candle aggregate of real liquidation
events** observed on the exchange WebSocket during the current bar. It
is the ground-truth signal — every field is derived from published
exchange data, not estimated.

## Why this exists

Liquidations are the **only major market microstructure event that
exchanges publish in near-real-time**. They are observable. They are
loud. They mark inflection points. The platform needs to track them
faithfully so cascade detection, risk scoring, and cluster estimation
have a real input rather than an estimated one.

## Data sources

- **Hyperliquid**: subscribe to `userFills` channel. Each fill entry
  has a `liquidation` field; non-empty values are force-closed positions.
- **Bitget**: subscribe to `fill` channel. Entries with `execType == "L"`
  are liquidations.

The raw events are persisted to `liquidation_events` (90-day
retention, enforced by hourly cleanup in the telemetry logger).

## Schema

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

## Sign convention

`net_liquidation_usd = long_liquidations_usd - short_liquidations_usd`

- **Positive** = more longs got dumped = bearish pressure (longs were
  forced sellers, adding to the sell side).
- **Negative** = more shorts got dumped = bullish pressure (short
  squeeze; shorts were forced buyers).

## Cascade state machine

The accumulator runs a rolling window of recent events. For each event,
it computes a "z-score" relative to the running mean per-bar intensity.
A single event crossing the threshold → `Detected`. Three or more
events crossing the threshold within the window → `Sustained`.
Declining intensity after `Sustained` → `Exhausted`.

## Frontend exposure

`MarketSnapshot.liquidity` rides the WebSocket frame to the frontend
under `liquidity`. The LiquidityPanel (Phase 4) renders the Flow tab
from this field.

## 6. Configuration Surface

The Liquidity Intelligence configuration is set in `config.json` under the `liquidity` block:

```json
{
  "liquidity": {
    "enabled": true,
    "cascade_z_score_threshold": 2.0,
    "cascade_rolling_window_bars": 10,
    "cascade_sustained_min_events": 3,
    "cluster_refresh_interval_secs": 300,
    "funding_flip_threshold_pct": 0.01,
    "oi_divergence_window_bars": 5
  }
}
```

| Field | Default | Description |
|------|---------|-------------|
| `enabled` | `true` | Master switch for the Liquidity Intelligence extension. |
| `cascade_z_score_threshold` | `2.0` | Z-score above which a single event triggers `Detected` state. |
| `cascade_rolling_window_bars` | `10` | Rolling window (in candles) for the z-score computation. |
| `cascade_sustained_min_events` | `3` | Min events in the window to escalate to `Sustained`. |
| `cluster_refresh_interval_secs` | `300` | Cluster matrix refresh interval (5 min default). |
| `funding_flip_threshold_pct` | `0.01` | Funding rate change threshold for `LIQUIDITY_FUNDING_FLIP` signal. |
| `oi_divergence_window_bars` | `5` | OI divergence detection window. |

All fields have safe defaults. See [01-05-liquidity-domain.md §Configuration](../conceptual-foundations/01-05-liquidity-domain.md) for the canonical source.