# Overview Matrix Specification

**Version:** 6.4.1 (2026-07-18) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 7 — Overview Layer
**Purpose:** This document defines the physical schema of the **Overview Matrix** — the global market-synthesis object. It aggregates every symbol's Decision Matrix plus instance metadata into cross-market breadth indices, asset rankings, synchronization measures, and a single Systemic Risk Score.

---

## 1. Conceptual Definition

Per the [Ontology](../conceptual-foundations/01-01-ontology.md) §3.17, **Market Overview** represents the aggregated state of the entire monitored universe. Where all prior matrices describe a *single asset*, the Overview Matrix describes the *collective market*.

```
[Decision Matrix: BTC-USDT]─┐
[Decision Matrix: ETH-USDT]─┼──► OVERVIEW LAYER (L7) ──► [Overview Matrix]
[Decision Matrix: SOL-USDT]─┘   compute_overview()      (global synthesis)
        + [Instance Metadata]
```

L7 aggregates each instance's slow-tier (300 s) Decision Matrix; the tier is a documented constant, not currently configurable.

Implemented as `OverviewMatrix` (`crates/core-domain/src/overview.rs`), produced by `compute_overview()`.

---

## 2. Physical Schema

### 2.1 OverviewMatrix Fields

| Field | Type | Description |
|-------|------|-------------|
| `global_market_bias` | `GlobalBias` | Universe-wide directional bias (§3.1). |
| `market_breadth` | `MarketBreadth` | Breadth classification (§3.2). |
| `low_coverage` | `bool` | `true` when breadth is computed over a reduced signal set (fewer than 4 of the 12 SignalKinds enabled); default `false` (§3.2). |
| `breadth_pct` | `f64 ∈ [-100, 100]` | Continuous numeric breadth: signed percentage of bullish-asset count. Source of the UI's −100 % to +100 % breadth gauge and the input to `market_breadth` and `market_synchronization`. |
| `regime_distribution` | `map<string, f64>` | Fraction of assets per regime. (`f64` because the regime-classification partition is exhaustive — entries sum to `1.0`.) |
| `opportunity_distribution` | `map<string, u32>` | Count of assets per opportunity type (incl. `LiquiditySqueeze` and `Scalp` since the v2.1 completeness sweep — see [02-08-opportunity-matrix.md §3](../matrices/02-08-opportunity-matrix.md)). (`u32` because opportunity types are not mutually exclusive — a single asset can simultaneously satisfy the preconditions of multiple setups, so the map is a per-type count rather than a partition.) |
| `risk_distribution` | `RiskDistribution` | Low/moderate/high risk share + environment label (§4). |
| `cascade_risk_index` | `RiskDimension` | Cross-symbol aggregate of L5 `cascade_risk` (Phase 3). |
| `systemic_risk_score` | `f64` | `0.6 × high_pct + 0.4 × sync_penalty`. The market-wide danger index the PME veto loop consumes (`≥` the operator-configured `systemic_risk_threshold`, default `80`, triggers the systemic-risk veto path per [03-04-05-pme-layer4-portfolio.md §4.1](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)). |
| `asset_ranking` | `AssetRank[]` | Assets ranked by composite score (§5). |
| `market_synchronization` | `SyncLevel` | Cross-asset correlation of direction (§3.3). |
| `market_health` | `HealthLevel` | Overall market health (§3.4). |
| `global_summary` | `string` | Natural-language synthesis. |
| `instance_count` | `u32` | Active monitoring instances. |
| `active_symbols` | `string[]` | Sorted list of active symbols. |

**Invariant.** `instance_count == active_symbols.length`. Each monitored symbol produces exactly one Overview instance. UI consumers and pre-trade consumers rely on this equality; multi-instance mode (multiple `MarketSnapshot` per symbol) is not currently supported.

### 2.2 AssetRank

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | `string` | Asset. |
| `score` | `f64` | Composite ranking score. |
| `bias` | `string` | Directional guidance label. |
| `confidence` | `f64` | Decision Matrix `confidence_assessment` value (mirror, in `[0, 100]`). |
| `regime` | `string` | Strategy environment label. |
| `risk_level` | `string` | Risk band. |

