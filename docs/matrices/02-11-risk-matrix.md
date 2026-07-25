# Risk Matrix Specification

**Version:** 6.5 (2026-07-24) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 5 — Risk Layer
**Purpose:** This document defines the physical schema and unipolar scoring model of the **Risk Matrix** — the direction-independent threat-assessment object. The Risk Matrix contains **eight unipolar danger sub-dimensions** plus `overall_risk` (the weighted aggregate of those eight) — **nine fields total** on a `0–100` unipolar scale. *(Phase 3 added `cascade_risk` for liquidation-cascade danger; the legacy `liquidity_risk` was renamed to `execution_liquidity_risk` to free the "liquidity" term for the positional concept.)*

---

## 1. Conceptual Definition

Per the [Ontology](../conceptual-foundations/01-01-ontology.md) §3.15, **Risk** is the structural, technical, and environmental danger present in the market — **independent of directional bias**. A bullish market can be high risk; a bearish market can be low risk.

Crucially, Risk is a property of an *interpretation*, not of raw observations: you cannot evaluate how risky a bullish trend is until you have first determined that a bullish trend exists. The Risk Matrix therefore consumes the [Analysis Matrix](02-02-analysis-matrix.md) plus the underlying [Metrics Matrix](02-07-metrics-matrix.md) indicators.

**L4/L5 strict orthogonality (institutional redesign).** L5 does **not** consume the L4 Opportunity Matrix. The Layer 4 (Opportunity) and Layer 5 (Risk) branches are strictly orthogonal at the matrix boundary: each reads L3 directly and runs in parallel. **Reward evaluation is a synthesis** and has been moved to the [Decision Matrix (L6)](02-04-decision-matrix.md) as the new `entry_danger` field. The Risk Matrix contains **eight unipolar danger sub-dimensions** plus `overall_risk` (the weighted aggregate) — nine fields total — no reward synthesis.

```
[Analysis Matrix] ─┐
                   ├──► RISK LAYER (L5) ──► [Risk Matrix]
[Metrics Matrix ]  ┘      compute_risk()      (eight sub-dims + overall_risk)
```

Implemented as `RiskMatrix` (`crates/core-domain/src/risk.rs`), produced by `compute_risk()`.

---

## 2. Physical Schema

### 2.1 RiskMatrix Fields (9 Dimensions)

| Field | Type | Threat Vector |
|-------|------|---------------|
| `symbol` | `string` | Entity under analysis. |
| `market_risk` | `RiskDimension` | General uncertainty from conflicting signals / weak structure. |
| `volatility_risk` | `RiskDimension` | Danger from abnormal price movement. |
| `execution_liquidity_risk` | `RiskDimension` | Poor market participation / thin volume. (Renamed from `liquidity_risk` in Phase 3 to free the term "liquidity" for the new positional concept.) |
| `structure_risk` | `RiskDimension` | Weak or damaged price structure. |
| `momentum_risk` | `RiskDimension` | Exhausted / diverging momentum. |
| `signal_risk` | `RiskDimension` | Conflicting or unreliable signals. |
| `execution_risk` | `RiskDimension` | Practical difficulty (spread, slippage, thin book). |
| `cascade_risk` | `RiskDimension` | Forced liquidation cascade danger. *(Added in Phase 3 — computed from `LiquidityFlow.cascade_intensity`, `cascade_state`, and `LiquidationClusterMatrix.cascade_asymmetry`.)* |
| `overall_risk` | `RiskDimension` | Weighted aggregate of the eight sub-dimensions above. |

### 2.2 RiskDimension

| Field | Type | Range | Description |
|-------|------|-------|-------------|
| `score` | `f64` | `[0, 100]` | **Unipolar** risk score — higher is riskier. |
| `level` | `RiskLevel` | — | `VERY_LOW` / `LOW` / `MODERATE` / `HIGH` / `EXTREME`. |
| `state` | `RiskState` | — | `STABLE` / `INCREASING` / `ELEVATED` / `CRITICAL` / `IMPROVING`. |
| `confidence` | `f64` | `[0, 100]` | Confidence in the measurement. |
| `evidence` | `string[]` | — | Human-readable contributing factors. |

