# Matrix Field Ownership

**Version:** 1.0
**Status:** Approved
**Purpose:** Canonical mapping of every matrix field to its producing layer. This document is the authoritative reference for which engine layer owns which JSON key.

This document was introduced as part of the institutional-grade architectural redesign (Option α). It enforces the principle that **L3 (Analysis) is pure state**, **L4 (Opportunity) is pure forecast**, **L5 (Risk) is pure danger**, and **L6 (Decision) is the only synthesis point**.

---

## 1. Ownership Hierarchy

```
                  ┌─────────────────────────┐
                  │   Metrics Matrix (L1)   │
                  │   (foundation)           │
                  └────────────┬────────────┘
                               │
                               ▼
              ┌────────────────────────────────┐
              │     Alignment Matrix (L2)      │
              │  (cross-timeframe consensus)   │
              └────────────────┬───────────────┘
                               │
                               ▼
                  ┌─────────────────────────┐
                  │   Analysis Matrix (L3)   │  ← pure state interpretation
                  │   bias, regime,           │
                  │   quality, assessments    │
                  └────┬──────────────┬───────┘
                       │              │
                       │              └────► L6 (Decision Synthesis)
                       ▼
       ┌──────────────────────┐    ┌──────────────────────┐
       │  Opportunity (L4)   │    │     Risk (L5)        │  ← strictly orthogonal
       │  primary_opportunity │    │  8 unipolar danger   │
       │  opportunity_score   │    │  dimensions +        │
       │  profiles[]          │    │  overall_risk        │
       └──────────┬──────────┘    └──────────┬───────────┘
                  │                            │
                  └─────────────┬──────────────┘
                                ▼
                  ┌─────────────────────────┐
                  │   Decision Matrix (L6)  │  ← only synthesis point
                  │   directional_guidance  │
                  │   trade_readiness        │
                  │   entry_danger│
                  │   expected_rr_ratio       │
                  └─────────────────────────┘
                                │
                                ▼
                  ┌─────────────────────────┐
                  │   Overview Matrix (L7)  │  ← cross-symbol aggregation
                  └─────────────────────────┘
```

---

## 2. Per-Field Ownership Table

### 2.1 Metrics Matrix (L1) — `02-07-metrics-matrix.md`

Owns: foundational indicator telemetry for a single Market Instance (Symbol × Timeframe).

| Field | Producer | Notes |
|---|---|---|
| `exchange`, `symbol`, `timeframe_secs`, `timestamp` | L1 (candle ingestion) | |
| `mid_price`, `bid_price`, `ask_price`, `bid_size`, `ask_size` | L1 | Top-of-book quotes |
| `funding_rate` | L1 | From DIE FundingRate event |
| OHLC + `volume`, `average_volume` | L1 (candle aggregation) | |
| `open_interest`, `oi_delta_1h`, `prev_day_px` | L1 | From DIE OpenInterest / AssetContext |
| `indicators` (map of `IndicatorEvaluation`) | L1 (indicator calculators) | 50 indicators with normalized scores, state_labels, signals |
| `context` (`MarketContext`) | L1 (`MarketContext::synthesize()`) | Per-TF context dimensions |
| `alignment`, `analysis`, `risk`, `advisory`, `decision_context`, `statistical_context` | **Attached matrices** | Composite envelope — L1 owns the envelope; the attached fields are *sourced* from L2–L6 for WebSocket delivery convenience (single frame carries the full cascade). The canonical sources of these fields are their respective layer matrices, NOT the Metrics Matrix. |

> **Composite envelope convention.** The Metrics Matrix is the **WebSocket delivery unit** — a single `MarketSnapshot` frame contains the per-TF Metrics data plus the attached higher-order matrices. This is a *delivery* pattern, not a *production* pattern. The canonical owners of `alignment`, `analysis`, `risk`, `advisory`, `decision_context` remain L2, L3, L5, L6, L6 respectively (see [02-07-metrics-matrix.md §2.1](02-07-metrics-matrix.md)).

### 2.2 Alignment Matrix (L2) — `02-01-alignment-matrix.md`

