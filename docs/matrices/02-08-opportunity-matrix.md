# Opportunity Matrix Specification

**Version:** 6.10 (2026-08-13) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Producing Layer:** Layer 4 — Opportunity Layer
**Purpose:** This document defines the physical schema, scoring model, and setup-quality classification of the **Opportunity Matrix** — the strategy-agnostic profiling object. It identifies and scores favourable market configurations (breakout, continuation, pullback, mean-reversion, reversal) on a 0–100 scale, independent of any execution parameters.

---

## 1. Conceptual Definition

Per the [Ontology](../conceptual-foundations/01-01-ontology.md) §3.14, **Opportunity** represents the positive market potential present in the current conditions. The Opportunity Matrix evaluates whether favourable setups exist and scores their statistical viability — **without** committing to a direction of exposure, position size, or entry price. Those belong to the [Decision Matrix](02-04-decision-matrix.md) and the Trade Automation Engine.

The Opportunity Matrix consumes the [Analysis Matrix](02-02-analysis-matrix.md) (context) and the underlying [Metrics Matrix](02-07-metrics-matrix.md) signals (evidence), and emits one profiled opportunity per candidate setup type.

```
[Analysis Matrix] ─┐
                   ├──► OPPORTUNITY LAYER (L4) ──► [Opportunity Matrix]
[Metrics Matrix ]  ┘        (profile + score 0-100)
```

This is a **strategy-agnostic, direction-neutral** contract: it describes only the shape, quality, and precondition satisfaction of the opportunity. Directional bias, entry price, position sizing, and execution authorization are *not* the Opportunity Matrix's responsibility; those belong to the [Decision Matrix](02-04-decision-matrix.md) and the Trade Automation Engine.

---

## 2. Physical Schema

### 2.1 OpportunityMatrix Fields

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | `string` | Entity under analysis. |
| `primary_opportunity` | `OpportunityType` | The dominant setup classification. |
| `opportunity_score` | `f64` | Overall setup viability in `[0, 100]`. |
| `setup_quality` | `SetupQuality` | Categorical quality band (§5). |
| `profiles` | `OpportunityProfile[]` | Per-setup-type scored profiles (§3). |
| `forecast_confidence` | `f64` | Confidence in the profiling `[0, 1]`. *(Renamed from `confidence` in the institutional redesign; see [02-00b-confidence-hierarchy.md](02-00b-confidence-hierarchy.md).)* |
| `contributing_signals` | `string[]` | Signal labels supporting the primary opportunity. |
| `invalidation_note` | `string` | Condition that would nullify the opportunity. |
| `entry_zone` | `PriceRange` | Recommended entry band. *(Added in the institutional redesign — institutional quant field.)* |
| `target_zone` | `PriceRange` | Expected target band. *(Added in the institutional redesign.)* |
| `invalidation_level` | `Decimal` | Structural invalidation price (the price level whose breach nullifies the thesis). *(Added in the institutional redesign; the prior L4/Decision and Position Matrix spellings were unified to the canonical `invalidation_level` in v2.1 — retired names recorded in `docs/CHANGELOG.md`.)* |
| `long_expected_rr_internal` | `f64` | Per-direction R:R for a long setup, derived from `long_target_zone`, `long_entry_zone`, and `long_invalidation_level`. The active side is resolved by `analysis.bias`; the legacy matrix-level `expected_rr_internal` was removed in v6.9. |
| `short_expected_rr_internal` | `f64` | Per-direction R:R for a short setup, derived from `short_target_zone`, `short_entry_zone`, and `short_invalidation_level`. |
| `long_geometry_consistent` | `bool` | Server-side flag for the matrix-level LONG bracket (`true` when the §2.2.2 invariants hold). |
| `short_geometry_consistent` | `bool` | Server-side flag for the matrix-level SHORT bracket. |
| `time_horizon` | `TimeHorizon` | Expected holding period: `SCALP` / `INTRADAY` / `SWING` / `POSITION`. The `TimeHorizon` enum is the **canonical four-variant** holding-period classifier; every value is reachable from at least one `OpportunityType` (see §3 precondition table). *(Added in the institutional redesign; `SCALP` reachability added in v2.1)* |

