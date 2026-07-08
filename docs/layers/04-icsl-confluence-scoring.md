# ICSL — Institutional Confluence Scoring Layer

> **Layer 4 of 10 in the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: COMPLETE** — 44-contributor weighted scoring with 7 gates implemented.

---

## Purpose

The Institutional Confluence Scoring Layer (ICSL) answers the synthesis question:

> **How much agreement exists across all indicators, and what is the net directional conviction?**

ICSL is the first synthesis layer. It takes 51 individual indicator readings and 115 discrete signal events from ITIL, regime context from IRCL, and structural context from ISML, and collapses them into a single quantifiable directional conviction score on a continuous [-100, +100] scale. This score is the primary numerical input to the IDCL probability engine.

ICSL also produces the opposite score used for position invalidation, and generates the gate pass/fail checks that prevent false signals.

**ICSL synthesizes agreement. It quantifies conviction, not certainty.**

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| 44 directional indicators | ITIL indicator map | NormalizedIndicatorValue with normalized ∈ [-1, +1] |
| 7 non-directional gate indicators | ITIL indicator map | NormalizedIndicatorValue (raw value gating) |
| Signal events | ITIL signal stream | Vec<IndicatorSignal> per indicator |
| Regime classification | IRCL | Regime label + confidence + stability |
| Structural levels | ISML | Level map with proximity scores |
| Per-indicator weights | `ScoringConfig` (from `config.toml` + user settings) | weight ∈ [0, 2.0], enabled ∈ {true, false} |
| Regime weight multipliers | IRCL per-regime table | Group-level multipliers |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| Confluence score | f64 ∈ [-100, +100] | IDCL (`confluence` field), IASL (Analyst Document `confluence_summary`) |
| Opposite score | f64 ∈ [-100, +100] | IEPL (position invalidation trigger) |
| Gate pass/fail map | HashMap<String, bool> per gate | IDCL (risk computation), IASL (caveats) |
| Consensus percentage | f64 ∈ [0, 1] | IDCL (`consensus` field) |
| Redundancy score | f64 ∈ [0, 1] | Internal (penalty applied to confluence) |
| Multi-timeframe confirmation | Per-TF agreement matrix | IDCL, IASL |
| Signal agreement percentage | f64 ∈ [0, 1] | IASL (active_signals section) |

---

## Sub-Components

---

### A. Directional Scoring Model

The core of ICSL is a weighted sum of all directional indicator values, normalized by active weight.

**Formula:**
```
confluence = Σ(weight_i × normalized_i × enabled_i) / Σ(weight_i × enabled_i) × 100

where:
  weight_i   = indicator's configured weight from ScoringConfig (default 1.0)
  normalized_i = indicator's normalized value ∈ [-1.0, +1.0]
  enabled_i  = 1 if indicator enabled in ScoringConfig, 0 otherwise
```

Only the 44 directional indicators enter the sum. The 7 non-directional gates (ADX, Volume, RVOL, ATR, BBWP, HV, Choppiness) do NOT enter the sum — they instead produce multipliers that scale the final output (see §B).

**Interpretation:**
| Score Range | Label | Meaning |
|-------------|-------|---------|
| +75 to +100 | Extremely Bullish | Near-unanimous bullish alignment. Rare. |
| +40 to +75 | Strongly Bullish | Broad agreement. Standard long entries valid. |
| +15 to +40 | Moderately Bullish | Majority bullish with some dissent. |
| −15 to +15 | Neutral / Mixed | Indicators split. No clear edge. |
| −40 to −15 | Moderately Bearish | Majority bearish with some dissent. |
| −75 to −40 | Strongly Bearish | Broad agreement. Standard short entries valid. |
| −100 to −75 | Extremely Bearish | Near-unanimous bearish alignment. Rare. |

**Directional contributors by group (44 total):**
| Group | Count | Indicators |
|-------|-------|------------|
| Trend | 9 | EMA Stack, Supertrend, Donchian, Keltner, VWAP, Anchored VWAP, Ichimoku, PSAR, Hull MA |
| Momentum | 11 | RSI, RSI Divergence, Stochastic, Stoch Divergence, ChandeMO, CMO Divergence, Williams %R, AO, CCI, MACD, MACD Divergence |
| Volume | 8 | Volume Profile, OBV, OBV Divergence, CMF, CMF Divergence, MFI, MFI Divergence, Force Index |
| Volatility | 4 | Bollinger, Squeeze, Squeeze Divergence, StdDev Channel |
| Structure | 5 | Fibonacci, Support/Resistance, Pivot Points, Patterns, Candlestick |
| Regime | 3 | Aroon, LinReg Slope, Z-Score |
| Institutional | 4 | SMC Structure, SMC Liquidity, SMC FVG, SMC Order Blocks |

**Code location:** `crates/engine/src/profile_evaluation/scoring.rs::calculate_registry_confluence()`.

---

