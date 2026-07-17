# 03-02-11: MME Liquidity Intelligence Extension (L1.5 + L2.5)

**Version:** 6.4 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**New layers:** L1.5 (Derivatives Telemetry) + L2.5 (Liquidity Synthesis)
**Existing layers:** unchanged

This document extends the MME architecture to include Liquidity
Intelligence. The extension preserves the unidirectional cascade and
the strict L4 / L5 orthogonality invariant.

## New layers

### L1.5: Derivatives Telemetry

**Inputs:**
- Hyperliquid `activeAssetCtx` channel (live mark price, OI, funding)
- Hyperliquid `metaAndAssetCtxs` REST polling (60s fallback)
- Bitget `ticker` channel (mark price)
- Bitget `funding-rate` channel
- Hyperliquid `userFills` (Phase 1: liquidation events)
- Bitget `fill` channel with `execType == "L"` (Phase 1: liquidation events)

**Outputs:**
- `latest_mark_px: Option<Decimal>` (per ActivePair)
- `latest_index_px: Option<Decimal>` (per ActivePair)
- `latest_oi: Option<Decimal>` (per ActivePair)
- `latest_funding: Option<Decimal>` (per ActivePair)
- `MarketSnapshot.mark_price / index_price / mark_index_spread_pct`
- `MarketSnapshot.liquidity: Option<LiquidityFlow>` (per completed candle)
- `liquidation_events` (DB table, 90-day retention)

**Producer/consumer contract:**
- Writes: `RwLock<Option<Decimal>>` on the ActivePair + DB writes.
- Reads: nothing (only the WS event stream).

### L2.5: Liquidity Synthesis

**Inputs:**
- L1.5 state: OI, funding, mark, index prices
- L1 state: history (last 200 micro candle closes)
- Configuration: leverage distribution assumption, maintenance
  margin rate, funding extreme threshold, refresh cadence

**Outputs:**
- `MarketSnapshot.cluster: Option<LiquidationClusterMatrix>`
  (5-min refreshed)
- `MarketSnapshot.liquidity_signals: Vec<LiquiditySignal>`
  (per-snapshot, derived from L1 + L2.5 outputs)

**Producer/consumer contract:**
- Writes: `RwLock<Option<LiquidationClusterMatrix>>` on ActivePair.
- Reads: `latest_oi`, `latest_funding`, `micro.history` (all shared
  state on the ActivePair).

## Strict architecture invariants

1. **Unidirectional cascade.**
   ```
   L1.5 (LiquidityFlow) ──┐
                            ├──► L4 (LiquiditySqueeze preconditions)
   L2.5 (LiquidationCluster) ──┐
                            ├──► L5 (cascade_risk) ──► L6 (Decision)
   ```
   L2.5 does NOT read from L1.5's `liquidity` field (avoids feedback where the cluster estimator's output would influence the next cluster estimation). L4 reads L1.5 and L2.5 telemetry to evaluate `LiquiditySqueeze` preconditions (`cascade_state` ∈ {Detected, Sustained} AND `|cascade_asymmetry| > 0.3` AND regime ∈ {EXPANSION, TRANSITION}); see [`02-08-opportunity-matrix.md §3`](../../matrices/02-08-opportunity-matrix.md) and the L4 producer-side description in [`03-02-05-mme-layer4-opportunity.md §2`](03-02-05-mme-layer4-opportunity.md).

2. **L4/L5 orthogonality preserved.** L4 and L5 still never read each other's matrices. The Phase 3 multi-source rule is strictly the additional L1.5/L2.5 feeds already listed in (1); L4 and L5 continue to read L3 directly. See [02-00-matrix-field-ownership.md §5](../../matrices/02-00-matrix-field-ownership.md) for the full edge table.

3. **No new engine.** The Liquidity Intelligence subsystem lives entirely within MME as two new layer pairs. The 5-engine count remains stable.

## Layer diagram (post-Phase 0-4)

```
L1   Metrics
L1.5 Derivatives Telemetry         ← NEW
L2   Alignment
L2.5 Liquidity Synthesis             ← NEW
L3   Analysis
L4   Opportunity (parallel to L5)    ← gains LiquiditySqueeze opportunity type
L5   Risk     (parallel to L4)       ← gains cascade_risk
L6   Decision                        ← downstream consumer of L4 + L5 (unchanged interface)
L7   Overview                        ← gains cascade_risk_index field on envelope (see [01-05-liquidity-domain.md §Open questions — Canonical deferred-work tracker](../../conceptual-foundations/01-05-liquidity-domain.md) for status)
```

## Cross-engine flow

L2.5 is MME-internal. It does not produce TAE or PME outputs directly.
The integration with the rest of the platform is:

