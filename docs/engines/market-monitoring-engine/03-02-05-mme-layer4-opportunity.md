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

The layer profiles each candidate `OpportunityType`. The canonical enum is **eight-valued** — the original six, plus `LiquiditySqueeze` added in the Phase 0-4 Liquidity Intelligence extension, plus `Scalp` added in the v2.1 institutional completeness sweep (see [02-08-opportunity-matrix.md §3](../matrices/02-08-opportunity-matrix.md) for the canonical precondition table and §4 for the decision tree):

| Setup | Precondition Signature |
|-------|------------------------|
| `TrendContinuation` | Strong/healthy trend + directional bias + non-exhausted momentum. |
| `Breakout` | Volatility expansion + healthy structure + compression release / level breach. |
| `Pullback` | Established trend + weakening momentum + retrace to dynamic level. |
| `MeanReversion` | Volatility compression + range regime + oscillator extreme. |
| `Reversal` | Confirmed divergence + structure break + reversing momentum. |
| `LiquiditySqueeze` | `LiquidityFlow.cascade_state ∈ {Detected, Sustained}` AND `|LiquidationClusterMatrix.cascade_asymmetry| > 0.3` AND `regime ∈ {EXPANSION, TRANSITION}`. Surface as a defensive opportunity — drives `CLOSE_ONLY` stance policy and tightens stops. |
| `Scalp` | High per-candle volatility (BBWP ∈ [70, 95)) + tight structural context (alignment dim 4 `Structure` ≥ 70) + directional bias + regime ∈ {TRENDING_BULL, TRENDING_BEAR}. Sub-minute-to-seconds holding period; maps to `time_horizon = SCALP`. Designed for HFT-adjacent, complementary to `Breakout` (multi-bar continuation) and `TrendContinuation` (multi-day). |
| `NoClearOpportunity` | No candidate met its preconditions (and no `LiquiditySqueeze` is active). |

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

The **primary opportunity** is determined by the priority-ordered decision tree in [02-08-opportunity-matrix.md §4](../../matrices/02-08-opportunity-matrix.md) (first match wins). The `opportunity_score` and `profiles[]` array expose the full scoring breakdown for downstream consumers but do **not** override the tree selection. In a tie, the profile with the higher `preconditions_met / preconditions_total` ratio wins.

---

## 4. Setup-Quality Classification

**Strict half-open intervals** — each `opportunity_score ∈ [0, 100]` maps to exactly one SetupQuality band, no endpoint ambiguity:

| SetupQuality | Score band | Interpretation |
|--------------|------------|----------------|
| `Prime` | `> 85` | High-conviction configuration, all key preconditions met. |
| `Strong` | `> 70 AND ≤ 85` | Robust setup with minor gaps. |
| `Moderate` | `> 50 AND ≤ 70` | Tradable but requires confirmation. |
| `Marginal` | `> 30 AND ≤ 50` | Weak edge; confluence-only. |
| `None` | `≤ 30` | No actionable opportunity. |

The canonical form (above) matches [Opportunity Matrix §5](../../matrices/02-08-opportunity-matrix.md) and [01-01-ontology.md §A.4](../conceptual-foundations/01-01-ontology.md). A previous version of this table used `[85, 100]`, `[70, 85]`, etc. — these closing both ends created two-band ambiguity at boundaries (e.g. a score of `85` would satisfy both `Prime` and `Strong` simultaneously). The strict half-open form eliminates this.

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