Owns: cross-timeframe consensus scores for one symbol.

| Field | Producer | Notes |
|---|---|---|
| `symbol`, `timeframes_present` | L2 | |
| `dimensions` (`AlignmentDimension[10]`) | L2 | Ordered: Trend, Momentum, Volume, Volatility, Structure, Signal, Regime, Confidence, Liquidity, **Tradability** (renamed from "Opportunity" in the institutional redesign — the L4 Opportunity Matrix is the canonical owner of opportunity concepts; this dimension measures TFs agreeing on tradability, see [Alignment Matrix §3](../matrices/02-01-alignment-matrix.md)) |
| `mtf_trend_alignment`, `mtf_momentum_alignment`, `mtf_volume_alignment`, `mtf_volatility_alignment` | L2 | Signed consensus in [-1, 1] |
| `mtf_overall_score`, `mtf_overall_label` | L2 | |
| `timeframe_alignments` (`TfAlignmentInfo[]`) | L2 | Per-TF breakdown |
| `signal_cross_tf_count`, `trend_agreement_pct` | L2 | |

### 2.3 Analysis Matrix (L3) — `02-02-analysis-matrix.md`

Owns: pure state interpretation. **No forecast, no reward, no danger.**

| Field | Producer | Notes |
|---|---|---|
| `symbol` | L3 | |
| `bias` (`MarketBias` 5-state) | L3 | `STRONG_BULLISH / BULLISH / NEUTRAL / BEARISH / STRONG_BEARISH` |
| `state_confidence` (`f64`, [0,1]) | L3 | Renamed from `confidence` for clarity in the institutional redesign |
| `market_regime` (`MarketRegime` 8-state) | L3 | |
| `trend_assessment`, `momentum_assessment`, `structure_assessment`, `volatility_assessment`, `volume_assessment` | L3 | Five qualitative assessments |
| `market_quality` (`QualityLevel` 5-state) | L3 | |
| `market_interpretation` (string) | L3 | Natural-language summary |
| `rationale` (string) | L3 | Explainability trace |
| `supporting_signals`, `contradicting_signals` | L3 | Per-TF evidence |
| `timeframes_considered` | L3 | |

**Removed (architectural redesign):**
- ~~`opportunity_analysis`~~ — moved to Opportunity Matrix (L4) as `primary_opportunity` (canonical source).

### 2.4 Opportunity Matrix (L4) — `02-08-opportunity-matrix.md`

Owns: forecast / setup identification. The **canonical source** of the `OpportunityType` enum.

| Field | Producer | Notes |
|---|---|---|
| `symbol` | L4 | |
| `primary_opportunity` (`OpportunityType`) | L4 | Canonical — the only producer |
| `opportunity_score` (`f64`, [0,100]) | L4 | Weighted blend of Q_ctx + S_sig + A_mtf + F_fresh |
| `setup_quality` (`SetupQuality`) | L4 | `Prime / Strong / Moderate / Marginal / None` |
| `profiles` (`OpportunityProfile[]`) | L4 | Per-setup-type scored profiles |
| `forecast_confidence` (`f64`, [0,1]) | L4 | Renamed from `confidence` |
| `contributing_signals` | L4 | Signal labels supporting primary opportunity |
| `invalidation_note` | L4 | Condition that nullifies the opportunity |
| `entry_zone` (`PriceRange`) | L4 | Recommended entry band *(institutional redesign)* |
| `target_zone` (`PriceRange`) | L4 | Expected target band *(institutional redesign)* |
| `invalid_level` (`Decimal`) | L4 | **Renamed in v2.1 — actual field name in the wire schema is `invalidation_level`** to align with the Decision Matrix and Position Matrix. The `invalid_level` row above is a legacy name kept here as a migration reference; see [Opportunity Matrix §2.1](../matrices/02-08-opportunity-matrix.md) and the ontology note at §3.14. |
| `invalid_level` legacy | L4 | Migrated to `invalidation_level`. The legacy alias is not serialized. |
| `expected_rr_internal` (`f64`) | L4 | Expected reward/risk ratio for this setup *(renamed from `expected_rr` in v2.1 to disambiguate from the Decision-Layer `expected_reward_risk_ratio`)* |
| `time_horizon` (`TimeHorizon`) | L4 | `SCALP` / `INTRADAY` / `SWING` / `POSITION` — all four variants are reachable from at least one `OpportunityType` (see [02-08-opportunity-matrix.md §3](../matrices/02-08-opportunity-matrix.md)). |

