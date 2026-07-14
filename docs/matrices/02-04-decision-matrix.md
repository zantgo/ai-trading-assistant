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

### 2.1 AdvisoryMatrix Fields

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | `string` | Entity. |
| `directional_guidance` | `DirectionalGuidance` | Recommended directional posture (§3.1). |
| `market_stance` | `MarketStance` | Aggressiveness posture (§3.2). |
| `opportunity_classification` | `OpportunityClass` | Mirror of the opportunity setup type. |
| `strategy_environment` | `StrategyEnvironment` | Which strategy family the environment favours (§3.3). |
| `entry_guidance` | `EntryGuidance` | How to time entry (§3.4). |
| `exit_guidance` | `ExitGuidance` | Early-warning exit trigger (§3.5). |
| `protection_strategy` | `ProtectionStrategy` | How to place the stop (§3.6). |
| `target_strategy` | `TargetStrategy` | How to place the target (§3.7). |
| `confidence_assessment` | `f64` | Guidance confidence in `[0, 100]`. |
| `final_recommendation` | `string` | Natural-language recommendation summary. |

### 2.2 DecisionContext Fields

| Field | Type | Description |
|-------|------|-------------|
| `score` | `f64` | Quantitative confluence score. |
| `bias` | `string` | `BULLISH` / `BEARISH` / `NEUTRAL`. |
| `confidence` | `f64` | `[0, 1]` derived from `|score| / 100`. |
| `contributing_indicators` | `string[]` | Indicators driving the decision. |

---

## 3. Guidance Vocabularies

### 3.1 DirectionalGuidance
`STRONG_LONG`, `LONG`, `NEUTRAL`, `SHORT`, `STRONG_SHORT`, `AVOID_DIRECTIONAL_EXPOSURE`.

Derived from `bias × overall_risk`:
```
STRONG_BULLISH + risk<50 → STRONG_LONG   | else LONG
BULLISH         + risk<40 → LONG         | else NEUTRAL
STRONG_BEARISH  + risk<50 → STRONG_SHORT | else SHORT
BEARISH         + risk<40 → SHORT        | else NEUTRAL
NEUTRAL                    → NEUTRAL
```

### 3.2 MarketStance
`AGGRESSIVE`, `CONSTRUCTIVE`, `NEUTRAL`, `CAUTIOUS`, `AVOID` — from `market_quality × overall_risk`.

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
reward_risk < 40             → RR_BASED
otherwise                    → VOLATILITY_BASED
```

---

## 4. Trade Readiness & Confidence

The Decision Matrix's headline confidence combines analysis conviction with risk discount:

$$\text{confidence\_assessment} = \text{clamp}\Big(\text{analysis.confidence} \times \big(1 - \tfrac{\text{overall\_risk}}{100}\big) \times 100,\ 0,\ 100\Big)$$

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
    "opportunity_classification": "BREAKOUT",
    "strategy_environment": "TREND_FOLLOWING",
    "entry_guidance": "IMMEDIATE",
    "exit_guidance": "NO_WARNING",
    "protection_strategy": "ATR_BASED",
    "target_strategy": "RESISTANCE_BASED",
    "confidence_assessment": 72.0,
    "final_recommendation": "Strong long bias: STRONG_BULLISH bias with 72% confidence, constructive stance in a trend-following environment. Breakout opportunity. Entry: immediate. Stop: ATR-based."
  },
  "decision_context": {
    "score": 100.0,
    "bias": "STRONG_BULLISH",
    "confidence": 1.0,
    "contributing_indicators": ["ema_stack", "macd", "adx", "squeeze"]
  }
}
```

**Self-consistency check** (the example values satisfy the §4 formula):
- Analysis Matrix `confidence` = 1.0; Risk Matrix `overall_risk.score` = 28 (matches the Risk Matrix JSON example).
- `confidence_assessment = clamp(1.0 × (1 − 28/100) × 100, 0, 100) = clamp(72.0, 0, 100) = 72.0` ✓
- `bias = STRONG_BULLISH` with `overall_risk = 28 < 50` ⇒ `directional_guidance = STRONG_LONG` per the §3.1 rule ✓
- `score = 100` and `bias_confidence = 1.0` ⇒ `decision_context.confidence = |score|/100 = 1.0` per the §2.2 mapping ✓

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