### 2.3 RiskDistribution

| Field | Type | Description |
|-------|------|-------------|
| `low_pct` | `f64` | % of assets at low risk. |
| `moderate_pct` | `f64` | % at moderate risk. |
| `high_pct` | `f64` | % at high risk. |
| `risk_environment` | `string` | `LOW_RISK` / `MODERATE` / `HIGH_RISK` / `NO_DATA`. |

`risk_environment` is derived from the distribution (ordered, first match wins):

| Priority | Condition | `risk_environment` |
|----------|-----------|--------------------|
| 1 | `instance_count = 0` | `NO_DATA` |
| 2 | `high_pct ≥ 50` | `HIGH_RISK` |
| 3 | `high_pct ≥ 25` | `MODERATE` |
| 4 | otherwise | `LOW_RISK` |

---

## 3. Classification Vocabularies

### 3.1 GlobalBias
`STRONG_BULLISH`, `BULLISH`, `NEUTRAL`, `BEARISH`, `STRONG_BEARISH`, `MIXED`.

```
priority 1: long_count/total ≥ 0.8  AND market_synchronization ∈ { HIGHLY_SYNCHRONIZED, SYNCHRONIZED } → STRONG_BULLISH
priority 1: short_count/total ≥ 0.8 AND market_synchronization ∈ { HIGHLY_SYNCHRONIZED, SYNCHRONIZED } → STRONG_BEARISH
priority 2: long_count/total  ≥ 0.6                                → BULLISH
priority 2: short_count/total ≥ 0.6                                → BEARISH
priority 3: neutral_count/total ≥ 0.6                              → NEUTRAL
priority 4: long_count > short_count                              → BULLISH
priority 4: short_count > long_count                              → BEARISH
priority 5: else                                                 → MIXED
```

All six variants are reachable from this rule.

### 3.2 MarketBreadth
`VERY_WEAK`, `WEAK`, `BALANCED`, `POSITIVE`, `STRONG_POSITIVE`, `NEGATIVE`, `STRONG_NEGATIVE`.

Breadth percentage: $\text{breadth\_pct} = \frac{\text{long\_count} - \text{short\_count}}{\text{total}} \times 100$.

```
# Priority order (first match wins):
1. breadth_pct >  60        → STRONG_POSITIVE
2. breadth_pct >  20        → POSITIVE
3. breadth_pct < -60        → STRONG_NEGATIVE
4. breadth_pct < -20        → NEGATIVE
5. |breadth_pct| < 10       → BALANCED
6. breadth_pct > 0         → WEAK
7. otherwise                → VERY_WEAK
```

> **`VERY_WEAK` semantics.** `VERY_WEAK` denotes marginally negative breadth — weaker than `BALANCED`, not yet a confirmed negative (`NEGATIVE` requires `breadth_pct < −20`).

> **Low-coverage flag (`low_coverage`).** The Overview Matrix carries an additive `low_coverage: bool` field (default `false`) alongside `market_breadth`: it is `true` when fewer than 4 of the 12 `SignalKind`s are enabled in the active configuration (see [03-02-12-mme-configurable-activation.md](../engines/market-monitoring-engine/03-02-12-mme-configurable-activation.md)); breadth is then computed over the enabled subset only. In the empty state (§7) `low_coverage` is `false`.

### 3.3 SyncLevel (Market Synchronization)
`HIGHLY_SYNCHRONIZED`, `SYNCHRONIZED`, `MIXED`, `FRAGMENTED`, `HIGHLY_FRAGMENTED` — from `|breadth_pct|`:
```
# Priority order (first match wins):
1. |breadth_pct| > 75  → HIGHLY_SYNCHRONIZED
2. |breadth_pct| > 50  → SYNCHRONIZED
3. |breadth_pct| > 25  → MIXED
4. |breadth_pct| > 10  → FRAGMENTED
5. otherwise           → HIGHLY_FRAGMENTED
```

