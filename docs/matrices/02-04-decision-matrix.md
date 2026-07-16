# Decision Matrix Specification

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 6 — Decision Layer
**Purpose:** This document defines the physical schema of the **Decision Matrix** — the strategic decision-support object. It synthesizes bias (Analysis), value (Opportunity), and vulnerability (Risk) into structured, human-facing guidance: trade readiness, directional guidance, dynamic protection strategy, target strategy, and scenario pathways. It is the terminal output the Trade Automation Engine consumes.

---

## 1. Conceptual Definition

Per the [Ontology](../conceptual-foundations/01-01-ontology.md) §3.16, **Decision Support** transforms market intelligence into actionable tactical guidance **without making autonomous execution choices**. It answers *"given everything we understand, what is the recommended posture — and what would change it?"*.

The Decision Matrix is realized by two complementary structures:

1. **`AdvisoryMatrix`** (`crates/core-domain/src/advisory.rs`) — the human-facing guidance layer (directional guidance, stance, entry/exit/protection/target strategy).
2. **`DecisionContext`** (`crates/core-domain/src/decision_context.rs`) — the quantitative decision metadata (score, bias, confidence, contributing indicators).

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
| `stop_loss_distance_pct` | `f64` | Concrete stop-loss distance as a raw percentage float (e.g. `1.5` = 1.5%). Type-boundary handoff from MME (f64) to TAE (Decimal cast at the execution boundary). Computed from the active `protection_strategy` (§3.6) and the current volatility/structure inputs. |
| `confidence_assessment` | `f64` | Guidance confidence in `[0, 100]`. |
| `trade_readiness` | `TradeReadiness` | Headline readiness state (§4). `READY` / `FORMING` / `WATCH` / `STAND_ASIDE`. *(Added to the schema in the institutional redesign; previously documented in §4 but missing from §2.1.)* |
| `entry_danger` | `RiskDimension` | Synoptic measure of how dangerous the current interpretive state is for entering a new position. High score = dangerous (do not enter); low score = safe to enter. Synthesized from L3 `market_quality` and L4 `opportunity_score` — see §3.8 for the derivation rule. *(Renamed from `environment_favorability` in v2.1; semantic successor of `Risk.reward_risk`. The semantic inversion reflects the RiskDimension convention: high score = danger, low score = safe. The previous name `environment_favorability` was misleading — high favorability would suggest low score, but the actual formula produces a danger measure where high score = danger.)* |
| `expected_reward_risk_ratio` | `f64` | Synthesized from `L4.expected_rr_internal × (1 − L5.overall_risk / 100.0)`. `L4.expected_rr_internal` is the L4 Opportunity Matrix's internal score (renamed from `expected_rr` in v2.1 to disambiguate from the Decision-Layer `expected_reward_risk_ratio`). *(Added in the institutional redesign.)* |

> **`expected_reward_risk_ratio` formula — unit normalization.** `overall_risk` is on the canonical `[0, 100]` scale; the formula divides by `100.0` before the subtraction: `L4.expected_rr_internal × (1 − L5.overall_risk / 100.0) = 2.5 × (1 − 0.283) = 1.79`. Without the `/100.0` normalization the formula produces nonsensical values for any non-trivial risk score (e.g. with `overall_risk = 28.3`, the unnormalized form gives `2.5 × (1 − 28.3) = −68.25`). The same formula appears in the canonical ownership map at [02-00-matrix-field-ownership.md §2.6](../matrices/02-00-matrix-field-ownership.md).
| `final_recommendation` | `string` | Natural-language recommendation summary. |

### 2.2 DecisionContext Fields

