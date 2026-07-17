# Liquidity Phase 0-4 — Architecture Spec

**Version:** 6.4 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Owner:** MME (Market Monitoring Engine), with extensions to TAE / PME

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
    ├─ liquidity:            Option<LiquidityFlow>                  (Liquidity Phase 1, top-level field)
    ├─ cluster:              Option<LiquidationClusterMatrix>      (Liquidity Phase 2, top-level field)
    ├─ liquidity_signals:    Vec<LiquiditySignal>                  (Liquidity Phase 3, top-level field — derived from liquidity + cluster)
    └─ statistical_context:  StatisticalContext                     (Monte Carlo + z-scores)

WS broadcast payload
    ├─ market_snapshots
    │   ├─ indicators (50 indicators + signals)
    │   ├─ context, alignment, analysis, decision_context, ...
    │   └─ liquidity, cluster, liquidity_signals              ← liquidity extension surface
    └─ sent as a single MarketSnapshot frame on /ws

> **Top-level liquidity fields.** The three liquidity fields (`liquidity`, `cluster`, `liquidity_signals`) are siblings of `indicators` on the `MarketSnapshot` wire frame — not nested within `indicators`. The canonical contract is in [`02-07-metrics-matrix.md §2.1`](../matrices/02-07-metrics-matrix.md); the underlying Rust type is in `crates/core-domain/src/models.rs`. Placement within `indicators` would have contradicted both the Metrics Matrix contract and the canonical wire-frame definition.
```

## Architectural placement

The Liquidity Intelligence subsystem extends the existing two-
dimensional architecture (5 engines × N layers) without creating a
sixth engine. The new components live in MME L1.5 (per-candle
liquidity accumulation) and MME L2.5 (cluster estimation). This
preserves the unidirectional cascade: MME L1.5 → {MME L4, MME L5};
MME L2.5 → {MME L4, MME L5}; MME L4 + MME L5 → MME L6 (Decision) →
TAE / PME. (L4 consumes the `LiquiditySqueeze` preconditions from
L1.5/L2.5 — see Decision integration below.)

The unidirectional invariant is preserved because:

- Liquidity Phase 1 reads only from the WS event stream (DIE → MME L1.5).
- Liquidity Phase 2 reads from Liquidity Phase 0 fields (OI, funding, mark) and
  computes a derived value. It does NOT read from Liquidity Phase 1 (avoids
  feedback).
- Liquidity Phase 3 reads from Liquidity Phase 1 + Liquidity Phase 2 and produces liquidity-derived signals that are emitted onto the existing `MarketSnapshot` wire as top-level fields alongside the `indicators` map.

## Risk integration

`RiskMatrix` contains an **8th** sub-dimension: `cascade_risk`. It is computed
from `LiquidityFlow.cascade_intensity` and `LiquidationClusterMatrix.
cascade_asymmetry`. The legacy 8th sub-dimension `expected_rr` was removed and
moved to the Decision Matrix as `entry_danger` (synthesis belongs
at L6, not pure danger L5). The new `cascade_risk` slot **replaces** `expected_rr`
in count: the matrix still has 8 unipolar danger sub-dimensions + `overall_risk`,
not 9. Weights were re-normalized so the overall score is still 0..100. See
[02-11-risk-matrix.md §2.1](../matrices/02-11-risk-matrix.md) for the
authoritative field list and [02-00-matrix-field-ownership.md §2.5](../matrices/02-00-matrix-field-ownership.md)
for the canonical producer mapping.

The legacy `liquidity_risk` field was renamed to
`execution_liquidity_risk` (with a serde alias) to free the
"liquidity" term for the positional concept.

## Decision integration

A 7th `OpportunityType::LiquiditySqueeze` variant was added in L4 ([02-08-opportunity-matrix.md §3](../matrices/02-08-opportunity-matrix.md)). The L4 Opportunity Matrix now publishes `primary_opportunity = LIQUIDITY_SQUEEZE` when its preconditions are satisfied (cascade_state in `Detected`/`Sustained` plus `|cascade_asymmetry| > 0.3` plus `EXPANSION`/`TRANSITION` regime). The Decision Layer reads the value from L4's `primary_opportunity` directly — there is no separate `opportunity_type` field on the Advisory Matrix (that field was removed in the institutional redesign; see [02-00-matrix-field-ownership.md §3](../matrices/02-00-matrix-field-ownership.md) and [02-04-decision-matrix.md §2](../matrices/02-04-decision-matrix.md)). The TAE Policy Layer can therefore match on `opportunity.primary_opportunity` to dispatch `CLOSE_ONLY`-stance reduce-only orders (see [03-03-03-tae-layer2-execution.md §3.3](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md)).

## Backward compatibility

All schema additions use `#[serde(default)]` so older snapshots
without the new fields deserialize cleanly. The legacy
`liquidity_risk` field is kept as an optional alias of
`execution_liquidity_risk`.

## Configuration