**Ownership rules for L4:**
- The setup-selection decision tree (formerly in `02-02-analysis-matrix.md §4.3`) is **moved** to the Opportunity Matrix and is the canonical source for `OpportunityType`.
- L4 reads from L3 (Analysis) for `bias`, `state_confidence`, `market_quality`, and the qualitative assessments.

> **Serialization convention.** `primary_opportunity` and `time_horizon` serialize as **SCREAMING_SNAKE_CASE strings on the wire / in policy conditions** (`"BREAKOUT"`, `"LIQUIDITY_SQUEEZE"`, `"INTRADAY"`, …); PascalCase is reserved for Rust internals. See the canonical note in [02-08-opportunity-matrix.md §7 Serialization note](../matrices/02-08-opportunity-matrix.md).
- The setup-selection decision tree (formerly in `02-02-analysis-matrix.md §4.3`) is **moved** to the Opportunity Matrix and is the canonical source for `OpportunityType`.
- L4 reads from L3 (Analysis) for `bias`, `state_confidence`, `market_quality`, and the qualitative assessments.

### 2.5 Risk Matrix (L5) — `02-11-risk-matrix.md`

Owns: pure environmental danger. **No reward, no opportunity, no state.**

| Field | Producer | Notes |
|---|---|---|
| `symbol` | L5 | |
| `market_risk`, `volatility_risk`, `execution_liquidity_risk`, `structure_risk`, `momentum_risk`, `signal_risk`, `execution_risk`, `cascade_risk` | L5 | **Eight** unipolar risk sub-dimensions in [0, 100]. The legacy `liquidity_risk` was renamed to `execution_liquidity_risk` (with serde alias). `cascade_risk` is the 8th sub-dimension (added in the Phase 0-4 Liquidity Intelligence extension, replacing the retired `reward_risk`/`correlation_risk`). |
| `overall_risk` | L5 | Weighted aggregate of the **eight** sub-dimensions: `overall = 0.14·M + 0.14·V + 0.14·L_ex + 0.10·S + 0.14·Mo + 0.10·Sig + 0.10·E + 0.14·C` (total = 1.0; where `M`=market_risk, `V`=volatility_risk, `L_ex`=execution_liquidity_risk, `S`=structure_risk, `Mo`=momentum_risk, `Sig`=signal_risk, `E`=execution_risk, `C`=cascade_risk; see [Risk Matrix §4.9](02-11-risk-matrix.md) and [MME Layer 5 §3](../engines/market-monitoring-engine/03-02-06-mme-layer5-risk.md)). **Nine fields total** (eight sub-dimensions + `overall_risk`). |

**Removed (architectural redesign):**
- ~~`reward_risk`~~ — moved to Decision Matrix (L6) as `entry_danger`. The reward dimension is a **synthesis** concept and belongs in L6, not L5.

**Ownership rules for L5:**
- L5 reads from L3 (Analysis) for `bias`, `market_regime`, `market_quality`, and the qualitative assessments.
- L5 does **not** consume the L4 Opportunity Matrix.
- L5 does **not** compute any reward or opportunity score.

### 2.6 Decision Matrix (L6) — `02-04-decision-matrix.md`

Owns: the **only synthesis point** in the pipeline. Combines L3 (state) + L4 (opportunity) + L5 (risk) into actionable guidance.

**`AdvisoryMatrix` (user-facing guidance):**