### B. Seven Non-Directional Gates

Each gate produces a multiplier ∈ [0, 1] based on current conditions. The multipliers are combined multiplicatively with the confluence score:

```
effective_confluence = confluence × ∏(gate_multiplier_i)
```

If any gate returns 0, the entire confluence is nullified (override to 0.0 — no directional edge).

| Gate | Indicator | Multiplier Logic |
|------|-----------|-----------------|
| **Trend Gate** | ADX | ADX < 20 → 0.50 (congestion), 20-25 → 0.75 (emerging), 25-40 → 1.00 (strong), >40 → 0.60 (exhaustion, fading trend) |
| **Volatility Gate** | ATR | Contracting → 0.80 (tight stops, low conviction), Stable → 1.00, Expanding → 1.00 (confirmed by volume if present) |
| **Compression Gate** | BBWP | <10% → 0.50 (coiling, no trade yet), 10-90% → 1.00, >90% → 0.40 (climax, reversal risk) |
| **Statistical Vol Gate** | HV | >100% annualized → 0.60 (extreme volatility, reduce conviction), 60-100% → 0.80 (elevated), 20-60% → 1.00, <20% → 0.90 (too quiet) |
| **Volume Magnitude Gate** | Volume | Below 20th percentile of average → 0.70 (low participation), else 1.00 |
| **Relative Volume Gate** | RVOL | <1.0 → 0.50 (consolidation, reject signals), 1.0-3.0 → 1.00 (normal), ≥3.0 → 0.30 (exhaustion climax) |
| **Noise Gate** | Choppiness | >61.8 → 0.40 (choppy, reject trend signals), 38.2-61.8 → 0.80 (transitional), <38.2 → 1.00 (trending, clean) |

**Gate application order:** All 7 gates are evaluated independently. The product of all multipliers is applied. This ensures that a single critical failure (e.g., RVOL < 1.0 on a breakout signal) can dramatically reduce the effective score even if all other gates pass.

---

### C. Volume Confirmation Protocol

Volume is the universal truth test for any breakout or signal. ICSL enforces a tiered volume confirmation system:

| RVOL Range | Regime | Action |
|-----------|--------|--------|
| < 1.0 | Consolidation | **Reject ALL breakout/trend signals.** Gate multiplier = 0.50 across all signal types. Low participation = high fakeout risk. |
| 1.0 – 1.5 | Normal | Standard entry execution. Normal allocation. |
| ≥ 1.5 | Institutional | **REQUIRED** to validate: S/R breakouts, Squeeze releases, MACD crossovers, Fibonacci level tests. Without RVOL ≥ 1.5 these signals are unconfirmed. |
| ≥ 3.0 | Exhaustion Climax | No new entries permitted. Tighten existing stops. Gate multiplier = 0.30. |

**Specific signal requirements:**
- S/R breakout candle with RVOL < 1.5 → **head fake** — rejected
- MACD crossover below zero requires RVOL ≥ 1.2 for validation
- Squeeze release trigger requires RVOL ≥ 1.5 for validation
- Chart pattern breakout requires RVOL ≥ 1.5 for confirmation
- Any signal with RVOL ≥ 3.0 → exhaustion flag raised regardless of signal quality

---

### D. Indicator Redundancy Detection

When many indicators are measuring essentially the same thing (e.g., RSI, Stoch, CCI all near overbought), the apparent agreement inflates confidence artificially. Redundancy is measured using effective rank:

```
redundancy = 1 − (effective_rank / active_directional_count)
effective_rank = number of statistically independent indicator clusters
```

**How it works:**
1. Pairwise correlations are computed across all directional indicators over a rolling window
2. Indicators with correlation > 0.70 are grouped into the same cluster
3. `effective_rank` = number of distinct clusters
4. If 15 indicators all fall into 3 clusters, redundancy = 1 − 3/15 = 0.80

**Penalty application:**
```
If redundancy > 0.70:
    penalty = 1.0 − (redundancy − 0.70) × 1.5
    confluence = confluence × max(penalty, 0.25)
```

This prevents the system from being overconfident when multiple oscillators are simply confirming the same underlying movement. High redundancy is common in strong trends (all momentum indicators bullish) — the penalty ensures conviction doesn't outrun reality.

---

### E. Multi-Timeframe Confirmation

Single-timeframe confluence is necessary but insufficient. ICSL verifies indicator agreement across multiple temporal resolutions.

**Timeframes evaluated:**
| Timeframe | Alias | Resolution | Role |
|-----------|-------|------------|------|
| Micro | 15s | Scalp | Entry precision, signal freshness |
| Fast | 1m | Micro | Signal confirmation, breakout validation |
| Slow | 5m | Primary | Core execution, confluence computation |
| Macro | 15m | Directional | Trend bias, structural alignment |
| Structural | 1h | Macro | Regime confirmation, EMA stack authority |