| Field | Type | Description |
|-------|------|-------------|
| `score` | `f64` | Quantitative confluence score. |
| `bias` | `MarketBias` (5-state) | `STRONG_BULLISH` / `BULLISH` / `NEUTRAL` / `BEARISH` / `STRONG_BEARISH`. **Same 5-state vocabulary as `Analysis.bias`** — no 3-state collapse is applied. |
| `score_confidence` | `f64` | `[0, 1]` derived from `|score| / 100`. *(Renamed from `confidence` in the institutional redesign; see [02-00b-confidence-hierarchy.md](02-00b-confidence-hierarchy.md).)* |
| `contributing_indicators` | `string[]` | Indicators driving the decision. |

### 2.3 `confluence_score` formula (canonical)

The Decision Matrix's `decision_context.score` is a weighted blend of three upstream dimensions, computed at synthesis time:

```
decision_context.score = 0.50 · alignment.tradability_dim
                       + 0.30 · analysis.market_quality_score
                       + 0.20 · opportunity.opportunity_score
```

where `alignment.tradability_dim` is dimension 9 of the [Alignment Matrix](../matrices/02-01-alignment-matrix.md) (renamed from "Opportunity" in the institutional redesign because L4 owns opportunity concepts; this dimension measures cross-TF agreement on tradability), `analysis.market_quality_score` is the L3 quality score in `[0, 100]`, and `opportunity.opportunity_score` is the L4 score in `[0, 100]`. Weights sum to 1.00.

The canonical worked example below (§6) recomputes under this formula.

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

> **Reachability and gating.** Priority 1 (`market_stance = AVOID`) is the **only** path to `AVOID_DIRECTIONAL_EXPOSURE`. This is intentional: the L6 `DirectionalGuidance` is conditional on the L6 `MarketStance` (see §3.2 below), and an `AVOID` stance is itself determined by `market_quality` and `overall_risk` (e.g. `market_quality ∈ {POOR}` or `overall_risk ≥ 80`). The derivation table above is therefore only "live" when `market_stance ∈ {AGGRESSIVE, CONSTRUCTIVE, NEUTRAL, CAUTIOUS}`; whenever the L6 `MarketStance` becomes `AVOID`, every bias × risk combination below priority 1 collapses to `AVOID_DIRECTIONAL_EXPOSURE`. The two `market_stance` families are independent enums (this `MarketStance` is the L6 Decision Matrix field; the PME-managed per-symbol `Stance` of `ACTIVE / CLOSE_ONLY / AVOID` is a separate field — see the disambiguation note in §4 below).

### 3.2 MarketStance
`AGGRESSIVE`, `CONSTRUCTIVE`, `NEUTRAL`, `CAUTIOUS`, `AVOID`.

Derived from `market_quality × overall_risk`:
```
# Priority order (first match wins):
1. market_quality ∈ {POOR}                  OR overall_risk ≥ 80           → AVOID
2. market_quality ∈ {POOR, WEAK}            OR overall_risk ≥ 60           → CAUTIOUS
3. market_quality ∈ {AVERAGE}               AND overall_risk <  40         → NEUTRAL
4. market_quality ∈ {GOOD}                  AND overall_risk <  30         → CONSTRUCTIVE
5. market_quality ∈ {EXCELLENT}             AND overall_risk <  20         → AGGRESSIVE
6. otherwise                                                              → CAUTIOUS  (default)
```

> **Unit convention.** The `overall_risk` thresholds above are on the canonical `[0, 100]` unipolar scale (matching the [Risk Matrix](../matrices/02-11-risk-matrix.md) `RiskDimension.score` unit). The thresholds `80 / 60 / 40 / 30 / 20` correspond to the documented bands in §3.8; the fractional form (`0.80`, `0.60`, `0.40`, `0.30`, `0.20`) would map `overall_risk = 28.3` to "above the AVOID threshold" and is not used.

All five `MarketStance` values are reachable. The `AVOID` and `CAUTIOUS` guards are "sticky" — they fire on either bad quality or high risk, so the stance correctly reflects "do not engage" when either condition is met. Note the tightened risk thresholds for the higher-quality stances: AGGRESSIVE requires risk < 20, CONSTRUCTIVE requires risk < 30. The previous version had CONSTRUCTIVE at risk < 60 and AGGRESSIVE at risk < 40, which created a counterintuitive situation where a mediocre setup (`AVERAGE` quality) with elevated risk could still yield `CONSTRUCTIVE` (via the default rule). The tightened thresholds eliminate this anti-pattern.

