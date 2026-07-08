# IRCL — Institutional Regime Classification Layer

> **Layer 2 of 10 in the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: COMPLETE** — 5-regime classifier with 6 voting indicators implemented.

---

## Purpose

The Institutional Regime Classification Layer (IRCL) answers the strategic gating question:

> **What kind of market are we trading in right now?**

IRCL classifies every snapshot into one of five distinct market regimes. This is the single most important structural gating decision in the pipeline — the regime determines which strategies are permitted, which indicator groups receive amplified weight, what allocation tiers are allowed, and whether directional positions are even eligible.

A signal that is valid in a Trending regime may be a trap in a Range regime. IRCL prevents the system from applying the same logic in every environment.

**IRCL is purely classificatory. It gates, but never decides.**

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| ADX value + DI+/DI− | ITIL indicator map (key: `adx`) | NormalizedIndicatorValue |
| BBWP percentile | ITIL indicator map (key: `bbwp`) | NormalizedIndicatorValue |
| Squeeze state | ITIL indicator map (key: `squeeze`) | NormalizedIndicatorValue |
| Choppiness Index | ITIL indicator map (key: `choppiness`) | NormalizedIndicatorValue |
| ATR regime | ITIL indicator map (key: `atr`) | NormalizedIndicatorValue |
| Aroon oscillator | ITIL indicator map (key: `aroon`) | NormalizedIndicatorValue |
| MarketContext | `shared/src/market_context.rs` | ContextDimension per dimension |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| Regime label | Enum: `Trending \| Compression \| Expansion \| Range \| Transitional` | ICSL (weight multipliers), IEPL (allowed trade types, allocation tier), IASL (document context) |
| Regime confidence | f64 ∈ [0, 1] | IDCL (`regime_confidence` field), ICSL (gate multiplier) |
| Regime stability | f64 ∈ [0, 1] | ICSL (transitional penalty), IASL (uncertainty context) |
| Per-regime strategy gates | Table of permitted/prohibited actions | IEPL (entry protocol gating) |
| Transition warnings | Early-warning flags when 3+ indicators shift | ICSL (uncertainty multiplier), IASL (cautions section) |

---

## Sub-Components

---

### A. Five Regime Types

The market is classified into exactly one of five regimes at every evaluation cycle.

| Regime | Key Characteristics | Preferred Strategies | Prohibited Actions | Max Allocation |
|--------|--------------------|---------------------|-------------------|----------------|
| **Trending** | ADX > 25, BBWP expanding, price respecting EMA structure, consecutive HH/HL or LH/LL | Pullbacks, continuation breakouts, trend-following entries | Counter-trend entries prohibited | Maximum (100%) |
| **Compression** | BBWP < 20th percentile, Squeeze active, ATR contracting, low volatility | Breakout preparation, liquidity accumulation monitoring | No aggressive entries before breakout confirmation | Minimal (25%) |
| **Expansion** | BBWP rapidly increasing, ATR expanding, Squeeze recently released (>5 bar duration) | Momentum continuation, trend acceleration trades | Fading the expansion | Maximum (100%) |
| **Range** | ADX < 20, flat EMA structure (tangled), price oscillating between S/R boundaries | Mean reversion, S/R bounces | Trend-following trades prohibited | Reduced (50%) |
| **Transitional** | 3+ regime indicators shifting, low regime confidence (<0.40), recent regime change | Wait for stabilization, monitor breakout direction | All directional trades | Zero (0%) |

---

### B. Six Voting Indicators

IRCL uses a weighted vote across six indicator inputs. Each casts a directional vote for one regime; the regime with the highest weighted sum wins.

| Indicator | Weight | Rationale | Voting Logic |
|-----------|--------|-----------|-------------|
| **ADX** | 25% | Dedicated trend-strength indicator; 5× more weight than EMA because it measures trend directly | <20 → Range, 20-40 → Trending, >40 → Expansion (exhaustion) |
| **Choppiness** | 25% | Quantifies trend vs noise; equal weight to ADX as primary regime discriminator | >61.8 → Range, 38.2-61.8 → Transitional, <38.2 → Trending |
| **BBWP** | 15% | Compression/expansion percentile; leading indicator of volatility regime shifts | <20% → Compression, >90% → Expansion (climax), 20-90% → Normal (defers to other indicators) |
| **Squeeze** | 15% | TTM Squeeze state; coiling energy detection | ON + duration ≥ 5 → Compression, recently released (within 2 bars) → Expansion |
| **ATR** | 10% | Volatility expansion/contraction regime | Expanding → Trending/Expansion (context-dependent), Contracting → Compression, Stable → normal |
| **Aroon** | 10% | Trend emergence vs consolidation | AroonUp > 70 → Trending (bull), AroonDown > 70 → Trending (bear), both < 30 → Range |

**Vote aggregation formula:**
```
regime_confidence[regime] = Σ (weight_i × vote_i[regime]) / Σ (weight_i)
```

The regime with the highest `regime_confidence` is the active regime. The confidence value itself is bounded ∈ [0, 1].

---

### C. Regime Confidence

Regime confidence quantifies how strongly the voting indicators agree on the classified regime.

```
regime_confidence = Σ(weight_i × vote_match_i) / Σ(weight_i)
vote_match_i = 1.0 if indicator i's primary vote matches the winning regime, else 0.5 if secondary match, else 0.0
```