- **TAE** reads the L4 Opportunity Matrix's `primary_opportunity`. A
  `LiquiditySqueeze` value drives a `CLOSE_ONLY` policy stance and forces
  `reduce_only = true` on all dispatched orders per the §3.3 invariant in
  [03-03-03-tae-layer2-execution.md](../trade-automation-engine/03-03-03-tae-layer2-execution.md).
- **PME** reads `RiskMatrix.cascade_risk` as a veto signal. If cascade
  risk is extreme, PME can force positions into `CLOSE_ONLY`.
- **PAE** (future work) can read the `liquidation_events` table for
  cascade-conditioned backtesting.

**Threshold split for `cascade_asymmetry`.** Three rules reference this quantity with intentionally different thresholds. **All three are correct**; the split reflects event type:

| Site | Type | Threshold | Reason |
|---|---|---|---|
| L4 LiquiditySqueeze precondition | Continuous forecast eligibility | `|asymmetry| > 0.3` | Forward-looking pressure into setup viability (continuous weighting). |
| L5 `cascade_risk.score` incremental | Continuous risk score contribution | `|asymmetry| > 0.3 → up to +30 risk points` | Linear contribution into the weighted aggregate. |
| Phase 3, snapshot-level `LIQUIDITY_CLUSTER_PRESSURE_HIGH` signal | Discrete event | `|asymmetry| > 0.5` | Stricter event gate so that the signal only fires on meaningful cluster pressure, while the continuous scoring still weights asymmetry at 0.3+ into the Risk aggregate. |

## Phase 3 LiquiditySignalKind Registry

The Phase 3 `LiquiditySignalKind` enum defines **7** signals derived per snapshot from `liquidity` + `cluster` + `funding`:

| # | Signal | Trigger |
|---|--------|---------|
| 1 | `LIQUIDITY_CASCADE_DETECTED` | `flow.cascade_state` transitions from `None` → `Detected` |
| 2 | `LIQUIDITY_CASCADE_SUSTAINED` | `flow.cascade_state = Sustained` for ≥ 3 consecutive candles |
| 3 | `LIQUIDITY_CASCADE_EXHAUSTED` | `flow.cascade_state` transitions to `Exhausted` |
| 4 | `LIQUIDITY_CLUSTER_PRESSURE_HIGH` | `|cluster.cascade_asymmetry| > 0.5` |
| 5 | `LIQUIDITY_CLUSTER_FORWARD_PRESSURE` | `cluster.cascade_asymmetry` sign aligns with detected cascade direction |
| 6 | `LIQUIDITY_FUNDING_FLIP` | `funding_rate` changes sign (long → short funding) |
| 7 | `LIQUIDITY_OI_DIVERGENCE` | `oi_delta` disagrees with price direction (liquidity-focused divergence) |

All 7 are emitted on the `liquidity_signals` Vec field of `MarketSnapshot`. See [01-05-liquidity-domain.md §Phase 3](../../conceptual-foundations/01-05-liquidity-domain.md).

> **Field-naming note.** A previous version of this section referred to the
> Advisory Matrix's `opportunity_type` field. That field was removed
> in the institutional redesign; the canonical opportunity classifier now
> lives on the L4 Opportunity Matrix as `primary_opportunity` (see
> [02-00-matrix-field-ownership.md §3](../../matrices/02-00-matrix-field-ownership.md)).

## Configuration surface

The `[liquidity]` block in **`config.toml`** (the platform's single source of configuration truth) is the only new configuration surface. All fields have safe defaults. See [02-12-liquidity-matrix.md](../../matrices/02-12-liquidity-matrix.md) for the field reference, and [01-05-liquidity-domain.md §Configuration](../../conceptual-foundations/01-05-liquidity-domain.md) for the canonical TOML shape.

## Test coverage

The full Liquidity Intelligence test inventory (55 unit + 1 integration = 56 tests) is the **canonical source of truth** in [`01-05-liquidity-domain.md §Test Coverage`](../../conceptual-foundations/01-05-liquidity-domain.md). This table mirrors it. **Nested functions** (`assess_cascade_risk` under Phase 3, `compute_cluster_matrix` under Phase 2) are already contained within their parent phase's totals — they are not counted twice here.

| Component | Unit | Integration |
|---|---|---|
| Phase 0 — derivatives telemetry (`mark_price_poll`, `funding_refresh`) | 11 | 0 |
| `LiquidityEventAccumulator` (Phase 1) | 15 | 0 |
| `estimate_clusters` (Phase 2; `compute_cluster_matrix` is nested) | 14 | 0 |
| `derive_liquidity_signals` (Phase 3; `assess_cascade_risk` is nested) | 10 | 0 |
| Liquidation event → snapshot e2e (Phase 1) | 0 | 1 |
| `LiquidityPanel` data types (Phase 4) | 5 | 0 |
| **Total** | **55** | **1** |