The platform uses **`config.toml`** as the single source of configuration truth (the legacy `config.json` form is still recognized by `load_config()` as a fallback; see [08-01-user-manual.md §5](../operations-and-compliance/08-01-user-manual.md) for the operator-facing install path). The Liquidity Intelligence extension contributes a `[liquidity]` table inside `config.toml`:

```toml
[liquidity]
enabled = true                          # master switch
mark_price_poll_ms = 60000              # HL mark/OI/funding poll cadence
funding_refresh_ms = 60000              # Bitget funding refresh floor
event_retention_days = 90               # raw liquidation_events retention
bucket_retention_days = 7               # aggregated buckets retention
cluster_refresh_secs = 300              # cluster matrix refresh
maintenance_margin_rate = 0.005         # 0.5% (industry standard for perps)
cascade_detected_zscore = 2.5           # single-event cascade trigger
cascade_sustained_events = 3            # events in window for Sustained
cascade_baseline_window_bars = 200      # baseline stats window for cascade_intensity z-score computation
cascade_min_warmup_bars = 30            # min completed bars before z-score is statistically meaningful
funding_extreme_pct = 0.0005            # 0.05% / 8h
magnet_activation_distance_pct = 0.5    # 0.5% from mid
liquidity_vacuum_threshold = 0.3
oi_funding_divergence_pct = 2.0
```

> **Single source of truth.** Every operator-tunable parameter, including the Liquidity Intelligence knobs, lives in `config.toml` (the user-editable configuration file served via `GET /api/config` and `POST /api/config`). The platform previously used `config.json`; the TOML form became canonical at v5.0 with the workspace restructure (see `docs/CHANGELOG.md`). `config.json` is still recognized as a legacy alias by `load_config()` for backward compatibility. **Config format note (v5.0).** The canonical config format is `config.toml`. `config.json` is still recognized by `config-models/src/lib.rs::load_config()` as a legacy fallback (the legacy reader code path is preserved for backward compatibility with existing user installations but is not documented for new deploys).

## Performance

- Liquidity Phase 0 (HL polling): 60s REST round-trip per pair, ~few KB.
- Liquidity Phase 1 (event ingestion): O(1) per event, ~1µs.
- Liquidity Phase 1 (per-candle flow): O(events in bar) ≈ O(50) typical.
- Liquidity Phase 2 (cluster estimation): O(P × L) per refresh ≈ O(3,500) ops.
  Refresh every 5 min, <1ms compute.
- Liquidity Phase 3 (signal derivation): O(1) per snapshot.
- Liquidity Phase 4 (frontend): no measurable overhead — fields ride existing
  WS frame.

Total per-candle overhead: <5ms. Total memory: <300KB per pair per TF.

## Test coverage

| Phase | Unit tests | Integration tests | Total | Source files |
|---|---|---|---|---|
| 0 | 11 | 0 | 11 | `crates/portfolio-supervisor/tests/phase0_derivatives.rs` |
| 1 | 15 | 1 | 16 | `crates/core-domain/tests/phase1_liquidity_flow.rs` + `crates/portfolio-supervisor/tests/phase1_liquidation_e2e.rs` |
| 2 | 14 | 0 | 14 | `crates/core-domain/tests/phase2_cluster_matrix.rs` |
| 3 | 10 | 0 | 10 | `crates/core-domain/tests/phase3_signals.rs` |
| 4 | 5 | 0 | 5 | `ui/src/components/LiquidityPanel.test.ts` |
| **Total** | **55** | **1** | **56** | |

All 56 new tests pass. No existing tests were broken by the
implementation.

> **Sub-test nesting clarification.** The nested test functions `assess_cascade_risk` (under Liquidity Phase 3) and `compute_cluster_matrix` (under Liquidity Phase 2) are already contained within their parent phase's totals (`Liquidity Phase 3 = 10`, `Liquidity Phase 2 = 14`); they must not be summed twice when computing the platform-wide test count. The authoritative totals are `55 unit + 1 integration = 56`. The table above sum-checks: `11 + 15 + 14 + 10 + 5 = 55` unit; `1` integration (`phase1_liquidation_e2e.rs` end-to-end pipeline test).

## Open questions / future work

All deferred-work items in this document are tracked exclusively in [docs/CHANGELOG.md §Open Items](../CHANGELOG.md#open-items-forwarded-to-future-versions). The trackers below were previously re-stated here and have been moved to the single canonical home to prevent multi-doc drift.

- **`cascade_risk_index` aggregation** — see [docs/CHANGELOG.md §Open Items](../CHANGELOG.md#open-items-forwarded-to-future-versions) (`AUDIT-V4-005`).
- **PriceChart marker overlay for cluster positions** — see [docs/CHANGELOG.md §Open Items](../CHANGELOG.md#open-items-forwarded-to-future-versions) (`AUDIT-V4-079`).
- **`liquidation_events` → PAE backtest ingestion** — see [docs/CHANGELOG.md §Open Items](../CHANGELOG.md#open-items-forwarded-to-future-versions) (`AUDIT-V4-080`).