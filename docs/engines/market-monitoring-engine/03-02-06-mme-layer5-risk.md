# MME Layer 5 — Risk Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Layer:** 5 of 7
**Output Contract:** [Risk Matrix](../../matrices/02-11-risk-matrix.md)
**Purpose:** This document specifies the Risk Layer — the process that quantifies ex-ante threat dimensions (volatility, liquidity, structure, momentum, signal, execution) on a direction-independent unipolar 0–100 scale.

---

## 1. Purpose

The Risk Layer measures **danger**, independent of direction. It consumes the [Analysis Matrix](../../matrices/02-02-analysis-matrix.md) plus the underlying indicator map and produces the **eight-dimensional** [Risk Matrix](../../matrices/02-11-risk-matrix.md). *(Reduced from nine in the institutional redesign; `reward_risk` moved to Decision Layer as `environment_favorability`.)*

```
[Analysis Matrix (L3)] ─┐
                         ├──► RISK LAYER (L5) ──► [Risk Matrix]
[Metrics indicators (L1)]┘      compute_risk()      (8 unipolar dimensions)
                                                │
                                                ▼
                                          L6 (Decision)
```

**Dependency edges:** L5 reads L3 only. L5 does **not** read L4. L5 outputs to L6 only. See [02-00-matrix-field-ownership.md](../../matrices/02-00-matrix-field-ownership.md).

Implementation: `crates/shared/src/risk.rs::compute_risk()`.

Risk is a property of an *interpretation*: it consumes the Analysis Matrix because you cannot evaluate how risky a bullish trend is until you know a bullish trend exists.

---

## 2. Unipolar Threat Dimensions

| Dimension | Threat Vector |
|-----------|---------------|
| `market_risk` | General uncertainty from conflict / weak structure. |
| `volatility_risk` | Abnormal price movement (BBWP, ATR, squeeze). |
| `liquidity_risk` | Thin participation (RVOL, spread). |
| `structure_risk` | Weak / damaged / flipped structure. |
| `momentum_risk` | Exhausted / diverging momentum. |
| `signal_risk` | Conflicting / unreliable signals. |
| `execution_risk` | Spread / slippage / thin-book difficulty. |
| `overall_risk` | Weighted aggregate. |

All scores are **unipolar** in `[0, 100]` (higher = riskier). Per-dimension additive scoring contracts are specified in [Risk Matrix §4](../../matrices/02-11-risk-matrix.md).

> **Removed in the institutional redesign.** The previous `reward_risk` dimension has been **removed** from the Risk Matrix. Reward synthesis now lives at [Decision Matrix `environment_favorability`](02-04-decision-matrix.md) (semantic successor).

---

## 3. Overall Aggregation

$$\text{overall} = 0.15M + 0.20V + 0.15L + 0.10S_{tr} + 0.15M_{om} + 0.15S_{ig} + 0.10E$$

where M=market, V=volatility, L=liquidity, S_tr=structure, M_om=momentum, S_ig=signal, E=execution. Weights re-normalized after `reward_risk` removal: total = 1.0.

RiskLevel banding: `≥80` Extreme · `≥60` High · `≥40` Moderate · `≥20` Low · else VeryLow.

---

## 4. Evidence & Explainability

Each `RiskDimension` carries an `evidence` list of the specific factors that raised or lowered its score (e.g. `"BBWP extreme expansion"`, `"Strong participation"`). This makes every risk score auditable.

---

## 5. Interaction with Opportunity

The Risk Layer (L5) and the Opportunity Layer (L4) are **strictly orthogonal branches** of the MME pipeline. L5 does **not** consume the L4 Opportunity Matrix; both layers read the [Analysis Matrix (L3)](../../matrices/02-02-analysis-matrix.md) directly and execute in parallel. Evaluating risk must never depend on the opportunity score, and scoring an opportunity must never be limited by risk.

**With the institutional redesign (Option α):** the previous `reward_risk` dimension (which synthesized L3 + L4 fields) has been **removed** from the Risk Matrix — reward evaluation is a synthesis concept and belongs in the [Decision Layer (L6)](03-02-07-mme-layer6-decision-support.md) as the new `environment_favorability` field. The Risk Matrix now contains **8 unipolar danger dimensions** + `overall_risk` — pure environmental danger, no reward synthesis.

The convergence of the L4 and L5 branches happens at [Layer 6 (Decision Support)](03-02-07-mme-layer6-decision-support.md), where L6 combines the orthogonal L3 + L4 + L5 vectors to produce guidance — a high-opportunity, high-risk configuration yields a cautious stance even with strong bias. L6 is the **only** synthesis point.

---

## 6. Guarantees

| Property | Guarantee |
|----------|-----------|
| **Direction independence** | No dimension references bullish/bearish direction. |
| **Unipolar bounding** | Every score ∈ `[0, 100]`. |
| **Explainability** | Every dimension exposes contributing evidence. |
| **Empty safety** | Zero timeframes → all dimensions default to 50 (Moderate). |

---

## 7. Cross-References

- [Analysis Matrix](../../matrices/02-02-analysis-matrix.md) — Input.
- [Risk Matrix](../../matrices/02-11-risk-matrix.md) — Output contract.
- [MME Layer 6 — Decision Support](03-02-07-mme-layer6-decision-support.md) — Consumer.
- [PME Layer 4 — Portfolio](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — Systemic risk consumer.