### 3.4 HealthLevel
`POOR`, `WEAK`, `NEUTRAL`, `HEALTHY`, `STRONG` — derived from `global_market_bias` × `risk_environment` (ordered, first match wins):

| Priority | Condition | `market_health` |
|----------|-----------|-----------------|
| 1 | `risk_environment = HIGH_RISK` | `POOR` |
| 2 | `global_market_bias ∈ {STRONG_BULLISH, STRONG_BEARISH}` | `STRONG` |
| 3 | `global_market_bias ∈ {BULLISH, BEARISH}` | `HEALTHY` |
| 4 | `global_market_bias = NEUTRAL` | `NEUTRAL` |
| 5 | `global_market_bias = MIXED` | `WEAK` |

---

## 4. Risk Distribution & Systemic Risk Score

The `risk_distribution` bins assets by their Decision Matrix `confidence_assessment` (in `[0, 100]`; high confidence ⇒ low risk):

> **Confidence source clarification.** The `confidence_assessment` used by `risk_distribution` is the L6 Decision Matrix's **risk-attenuated terminal value** (see [02-00b-confidence-hierarchy.md](./02-00b-confidence-hierarchy.md)), not the L3 Analysis Matrix's `state_confidence`. The two are distinct: `state_confidence ∈ [0, 1]` is the L3 *state-interpretation* confidence driven by MTF agreement; `confidence_assessment ∈ [0, 100]` is the L6 *user-facing* confidence, attenuated by `overall_risk` per the formula in [02-04-decision-matrix.md §4](../matrices/02-04-decision-matrix.md). High `confidence_assessment` ⇒ low per-symbol risk ⇒ the asset is binned in `low_pct`.

```
low_pct  = % of Decision Matrices with confidence_assessment > 70
high_pct = % of Decision Matrices with confidence_assessment < 30
moderate_pct = 100 − low_pct − high_pct
```

The **Systemic Risk Score** is the market-wide danger index published for the Portfolio Management Engine's safety veto. It is derived from the risk distribution and synchronization:

$$\text{SystemicRisk} = 0.6 \cdot \text{high\_pct} + 0.4 \cdot \text{sync\_penalty}$$

`sync_penalty` (0–100) captures the danger of **correlated downside** — synchronized declines are systemically dangerous. It is `0` unless the global bias is in the bearish family, then it scales with the synchronization level:

| Condition | `sync_penalty` |
|-----------|----------------|
| `global_market_bias ∈ {BEARISH, STRONG_BEARISH}` + `HIGHLY_SYNCHRONIZED` | 100 |
| `global_market_bias ∈ {BEARISH, STRONG_BEARISH}` + `SYNCHRONIZED` | 60 |
| `global_market_bias ∈ {BEARISH, STRONG_BEARISH}` + `MIXED` | 30 |
| `global_market_bias ∈ {BEARISH, STRONG_BEARISH}` + `FRAGMENTED` | 10 |
| `global_market_bias ∈ {BEARISH, STRONG_BEARISH}` + `HIGHLY_FRAGMENTED` | 0 |
| `global_market_bias ∉ {BEARISH, STRONG_BEARISH}` (any bullish / neutral / mixed state) | 0 |

> **STRONG_BEARISH coverage (correction).** `GlobalBias` is a 6-state enum that includes both `BEARISH` and `STRONG_BEARISH` as separate bearish-class members (see §3.1). A previous version of this table used `global_market_bias != BEARISH`, which silently excluded `STRONG_BEARISH` — the regime with the worst correlated downside would have bypassed the safety penalty entirely. The corrected condition is `∈ {BEARISH, STRONG_BEARISH}` (member-set inclusion).

The resulting `risk_environment` label gates the PME [Ontological Priority Veto](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md).

#### 4.0.1 Cross-Reference

The Systemic Risk Score is consumed by the PME in [03-04-05-pme-layer4-portfolio.md §5](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) and is read by the operator-configurable threshold gate in [08-02-pre-trade-risk-controls.md Gate 7](../operations-and-compliance/08-02-pre-trade-risk-controls.md) (`systemic_risk_threshold`, default `≥ 80`). Systemic risk does not alter PME `safety_state`: a breach is enforced by Gate 7 (blocks new entries) and the PME veto loop (AVOID + Hard Exit).

