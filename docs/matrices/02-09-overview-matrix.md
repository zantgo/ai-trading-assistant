# Overview Matrix Specification

**Version:** 2.0
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

Implemented as `OverviewMatrix` (`crates/shared/src/overview.rs`), produced by `compute_overview()`.

---

## 2. Physical Schema

### 2.1 OverviewMatrix Fields

| Field | Type | Description |
|-------|------|-------------|
| `global_market_bias` | `GlobalBias` | Universe-wide directional bias (§3.1). |
| `market_breadth` | `MarketBreadth` | Breadth classification (§3.2). |
| `regime_distribution` | `map<string, f64>` | Fraction of assets per regime. |
| `opportunity_distribution` | `map<string, u32>` | Count of assets per opportunity type. |
| `risk_distribution` | `RiskDistribution` | Low/moderate/high risk share + environment label (§4). |
| `asset_ranking` | `AssetRank[]` | Assets ranked by composite score (§5). |
| `market_synchronization` | `SyncLevel` | Cross-asset correlation of direction (§3.3). |
| `market_health` | `HealthLevel` | Overall market health (§3.4). |
| `global_summary` | `string` | Natural-language synthesis. |
| `instance_count` | `u32` | Active monitoring instances. |
| `active_symbols` | `string[]` | Sorted list of active symbols. |

### 2.2 AssetRank

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | `string` | Asset. |
| `score` | `f64` | Composite ranking score. |
| `bias` | `string` | Directional guidance label. |
| `confidence` | `f64` | Decision Matrix confidence. |
| `regime` | `string` | Strategy environment label. |
| `risk_level` | `string` | Risk band. |

### 2.3 RiskDistribution

| Field | Type | Description |
|-------|------|-------------|
| `low_pct` | `f64` | % of assets at low risk. |
| `moderate_pct` | `f64` | % at moderate risk. |
| `high_pct` | `f64` | % at high risk. |
| `risk_environment` | `string` | `LOW_RISK` / `MODERATE` / `HIGH_RISK` / `NO_DATA`. |

---

## 3. Classification Vocabularies

### 3.1 GlobalBias
`STRONG_BULLISH`, `BULLISH`, `NEUTRAL`, `BEARISH`, `STRONG_BEARISH`, `MIXED`.
```
long/total  ≥ 0.6 → BULLISH
short/total ≥ 0.6 → BEARISH
neutral/total ≥ 0.6 → NEUTRAL
long > short → BULLISH · short > long → BEARISH · else MIXED
```

### 3.2 MarketBreadth
`VERY_WEAK`, `WEAK`, `BALANCED`, `POSITIVE`, `STRONG_POSITIVE`, `NEGATIVE`, `STRONG_NEGATIVE`.

Breadth percentage: $\text{breadth\_pct} = \frac{\text{long\_count} - \text{short\_count}}{\text{total}} \times 100$.

```
> 60 → STRONG_POSITIVE · > 20 → POSITIVE
< -60 → STRONG_NEGATIVE · < -20 → NEGATIVE
|pct| < 10 → BALANCED · pct > 0 → WEAK · else → VERY_WEAK
```

### 3.3 SyncLevel (Market Synchronization)
`HIGHLY_SYNCHRONIZED`, `SYNCHRONIZED`, `MIXED`, `FRAGMENTED`, `HIGHLY_FRAGMENTED` — from `|breadth_pct|`:
```
> 75 → HIGHLY_SYNCHRONIZED · > 50 → SYNCHRONIZED · > 25 → MIXED · > 10 → FRAGMENTED · else → HIGHLY_FRAGMENTED
```

### 3.4 HealthLevel
`POOR`, `WEAK`, `NEUTRAL`, `HEALTHY`, `STRONG` — from `global_market_bias`.

---

## 4. Risk Distribution & Systemic Risk Score

The `risk_distribution` bins assets by their Decision Matrix confidence proxy (high confidence ⇒ low risk):

```
low_pct  = % of Decision Matrices with confidence > 70
high_pct = % of Decision Matrices with confidence < 30
moderate_pct = 100 − low_pct − high_pct
```

The **Systemic Risk Score** is the market-wide danger index published for the Portfolio Management Engine's safety veto. It is derived from the risk distribution and synchronization:

$$\text{SystemicRisk} = 0.6 \cdot \text{high\_pct} + 0.4 \cdot \text{sync\_penalty}$$

`sync_penalty` (0–100) captures the danger of **correlated downside** — synchronized declines are systemically dangerous. It is `0` unless the global bias is bearish, then it scales with the synchronization level:

| Condition | `sync_penalty` |
|-----------|----------------|
| `global_market_bias != BEARISH` | 0 |
| `BEARISH` + `HIGHLY_SYNCHRONIZED` | 100 |
| `BEARISH` + `SYNCHRONIZED` | 60 |
| `BEARISH` + `MIXED` | 30 |
| `BEARISH` + `FRAGMENTED` | 10 |
| `BEARISH` + `HIGHLY_FRAGMENTED` | 0 |

The resulting `risk_environment` label gates the PME [Ontological Priority Veto](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md).

---

## 5. Asset Ranking

Each active Decision Matrix produces an `AssetRank`. The composite score favours high-confidence, actionable assets:

$$\text{score} = 0.5 \cdot \text{confidence} + \big(100 - \min(\text{confidence}, 50) \cdot 0.5\big)$$

Rankings sort descending, producing a leaderboard of relative strength/weakness for portfolio-level allocation.

---

## 6. JSON Serialization Contract

```json
{
  "global_market_bias": "BULLISH",
  "market_breadth": "POSITIVE",
  "regime_distribution": { "TRENDING_BULL": 0.6, "RANGE": 0.4 },
  "opportunity_distribution": { "BREAKOUT": 2, "TREND_CONTINUATION": 1 },
  "risk_distribution": { "low_pct": 60.0, "moderate_pct": 40.0, "high_pct": 0.0, "risk_environment": "LOW_RISK" },
  "asset_ranking": [
    { "symbol": "BTC-USDT", "score": 87.0, "bias": "Long", "confidence": 75.0, "regime": "TrendFollowing", "risk_level": "MODERATE" }
  ],
  "market_synchronization": "SYNCHRONIZED",
  "market_health": "HEALTHY",
  "global_summary": "3 active instances across 3 symbols. Global bias: BULLISH with positive market breadth.",
  "instance_count": 3,
  "active_symbols": ["BTC-USDT", "ETH-USDT", "SOL-USDT"]
}
```

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
