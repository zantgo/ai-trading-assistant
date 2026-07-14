# MME Layer 5 — Risk Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Layer:** 5 of 7
**Output Contract:** [Risk Matrix](../../matrices/02-11-risk-matrix.md)
**Purpose:** This document specifies the Risk Layer — the process that quantifies ex-ante threat dimensions (volatility, liquidity, structure, momentum, signal, execution, reward) on a direction-independent unipolar 0–100 scale.

---

## 1. Purpose

The Risk Layer measures **danger**, independent of direction. It consumes the [Analysis Matrix](../../matrices/02-02-analysis-matrix.md) plus the underlying indicator map and produces the nine-dimensional [Risk Matrix](../../matrices/02-11-risk-matrix.md).

```
[Analysis Matrix] ─┐
                   ├──► RISK LAYER (L5) ──► [Risk Matrix]
[Metrics indicators]┘      compute_risk()      (9 unipolar dimensions)
```

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
| `reward_risk` | Opportunity quality vs environmental uncertainty. |
| `overall_risk` | Weighted aggregate. |

All scores are **unipolar** in `[0, 100]` (higher = riskier). Per-dimension additive scoring contracts are specified in [Risk Matrix §4](../../matrices/02-11-risk-matrix.md).

---

## 3. Overall Aggregation

$$\text{overall} = 0.15M + 0.15V + 0.15L + 0.10S_{tr} + 0.15M_{om} + 0.10S_{ig} + 0.10E + 0.10R$$

RiskLevel banding: `≥80` Extreme · `≥60` High · `≥40` Moderate · `≥20` Low · else VeryLow.

---

## 4. Evidence & Explainability

Each `RiskDimension` carries an `evidence` list of the specific factors that raised or lowered its score (e.g. `"BBWP extreme expansion"`, `"Strong participation"`). This makes every risk score auditable.

---

## 5. Interaction with Opportunity

The Risk Layer (L5) and the Opportunity Layer (L4) are **independent orthogonal branches** of the MME pipeline. L5 does **not** consume the L4 Opportunity Matrix; both layers read the [Analysis Matrix (L3)](../../matrices/02-02-analysis-matrix.md) directly and execute in parallel. Evaluating risk must never depend on the opportunity score, and scoring an opportunity must never be limited by risk.

The `reward_risk` dimension does use an opportunity-quality signal, but it pulls it from the **Analysis Matrix's** `opportunity_analysis` field (an L3 input summarising setup viability) — **not** from the L4 Opportunity Matrix. A high-quality L3 opportunity in an otherwise dangerous environment reduces *reward risk*; a low-quality or absent opportunity amplifies it.

The convergence of the L4 and L5 branches happens at [Layer 6 (Decision Support)](03-02-07-mme-layer6-decision-support.md), where L6 combines the orthogonal vectors with the directional bias from L3 to produce guidance — a high-opportunity, high-risk configuration yields a cautious stance even with strong bias.

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
