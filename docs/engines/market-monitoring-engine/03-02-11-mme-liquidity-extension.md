# 03-02-11: MME Liquidity Intelligence Extension (L1.5 + L2.5)

**Status:** Implemented (Phases 0-4)
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
- `MarketSnapshot.liquiditySignals: Vec<LiquiditySignal>`
  (per-snapshot, derived from L1 + L2.5 outputs)

**Producer/consumer contract:**
- Writes: `RwLock<Option<LiquidationClusterMatrix>>` on ActivePair.
- Reads: `latest_oi`, `latest_funding`, `micro.history` (all shared
  state on the ActivePair).

## Strict architecture invariants

1. **Unidirectional cascade.** L1.5 → L2.5 → L5 (Risk) → L6 (Decision).
   L2.5 does NOT read from L1.5's `liquidity` field (avoids feedback
   where the cluster estimator's output would influence the next
   cluster estimation).

2. **L4/L5 orthogonality preserved.** L5 (Risk) continues to read from
   L3 (Analysis) directly, not from L4 (Opportunity). The new
   `cascade_risk` dimension is read from L2.5 (Liquidity
   Synthesis), not L4 — preserving orthogonality.

3. **No new engine.** The Liquidity Intelligence subsystem lives
   entirely within MME as two new layer pairs. The 5-engine count
   remains stable.

## Layer diagram (post-Phase 0-4)

```
L1   Metrics
L1.5 Derivatives Telemetry         ← NEW
L2   Alignment
L2.5 Liquidity Synthesis             ← NEW
L3   Analysis
L4   Opportunity (parallel to L5)
L5   Risk     (parallel to L4)       ← gains cascade_risk
L6   Decision                        ← gains LiquiditySqueeze opportunity
L7   Overview                        ← gains cascade_risk_index
```

## Cross-engine flow

L2.5 is MME-internal. It does not produce TAE or PME outputs directly.
The integration with the rest of the platform is:

- **TAE** reads the Advisory Matrix's `opportunity_classification`. A
  `LiquiditySqueeze` value can drive a new execution policy.
- **PME** reads `RiskMatrix.cascade_risk` as a veto signal. If cascade
  risk is extreme, PME can force positions into `CLOSE_ONLY`.
- **PAE** (future work) can read the `liquidation_events` table for
  cascade-conditioned backtesting.

## Configuration surface

The `[liquidity]` section in `config.toml` is the only new
configuration surface. All fields have safe defaults. See
[02-12-liquidity-matrix.md](../matrices/02-12-liquidity-matrix.md) for
the field reference.

## Test coverage

| Component | Unit | Integration |
|---|---|---|
| `LiquidityEventAccumulator` (Phase 1) | 15 | 0 |
| `estimate_clusters` (Phase 2) | 14 | 0 |
| `derive_liquidity_signals` (Phase 3) | 10 | 0 |
| `assess_cascade_risk` (Phase 3) | 2 | 0 |
| `compute_cluster_matrix` (Phase 2) | 2 | 0 |
| Liquidation event → snapshot e2e (Phase 1) | 0 | 1 |
| `LiquidityPanel` data types (Phase 4) | 5 | 0 |
| **Total** | **48** | **1** |

All 49 new tests pass. No existing tests were broken.