#### 2.1.1 PriceRange

| Field | Type | Description |
|-------|------|-------------|
| `low` | `Decimal` | Lower price bound. |
| `high` | `Decimal` | Upper price bound. |

#### 2.1.2 TimeHorizon & Update Cadence (L6)

The `TimeHorizon` enum has four variants: `SCALP` (held for seconds to minutes), `INTRADAY` (held for minutes to hours), `SWING` (held for hours to days), and `POSITION` (held for days to weeks). Drives the cadence at which the Decision Layer's `exit_guidance` is updated. Cadence by TimeHorizon:

| `TimeHorizon` | Update cadence | Rationale |
|---------------|----------------|-----------|
| `SCALP` | Every completed sub-minute candle | Sub-minute setups re-evaluated at each completed candle on the configured sub-minute timeframe (e.g. every 15-second candle for a 15-second timeframe). |
| `INTRADAY` | Every completed candle | Hourly setups re-evaluated at each candle close. |
| `SWING` | Every 5 completed candles | Multi-day setups re-evaluated less frequently. |
| `POSITION` | Every 15 completed candles | Multi-week setups re-evaluated only on structural change. |

The cadence is implemented as a debounced scheduler on the L6 Decision Layer (see [03-02-07-mme-layer6-decision-support.md §4](../engines/market-monitoring-engine/03-02-07-mme-layer6-decision-support.md)), not as a wall-clock timer — every evaluation also re-runs when the upstream matrices change. The completed-cascade invariant ([01-03-systemic-data-flow.md §4.1 Immutability Guarantees](../conceptual-foundations/01-03-systemic-data-flow.md)) is preserved: only `is_completed = true` snapshots enter the L4/L5/L6 cascade. Raw `is_completed = false` shadow snapshots are for live UI display only.

### 2.2 OpportunityProfile

| Field | Type | Description |
|-------|------|-------------|
| `opportunity_type` | `OpportunityType` | Setup being profiled. |
| `score` | `f64` | Viability `[0, 100]` for this specific setup. |
| `preconditions_met` | `u32` | Count of satisfied preconditions. |
| `preconditions_total` | `u32` | Total preconditions evaluated. |
| `notes` | `string` | Human-readable profiling rationale. |
| `direction_family` | `DirectionFamily \| null` | Direction family this profile implies. Populated for every profile so the UI can resolve its trade side from the wire. |
| `long_entry_zone` | `PriceRange \| null` | LONG-side entry zone (entry below close). Populated only when the profile resolves to LONG. |
| `long_target_zone` | `PriceRange \| null` | LONG-side target zone (target above close). |
| `long_invalidation_level` | `f64 \| null` | LONG-side invalidation (price below which the long thesis is invalidated). |
| `short_entry_zone` | `PriceRange \| null` | SHORT-side entry zone (entry above close). Populated only when the profile resolves to SHORT. |
| `short_target_zone` | `PriceRange \| null` | SHORT-side target zone (target below close). |
| `short_invalidation_level` | `f64 \| null` | SHORT-side invalidation (price above which the short thesis is invalidated). |
| `long_expected_rr_internal` | `f64 \| null` | Per-side R:R derived from the per-profile zones (faithful sign-aware R:R; never the legacy `2.5` mask). |
| `short_expected_rr_internal` | `f64 \| null` | Same, for the SHORT side. |
| `long_geometry_consistent` | `bool` | Server-side flag: `true` when the LONG bracket satisfies the §2.2.2 invariants (`invalidation_level < entry_zone.low` AND `target_zone.low > entry_zone.high`). Defaults to `false` when zones are absent or geometry is inverted. |
| `short_geometry_consistent` | `bool` | Server-side flag for the SHORT bracket. |

#### 2.2.1 DirectionFamily