**Interpretation thresholds:**

| Confidence Range | Label | Action |
|-----------------|-------|--------|
| ≥ 0.85 | Strong consensus | Full confidence in regime classification. Standard gates apply. |
| 0.60 – 0.85 | Moderate agreement | Regime likely correct but indicators not unanimous. Slight caution. |
| 0.40 – 0.60 | Weak agreement | Indicators conflicted. Consider Transitional treatment. |
| < 0.40 | Transitional | 3+ indicators disagree. Market in flux. No directional trades. Override regime to Transitional. |

When `regime_confidence < 0.40`, the regime is forcibly classified as Transitional regardless of the vote winner, because the disagreement itself is the most important signal.

---

### D. Regime Stability

Regime stability measures how long the current regime has persisted and how likely it is to continue.

```
stability = min(bars_since_regime_change / min_stable_bars, 1.0)
min_stable_bars = 5 (configurable)
```

**Interpretation:**

| Stability | Meaning |
|-----------|---------|
| 0.0 – 0.3 | Just transitioned — high uncertainty, recent regime change within last 1-2 bars |
| 0.3 – 0.5 | Settling — regime establishing, some confidence building |
| 0.5 – 0.8 | Entrenched — regime has persisted, characteristic behavior reliable |
| 0.8 – 1.0 | Long-standing — deeply entrenched, transition increasingly probable (paradoxically) |

Low stability produces a multiplicative uncertainty penalty in ICSL: `stability_penalty = 0.5 + stability × 0.5`, which dampens confluence scoring until the regime stabilizes.

---

### E. Regime-Specific Strategy Gates

Each regime activates a specific set of strategy rules that propagate to downstream layers.

| Rule | Trending | Compression | Expansion | Range | Transitional |
|------|----------|-------------|-----------|-------|-------------|
| Directional entries allowed | Long & Short (with-trend) | None (breakout prep only) | Long & Short (momentum) | Long & Short (mean-reversion) | None |
| Max capital allocation | 100% of base | 25% of base | 100% of base | 50% of base | 0% |
| Trend group weight multiplier | ×1.5 | ×0.5 | ×1.3 | ×0.5 | ×1.0 |
| Momentum group weight multiplier | ×1.0 | ×0.8 | ×1.2 | ×1.5 | ×0.7 |
| Volume confirmation required | RVOL ≥ 1.0 | RVOL ≥ 1.5 | RVOL ≥ 1.2 | RVOL ≥ 1.0 | N/A |
| Stop ATR multiplier | 2.0× | 1.5× | 2.5× | 1.0× | N/A |
| TP target priority | 1.618 ext | Nearest S/R | 2.618 ext | POC/VWAP | N/A |
| Invalidation sensitivity | Normal | Loose (compression false-breaks) | Tight (rapid reversals) | Normal | N/A |

These gates are enforced by IEPL (execution) and ICSL (scoring weights).

---

### F. Regime Transition Detection

The transition detection system provides early warning before `regime_confidence` drops below 0.40.

**Detection logic:**
1. Track each voting indicator's primary regime assignment over the last 3 bars
2. If 3+ indicators have shifted their primary vote to a different regime within the window → flag `transition_warning = true`
3. If 4+ indicators have shifted → flag `imminent_transition = true`, override confidence dampening

**Example:**
```
Bar T-2: ADX→Trending, Choppiness→Trending, BBWP→Compression, Squeeze→Compression, ATR→Trending, Aroon→Trending
Bar T-1: ADX→Trending, Choppiness→Transitional, BBWP→Compression, Squeeze→Expansion, ATR→Trending, Aroon→Trending
Bar T-0: ADX→Transitional, Choppiness→Transitional, BBWP→Compression, Squeeze→Expansion, ATR→Expansion, Aroon→Trending
Transition: 4 indicators shifted (Choppiness, Squeeze, ATR, ADX) → imminent_transition = true
```

When `imminent_transition = true`:
- Regime confidence is halved
- All new position entries are blocked
- Existing positions get a tighter invalidation trigger
- The IASL Analyst Agent receives a `TRANSITION WARNING` flag

---

## Integration

### Feeds Into
- **ICSL (Layer 4)** — Regime-aware weight multipliers (F), transitional damping penalty
- **IDCL (Layer 5)** — `regime_confidence` field in the 17-metric computation
- **IEPL (Layer 9)** — Allowed trade types, max allocation tier, stop ATR multiplier, TP priority
- **IASL (Layer 8)** — Market regime context in Analyst Agent's `market_summary` section

### Receives From
- **ITIL (Layer 1)** — ADX, BBWP, Squeeze, Choppiness, ATR, Aroon indicator values

### Cross-References
- [ITIL: §B Trend Group](../layers/01-itil-technical-indicator.md) — ADX, EMA stack, PSAR, Ichimoku trend indicators
- [ITIL: §C Volatility Group](../layers/01-itil-technical-indicator.md) — BBWP, Squeeze, ATR, HV
- [ITIL: §F Regime Group](../layers/01-itil-technical-indicator.md) — Aroon, Choppiness, LinReg Slope, Z-Score
- [ICSL: §F Regime-Aware Weights](../layers/04-icsl-confluence-scoring.md) — How regime multipliers affect scoring
- [IEPL: §A Entry Protocol](../layers/09-iepl-execution-protocol.md) — Regime-gated entry conditions