### 2.3 RiskLevel Bands

```
score ≥ 80 → Extreme
score ≥ 60 → High
score ≥ 40 → Moderate
score ≥ 20 → Low
otherwise  → VeryLow
```

---

## 3. Unipolar Scoring Principle

Unlike the signed `[-1, 1]` scores of the Metrics/Analysis matrices, all Risk scores are **unipolar** in `[0, 100]`. Direction is deliberately discarded — the Risk Matrix answers *"how dangerous is the current environment?"*, never *"which way?"*. This lets the Decision Matrix combine an orthogonal danger axis with the directional bias axis.

---

## 4. Per-Dimension Assessment Contracts

Each assessment starts from a baseline and adjusts by additive evidence. Threshold tiers within a dimension are non-cumulative; only the highest satisfied tier applies (e.g. in §4.2, BBWP 95 yields +30, not +30 + 15 = +45). All final scores clamp to `[0, 100]`.

### 4.1 Market Risk (baseline 50)
```
+15 weak trend · +15 broken structure · +10 poor quality
+10 low state_confidence (<0.4) · +10 conflicting signals present
-10 strong trend · -10 high state_confidence (>0.7)
```

> **Rename note.** The previous name "analysis confidence" was renamed to `state_confidence` in the institutional redesign (see [02-00b-confidence-hierarchy.md §3](../matrices/02-00b-confidence-hierarchy.md)). Implementations read the L3 field as `state_confidence`, not `confidence`.

### 4.2 Volatility Risk (baseline 30)
```
+30 BBWP ≥ 90 (extreme expansion) · +15 BBWP ≥ 70
+10 squeeze compression active
if ATR present: score = mean(score, relative_atr)
```

### 4.3 Execution Liquidity Risk (baseline 30)
```
+30 RVOL < 0.5 · +15 RVOL < 0.8 · -15 RVOL > 2.0
+20 spread > 0.2% · -10 spread < 0.05%
```

### 4.4 Structure Risk (baseline 40)
```
+30 structure broken · +15 structure weak · -15 structure strong/healthy
+15 S/R level flip detected
```

### 4.5 Momentum Risk (baseline 30)
```
+40 momentum exhausted · +30 reversing · +15 weakening · -10 increasing
```

### 4.6 Signal Risk (baseline 30)
```
+min(10·n, 40) for n contradicting signals
+10 no signals active · +15 state_confidence < 0.5
```

> **Rename note.** Same rename as above: the L3 field is `state_confidence`. The trigger `state_confidence < 0.5` reads the L3 field directly.

### 4.7 Execution Risk (baseline 25)
```
+25 spread > 0.15% · +10 spread > 0.08%
+15 RVOL < 0.7 (low participation)
```

### 4.8 Cascade Risk (baseline 30, Phase 3)

Cascade risk quantifies the danger from forced liquidation cascades. It consumes the [Liquidity Matrix](02-12-liquidity-matrix.md) (`LiquidityFlow`) and the [Liquidation Cluster Matrix](02-13-liquidation-cluster-matrix.md) (`LiquidationClusterMatrix`). The derivation mirrors `crates/core-domain/src/risk.rs::assess_cascade_risk`:

```
score = baseline 30.0
score = max(score, flow.cascade_intensity)                       // pull in 0..100 intensity
+30  if flow.cascade_state == Sustained                          // premium 0..30 per state
+15  if flow.cascade_state == Detected                           // premium 0..15 per state
 +0  if flow.cascade_state == Exhausted                          // decaying, no premium
+0..30  if |cluster.cascade_asymmetry| > 0.3                     // forward-looking pressure
```