> **Default-stance rationale.** The default fallback is `CAUTIOUS`, which preserves monotonic escalation: as risk rises, the stance retreats `CONSTRUCTIVE → NEUTRAL → CAUTIOUS → AVOID` without ever advancing again. Choosing `CONSTRUCTIVE` as the default would create the inverse anomaly (higher-risk environments with `POOR` quality receiving a more aggressive stance than lower-risk ones via the same rule, since `CONSTRUCTIVE` would be the unconditional fall-through while `AVOID` requires `POOR`-quality to fire).

### 3.3 StrategyEnvironment
`TREND_FOLLOWING`, `BREAKOUT`, `MEAN_REVERSION`, `HIGH_VOLATILITY`, `LOW_ACTIVITY`, `UNFAVORABLE` — from `market_regime`.

### 3.4 EntryGuidance
`IMMEDIATE`, `WAIT_FOR_CONFIRMATION`, `PULLBACK`, `BREAKOUT`, `NO_ENTRY_CONTEXT` — from `trend_assessment × volatility_risk`.

### 3.5 ExitGuidance
`TREND_WEAKENING`, `MOMENTUM_EXHAUSTION`, `STRUCTURE_BREAKDOWN`, `RISK_INCREASING`, `NO_WARNING` — from `momentum_assessment × overall_risk`.

### 3.6 ProtectionStrategy (Dynamic Stops)
`STRUCTURE_BASED`, `VOLATILITY_BASED`, `ATR_BASED`, `SR_BASED`, `NO_RECOMMENDATION`.

```
volatility_assessment = COMPRESSED                                              → STRUCTURE_BASED
VolatilityAssessment-risk score > 60  AND  volatility_assessment ∈ {EXPANDING, EXTREME} → VOLATILITY_BASED
market_regime = RANGE  AND  StructureAssessment ∈ {STRONG, HEALTHY}  AND  distance_to_nearest_SR < 0.5 · ATR → SR_BASED
no indicators available (empty state per §7)                                     → NO_RECOMMENDATION
otherwise                                                                       → ATR_BASED
```

All five `ProtectionStrategy` values are reachable from the documented rules. `STRUCTURE_BASED` consumes `volatility_assessment = COMPRESSED` (from `02-02-analysis-matrix.md §3.6`). `VOLATILITY_BASED` requires a high `volatility_risk` score (from the L5 Risk Matrix). `SR_BASED` requires range regime with healthy structure and proximity to a structural S/R level. `ATR_BASED` is the production default. `NO_RECOMMENDATION` is reached only on the empty-state fallback path (no indicators completed; see §7).

### 3.7 TargetStrategy (Target Zones)
`RESISTANCE_BASED`, `RR_BASED`, `VOLATILITY_BASED`, `TRAILING_METHOD`, `NO_RECOMMENDATION`.

```
structure_assessment ∈ {STRONG, HEALTHY}                                         → RESISTANCE_BASED
entry_danger.level ∈ {VERY_LOW, LOW}                                             → RR_BASED
entry_danger.level = MODERATE  AND  a confirmed trailing-signal sequence is active → TRAILING_METHOD
no indicators available (empty state per §7)                                     → NO_RECOMMENDATION
otherwise                                                                       → VOLATILITY_BASED
```

All five `TargetStrategy` values are reachable from the documented rules. `RESISTANCE_BASED` consumes `structure_assessment ∈ {STRONG, HEALTHY}` (from `02-02-analysis-matrix.md §3.5`). `RR_BASED` requires low entry danger (the setup is clean enough to commit to a fixed R:R target). `TRAILING_METHOD` requires a confirmed trailing-signal sequence. `VOLATILITY_BASED` is the production default. `NO_RECOMMENDATION` is reached only on the empty-state fallback path (see §7).

