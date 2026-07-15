# MME Layer 4 — Opportunity Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Layer:** 4 of 7
**Output Contract:** [Opportunity Matrix](../../matrices/02-08-opportunity-matrix.md)
**Purpose:** This document specifies the Opportunity Layer — the process that sets parameters for and scores strategy-agnostic opportunities (breakout, continuation, pullback, mean-reversion, reversal) on a 0–100 scale.

---

## 1. Purpose

The Opportunity Layer identifies **positive** market configurations and scores their viability, independent of direction of exposure or execution parameters. It consumes the [Analysis Matrix](../../matrices/02-02-analysis-matrix.md) and the underlying Metrics Matrix signals, producing the [Opportunity Matrix](../../matrices/02-08-opportunity-matrix.md).

```
[Analysis Matrix (L3)] ─┐
                         ├──► OPPORTUNITY LAYER (L4) ──► [Opportunity Matrix]
[Metrics signals (L1)]  ┘        profile + score
                                                │
                                                ▼
                                          L6 (Decision)
```

**Dependency edges:** L4 reads L3 only. L4 does **not** read L5. L4 outputs to L6 only. See [02-00-matrix-field-ownership.md](../../matrices/02-00-matrix-field-ownership.md).

---

## 2. Candidate Setup Types

The layer profiles each candidate `OpportunityType`:

| Setup | Precondition Signature |
|-------|------------------------|
| `TrendContinuation` | Strong/healthy trend + directional bias + non-exhausted momentum. |
| `Breakout` | Volatility expansion + healthy structure + compression release / level breach. |
| `Pullback` | Established trend + weakening momentum + retrace to dynamic level. |
| `MeanReversion` | Volatility compression + range regime + oscillator extreme. |
| `Reversal` | Confirmed divergence + structure break + reversing momentum. |
| `NoClearOpportunity` | Opportunity dimension < 30 or conflicting evidence. |

---

## 3. Scoring Model

Each candidate's `score ∈ [0, 100]` blends four factors:

$$\text{score} = 0.35\,Q_{ctx} + 0.30\,S_{sig} + 0.20\,A_{mtf} + 0.15\,F_{fresh}$$

| Factor | Source |
|--------|--------|
| `Q_ctx` — context quality | Analysis `market_quality` + relevant assessment dimension. |
| `S_sig` — signal support | Strength + confirmation status of contributing signals. |
| `A_mtf` — MTF agreement | Alignment `trend_agreement_pct` for directional setups. |
| `F_fresh` — freshness | Inverse of youngest contributing signal `age_bars`. |

The **primary opportunity** is the highest-scoring profile; ties resolve toward the highest-precondition-satisfaction profile.

---

## 4. Setup-Quality Classification

| SetupQuality | Score band |
|--------------|-----------|
| `Prime` | ≥ 85 |
| `Strong` | 70–85 |
| `Moderate` | 50–70 |
| `Marginal` | 30–50 |
| `None` | < 30 |

---

## 5. Parameter Setting

For each profiled opportunity the layer records the preconditions evaluated and satisfied (`preconditions_met / preconditions_total`), the contributing signal labels, and an invalidation note describing what would nullify the setup. These parameters flow to the Decision Layer to shape entry/target/protection strategy.

---

## 6. Guarantees

| Property | Guarantee |
|----------|-----------|
| **Direction-neutral scoring** | Score reflects viability, not profit expectation. |
| **Strategy-agnostic** | No assumption of a specific trading methodology. |
| **Explainable** | Score decomposes into four weighted factors + precondition fractions. |
| **Bounded** | All scores clamp to `[0, 100]`. |

---

## 7. Cross-References

- [Analysis Matrix](../../matrices/02-02-analysis-matrix.md) — Input.
- [Opportunity Matrix](../../matrices/02-08-opportunity-matrix.md) — Output contract.
- [MME Layer 6 — Decision Support](03-02-07-mme-layer6-decision-support.md) — Consumer.
