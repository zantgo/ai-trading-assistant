# Decision Matrix Specification

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 6 — Decision Layer
**Purpose:** This document defines the physical schema of the **Decision Matrix** — the strategic decision-support object. It synthesizes bias (Analysis), value (Opportunity), and vulnerability (Risk) into structured, human-facing guidance: trade readiness, directional guidance, dynamic protection strategy, target strategy, and scenario pathways. It is the terminal output the Trade Automation Engine consumes.

---

## 1. Conceptual Definition

Per the [Ontology](../conceptual-foundations/01-01-ontology.md) §3.16, **Decision Support** transforms market intelligence into actionable tactical guidance **without making autonomous execution choices**. It answers *"given everything we understand, what is the recommended posture — and what would change it?"*.

The Decision Matrix is realized by two complementary structures:

1. **`AdvisoryMatrix`** (`crates/shared/src/advisory.rs`) — the human-facing guidance layer (directional guidance, stance, entry/exit/protection/target strategy).
2. **`DecisionContext`** (`crates/shared/src/decision_context.rs`) — the quantitative decision metadata (score, bias, confidence, contributing indicators).

```
[Analysis Matrix] ─┐
[Opportunity Mat.] ─┼──► DECISION LAYER (L6) ──► [Decision Matrix]
[Risk Matrix     ] ─┘     compute_advisory()       Advisory + DecisionContext
```

---

## 2. Decision Matrix Schema (AdvisoryMatrix fields)

> **Type-Boundary Note.** The `protection_strategy` (§3.6) resolves to a concrete **`stop_loss_distance_pct`**, which is represented as an **`f64`** (a raw percentage float, e.g. `1.5` = 1.5%). This `f64` is the canonical **type-boundary handoff variable** between the MME (hot path, `f64`) and the TAE (cold path, `Decimal`): the TAE casts it to `Decimal` at the execution boundary before sizing (see [Global Architecture §6.3](../conceptual-foundations/01-02-global-architecture.md) and [TAE Layer 2 §2.1](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md)).
>
> **Removed in the institutional redesign.** The previous `opportunity_classification` field has been **removed** from the Decision Matrix. The Decision Layer reads `Opportunity.primary_opportunity` directly from the L4 Opportunity Matrix. The `OpportunityClass` enum that the removed field used is now sourced from L4; see [02-00-matrix-field-ownership.md](02-00-matrix-field-ownership.md).

### 2.1 AdvisoryMatrix Fields

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | `string` | Entity. |
| `directional_guidance` | `DirectionalGuidance` | Recommended directional posture (§3.1). |
| `market_stance` | `MarketStance` | Aggressiveness posture (§3.2). |
| `strategy_environment` | `StrategyEnvironment` | Which strategy family the environment favours (§3.3). |
| `entry_guidance` | `EntryGuidance` | How to time entry (§3.4). |
| `exit_guidance` | `ExitGuidance` | Early-warning exit trigger (§3.5). |
| `protection_strategy` | `ProtectionStrategy` | How to place the stop (§3.6). |
| `target_strategy` | `TargetStrategy` | How to place the target (§3.7). |
| `confidence_assessment` | `f64` | Guidance confidence in `[0, 100]`. |
| `trade_readiness` | `TradeReadiness` | Headline readiness state (§4). `READY` / `FORMING` / `WATCH` / `STAND_ASIDE`. *(Added to the schema in the institutional redesign; previously documented in §4 but missing from §2.1.)* |
| `environment_favorability` | `RiskDimension` | Synoptic measure of how favorable current conditions are for entering a position. Synthesized from L3 `market_quality` and L4 `opportunity_score` — see §3.8 for the derivation rule. *(Added in the institutional redesign — semantic successor of `Risk.reward_risk`.)* |
| `expected_reward_risk_ratio` | `f64` | Synthesized from `L4.expected_rr` × `1 − L5.overall_risk / 100`. *(Added in the institutional redesign.)* |
| `final_recommendation` | `string` | Natural-language recommendation summary. |

### 2.2 DecisionContext Fields

| Field | Type | Description |
|-------|------|-------------|
| `score` | `f64` | Quantitative confluence score. |
| `bias` | `MarketBias` (5-state) | `STRONG_BULLISH` / `BULLISH` / `NEUTRAL` / `BEARISH` / `STRONG_BEARISH`. **Same 5-state vocabulary as `Analysis.bias`** — no 3-state collapse is applied. |
| `score_confidence` | `f64` | `[0, 1]` derived from `|score| / 100`. *(Renamed from `confidence` in the institutional redesign; see [02-00b-confidence-hierarchy.md](02-00b-confidence-hierarchy.md).)* |
| `contributing_indicators` | `string[]` | Indicators driving the decision. |

---

## 3. Guidance Vocabularies

### 3.1 DirectionalGuidance
`STRONG_LONG`, `LONG`, `NEUTRAL`, `SHORT`, `STRONG_SHORT`, `AVOID_DIRECTIONAL_EXPOSURE`.