---

## 5. Asset Ranking

Each active Decision Matrix produces an `AssetRank`. The composite score is monotonically increasing with `confidence_assessment ∈ [0, 100]` and maps to a sortable `[50, 100]` range so even no-confidence assets remain orderable:

$$\text{score} = 0.5 \cdot \text{confidence\_assessment} + 50$$

Properties:
- `confidence_assessment = 0` ⇒ `score = 50` (worst, neutral baseline).
- `confidence_assessment = 100` ⇒ `score = 100` (best).
- `confidence_assessment = 75` ⇒ `score = 87.5` (matches §6).

Rankings sort descending, producing a leaderboard of relative strength/weakness for portfolio-level allocation. The formula input is `confidence_assessment` from the Decision Matrix (§4), normalized to `[0, 100]`.

---

## 6. JSON Serialization Contract

```json
{
  "global_market_bias": "BULLISH",
  "market_breadth": "POSITIVE",
  "low_coverage": false,
  "breadth_pct": 60.0,
  "regime_distribution": { "TRENDING_BULL": 0.6, "RANGE": 0.4 },
  "opportunity_distribution": { "BREAKOUT": 2, "TREND_CONTINUATION": 1 },
  "risk_distribution": { "low_pct": 60.0, "moderate_pct": 40.0, "high_pct": 0.0, "risk_environment": "LOW_RISK" },
  "systemic_risk_score": 0.0,
  "asset_ranking": [
    { "symbol": "BTC-USDT", "score": 87.5, "bias": "STRONG_LONG", "confidence": 75.0, "regime": "TREND_FOLLOWING", "risk_level": "MODERATE" }
  ],
  "market_synchronization": "SYNCHRONIZED",
  "market_health": "HEALTHY",
  "global_summary": "5 active instances across 5 symbols. Global bias: BULLISH with positive market breadth.",
  "instance_count": 5,
  "active_symbols": ["BTC-USDT", "ETH-USDT", "SOL-USDT", "AVAX-USDT", "MATIC-USDT"]
}
```

> **Worked example for `systemic_risk_score`.** With `global_market_bias = BULLISH`, `sync_penalty = 0` regardless of synchronization level. The score reduces to `0.6 × high_pct + 0.4 × 0 = 0.6 × high_pct`. The example above (3 low-risk + 2 moderate-risk = 0 high-risk out of 5) yields `high_pct = 0` and `systemic_risk_score = 0.0`. A biased-bearish example with `high_pct = 60.0` and `sync_penalty = 0` (e.g. `HIGHLY_FRAGMENTED`) would yield `systemic_risk_score = 36.0`.

Enum values serialize as `SCREAMING_SNAKE_CASE`.

---

## 7. Empty State

When there are no Decision Matrices and no active instances, `compute_overview` returns `OverviewMatrix::empty()`: `global_market_bias = Neutral`, `market_breadth = Balanced`, `market_synchronization = HighlyFragmented`, `instance_count = 0`, summary `"No active instances — no market data available."`.

---

## 8. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Aggregation only** | The Overview Matrix never recomputes per-asset analysis; it aggregates published Decision matrices. |
| **Systemic focus** | Its Systemic Risk Score is the sole market-wide danger signal consumed by the PME veto loop. |
| **Deterministic ranking** | Ties in `AssetRank` resolve by stable sort. |

---

## 9. Cross-References

- [Decision Matrix](02-04-decision-matrix.md) — Per-asset input.
- [MME Layer 7 — Overview](../engines/market-monitoring-engine/03-02-08-mme-layer7-overview.md) — Producing-layer specification.
- [PME Layer 4 — Portfolio](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — Consumes the Systemic Risk Score for the veto.
- [Ontology — Market Overview](../conceptual-foundations/01-01-ontology.md) — Conceptual definition.
