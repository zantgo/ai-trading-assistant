# Risk Matrix Specification

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 5 — Risk Layer
**Purpose:** This document defines the physical schema and unipolar scoring model of the **Risk Matrix** — the direction-independent threat-assessment object. It quantifies the danger surrounding the current market interpretation across nine dimensions on a `0–100` unipolar scale.

---

## 1. Conceptual Definition

Per the [Ontology](../conceptual-foundations/01-01-ontology.md) §3.15, **Risk** is the structural, technical, and environmental danger present in the market — **independent of directional bias**. A bullish market can be high risk; a bearish market can be low risk.

Crucially, Risk is a property of an *interpretation*, not of raw observations: you cannot evaluate how risky a bullish trend is until you have first determined that a bullish trend exists. The Risk Matrix therefore consumes the [Analysis Matrix](02-02-analysis-matrix.md) plus the underlying [Metrics Matrix](02-07-metrics-matrix.md) indicators. **The Risk Matrix does NOT consume the Opportunity Matrix (L4).** The only opportunity-derived evidence that enters `reward_risk` is the `opportunity_analysis` selector string (e.g. `NO_CLEAR_OPPORTUNITY`) and the `market_quality` enum — both of which are fields of the Analysis Matrix itself (L3), not of the L4 Opportunity Matrix. The Layer 4 (Opportunity) and Layer 5 (Risk) branches are orthogonal: each reads L3 directly and runs in parallel.

```
[Analysis Matrix] ─┐
                   ├──► RISK LAYER (L5) ──► [Risk Matrix]
[Metrics Matrix ]  ┘      compute_risk()      (9 unipolar dimensions)
```

Implemented as `RiskMatrix` (`crates/shared/src/risk.rs`), produced by `compute_risk()`.

---

## 2. Physical Schema

### 2.1 RiskMatrix Fields (9 Dimensions)

| Field | Type | Threat Vector |
|-------|------|---------------|
| `symbol` | `string` | Entity under analysis. |
| `market_risk` | `RiskDimension` | General uncertainty from conflicting signals / weak structure. |
| `volatility_risk` | `RiskDimension` | Danger from abnormal price movement. |
| `liquidity_risk` | `RiskDimension` | Poor market participation / thin volume. |
| `structure_risk` | `RiskDimension` | Weak or damaged price structure. |
| `momentum_risk` | `RiskDimension` | Exhausted / diverging momentum. |
| `signal_risk` | `RiskDimension` | Conflicting or unreliable signals. |
| `execution_risk` | `RiskDimension` | Practical difficulty (spread, slippage, thin book). |
| `reward_risk` | `RiskDimension` | Opportunity quality vs environmental uncertainty. |
| `overall_risk` | `RiskDimension` | Weighted aggregate of the eight dimensions above. |

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

Each assessment starts from a baseline and adjusts by additive evidence. All final scores clamp to `[0, 100]`.

### 4.1 Market Risk (baseline 50)
```
+15 weak trend · +15 broken structure · +10 poor quality
+10 low confidence (<0.4) · +10 conflicting signals present
-10 strong trend · -10 high confidence (>0.7)
```

### 4.2 Volatility Risk (baseline 30)
```
+30 BBWP ≥ 90 (extreme expansion) · +15 BBWP ≥ 70
+10 squeeze compression active
if ATR present: score = mean(score, relative_atr)
```

### 4.3 Liquidity Risk (baseline 30)
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
+10 no signals active · +15 analysis confidence < 0.5
```

### 4.7 Execution Risk (baseline 25)
```
+25 spread > 0.15% · +10 spread > 0.08%
+15 RVOL < 0.7 (low participation)
```

### 4.8 Reward Risk (baseline 40)
```
score = mean(40, quality_penalty)   // quality_penalty: Excellent 10 … Poor 80
+20 no clear opportunity
```

### 4.9 Overall Risk (weighted aggregate)

$$\text{overall} = 0.15\,M + 0.15\,V + 0.15\,L + 0.10\,S_{tr} + 0.15\,M_{om} + 0.10\,S_{ig} + 0.10\,E + 0.10\,R$$

where M=market, V=volatility, L=liquidity, S_tr=structure, M_om=momentum, S_ig=signal, E=execution, R=reward.

---

## 5. JSON Serialization Contract

A representative Risk Matrix frame. The example illustrates the JSON shape and the `score → level` band translation from §2.3. Per-dimension derivation rules are in §4.

```json
{
  "symbol": "BTC-USDT",
  "market_risk":     { "score": 35.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0, "evidence": ["High confidence"] },
  "volatility_risk": { "score": 45.0, "level": "MODERATE", "state": "STABLE", "confidence": 50.0, "evidence": ["BBWP elevated"] },
  "liquidity_risk":  { "score": 15.0, "level": "VERY_LOW", "state": "STABLE", "confidence": 50.0, "evidence": ["Strong participation"] },
  "structure_risk":  { "score": 25.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "momentum_risk":   { "score": 20.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "signal_risk":     { "score": 30.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "execution_risk":  { "score": 25.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "reward_risk":     { "score": 30.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 },
  "overall_risk":    { "score": 28.0, "level": "LOW",      "state": "STABLE", "confidence": 50.0 }
}
```

Empty `evidence` arrays are omitted. Enum values serialize as `SCREAMING_SNAKE_CASE`.

---

## 6. Empty State

When `analysis.timeframes_considered == 0`, `compute_risk` returns `RiskMatrix::empty()` — all nine dimensions defaulting to score `50.0` (`MODERATE`), reflecting maximal uncertainty in the absence of data.

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