Derived from `bias × overall_risk × market_stance`:
```
# Priority order (first match wins):
1. market_stance = AVOID                                  → AVOID_DIRECTIONAL_EXPOSURE
2. STRONG_BULLISH + risk<50                               → STRONG_LONG
3. STRONG_BULLISH + risk≥50                               → LONG
4. BULLISH         + risk<40                              → LONG
5. BULLISH         + risk≥40                              → NEUTRAL
6. STRONG_BEARISH  + risk<50                             → STRONG_SHORT
7. STRONG_BEARISH  + risk≥50                             → SHORT
8. BEARISH         + risk<40                              → SHORT
9. BEARISH         + risk≥40                              → NEUTRAL
10. NEUTRAL                                                 → NEUTRAL
```

All six `DirectionalGuidance` values are reachable. The `AVOID_DIRECTIONAL_EXPOSURE` guard at priority 1 ensures the L6 output correctly reflects the L6's own `market_stance = AVOID` determination (e.g. when the L6 stance is forced AVOID by a PME veto or a system safety trigger).

### 3.2 MarketStance
`AGGRESSIVE`, `CONSTRUCTIVE`, `NEUTRAL`, `CAUTIOUS`, `AVOID`.

Derived from `market_quality × overall_risk`:
```
# Priority order (first match wins):
1. market_quality ∈ {POOR}                  OR overall_risk ≥ 80           → AVOID
2. market_quality ∈ {POOR, WEAK}            OR overall_risk ≥ 60           → CAUTIOUS
3. market_quality ∈ {AVERAGE}               AND overall_risk <  40         → NEUTRAL
4. market_quality ∈ {GOOD}                  AND overall_risk <  60         → CONSTRUCTIVE
5. market_quality ∈ {EXCELLENT}             AND overall_risk <  40         → AGGRESSIVE
6. otherwise                                                              → CONSTRUCTIVE  (default)
```

All five `MarketStance` values are reachable. The `AVOID` and `CAUTIOUS` guards are "sticky" — they fire on either bad quality or high risk, so the stance correctly reflects "do not engage" when either condition is met.

### 3.3 StrategyEnvironment
`TREND_FOLLOWING`, `BREAKOUT`, `MEAN_REVERSION`, `HIGH_VOLATILITY`, `LOW_ACTIVITY`, `UNFAVORABLE` — from `market_regime`.

### 3.4 EntryGuidance
`IMMEDIATE`, `WAIT_FOR_CONFIRMATION`, `PULLBACK`, `BREAKOUT`, `NO_ENTRY_CONTEXT` — from `trend_assessment × volatility_risk`.

### 3.5 ExitGuidance
`TREND_WEAKENING`, `MOMENTUM_EXHAUSTION`, `STRUCTURE_BREAKDOWN`, `RISK_INCREASING`, `NO_WARNING` — from `momentum_assessment × overall_risk`.

### 3.6 ProtectionStrategy (Dynamic Stops)
`STRUCTURE_BASED`, `VOLATILITY_BASED`, `ATR_BASED`, `SR_BASED`, `NO_RECOMMENDATION`.
```
volatility compressed        → STRUCTURE_BASED
volatility_risk > 60         → VOLATILITY_BASED
otherwise                    → ATR_BASED
```

### 3.7 TargetStrategy (Target Zones)
`RESISTANCE_BASED`, `RR_BASED`, `VOLATILITY_BASED`, `TRAILING_METHOD`, `NO_RECOMMENDATION`.
```
structure strong/healthy     → RESISTANCE_BASED
environment_favorability.score < 40  → RR_BASED
otherwise                    → VOLATILITY_BASED
```

### 3.8 `environment_favorability` (Synoptic Favorability)

`environment_favorability` is a `RiskDimension` (score, level, state, confidence, evidence) — semantic successor of the old `Risk.reward_risk` (which was removed in the institutional redesign). It synthesizes the **favorability of entering a position in the current interpretive state** by combining L3 `market_quality` and L4 `opportunity_score` — the two forward-looking signals most relevant to "is the environment supportive of a new trade?".

```
# Base from market_quality (institutional bands):
quality_penalty = EXCELLENT → 10 · GOOD → 25 · AVERAGE → 50 · WEAK → 70 · POOR → 80

# Combine with opportunity_score (institutional 0–100):
score = mean(quality_penalty, 100 − opportunity_score)  // ∈ [0, 100]

# RiskLevel banding: same as Risk Matrix §2.3
```

**Why this derivation:** `quality_penalty` reflects "how *poor* is the environment" (low = excellent conditions, high = dangerous conditions). `100 − opportunity_score` reflects "how *poor* is the absence of a setup" (low = great setup, high = no viable setup). Averaging the two gives a synoptic "how dangerous is it to enter here?" measure. This is the natural successor of the old `Risk.reward_risk` formula, with the addition of `opportunity_score` as an L4 input (legitimate L6 synthesis of state + forecast + danger).

---

## 4. Trade Readiness & Confidence

The Decision Matrix's headline confidence combines analysis conviction with risk discount:

$$\text{confidence\_assessment} = \text{clamp}\Big(\text{analysis.state\_confidence} \times \big(1 - \tfrac{\text{overall\_risk}}{100}\big) \times 100,\ 0,\ 100\Big)$$

Trade readiness is a function of this confidence and the directional guidance:

| Readiness | Condition |
|-----------|-----------|
| `READY` | Non-neutral guidance + `confidence_assessment ≥ 60` + stance ∈ {AGGRESSIVE, CONSTRUCTIVE}. |
| `FORMING` | Directional guidance present but confidence `40–60` or entry = WAIT_FOR_CONFIRMATION. |
| `WATCH` | Neutral guidance or `confidence_assessment 20–40`. |
| `STAND_ASIDE` | Stance = AVOID or `confidence_assessment < 20`. |

---

## 5. Scenario Pathways & Invalidation

The Decision Matrix carries the structural invalidation and target context used by the TAE Position Sizing Protocol:

| Concept | Source | Consumer |
|---------|--------|----------|
| **Stop-loss distance (`D_sl`, %)** | `protection_strategy` applied to ATR / structure levels. | TAE Execution ($S = \frac{E \times R}{D_{sl} / 100}$). |
| **Target zone** | `target_strategy` applied to resistance / R:R / volatility. | TAE Execution, PME trailing. |
| **Invalidation level** | Structural level whose breach nullifies the thesis. | PME dynamic stop management. |
| **Bull / Bear scenario** | Conditional pathways described in `final_recommendation`. | Human operator / observability. |

---

## 6. JSON Serialization Contract

```json
{
  "advisory": {
    "symbol": "BTC-USDT",
    "directional_guidance": "STRONG_LONG",
    "market_stance": "CONSTRUCTIVE",
    "strategy_environment": "TREND_FOLLOWING",
    "entry_guidance": "IMMEDIATE",
    "exit_guidance": "NO_WARNING",
    "protection_strategy": "ATR_BASED",
    "target_strategy": "RESISTANCE_BASED",
    "confidence_assessment": 71.25,
    "trade_readiness": "READY",
    "environment_favorability": { "score": 50.0, "level": "MODERATE", "state": "STABLE", "confidence": 50.0, "evidence": ["Strong trend", "Volatility moderate", "Opportunity score 85"] },
    "expected_reward_risk_ratio": 2.4,
    "final_recommendation": "Strong long bias: STRONG_BULLISH bias with 71% confidence, constructive stance in a trend-following environment. Breakout opportunity. Entry: immediate. Stop: ATR-based."
  },
  "decision_context": {
    "score": 100.0,
    "bias": "STRONG_BULLISH",
    "score_confidence": 1.0,
    "contributing_indicators": ["ema_stack", "macd", "adx", "squeeze"]
  }
}
```

**Self-consistency check** (the example values satisfy the §4 formula):
- Analysis Matrix `state_confidence` = 1.0; Risk Matrix `overall_risk.score` = 28.75 (matches the Risk Matrix JSON example).
- `confidence_assessment = clamp(1.0 × (1 − 28.75/100) × 100, 0, 100) = clamp(71.25, 0, 100) = 71.25` ✓
- `bias = STRONG_BULLISH` with `overall_risk = 28.75 < 50` ⇒ `directional_guidance = STRONG_LONG` per the §3.1 rule ✓
- `score = 100` and `score_confidence = 1.0` ⇒ `decision_context.score_confidence = |score|/100 = 1.0` per the §2.2 mapping ✓

Enum values serialize as `SCREAMING_SNAKE_CASE`.

---

## 7. Empty State

When `analysis.timeframes_considered == 0`, `compute_advisory` returns `AdvisoryMatrix::empty()`: `directional_guidance = NEUTRAL`, `market_stance = NEUTRAL`, `confidence_assessment = 0.0`, recommendation `"Insufficient data to provide guidance."`.

---

## 8. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **No autonomous execution** | The Decision Matrix recommends; it never places orders. Execution is the TAE's responsibility. |
| **Explainability** | `final_recommendation` and `contributing_indicators` trace every recommendation to its evidence. |
| **Risk-discounted confidence** | Confidence is always attenuated by overall risk. |
| **Stable contract** | The TAE Policy Layer depends only on these public fields, not on internal derivation. |

---

## 9. Cross-References

- [Analysis Matrix](02-02-analysis-matrix.md) · [Opportunity Matrix](02-08-opportunity-matrix.md) · [Risk Matrix](02-11-risk-matrix.md) — Inputs.
- [Overview Matrix](02-09-overview-matrix.md) — Aggregates Decision matrices across symbols.
- [MME Layer 6 — Decision Support](../engines/market-monitoring-engine/03-02-07-mme-layer6-decision-support.md) — Producing-layer specification.
- [TAE Layer 1 — Policy](../engines/trade-automation-engine/03-03-02-tae-layer1-policy.md) — Primary downstream consumer.
- [TAE Layer 2 — Execution](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md) — Position Sizing Protocol.