### 3.8 `entry_danger` (Synoptic Danger)

`entry_danger` is a `RiskDimension` (score, level, state, confidence, evidence) — renamed from `environment_favorability` in v2.1 to disambiguate the semantic convention. The RiskDimension convention is that **high score = danger, low score = safe** (consistent with all other Risk Matrix dimensions). The previous name `environment_favorability` was semantically misleading: a high `environment_favorability` would intuitively mean "favorable conditions", but the actual formula produces a danger measure (high = dangerous).

`entry_danger` synthesizes the **danger of entering a new position in the current interpretive state** by combining L3 `market_quality` and L4 `opportunity_score` — the two forward-looking signals most relevant to "is the environment dangerous for a new trade?".

```
# Base from market_quality (institutional bands):
quality_penalty = EXCELLENT → 10 · GOOD → 25 · AVERAGE → 50 · WEAK → 70 · POOR → 80

# Combine with opportunity_score (institutional 0–100):
score = mean(quality_penalty, 100 − opportunity_score)  // ∈ [0, 100]

# RiskLevel banding (aligned with Risk Matrix §2.3 — strict half-open intervals):
# EXTREME  = score ≥ 80
# HIGH     = score ∈ [60, 80)
# MODERATE = score ∈ [40, 60)
# LOW      = score ∈ [20, 40)
# VERY_LOW = score < 20
```

> **`entry_danger` band boundaries.** Bands are strict half-open intervals aligned with the canonical [Risk Matrix §2.3](../matrices/02-11-risk-matrix.md) `RiskLevel` enum (`EXTREME / HIGH / MODERATE / LOW / VERY_LOW`, thresholds `80 / 60 / 40 / 20`). Boundary values map to exactly one band. This unifies the cross-engine vocabulary so `entry_danger.level` and `risk.<dim>.level` share the same enum and the same numeric bands.

**Why this derivation:** `quality_penalty` reflects "how *poor* is the environment" (low = excellent conditions, high = dangerous conditions). `100 − opportunity_score` reflects "how *poor* is the absence of a setup" (low = great setup, high = no viable setup). Averaging the two gives a synoptic "how dangerous is it to enter here?" measure. This is the natural successor of the old `Risk.reward_risk` formula, with the addition of `opportunity_score` as an L4 input (legitimate L6 synthesis of state + forecast + danger).

---

## 4. Trade Readiness & Confidence

The Decision Matrix's headline confidence combines analysis conviction with risk discount:

$$\text{confidence\_assessment} = \text{clamp}\Big(\text{analysis.state\_confidence} \times \big(1 - \tfrac{\text{overall\_risk}}{100}\big) \times 100,\ 0,\ 100\Big)$$

Trade readiness is a function of this confidence and the directional guidance:

| Readiness | Condition |
|-----------|-----------|
| `READY` | Non-neutral guidance + `confidence_assessment ≥ 60` + `market_stance` ∈ {AGGRESSIVE, CONSTRUCTIVE}. |
| `FORMING` | Directional guidance present but confidence `40–60` or entry = WAIT_FOR_CONFIRMATION. |
| `WATCH` | Neutral guidance or `confidence_assessment 20–40`. |
| `STAND_ASIDE` | `market_stance = AVOID` or `confidence_assessment < 20`. |

> **Stance-vs-market-stance disambiguation.** The readiness rules reference `market_stance` (the L6 `MarketStance` 5-state enum: `AGGRESSIVE` / `CONSTRUCTIVE` / `NEUTRAL` / `CAUTIOUS` / `AVOID`, derived from L3 `market_quality` × L5 `overall_risk`). They do **not** reference the symbol **stance** (L1 `Stance` 3-state enum: `ACTIVE` / `CLOSE_ONLY` / `AVOID`, managed by PME Veto). Although both enums share a `CLOSE_ONLY` / `AVOID` semantic neighborhood, they are independent and serve different purposes — `market_stance` is the *environmental aggressiveness assessment* of the L6 Decision Layer, while symbol `stance` is the *execution-authorization state* enforced by the PME safety veto. The pre-trade gate in [08-02-pre-trade-risk-controls.md Gate 1](../operations-and-compliance/08-02-pre-trade-risk-controls.md) already filters by symbol stance before the readiness check.

