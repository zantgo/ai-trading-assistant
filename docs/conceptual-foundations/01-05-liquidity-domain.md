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
    ├─ liquidity:            Option<LiquidityFlow>                  (Phase 1, top-level field)
    ├─ cluster:              Option<LiquidationClusterMatrix>      (Phase 2, top-level field)
    ├─ liquidity_signals:    Vec<LiquiditySignal>                  (Phase 3, top-level field — derived from liquidity + cluster)
    └─ statistical_context:  StatisticalContext                     (Monte Carlo + z-scores)

WS broadcast payload
    ├─ market_snapshots
    │   ├─ indicators (50 indicators + signals)
    │   ├─ context, alignment, analysis, decision_context, ...
    │   └─ liquidity, cluster, liquidity_signals              ← liquidity extension surface
    └─ sent as a single MarketSnapshot frame on /ws

> **Top-level liquidity fields (MAT-17 — correction).** A previous version of this data-flow diagram showed the Phase 3 signals as "attached to `MarketSnapshot.indicators` as a JSON-encoded list". That placement contradicted the canonical Metrics Matrix contract ([02-07-metrics-matrix.md §2.1](../matrices/02-07-metrics-matrix.md)) and the Rust type at `crates/shared/src/models.rs`, which both declare `liquidity_signals: Vec<LiquiditySignal>` as a **top-level** field on `MarketSnapshot`, *separate* from the nested `indicators` map. The corrected diagram above shows the three liquidity fields (`liquidity`, `cluster`, `liquidity_signals`) as siblings of `indicators` on the `MarketSnapshot` wire frame.
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

`RiskMatrix` contains an **8th** sub-dimension: `cascade_risk`. It is computed
from `LiquidityFlow.cascade_intensity` and `LiquidationClusterMatrix.
cascade_asymmetry`. The legacy 8th sub-dimension `reward_risk` was removed and
moved to the Decision Matrix as `entry_danger` (synthesis belongs
at L6, not pure danger L5). The new `cascade_risk` slot **replaces** `reward_risk`
in count: the matrix still has 8 unipolar danger sub-dimensions + `overall_risk`,
not 9. Weights were re-normalized so the overall score is still 0..100. See
[02-11-risk-matrix.md §2.1](../matrices/02-11-risk-matrix.md) for the
authoritative field list and [02-00-matrix-field-ownership.md §2.5](../matrices/02-00-matrix-field-ownership.md)
for the canonical producer mapping.

The legacy `liquidity_risk` field was renamed to
`execution_liquidity_risk` (with a serde alias) to free the
"liquidity" term for the positional concept.

## Decision integration

A 7th `OpportunityType::LiquiditySqueeze` variant was added in L4 ([02-08-opportunity-matrix.md §3](../matrices/02-08-opportunity-matrix.md)). The L4 Opportunity Matrix now publishes `primary_opportunity = LIQUIDITY_SQUEEZE` when its preconditions are satisfied (cascade_state in `Detected`/`Sustained` plus `|cascade_asymmetry| > 0.3` plus `EXPANSION`/`TRANSITION` regime). The Decision Layer reads the value from L4's `primary_opportunity` directly — there is no separate `opportunity_classification` field on the Advisory Matrix (that field was removed in the institutional redesign; see [02-00-matrix-field-ownership.md §3](../matrices/02-00-matrix-field-ownership.md) and [02-04-decision-matrix.md §2](../matrices/02-04-decision-matrix.md)). The TAE Policy Layer can therefore match on `opportunity.primary_opportunity` to dispatch `CLOSE_ONLY`-stance reduce-only orders (see [03-03-03-tae-layer2-execution.md §3.3](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md)).

## Backward compatibility

All schema additions use `#[serde(default)]` so older snapshots
without the new fields deserialize cleanly. The legacy
`liquidity_risk` field is kept as an optional alias of
`execution_liquidity_risk`.

## Configuration

The platform uses **`config.json`** as the single source of configuration truth — no `config.toml` exists. The Liquidity Intelligence extension contributes a `"liquidity"` sub-section inside `config.json` (see [08-01-user-manual.md §5](../operations-and-compliance/08-01-user-manual.md)):