**Confirmation matrix:** For each indicator, compute the normalized value across all available timeframes. An indicator is "confirmed" when:
- 3+ timeframes agree on direction (sign matches)
- The macro (15m) timeframe direction aligns with the structural (1h) timeframe
- No timeframe shows a strong opposing signal (|normalized| > 0.7 in opposite direction)

**TF Alignment Score:**
```
tf_alignment = (aligned_indicators / total_indicators) × macro_agreement_bonus
macro_agreement_bonus = 1.0 if slow + macro + structural agree, else 0.8
```

This score feeds into the IDCL `consensus` computation and the IASL Analyst Document.

**Code location:** `GET /api/monitor` returns per-indicator agreement matrix across timeframes.

---

### F. Regime-Aware Weight Multipliers

The IRCL regime classification dynamically reweights indicator groups. This is the mechanism that makes the system adapt its analytical lens to the current market environment.

**Per-regime group multipliers:**

| Group | Trending | Compression | Expansion | Range | Transitional |
|-------|----------|-------------|-----------|-------|-------------|
| **Trend** | ×1.5 | ×0.5 | ×1.3 | ×0.5 | ×1.0 |
| **Momentum** | ×1.0 | ×0.8 | ×1.2 | ×1.5 | ×0.7 |
| **Volume** | ×1.1 | ×0.9 | ×1.2 | ×1.0 | ×0.8 |
| **Volatility** | ×0.9 | ×1.3 | ×1.3 | ×0.8 | ×1.0 |
| **Structure** | ×1.2 | ×0.7 | ×1.0 | ×1.2 | ×0.6 |
| **Regime** | ×1.0 | ×1.0 | ×1.0 | ×1.0 | ×1.0 |
| **Institutional** | ×1.0 | ×1.0 | ×1.0 | ×1.0 | ×0.5 |

**Rationale per regime:**
- **Trending:** Amplify trend-following indicators (EMA, ADX, Ichimoku), reduce mean-reversion oscillators
- **Compression:** Amplify volatility indicators (BBWP, Squeeze, ATR) to detect breakout, reduce trend (no trend exists)
- **Expansion:** Amplify momentum + volatility for continuation signals
- **Range:** Amplify oscillators for mean-reversion entries, heavily penalize trend
- **Transitional:** Reduce everything — conviction impossible during regime shifts

**Configuration:** These multipliers are stored in `ScoringConfig.regime_weight_multipliers` and are user-configurable via the Scoring Weights settings panel at `POST /api/config/scoring-weights`.

---

### G. Opposite Score Engine

The opposite score uses the exact same weighted-sum formula as the directional score but with all signs inverted. It answers: "If we were trading the opposite direction, how strong would the conviction be?"

```
opposite_score = Σ(weight_i × (−normalized_i) × enabled_i) / Σ(weight_i × enabled_i) × 100
```

This is mathematically equivalent to `−confluence` only when all weights and enabled states are symmetric. In practice, it can differ because:
- Non-directional gates apply differently (e.g., volume confirmation for breakouts)
- Some indicators have asymmetric weight configurations
- The regime-aware weight multipliers may favor one direction

**Usage for position invalidation:**
```
if opposite_score > REGISTRY_OPPOSITE_EXIT_THRESHOLD (default 60%):
    → trigger position invalidation
    → exit all active positions
```

The opposite score is the primary quantitative invalidation trigger. It replaces the old "5 opposite signals" rule with a weighted, continuous metric. A score > 60% means the opposing direction has stronger conviction than 60% of the indicator weight, which represents a decisive structural shift.

**Code location:** `crates/engine/src/profile_evaluation/scoring.rs::calculate_registry_opposite_score()`, `REGISTRY_OPPOSITE_EXIT_THRESHOLD`.

---

## Integration

### Feeds Into
- **IDCL (Layer 5)** — `confluence` field, `consensus` field, gate pass/fail contributing to `risk_level` and `trade_quality`
- **IEPL (Layer 9)** — Opposite score for position invalidation trigger, volume gate for entry validation
- **IASL (Layer 8)** — Confluence summary in Analyst Document, signal agreement context

### Receives From
- **ITIL (Layer 1)** — All 51 indicator normalized values + signal events
- **IRCL (Layer 2)** — Regime classification + confidence + stability (for weight multipliers)
- **ISML (Layer 3)** — Structural level proximity (for context on LevelTest signal strength)

### Cross-References
- [ITIL: §C Full Inventory](../layers/01-itil-technical-indicator.md) — Complete 51-entry table with signal counts
- [IRCL: §E Strategy Gates](../layers/02-ircl-regime-classification.md) — Per-regime allocation and confirmation rules
- [IDCL: §Confluence Metrics](../layers/05-idcl-decision-context.md) — How confluence feeds probability and consensus
- [IEPL: §E Position Invalidation](../layers/09-iepl-execution-protocol.md) — Opposite score exit trigger
