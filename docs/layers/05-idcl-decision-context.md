# IDCL — Institutional Decision Context Layer

> **Layer 5 of 10 in the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: COMPLETE** — 17 metrics, zero new indicators.
>
> **Purpose:** Compute 17 probabilistic, consensus, range, volatility, risk, quality, and decision-readiness metrics from the 51-indicator normalized map. Introduces zero new indicators, zero new calculators, and zero state — every field is derived deterministically from the current snapshot.
>
> **Inputs:** ITIL (51 normalized indicator values), IRCL (regime confidence), ISML (level map, stop hierarchy), ICSL (confluence score, consensus fraction, gate results).
>
> **Outputs:** 17 quantitative decision-support metrics → ISIL (statistical enrichment), IRML (risk computation inputs), IASL (Analyst Agent context).

## 1. Concept

`DecisionContext` is a **read-only quantitative layer** that computes 17 probabilistic, consensus, range, volatility, risk, quality, and decision-readiness metrics from the existing 51-indicator normalized map. It introduces **zero new indicators, zero new calculators, and zero state** — every field is derived deterministically from the current snapshot.

It transforms raw indicator readings into structured decision-support metrics designed for AI consumption and institutional trade planning.

## 2. Fields — Complete Reference (17 fields)

### Directional (3 fields)

| Field | Range | Formula |
|-------|-------|---------|
| `bullish_probability` | [0,1] | Σ(w × n × c) for n>0 ÷ total (weighted vote) |
| `bearish_probability` | [0,1] | 1 − P(bullish) |
| `directional_bias` | [−1,+1] | (bull_votes − bear_votes) ÷ total |

Each directional indicator casts a vote. The vote weight is `|normalized| × confidence × registry_weight`. An indicator with `normalized=+0.85, confidence=0.90` casts a strong bullish vote. One with `normalized=−0.12, confidence=0.08` casts a weak bearish vote. Non-directional gates are excluded.

### Consensus (1 field)

| Field | Range | Meaning |
|-------|-------|---------|
| `consensus` | [0,1] | Fraction of directional indicators agreeing on the dominant side |

- 0.92 = 92% of indicators agree → strong conviction, trend is clear
- 0.51–0.60 = nearly split → high uncertainty, likely range-bound
- 0.30–0.50 = extreme fragmentation → avoid position-taking

### Expected Range (3 fields)

| Field | Unit | Formula |
|-------|------|---------|
| `expected_range_1bar` | % of price | ATR/price × regime_factor × √1 |
| `expected_range_5bar` | % of price | ATR/price × regime_factor × √5 |
| `expected_range_20bar` | % of price | ATR/price × regime_factor × √20 |

The regime factor adjusts for market conditions: trending (choppiness ≤ 38.2) → 1.3× (wider swings), choppy (≥ 61.8) → 0.6× (narrower swings), otherwise 1.0. The √N scaling follows random-walk assumptions. Use 1-bar for immediate noise, 5-bar for swing trades, 20-bar for position trades.

### Expected Volatility (1 field)

| Field | Unit | Formula |
|-------|------|---------|
| `expected_volatility` | annualized σ | HV × coil_factor × atr_factor |

The coil factor is forward-looking: Squeeze coiling → 1.5× (impending expansion), BBWP > 95 → 1.3× (extreme compression), BBWP < 10 → 0.9× (already expanded). The ATR factor confirms: expanding → 1.2×, contracting → 0.8×.

### Confluence (1 field)

| Field | Range | Formula |
|-------|-------|---------|
| `confluence` | [−100,+100] | Registry-weighted directional mean across all 44 directional indicators |

Carried from the existing scoring engine. Positive = bullish; negative = bearish; near 0 = neutral.

### Risk & Reward (3 fields)

| Field | Range | Formula |
|-------|-------|---------|
| `risk_level` | [0,1] | 0.25×vol_risk + 0.20×dis_risk + 0.15×ex_risk + 0.15×unc_risk + 0.15×trend_instability + 0.10×liq_risk |
| `reward_risk_ratio` | [0,∞) | target_dist ÷ stop_dist, using priority-ranked institutional levels |
| `recommended_stop` | price | Hierarchical fallback: institutional OB → swing level → VWAP → Volume Profile → pivot → ATR × 2 (never returns None) |