| Field | Producer | Notes |
|---|---|---|
| `symbol` | L6 | |
| `directional_guidance` (`DirectionalGuidance` 6-state) | L6 | Derived from L3 `bias` × L5 `overall_risk` × L6 `market_stance` (priority order — see Decision Matrix §3.1) |
| `market_stance` (`MarketStance` 5-state) | L6 | Derived from L3 `market_quality` × L5 `overall_risk` (sticky AVOID/CAUTIOUS guards — see Decision Matrix §3.2) |
| `strategy_environment` (`StrategyEnvironment` 6-state) | L6 | Derived from L3 `market_regime` |
| `entry_guidance` (`EntryGuidance` 5-state) | L6 | |
| `exit_guidance` (`ExitGuidance` 5-state) | L6 | |
| `protection_strategy` (`ProtectionStrategy` 5-state) | L6 | |
| `target_strategy` (`TargetStrategy` 5-state) | L6 | |
| `confidence_assessment` (`f64`, [0,100]) | L6 | Risk-attenuated: `clamp(L3.state_confidence × (1 − L5.overall_risk/100) × 100, 0, 100)` |
| `trade_readiness` (`TradeReadiness` 4-state) | L6 | **Added in institutional redesign** — was documented in §4 but missing from §2.1 schema |
| `entry_danger` (`RiskDimension`) | L6 | **Renamed from `environment_favorability` in v2.1** (semantic successor of `Risk.reward_risk`). The RiskDimension convention is **high score = danger, low score = safe** — consistent with all other Risk Matrix dimensions. |
| `expected_reward_risk_ratio` (`f64`) | L6 | **Added in institutional redesign** — risk-discounted synthesis: `L4.expected_rr_internal × (1 − L5.overall_risk / 100.0)` (canonical: [Decision Matrix §2.1](../matrices/02-04-decision-matrix.md)). Note the `/100.0` divisor — `overall_risk` is on the canonical `[0, 100]` scale. |
| `final_recommendation` (string) | L6 | Natural-language summary |

**`DecisionContext` (quantitative metadata):**

| Field | Producer | Notes |
|---|---|---|
| `score` (`f64`) | L6 | Quantitative confluence score |
| `bias` (`MarketBias` 5-state) | L6 | Mirror of L3 `bias` (5-state per platform convention; no 3-state collapse) |
| `score_confidence` (`f64`, [0,1]) | L6 | Renamed from `confidence` |
| `contributing_indicators` | L6 | |

### 2.7 Overview Matrix (L7) — `02-09-overview-matrix.md`

Owns: cross-symbol aggregation.

| Field | Producer | Notes |
|---|---|---|
| `global_market_bias` (`GlobalBias` 6-state) | L7 | |
| `market_breadth` (`MarketBreadth` 7-state) | L7 | |
| `regime_distribution` (`map<string, f64>`) | L7 | |
| `opportunity_distribution` (`map<string, u32>`) | L7 | Aggregated from L4 `primary_opportunity` |
| `risk_distribution` (`RiskDistribution`) | L7 | |
| `asset_ranking` (`AssetRank[]`) | L7 | score = `0.5 × confidence_assessment + 50` |
| `market_synchronization` (`SyncLevel` 5-state) | L7 | |
| `market_health` (`HealthLevel` 5-state) | L7 | |
| `global_summary` (string) | L7 | |
| `instance_count`, `active_symbols` | L7 | |
| `systemic_risk_score` (derived) | L7 | `0.6 × high_pct + 0.4 × sync_penalty` |

---

## 3. Removed Fields (Migration Map)

| Removed From | Old Field | New Location | Migration Note |
|---|---|---|---|
| Analysis Matrix (L3) | `opportunity_analysis` | Opportunity Matrix (L4) `primary_opportunity` | The setup selector moved to where it belongs: L4 owns forecasts. |
| Risk Matrix (L5) | `reward_risk` | Decision Matrix (L6) `entry_danger` | The reward dimension is a synthesis, not pure danger; moved to L6. |

---

## 4. Confidence-Field Renames

Per the institutional redesign (no backwards-compat aliases):