| Variant | Setup families | Side resolution |
|---------|----------------|------------------|
| `TREND_RIDING` | `TrendContinuation`, `Breakout`, `Pullback`, `Scalp`, `LiquiditySqueeze` | Resolves to LONG when `Analysis.bias ∈ {Bullish, StrongBullish}`, SHORT when `Bearish / StrongBearish`, NEUTRAL otherwise. |
| `COUNTER_TREND` | `MeanReversion`, `Reversal` | Resolves to the OPPOSITE of the macro bias (LONG when bearish, SHORT when bullish, NEUTRAL otherwise). |
| `NEUTRAL` | `NoClearOpportunity` | Carries no zones (the family is direction-neutral by definition). |

The mapping is total over all eight `OpportunityType` values. The frontend's `selectProfileSide(profile, macroBias)` resolves the family × bias combination to a concrete `LONG / SHORT / NEUTRAL` direction.

#### 2.2.2 Per-profile geometry invariants

For every populated per-side zone the L4 producer enforces the same per-side invariants the aggregated `OpportunityMatrix.long_* / short_*` fields use:

- `LONG`: `invalidation_level < entry_zone.low` AND `target_zone.low > entry_zone.high` AND `target_zone.low > 0` AND `target_zone.high > 0` AND `entry_zone.low > 0`.
- `SHORT`: `invalidation_level > entry_zone.high` AND `target_zone.high < entry_zone.low` AND `target_zone.low > 0` AND `target_zone.high > 0` AND `entry_zone.low > 0`.

The non-positive bound invariant (`low > 0`, `high > 0` for entry and target) was added in v6.10.x after observing `short_target_zone.low = 0` on BTC-USDT (Bitget) 2026-08-11: the `pivot_points` indicator emits `s1=s2=s3=r1=r2=r3=pivot=0.0` with `state_label: PIVOT_UNAVAILABLE` when its window has not yet filled, and the previous SHORT-target candidate filter (`v < close`) accepted those zeros — every `0 < close` is true. Those zero candidates propagated into `short_target_zone.low`, which the frontend surfaced verbatim as `$0–$X`. The Rust producer now (1) filters `v > 0.0` on every target-candidate push in `collect_candidate_levels` and (2) pins `short_target_zone.low` to a positive floor (`close − atr · 1.5`) in `derive_side_zones` as a defensive backstop. The frontend's `aggregateZones` also rejects zones with `target.low <= 0` as a third layer of defence.

If the confluent pick violates the invariant, the L4 producer falls back to the directional ATR-only bracket; if even the ATR fallback can't satisfy the invariant (e.g. fresh symbol with no historical candles), the profile emits `null` for every zone and the consumer falls back to the aggregated `long_* / short_*` fields.

The server-side `long_geometry_consistent` / `short_geometry_consistent` flags are computed from the same per-side `compute_side_rr_v2()` status that the `trade_viability` badge uses. The frontend prefers these flags over local re-computation.

**Geometry examples (correct):**

| Side | Entry zone | Target zone | Invalidation | Valid? |
|------|-----------|-------------|--------------|--------|
| LONG | $62,000–$62,500 (below close) | $64,000–$65,000 (above entry) | $61,500 (< entry.low) | Yes |
| SHORT | $66,000–$66,500 (above close) | $62,000–$63,000 (below entry) | $67,000 (> entry.high) | Yes |

**Geometry examples (inverted):**

| Side | Entry zone | Target zone | Problem |
|------|-----------|-------------|---------|
| SHORT | $62,000–$62,500 (below close) | $64,000–$65,000 (above close) | Entry below target — would require price to rise for profit |
| LONG | $66,000–$66,500 (above close) | $62,000–$63,000 (below close) | Entry above target — would require price to fall for profit |

### 2.3 Cross-panel consistency contract

Both `OpportunitiesPanel` (Market Monitoring → Opportunities) and `RecommendationPanel` (Market Monitoring → Recommendation) read from the same wire payload (`OpportunityMatrix.profiles`) and call the same helper functions (`selectProfileSide`, `profileZones`) so their numbers always agree:

- The Opportunities panel renders **one actionable card per qualifying profile** (the leaderboard).
- The Recommendation panel renders **only the highest-scored qualifying profile** as the operator's actionable decision.
- The Trade Setups cards' entry/target/SL/R:R on the Opportunities panel match the Top Setup card's per-profile zones on the Recommendation panel for the same profile.

---

## 3. Opportunity Types & Preconditions

The `OpportunityType` enum is the **canonical home** of the setup selector (in the institutional redesign, this enum was removed from the Analysis Matrix and moved here, where it belongs as a forecast field). **Eight** values — the original six, plus `LiquiditySqueeze` added in the Phase 0-4 Liquidity Intelligence extension ([01-05-liquidity-domain.md §Decision integration](../conceptual-foundations/01-05-liquidity-domain.md), [03-02-11-mme-liquidity-extension.md §Decision integration](../engines/market-monitoring-engine/03-02-11-mme-liquidity-extension.md)) and `Scalp` added in the v2.1 institutional completeness sweep to make all four `TimeHorizon` values reachable from the setup selector:

| OpportunityType | Precondition Signature | Default `time_horizon` |
|-----------------|------------------------|------------------------|
| `TrendContinuation` | Strong/healthy trend (dim ≥ 75) + directional bias + momentum not exhausted. | `SWING` |
| `Breakout` | Volatility expansion (dim ≥ 70) + healthy structure (dim ≥ 60) + compression release or level breach. | `INTRADAY` |
| `Pullback` | Established trend (dim ≥ 60) + weakening momentum + price retracing toward a dynamic level. | `SWING` |
| `MeanReversion` | Volatility compression (dim ≤ 30) + range regime + oscillator extreme. | `INTRADAY` |
| `Reversal` | Confirmed divergence + structure break + momentum reversing. | `POSITION` |
| `LiquiditySqueeze` | Force-liquidation cascade is imminent or in progress. Reads L1.5 `LiquidityFlow.cascade_state ∈ {Detected, Sustained}` AND `LiquidationClusterMatrix.cascade_asymmetry` has `|asymmetry| > 0.3` (cluster forward-pressure present). Regime context must be `EXPANSION` or `TRANSITION` (not a flat range). Maps to a defensive opportunity — the platform tracks the cascade flow and triggers reduce-only / protective-tightening policies. | `INTRADAY` |
| `Scalp` | High per-candle volatility (BBWP ∈ [70, 95)) + tight structural context (alignment dimension 4 `Structure` ≥ 70) + directional bias (BULLISH / STRONG_BULLISH / BEARISH / STRONG_BEARISH) + regime ∈ {TRENDING_BULL, TRENDING_BEAR} (intraday-trending context, not swing). Designed for sub-minute-to-seconds holding periods, complementary to `Breakout` (which targets multi-bar continuation) and `TrendContinuation` (which targets multi-day). Every `Scalp` setup maps to `time_horizon = SCALP`, making the SCALP variant of `TimeHorizon` reachable from at least one `OpportunityType`. | `SCALP` |
| `NoClearOpportunity` | Tradability dimension < 30 or conflicting evidence (and no `LiquiditySqueeze` precondition active). | `INTRADAY` |

> **Horizon remapping.** `LiquiditySqueeze` defaults to `INTRADAY` (a defensive liquidation-cascade play cannot be a weeks-long hold) and `Reversal` to `POSITION` (divergence-driven reversals play out over weeks). Reachability across the `TimeHorizon` enum is preserved: `SCALP` ← `Scalp`, `INTRADAY` ← `Breakout` / `MeanReversion` / `LiquiditySqueeze` / `NoClearOpportunity`, `SWING` ← `TrendContinuation` / `Pullback`, `POSITION` ← `Reversal`. The §2.1.2 cadence table is keyed by `TimeHorizon` (not by `OpportunityType`) and is unaffected.

---

## 4. Setup-Selection Rule