**Risk sub-factors:** `vol_risk` (HV + ATR proximity), `dis_risk` (1 − consensus), `ex_risk` (BBWP > 95 + RVOL ≥ 3.0), `unc_risk` (choppiness ≥ 61.8), `trend_instability` (ADX < 20), `liq_risk` (squeeze coiling).

**Risk interpretation:** 0.0 = Very Low, 0.25 = Low, 0.50 = Moderate, 0.75 = High, 1.0 = Extreme.

**Stop hierarchy (bullish direction example):**
1. Bullish Order Block low (if active and below price)
2. Recent swing low (from pivot points S1)
3. VWAP level (if below price)
4. Volume Profile VAL
5. Pivot S1
6. Fallback: price − 2.0 × ATR (always returns a value)

### Quality (2 fields)

| Field | Range | Meaning |
|-------|-------|---------|
| `trade_quality` | [0,1] | **Direction-aware** — evaluates setup quality in the direction of `directional_bias`. 7 positive factors + 1 contradiction penalty. |
| `market_quality` | [0,1] | **Regime-agnostic** — "is this market clean enough to trade at all?" 6 structural factors. |

**Trade quality positive factors (7):** confluence_score, probability_score, trend_score (ADX), volume_score (RVOL), cleanliness_score (choppiness), consensus_score, confirmation_score (OBV/FI alignment).

**Contradiction penalty:** When SMC CHoCH, liquidity sweeps, or MACD divergence signals oppose the `directional_bias`, `trade_quality` is **halved (×0.5)**. A bullish probability of 84% with a bearish CHoCH is NOT a high-quality bullish trade.

**Market quality factors (6):** trend quality (ADX), cleanliness (choppiness), volatility quality (BBWP), structure quality (EMA alignment), regime quality (Aroon), pattern presence (candlestick).

**Key distinction:** A market can have `market_quality = 0.95` (excellent, clean trend) with `trade_quality = 0.22` (no setup yet). Conversely, `market_quality = 0.35` with `trade_quality = 0.82` should be rare — the engine naturally discourages this.

### Regime & Trend (2 fields)

| Field | Range | Formula |
|-------|-------|---------|
| `regime_confidence` | [0,1] | Weighted agreement of 6 regime indicators per direction |
| `trend_persistence` | [0,1] | Confirmations count ÷ 9 (trend continuation checklist) |

**Regime confidence weights:** ADX 25%, Choppiness 25%, Ichimoku 20%, Aroon 15%, Supertrend 10%, EMA Stack 5%. ADX is weighted 5× more than EMA because it is a dedicated regime indicator. All 6 cast a directional vote (bullish or bearish); the dominant direction's weighted sum is the confidence.

**Trend persistence confirmations (9 checklist items):**
1. ADX rising (slope > 0)
2. EMA stack aligned (|normalized| > 0.5)
3. MACD positive/negative matching trend (|normalized| > 0.3)
4. OBV confirming flow (|normalized| > 0.2)
5. Aroon strong (|normalized| > 0.5)
6. Supertrend strong (|normalized| > 0.6)
7. Volume Profile breakout (label contains "BREAKOUT")
8. No RSI divergence present (|rsi_divergence| < 0.4)
9. No MACD divergence present (|macd_divergence| < 0.4)
10. No SMC CHoCH (choch_bullish/bearish both false)

### Synthesis (1 field)

| Field | Range | Thresholds | Formula |
|-------|-------|-----------|---------|
| `trade_readiness` | [0,1] | WAIT: 0–0.25, PREPARE: 0.25–0.50, READY: 0.50–0.75, ACT: 0.75–1.0 | 0.30×trade_quality + 0.25×(1−risk_level) + 0.20×market_quality + 0.10×regime_confidence + 0.15×trend_persistence |

This is the **single metric for "Should I act now?"** It synthesizes all quality, risk, regime, and persistence information into one actionable score. All 16 sub-components remain independently accessible for diagnostic explanation.

---

## 3. Usage Guidelines for AI