| Old Name | New Name | Producer |
|---|---|---|
| `Analysis.confidence` | **`Analysis.state_confidence`** | L3 |
| `Opportunity.confidence` | **`Opportunity.forecast_confidence`** | L4 |
| `DecisionContext.confidence` | **`DecisionContext.score_confidence`** | L6 |
| `AdvisoryMatrix.confidence_assessment` | (unchanged) | L6 |

The four `confidence`-bearing fields follow a strict hierarchy: indicator → state → forecast → score → risk-attenuated assessment. See [02-00b-confidence-hierarchy.md](02-00b-confidence-hierarchy.md) for details.

---

## 5. Dependency Edges (Allowed and Forbidden)

**Allowed (forward-only, unidirectional):**
- L1 → L2, L3, L4, L5, L6
- L2 → L3, L6
- L3 → L4, L5, L6 (the L3 fan-out — the only legitimate multi-edge from a single matrix)
- L4 → L6
- L5 → L6
- L6 → L7
- **L1.5 → L5** *(Phase 3 — Liquidity Intelligence extension)*: per-candle `LiquidityFlow` (cascade state + intensity) feeds into `cascade_risk`. This is an explicit multi-source exception for the Liquidity Intelligence layer; it preserves the unidirectional forward cascade by reading L1.5 telemetry into L5 synthesis only.
- **L2.5 → L4 / L5** *(Phase 3 — Liquidity Intelligence extension)*: the 5-minute `LiquidationClusterMatrix` (forward-pressure `cascade_asymmetry`) feeds into both L4 (LiquiditySqueeze opportunity) and L5 (cascade_risk weighting). Same forward-cascade preservation rationale as L1.5 → L5.

**Forbidden:**
- L4 ↔ L5 (no edge in either direction — they are strictly orthogonal; the only cross-coupling is via the shared L3 fan-out plus the L1.5/L2.5 multi-source exceptions above)
- Anything ← L6 (L6 is terminal; nothing consumes the Decision Matrix except L7, TAE, PME)
- Anything ← L7
- Anything ← TAE / PME / PAE (those engines read from earlier matrices but their outputs are not feedback inputs)

> **L4↔L1.5 / L4↔L2.5 architecture clarification (MAT-02).** The Opportunity Matrix `LiquiditySqueeze` precondition requires reading `LiquidityFlow.cascade_state` (L1.5 derivatives telemetry) and `LiquidationClusterMatrix.cascade_asymmetry` (L2.5 cluster matrix). A previous version of this section forbidden-listed all intermediate-layer reads (`L4 ↔ L1.5 / L2.5 forbidden`); the corrected rules above formalise L4's *forward-only* access to L1.5/L2.5 — the reverse edge (L1.5/L2.5 reading L4) remains forbidden.
>
> **L5↔L1.5 / L5↔L2.5 architecture clarification (MAT-08).** Per [Risk Matrix §4](../matrices/02-11-risk-matrix.md) and [MME Layer 5 §1](../engines/market-monitoring-engine/03-02-06-mme-layer5-risk.md), `cascade_risk` combines `LiquidityFlow.cascade_intensity` (L1.5) with `cascade_asymmetry` (L2.5). The dependency edge above is `L1.5 → L5` and `L2.5 → L5`. The 7-layer architecture is preserved by treating these as multi-source exceptions for the Liquidity Intelligence extension; the foundational 7-layer model remains the spine for non-liquidity layers.

---

## 6. Cross-References

- [02-00b-confidence-hierarchy.md](02-00b-confidence-hierarchy.md) — Confidence field rename & flow.
- [02-02-analysis-matrix.md](02-02-analysis-matrix.md) · [02-08-opportunity-matrix.md](02-08-opportunity-matrix.md) · [02-11-risk-matrix.md](02-11-risk-matrix.md) · [02-04-decision-matrix.md](02-04-decision-matrix.md) — Per-matrix specs.
- [01-01-ontology.md](../conceptual-foundations/01-01-ontology.md) — Conceptual layer definitions.
- [01-03-systemic-data-flow.md](../conceptual-foundations/01-03-systemic-data-flow.md) — Sequence diagrams.