Evidence strings record both the cascade state (when active) and any significant cluster asymmetry.

### 4.9 Overall Risk (weighted aggregate)

The overall risk score is a weighted aggregate of the **eight sub-dimensions** (no `expected_rr` — reward synthesis lives at the [Decision Layer](02-04-decision-matrix.md) as `entry_danger`). Final normalized weights are defined in [MME Layer 5 §3](../engines/market-monitoring-engine/03-02-06-mme-layer5-risk.md) and applied by `crates/core-domain/src/risk.rs::compute_risk`.

> **Self-consistency check (v2.1 — correction).** The JSON example below uses the eight sub-dimension scores `(M=35, V=45, L_ex=15, S=25, Mo=20, Sig=30, E=25, C=30)`. Plugging these into the canonical weighted formula `0.14·M + 0.14·V + 0.14·L_ex + 0.10·S + 0.14·Mo + 0.10·Sig + 0.10·E + 0.14·C`:
>
> `0.14·35 + 0.14·45 + 0.14·15 + 0.10·25 + 0.14·20 + 0.10·30 + 0.10·25 + 0.14·30 = 4.9 + 6.3 + 2.1 + 2.5 + 2.8 + 3.0 + 2.5 + 4.2 = 28.3`
>
> The previous example value `28.75` was internally inconsistent with the formula. The corrected `overall_risk.score = 28.3` is the authoritative worked example; this value cascades into the Decision Matrix §6 example (`confidence_assessment = 71.7`, `expected_reward_risk_ratio = 1.79`).

---

## 5. JSON Serialization Contract

A representative Risk Matrix frame. The example illustrates the JSON shape and the `score → level` band translation from §2.3. Per-dimension derivation rules are in §4.

```json
{
  "symbol": "BTC-USDT",
  "market_risk":     { "score": 35.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0, "evidence": ["High confidence"] },
  "volatility_risk": { "score": 45.0, "level": "MODERATE", "state": "STABLE", "confidence": 50.0, "evidence": ["BBWP elevated"] },
  "execution_liquidity_risk": { "score": 15.0, "level": "VERY_LOW", "state": "STABLE", "confidence": 50.0, "evidence": ["Strong participation"] },
  "structure_risk":  { "score": 25.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "momentum_risk":   { "score": 20.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "signal_risk":     { "score": 30.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "execution_risk":  { "score": 25.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "cascade_risk":    { "score": 30.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "overall_risk":    { "score": 28.3, "level": "LOW",      "state": "STABLE", "confidence": 50.0 }
}
```

Empty `evidence` arrays are omitted. Enum values serialize as `SCREAMING_SNAKE_CASE`.

---

## 6. Empty State

When `analysis.timeframes_considered == 0`, `compute_risk` returns `RiskMatrix::empty()` — all **nine fields** (market, volatility, execution_liquidity, structure, momentum, signal, execution, cascade, overall_risk) defaulting to score `50.0` (`MODERATE`), reflecting maximal uncertainty in the absence of data.

---

## 7. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Direction independence** | No dimension references bullish/bearish direction. |
| **Unipolar bounding** | Every score clamps to `[0, 100]`. |
| **Explainability** | Each dimension exposes an `evidence` list of contributing factors. |
| **Additive determinism** | Scores are deterministic functions of the Analysis Matrix + indicator map. |

---

## 8. Cross-References

- [Analysis Matrix](02-02-analysis-matrix.md) — Primary input.
- [Opportunity Matrix](02-08-opportunity-matrix.md) — Directional-neutral counterpart.
- [Decision Matrix](02-04-decision-matrix.md) — Combines risk with opportunity and bias.
- [MME Layer 5 — Risk](../engines/market-monitoring-engine/03-02-06-mme-layer5-risk.md) — Producing-layer specification.
- [Ontology — Risk](../conceptual-foundations/01-01-ontology.md) — Conceptual definition.
