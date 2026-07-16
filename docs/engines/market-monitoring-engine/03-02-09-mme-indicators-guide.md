# MME Indicators Guide — Readable Technical Rulebook

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Purpose:** This is the human-readable rulebook for the platform's technical indicators. It condenses the interpretation rules, thresholds, and scoring behaviour of every indicator group into a single reference. For the exact per-indicator mathematics and signal tables, see the individual specifications in [indicators/](indicators/04-02-00-indicator-index.md).

> This guide is served to consumers via the `GET /api/rules` endpoint and is the readable companion to the authoritative registry in `crates/shared/src/indicators/registry.rs`.

---

## 1. How to Read an Indicator

Every indicator is projected across 8 Evaluation Axes (see [Ontology](../../conceptual-foundations/01-01-ontology.md)). Practically, each indicator emits:

- **`raw_value`** — the native reading (e.g. `RSI = 68.4`).
- **`normalized ∈ [-1, 1]`** — the platform's unified score (bullish +, bearish −).
- **`state_label`** — a qualitative bucket (e.g. `OVERBOUGHT_DISTRIBUTION`).
- **`signals[]`** — discrete events fired this bar.

**Directional** indicators contribute a signed score to confluence. **Non-directional gates** (ADX, Volume, RVOL, ATR, BBWP, HV, Choppiness, Funding, Spread, Open Interest) do not vote on direction — they modulate confidence.

---

## 2. Functional Groups

### 2.1 Trend (10)
EMA Ribbon, Supertrend, Donchian, Keltner, ADX (gate), VWAP, Anchored VWAP, Ichimoku, PSAR, Hull MA. Trend indicators establish directional structure. Interpret with the regime: in `TRENDING` they lead; in `RANGE` they whipsaw.

### 2.2 Momentum (7)
RSI, Stochastic, ChandeMO, Williams %R, Awesome Oscillator, CCI, MACD (+ divergence companions). Momentum indicators measure the *rate* of change and are the primary source of divergence signals. Overbought/oversold thresholds tighten in strong trends.

### 2.3 Volume (7)
Volume (gate), RVOL (gate), Volume Profile, OBV, CMF, MFI, Force Index. Volume confirms or contradicts price. Rising price on falling volume is a quality warning (feeds signal risk).

### 2.4 Volatility (6)
ATR (gate), Bollinger, BBWP (gate), TTM Squeeze, HV (gate), StdDev Channel. Volatility indicators set the compression/expansion regime and inform dynamic stop distance.

### 2.5 Structure (5)
Fibonacci, Support/Resistance, Pivot Points, Chart Patterns, Candlestick. Structure indicators define the levels that confirm signals (e.g. divergence confirmation requires an S/R break) and anchor targets/invalidation.

### 2.6 Regime (4)
Aroon, Choppiness (gate), LinReg Slope, Z-Score. Regime indicators classify trending vs ranging conditions and gate the interpretation of everything else.

### 2.7 Institutional (4)
SMC Structure (CHoCH), Liquidity, Fair Value Gap, Order Blocks. Smart-money-concept indicators track structural breaks and institutional footprints.

### 2.8 Derivatives Data (7)
Open Interest, OI Delta, Funding Rate, OI-Price Divergence, Order Flow Imbalance, Spread, Depth Bias. Perp-specific context feeding liquidity and execution risk.

---

## 3. Key Thresholds (Quick Reference)

| Indicator | Threshold | Meaning |
|-----------|-----------|---------|
| RSI | ≥70 / ≤30 (≥80/≤20 in strong trend) | Overbought / Oversold |
| ADX | ≥25 trend · ≥40 exhaustion | Trend strength gate |
| BBWP | ≥90 expansion climax · ≤10 max compression | Volatility regime |
| RVOL | ≥1.5 institutional · ≥3.0 climax | Participation gate |
| Choppiness | ≥61.8 chop · ≤38.2 trend | Regime gate |
| MACD | line × signal cross | Momentum crossover |
| Squeeze | on→off | Compression release |

Configurable in `config.json` `indicators`. Full per-indicator thresholds: [indicators/](indicators/04-02-00-indicator-index.md).

---

## 4. Normalization Philosophy

All indicators normalize to a common `[-1, 1]` scale so heterogeneous readings can be blended into confluence. Normalization is **regime-aware** and **divergence-aware** (confirmed divergences can override to ±1.0). The mapping formulas are documented per indicator (e.g. [rsi.md](indicators/04-02-11-rsi.md) §Normalization).

---

## 5. Divergence-Bearing Indicators

Eight indicators support divergence detection: RSI, MACD, Stochastic, ChandeMO, OBV, CMF, MFI, Squeeze. Divergence status progresses `Potential → Confirmed` when price breaks the nearest S/R level by a tolerance buffer. See [signals/divergence.md](signals/05-02-01-divergence.md).

---

## 6. Cross-References

- [Indicator Index](indicators/04-02-00-indicator-index.md) — Full registry manifest and per-indicator specs.
- [Signals Guide](03-02-10-mme-signals-guide.md) — Signal detection rulebook.
- [Metrics Matrix](../../matrices/02-07-metrics-matrix.md) — Indicator serialization contract.
- [MME Layer 1 — Metrics](03-02-02-mme-layer1-metrics.md) — Computation pipeline.