### Probability & Consensus
- **P(bullish) > 0.80 AND Consensus > 0.85:** High-conviction bullish. Standard position sizing.
- **P(bullish) > 0.80 AND Consensus < 0.55:** Bullish but fragmented — indicators are conflicted. Reduce size or wait.
- **P(bullish) ≈ 0.50:** No directional edge. Expected ranges still valid for neutral strategies.
- **Consensus < 0.50:** Avoid directional positions. The market is split — wait for alignment.

### Risk & Stop
- **Risk > 0.70:** Do not open new positions. Reduce existing exposure immediately.
- **Risk < 0.25:** Low-risk environment. Standard sizing acceptable.
- **Stop distance:** Use `recommended_stop` as a context-aware stop level. It prioritizes institutional levels (order blocks) over statistical levels (ATR). When an order block is nearby and defensible, the stop is tighter and more precise.

### Quality & Readiness
- **Trade Readiness > 0.75:** High-conviction signal. All systems aligned — risk is low, quality is high, trend is persistent, regime is confident. ACT.
- **Trade Readiness < 0.25:** WAIT. No action should be taken regardless of confluence.
- **Market Quality < 0.30 AND Trade Quality > 0.70:** Rare — should be naturally discouraged by the formulas. If it occurs, investigate the contradiction.
- **Contradiction penalty active:** The engine has detected structural or divergence signals opposing the `directional_bias`. Trade quality is halved. This is a strong warning — the oscillator momentum and market structure disagree.

### Trend & Volatility
- **Trend persistence < 0.3:** Trend is weakening. Consider reducing or exiting trend-following positions.
- **Expected volatility > 2× HV:** Regime change imminent (squeeze/BBWP signal). Tighten stops and reduce sizing.
- **Regime confidence < 0.4:** The market is in a transitional state. Avoid directional bets until regime stabilizes.

### Reward/Risk
- **reward_risk_ratio ≥ 3.0:** Asymmetric opportunity — target is 3× further than stop.
- **reward_risk_ratio < 1.0:** Stop is wider than target — unfavorable asymmetry. Avoid this setup.

---

## 4. Computation

`DecisionContext::compute(map, price, atr_value, confluence)` is a pure function called after `build_indicator_map()` on every completed candle. It reads the 51-entry normalized indicator map, the registry manifest, SMC institutional zone data from the indicator values sub-maps, and context labels from state_label strings. All 17 fields are stateless and reproducible — the same input always produces the same output.

The compute function is ~250 lines of Rust with no external dependencies beyond the existing indicator map and registry.

---

## 5. Transport

The `DecisionContext` is attached to every `MarketSnapshot` as `decision_context: Option<DecisionContext>`. It auto-serializes through the JSON blob → available via:
- `/api/history` (REST)
- WebSocket broadcast (per snapshot)
- DB persistence (auxiliary JSON blob)
- AI prompt context (via orchestrator prompt section describing all fields with usage guidelines)

---

## 6. Integration

No new indicators, registry entries, signal types, or frontend components. The layer is purely additive — it reads existing data and enriches it with structured quantitative metrics.

The AI orchestrator prompt includes a `DECISION SUPPORT` section that explains each field, its range, and how to interpret it for trade decisions.

---

## 7. Verification

**10 property tests verify:**

| Test | Verifies |
|------|----------|
| `test_unanimous_bullish` | P(bullish) > 0.95, Consensus > 0.9 when all indicators bullish |
| `test_unanimous_bearish` | P(bearish) > 0.95 when all indicators bearish |
| `test_split_vote_near_50` | P ≈ 0.5, Consensus < 0.6 when indicators evenly split |
| `test_range_scales_with_sqrt_n` | Expected ranges scale ×√N |
| `test_coil_boosts_expected_volatility` | Squeeze coiling → expected vol ≥ 1.5× HV |
| `test_risk_low_in_calm_trend` | risk_level < 0.30 in calm trending markets |
| `test_risk_high_in_chaos` | risk_level > 0.65 in chaotic conditions |
| `test_contradiction_penalty_reduces_trade_quality` | CHoCH + sweep opposing direction → trade_quality significantly lower |
| `test_stop_always_returns_value` | recommended_stop always > 0 and < price |
| `test_trade_readiness_synthesis` | trade_readiness ∈ [0,1] with reasonable midline values |

All tests pass. No regressions in the full 51-indicator test suite.