The Opportunity Layer applies the following decision tree (first match in the listed order: 0, 0.5, 1, 1b, 2–7) to derive `primary_opportunity`. This rule was formerly located in [02-02-analysis-matrix.md §4.3](02-02-analysis-matrix.md); it has been moved here as part of the institutional redesign because setup selection is a forecast, not a state interpretation.

```
# Priority order (first match wins):
0. cascade_state ∈ {Detected, Sustained} AND |cascade_asymmetry| > 0.3 AND regime ∈ {EXPANSION, TRANSITION}  → LIQUIDITY_SQUEEZE
0.5. BBWP ∈ [70, 95) AND structure_align ≥ 70 AND bias ∈ {BULLISH, STRONG_BULLISH, BEARISH, STRONG_BEARISH} AND regime ∈ {TRENDING_BULL, TRENDING_BEAR}  → SCALP
1. trend ≥ 75 AND (bias == BULLISH OR bias == STRONG_BULLISH) AND momentum_assessment NOT IN {EXHAUSTED, REVERSING}  → TREND_CONTINUATION
1b. trend ≥ 75 AND (bias == BEARISH OR bias == STRONG_BEARISH) AND momentum_assessment NOT IN {EXHAUSTED, REVERSING}  → TREND_CONTINUATION (bearish continuation)
2. volatility ≥ 70 AND structure ≥ 60                              → BREAKOUT
3. confirmed_divergence AND structure_broken AND momentum_exhausted → REVERSAL
4. trend ≥ 60 AND momentum weakening                              → PULLBACK
5. volatility ≤ 30                                                → MEAN_REVERSION
6. tradability_dim < 30                                           → NO_CLEAR_OPPORTUNITY
7. otherwise (default)                                             → TREND_CONTINUATION
```

Where `confirmed_divergence` is true when at least one `Divergence` indicator signal has reached `status = CONFIRMED` ([Metrics Matrix §4.2](02-07-metrics-matrix.md)), `structure_broken` is true when Alignment Matrix dimension 4 (`Structure`) score is below 40, `momentum_exhausted` is true when Alignment Matrix dimension 1 (`Momentum`) score is below 25, and `structure_align` is the same dimension 4 score interpreted as "tight structural context favorable for a sub-minute scalp". `BBWP` is the `bbwp` indicator's raw percentile output on the local timeframe (L1 Metrics, `[0, 100]`) — **not** `MarketContext.volatility.score` (a signed `[-1, 1]` dimension). All **eight** values of `OpportunityType` (including `LiquiditySqueeze` and `Scalp`) are reachable via the explicit branches; the `ELSE` (priority 7) is a defensive default that may also resolve to `TREND_CONTINUATION`.

> **Direction-neutrality (v2.1).** Rule 1 previously read `trend ≥ 75 AND bias bullish` which violated the direction-neutral contract of the Opportunity Matrix (a strong bearish trend would not match and would fall through to the default). The corrected rule is symmetric: it accepts both `BULLISH`/`STRONG_BULLISH` and `BEARISH`/`STRONG_BEARISH` bias and produces a directional `TREND_CONTINUATION` either way. The Decision Matrix owns the actual long/short decision.
>
> **`tradability_dim` (v2.1).** Rule 6 was previously `opportunity_dim < 30`. The Alignment Matrix dimension 9 was renamed from `opportunity_dim` to `tradability_dim` in the institutional redesign to disambiguate from the L4 Opportunity Matrix (L4 owns opportunity concepts; dimension 9 measures TFs agreeing on tradability).

The resulting `opportunity_score` is bucketed into the categorical `setup_quality` bands (`Prime` / `Strong` / `Moderate` / `Marginal` / `None`) — the canonical band table is defined in §5 below.

---

## 5. Setup-Quality Classification

The categorical `setup_quality` buckets the `opportunity_score`. This section is the **canonical home** of the band table (§4 keeps only a pointer here). The bands are **lower-inclusive half-open intervals** `[a, b)`, so each score maps to exactly one band:

| SetupQuality | `opportunity_score` | Interpretation |
|--------------|---------------------|----------------|
| `Prime` | `[85, 100]` | High-conviction configuration, all key preconditions met. |
| `Strong` | `[70, 85)` | Robust setup with minor gaps. |
| `Moderate` | `[50, 70)` | Tradable but requires confirmation. |
| `Marginal` | `[30, 50)` | Weak edge; confluence-only. |
| `None` | `< 30` | No actionable opportunity. |

> **Tiling note.** The bands tile `[0, 100]` with no gap and no overlap: `< 30` ∪ `[30, 50)` ∪ `[50, 70)` ∪ `[70, 85)` ∪ `[85, 100]` covers every score, and each boundary value belongs to exactly one band (`85.0 → Prime`, `70.0 → Strong`, `50.0 → Moderate`, `30.0 → Marginal`). The §7 example's `opportunity_score = 85.0` therefore maps to `Prime`.

---

## 6. Scoring Model

The `opportunity_score` for a candidate setup blends four factors, each normalized to `[0, 100]`:

$$\text{score} = 0.35\,Q_{ctx} + 0.30\,S_{sig} + 0.20\,A_{mtf} + 0.15\,F_{fresh}$$

| Factor | Symbol | Source |
|--------|--------|--------|
| Context quality | `Q_ctx` | Analysis `market_quality` + relevant assessment dimension. |
| Signal support | `S_sig` | Strength and confirmation status of contributing Metrics-Matrix signals. |
| MTF agreement | `A_mtf` | Alignment `trend_agreement_pct` for directional setups. |
| Freshness | `F_{fresh}` | Inverse of the youngest contributing signal's `age_bars`. |

**Activation vs viability.** The score above is the **raw viability blend** — it tells the operator *how favourable the underlying setup looks*, independent of whether the setup is currently firing. It is **not** gated by the precondition completion ratio. The previous v6.10 implementation multiplied the score by `preconditions_met / preconditions_total`, which collapsed every inactive setup (e.g. `preconditions 0/3 met`) to `score = 0`. That conflated two orthogonal signals: activation (handled separately) and viability (which the dashboard displays). The v6.10.1 fix returns the raw blend so every non-`NoClear` profile surfaces its true viability; the activation signal is communicated via the per-profile `preconditions_met` / `preconditions_total` fields on `OpportunityProfile` (rendered as the precondition progress bar in the UI: `ui/src/components/OpportunitiesPanel.svelte:430-437`) and via the Rust-only `scoring_factors.precondition_ratio` field for telemetry consumers. `OpportunityType::NoClearOpportunity` retains the unconditional-zero sentinel — it is the explicit "no setup detected" placeholder and can never surface as actionable.

The primary opportunity is determined by the **priority-ordered decision tree in §4** (first match wins). The `opportunity_score` and `profiles[]` array expose the full scoring breakdown for downstream consumers but do **not** override the tree selection. In a tie, the profile with the higher `preconditions_met / preconditions_total` ratio wins.

---

## 7. JSON Serialization Contract

A representative Opportunity Matrix frame. The values derive from the canonical scenario chain (seed: [02-01-alignment-matrix.md §6](02-01-alignment-matrix.md); analysis inputs from [02-02-analysis-matrix.md §5](02-02-analysis-matrix.md)). The example illustrates the JSON shape; the canonical scoring formula is in §6.