```json
{
  "liquidity": {
    "enabled": true,                          // master switch
    "mark_price_poll_ms": 60000,              // HL mark/OI/funding poll cadence
    "funding_refresh_ms": 60000,              // Bitget funding refresh floor
    "event_retention_days": 90,               // raw liquidation_events retention
    "bucket_retention_days": 7,               // aggregated buckets retention
    "cluster_refresh_secs": 300,              // cluster matrix refresh
    "maintenance_margin_rate": 0.005,         // 0.5% (industry standard for perps)
    "cascade_detected_zscore": 2.5,           // single-event cascade trigger
    "cascade_sustained_events": 3,            // events in window for Sustained
    "cascade_baseline_window_bars": 200,      // baseline stats window for cascade_intensity z-score computation
    "cascade_min_warmup_bars": 30,            // min completed bars before z-score is statistically meaningful
    "funding_extreme_pct": 0.0005,            // 0.05% / 8h
    "magnet_activation_distance_pct": 0.5,    // 0.5% from mid
    "liquidity_vacuum_threshold": 0.3,
    "oi_funding_divergence_pct": 2.0
  }
}
```

> **Single source of truth (Issue 5.A — correction).** A previous version of this section showed the same fields as a `[liquidity]` block of a `config.toml` file. The platform does not use `config.toml` — every operator-tunable parameter, including the Liquidity Intelligence knobs, lives in `config.json` (the user-editable configuration file served via `GET /api/config` and `POST /api/config`). The TOML form was retained from an early prototype that never shipped.

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

| Phase | Unit tests | Integration tests | Total | Source files |
|---|---|---|---|---|
| 0 | 11 | 0 | 11 | `crates/engine/tests/phase0_derivatives.rs` |
| 1 | 15 | 1 | 16 | `crates/shared/tests/phase1_liquidity_flow.rs` + `crates/engine/tests/phase1_liquidation_e2e.rs` |
| 2 | 14 | 0 | 14 | `crates/shared/tests/phase2_cluster_matrix.rs` |
| 3 | 10 | 0 | 10 | `crates/shared/tests/phase3_signals.rs` |
| 4 | 5 | 0 | 5 | `crates/frontend/src/components/LiquidityPanel.test.ts` |
| **Total** | **55** | **1** | **56** | |

All 56 new tests pass. No existing tests were broken by the
implementation.

> **Sub-test nesting clarification (MAT-18 — corrected count).** An earlier audit reported a total of 60 tests, arrived at by separately counting nested test functions (`assess_cascade_risk` under Phase 3, `compute_cluster_matrix` under Phase 2) as if they were independent items. The authoritative count is `55 unit + 1 integration = 56`. The nested functions are already contained within their parent phase's total (Phase 3 = 10, Phase 2 = 14) and must not be summed twice. The table above sum-checks: `11 + 15 + 14 + 10 + 5 = 55` unit; `1` integration (the `phase1_liquidation_e2e.rs` end-to-end pipeline test).

## Open questions / future work — Canonical deferred-work tracker

This section is the **canonical tracking point** for every deferred feature, placeholder field, or scheduled-for-future-version capability in the Liquidity Intelligence subsystem (and adjacent extensions that cross-reference this doc). Any downstream document that needs to refer to a field's *current* implementation status must link here rather than restating the status — the goal is **exactly one canonical statement per deferred item** so the corpus cannot drift on "is this wired or not?" questions.

**Tracker items:**

- **`cascade_risk_index` aggregation** (open). The Overview Matrix carries `cascade_risk_index` as a placeholder field on the L7 envelope (declared in [02-09-overview-matrix.md §2.1](../matrices/02-09-overview-matrix.md) and serialized in [01-01 §A.7](../conceptual-foundations/01-01-ontology.md)) but it is **not yet aggregated into `systemic_risk_score`**. The field is serialized with placeholder values (the canonical example uses a constant illustrative `score` — not a real value) so downstream consumers (UI, REST, PAE) have a stable contract to read; the aggregation formula is scheduled for a future Phase 3 follow-up.
- **PriceChart marker overlay** for cluster positions (deferred from Phase 4 to keep the initial render simple).
- **`liquidation_events` → PAE backtest ingestion** (deferred). The PAE could later consume the `liquidation_events` table for cascade-conditioned strategy backtesting; today the table is read-only from the cluster estimator's per-candle aggregation path.
- **Additional tracker items** (add here, not in any other doc, when opening new deferred work).

**How downstream docs must reference this section.** Any matrix, engine layer spec, schema doc, or operator doc that mentions a deferred item from this tracker should link here (e.g. "see [01-05 §Open questions](../conceptual-foundations/01-05-liquidity-domain.md) — canonical deferred-work tracker") and otherwise *not* restate the implementation status. This prevents the multi-doc drift the `cascade_risk_index` placeholder previously exhibited.