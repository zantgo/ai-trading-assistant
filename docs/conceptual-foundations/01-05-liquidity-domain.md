# Phase 0-4 Liquidity Intelligence — Architecture Spec

**Status:** Implemented (Phases 0-4)
**Owner:** MME (Market Monitoring Engine), with extensions to TAE / PME
**Version:** v1.0 (2026-07-15)

## Overview

The platform ingests real-time derivatives data, aggregates real
liquidation events, and reconstructs a probability-weighted estimate
of where the next cascade will come from. The user sees:

1. **Real flow** — what the exchange actually just told us about forced
   closes (no estimation, no ML, no smoothing).
2. **Estimated heatmap** — where the platform believes future
   liquidations will concentrate if price moves.
3. **Live signals** — discrete flags that drive risk scoring, decision
   guidance, and UI alerts.

## Phase breakdown

| Phase | Output | Source of truth |
|---|---|---|
| **0** | Mark price, OI, funding rate on every snapshot | Exchange WS (Hyperliquid activeAssetCtx / Bitget ticker+funding-rate) + REST polling fallback |
| **1** | `LiquidityFlow` per candle (real liquidation events) | Exchange WS userFills (HL) / fill (Bitget) |
| **2** | `LiquidationClusterMatrix` every 5 min (estimated heatmap) | Deterministic estimator on (OI + funding + price history) |
| **3** | 7 `LiquiditySignalKind` signals per snapshot | Discrete rules on (flow + cluster + funding) |
| **4** | Frontend `LiquidityPanel` (Flow / Cluster / Context) | WebSocket frame field `liquidity` + `cluster` + `liquidity_signals` |

## Data flow

```
Exchange WS
    │
    ├─ Trades → CandleGenerator → per-TF analyzer
    ├─ LiquidationEvent → LiquidityEventAccumulator
    │   └─ On candle close: flush_to_flow() → LiquidityFlow
    │
    ├─ Mark/Funding/OI → latest_*_px RwLock
    │   └─ On candle close: attach to MarketSnapshot
    │
    └─ 5-min cluster refresh task (per pair)
        └─ read OI + funding + history → estimate_clusters()
            └─ write to cluster_matrix RwLock
                └─ On candle close: attach to MarketSnapshot

MarketSnapshot (per candle)
    ├─ liquidity: Option<LiquidityFlow>     (Phase 1)
    ├─ cluster:   Option<LiquidationClusterMatrix>  (Phase 2)
    └─ on send → derive_liquidity_signals()  (Phase 3)
        └─ attached to MarketSnapshot.indicators as JSON-encoded list
```

## Architectural placement

The Liquidity Intelligence subsystem extends the existing two-
dimensional architecture (5 engines × N layers) without creating a
sixth engine. The new components live in MME L1.5 (per-candle
liquidity accumulation) and MME L2.5 (cluster estimation). This
preserves the unidirectional cascade: MME L1.5 → MME L2.5 → MME L5
(Risk) → MME L6 (Decision) → TAE / PME.

The unidirectional invariant is preserved because:

- Phase 1 reads only from the WS event stream (DIE → MME L1.5).
- Phase 2 reads from Phase 0 fields (OI, funding, mark) and
  computes a derived value. It does NOT read from Phase 1 (avoids
  feedback).
- Phase 3 reads from Phase 1 + Phase 2 and produces indicators that
  ride the existing `IndicatorMap` channel — no new architectural
  surface is exposed downstream.

## Risk integration

`RiskMatrix` gained a 9th dimension: `cascade_risk`. It is computed
from `LiquidityFlow.cascade_intensity` and `LiquidationClusterMatrix.
cascade_asymmetry`. The existing 8 dimensions are unchanged, with
weights re-normalized so the overall score is still 0..100.

The legacy `liquidity_risk` field was renamed to
`execution_liquidity_risk` (with a serde alias) to free the
"liquidity" term for the positional concept.

## Decision integration

A 7th `OpportunityType::LiquiditySqueeze` variant was added. The
AdvisoryMatrix maps it through to `OpportunityClass::LiquiditySqueeze`
so the existing decision rendering pipeline picks it up.

## Backward compatibility

All schema additions use `#[serde(default)]` so older snapshots
without the new fields deserialize cleanly. The legacy
`liquidity_risk` field is kept as an optional alias of
`execution_liquidity_risk`.

## Configuration

`config.toml` has a new `[liquidity]` section:

```toml
[liquidity]
enabled = true                       # master switch
mark_price_poll_ms = 60000           # HL mark/OI/funding poll cadence
funding_refresh_ms = 60000           # Bitget funding refresh floor
event_retention_days = 90            # raw liquidation_events retention
bucket_retention_days = 7            # aggregated buckets retention
cluster_refresh_secs = 300           # cluster matrix refresh
maintenance_margin_rate = 0.005      # 0.5% (industry standard for perps)
cascade_detected_zscore = 2.5        # single-event cascade trigger
cascade_sustained_events = 3         # events in window for Sustained
funding_extreme_pct = 0.0005         # 0.05% / 8h
magnet_activation_distance_pct = 0.5 # 0.5% from mid
liquidity_vacuum_threshold = 0.3
oi_funding_divergence_pct = 2.0
```

## Performance

- Phase 0 (HL polling): 60s REST round-trip per pair, ~few KB.
- Phase 1 (event ingestion): O(1) per event, ~1µs.
- Phase 1 (per-candle flow): O(events in bar) ≈ O(50) typical.
- Phase 2 (cluster estimation): O(P × L) per refresh ≈ O(3,500) ops.
  Refresh every 5 min, <1ms compute.
- Phase 3 (signal derivation): O(1) per snapshot.
- Phase 4 (frontend): no measurable overhead — fields ride existing
  WS frame.

Total per-candle overhead: <5ms. Total memory: <300KB per pair per TF.

## Test coverage

| Phase | Unit tests | Integration tests | Total |
|---|---|---|---|
| 0 | 11 | 0 | 11 |
| 1 | 15 | 1 | 16 |
| 2 | 14 | 0 | 14 |
| 3 | 10 | 0 | 10 |
| 4 | 5 | 0 | 5 |
| **Total** | **55** | **1** | **56** |

All 56 new tests pass. No existing tests were broken by the
implementation.

## Open questions / future work

- Add a marker overlay on PriceChart.svelte for cluster positions
  (deferred from Phase 4 to keep the initial render simple).
- Cross-symbol cascade_risk aggregation is exposed via
  `OverviewMatrix.cascade_risk_index` but not yet wired into the
  `systemic_risk_score` formula.
- The PAE (Performance Analytics Engine) could later consume
  `liquidation_events` for cascade-conditioned strategy backtesting.