```json
{
  "symbol": "BTC-USDT",
  "primary_opportunity": "TREND_CONTINUATION",
  "opportunity_score": 85.0,
  "setup_quality": "PRIME",
  "forecast_confidence": 0.81,
  "profiles": [
    { "opportunity_type": "TREND_CONTINUATION", "score": 85.0,
      "preconditions_met": 3, "preconditions_total": 3,
      "notes": "Trend 78 ≥ 75, bias BULLISH, momentum STABLE not in {EXHAUSTED, REVERSING} — §4 tree rule 1 fires first." },
    { "opportunity_type": "BREAKOUT", "score": 78.0,
      "preconditions_met": 3, "preconditions_total": 3,
      "notes": "Volatility 75 ≥ 70 and structure 65 ≥ 60, but loses §4 tree priority to trend continuation (rule 1 matched first)." }
  ],
  "contributing_signals": ["ema_stack:BULLISH_STACK", "macd:BULLISH_CROSSOVER"],
  "invalidation_note": "A close below 63440.0 — under the most recent higher-low structure — invalidates the trend-continuation thesis.",
  "entry_zone":  { "low": 64000.0, "high": 64200.0 },
  "target_zone": { "low": 65500.0, "high": 66000.0 },
  "invalidation_level": 63440.0,
  "long_expected_rr_internal": 2.5,
  "short_expected_rr_internal": 0.0,
  "time_horizon": "SWING"
}
```

> **Worked-example consistency.** `long_expected_rr_internal = (target_mid − entry_mid) / (entry_mid − invalidation_level) = (65750 − 64100) / (64100 − 63440) = 1650 / 660 = 2.5`. `setup_quality = PRIME` because `opportunity_score = 85.0 ∈ [85, 100]` per the §5 canonical bands. `time_horizon = SWING` matches the §3 default for `TrendContinuation`. The active-side R:R is resolved by `analysis.bias`: bullish → `long_expected_rr_internal`, bearish → `short_expected_rr_internal`, Neutral → 0. The legacy matrix-level `expected_rr_internal` was removed in v6.9.

Enum values serialize as `SCREAMING_SNAKE_CASE`.

> **Serialization note.** Across the platform, two surface forms appear:
> - **Wire JSON** and **policy conditions** (e.g. `opportunity.primary_opportunity IN ["TREND_CONTINUATION", "BREAKOUT"]` per [03-03-04-tae-execution-policy-spec.md §3.1](../engines/trade-automation-engine/03-03-04-tae-execution-policy-spec.md)): the variant is the SCREAMING_SNAKE_CASE string (`"BREAKOUT"`, `"TREND_CONTINUATION"`, `"LIQUIDITY_SQUEEZE"`, …).
> - **Rust internals** (enum variants in Rust code; the prose passages that document producer logic): the variant is PascalCase (`Breakout`, `TrendContinuation`, `LiquiditySqueeze`, …).
>
> The two forms refer to the same set of values; the policy author always types the SCREAMING_SNAKE_CASE string on the wire, and the Rust code translates between the two at the serde boundary. `TimeHorizon` follows the same rule (`INTRADAY`/`SWING`/`POSITION` on the wire, `Intraday`/`Swing`/`Position` in Rust).

---

## 8. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Direction-neutral scoring** | The score reflects setup *viability*, not profit expectation. |
| **Strategy-agnostic** | No strategy assumptions (scalping, swing, arbitrage) leak into the profiling. |
| **Explainability** | Every score decomposes into its four weighted factors and precondition fractions. |
| **Bounded** | `opportunity_score` and all profile scores clamp to `[0, 100]`. |
| **Canonical OpportunityType** | This matrix is the **only** producer of `OpportunityType`. The Analysis Matrix's former `opportunity_analysis` field has been removed (see [02-00-matrix-field-ownership.md](02-00-matrix-field-ownership.md)). |

---

## 9. Cross-References

- [Analysis Matrix](02-02-analysis-matrix.md) — Context input (`bias`, `market_quality`, `state_confidence`, qualitative assessments).
- [Risk Matrix](02-11-risk-matrix.md) — Parallel directional-neutral counterpart (danger).
- [Decision Matrix](02-04-decision-matrix.md) — Only synthesis point: combines opportunity + risk + state into trade readiness.
- [02-00-matrix-field-ownership.md](02-00-matrix-field-ownership.md) — Canonical producer-layer mapping.
- [MME Layer 4 — Opportunity](../engines/market-monitoring-engine/03-02-05-mme-layer4-opportunity.md) — Producing-layer specification.
- [Ontology — Opportunity](../conceptual-foundations/01-01-ontology.md) — Conceptual definition.