---

## 5. Scenario Pathways & Invalidation

The Decision Matrix carries the structural invalidation and target context used by the TAE Position Sizing Protocol:

| Concept | Source | Consumer |
|---------|--------|----------|
| **Stop-loss distance (`D_sl`, %)** | `protection_strategy` applied to ATR / structure levels. | TAE Execution ($S = \frac{E \times R}{D_{sl} / 100}$). |
| **Target zone** | `target_strategy` applied to resistance / R:R / volatility. | TAE Execution, PME trailing. |
| **Invalidation level** (`invalidation_level`) | Structural level whose breach nullifies the thesis. | PME dynamic stop management. |
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
    "stop_loss_distance_pct": 1.5,
    "confidence_assessment": 71.7,
    "trade_readiness": "READY",
    "entry_danger": { "score": 12.5, "level": "VERY_LOW", "state": "STABLE", "confidence": 50.0, "evidence": ["Strong trend", "Volatility moderate", "Opportunity score 85"] },
    "expected_reward_risk_ratio": 1.79,
    "final_recommendation": "Strong long bias: STRONG_BULLISH bias with 71% confidence, constructive stance in a trend-following environment. Breakout opportunity. Entry: immediate. Stop: ATR-based."
  },
  "decision_context": {
    "score": 97.0,
    "bias": "STRONG_BULLISH",
    "score_confidence": 0.97,
    "contributing_indicators": ["ema_stack", "macd", "adx", "squeeze"]
  }
}
```

**Self-consistency check** (the example values satisfy the §2.3 / §3.1 / §3.6 / §3.7 / §3.8 / §4 formulas):
- Analysis Matrix `state_confidence` = 1.0; Risk Matrix `overall_risk.score = 28.3` (matches the canonical Risk Matrix JSON example; expressed as a fraction, `0.283`).
- `confidence_assessment = clamp(1.0 × (1 − 0.283) × 100, 0, 100) = clamp(71.7, 0, 100) = 71.7` ✓
- `bias = STRONG_BULLISH` with `overall_risk = 28.3 < 50` ⇒ `directional_guidance = STRONG_LONG` per the §3.1 rule ✓
- `decision_context.score = 97.0` per the §2.3 confluence-score formula: with `alignment.tradability_dim = 100`, `analysis.market_quality_score = 100` (EXCELLENT → 100), and `opportunity.opportunity_score = 85`, the formula yields `0.50·100 + 0.30·100 + 0.20·85 = 97.0` ⇒ `score_confidence = |score| / 100 = 0.97` per the §2.2 mapping ✓
- `expected_reward_risk_ratio = L4.expected_rr_internal × (1 − L5.overall_risk / 100) = 2.5 × (1 − 0.283) = 2.5 × 0.717 = 1.79` (using `L4.expected_rr_internal = 2.5` from the Opportunity Matrix example and `L5.overall_risk.score = 28.3` on the canonical `[0, 100]` scale) ✓
- `entry_danger.score = mean(quality_penalty, 100 − opportunity_score) = mean(10, 100 − 85) = mean(10, 15) = 12.5` (with `market_quality = EXCELLENT` → `quality_penalty = 10` and `opportunity_score = 85`). Score `12.5` falls in the `VERY_LOW` band (`< 20` per the §3.8 half-open intervals) ✓
- `stop_loss_distance_pct = 1.5` is the f64 type-boundary handoff to TAE (see §5 Scenario Pathways / `01-02-global-architecture.md §6.3`); TAE casts to Decimal at the execution boundary.

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
