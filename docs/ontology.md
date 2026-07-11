# Market Monitor — Formal Ontology

## The Market Monitor Identity

The Market Monitor is an **observational market analysis tool**. It streams live
cryptocurrency data from exchanges (Hyperliquid, Bitget), computes technical
indicators in real time, and synthesizes market state across multiple timeframes
and symbols. It **does not execute trades** — its output is informational:
market bias, signal activity, and regime classification.

This document defines the formal ontology — what exists, how it is classified,
and how the pieces relate — as the foundation for all documentation, code
architecture, and future development.

---

## Ontological Levels

The system models market information as a pipeline spanning **six analytical matrices** across **eight ontological levels** (Entity, Metric, and Signal are building-block levels that feed into the matrices):

```
Entity → Metric → Signal → Alignment → Analysis → Risk → Advisory → Overview
  │        │        │        │            │         │        │          │
  └────────┴────────┴────────┴────────────┴─────────┴────────┴──────────┘
                              described by AXES

Instance level             Symbol level                   System level
(per symbol×TF)            (per symbol)                  (all instances)
```

The levels ascend in aggregation scope: Entity, Metric, and Signal are
per-instance (one symbol on one timeframe). Alignment, Analysis, Risk, and
Advisory are per-symbol (cross-timeframe). Overview is system-wide (all symbols).

### Level 1: Entity

An **Entity** is the subject of observation. In the Market Monitor, an entity
is a **trading pair on a specific timeframe** — for example, `BTC-USD` on the
`15-minute` timeframe. An entity defines *what* and *when* we measure.

- **Rule**: Every Metric, Signal, and State belongs to exactly one Entity.
- **Instances**: The system supports multiple entities simultaneously (multiple
  symbols × 4 timeframes each).
- **Runtime identity**: `(symbol, timeframe)` tuple. Example: `("BTC-USD", "fast180")`.

### Level 2: Metric

A **Metric** is a quantitative measurement derived from market data. Each
metric is computed by a single indicator calculator and produces a numerical
value in native units (e.g., RSI = 72.3, MACD histogram = 1.45, ATR = 125.80).

After normalization, every Metric is also expressed on a unified **[-1.0, +1.0]**
scale where:
- `+1.0` = absolute bullish conviction
- `0.0` = equilibrium / neutral / compression
- `-1.0` = absolute bearish conviction

Each Metric carries:
- `raw_value`: the original value in native indicator units
- `normalized`: signed [-1.0, +1.0] score
- `state_label`: human-readable classification (e.g., "OVERSOLD", "BULLISH_TREND")
- `values`: optional sub-metric map for multi-line indicators (e.g., MACD line + signal + histogram)
- `signals`: discrete signal events detected this bar (empty for most snapshots)
- `confidence`: 0.0–1.0 conviction derived from `|normalized|` + confirmed signal boost

### Level 3: Signal

A **Signal** is a discrete, named event emitted by a Metric when specific
conditions are met. While a Metric yields a continuous value every bar, Signals
fire sparingly — only when a threshold is breached, a crossover occurs, a
divergence is detected, etc.

Each Signal carries:
- `kind`: the type of event (12 SignalKind variants)
- `direction`: bullish / bearish / neutral
- `status`: lifecycle — Potential → Confirmed, or Active (point-in-time events)
- `label`: human-readable string (e.g., "CONFIRMED_BULLISH_DIVERGENCE")
- `strength`: 0.0–1.0
- `age_bars`: how many bars since first appearance

Signals are the bridge between per-instance Metrics and per-symbol Alignment.

### Level 4: Alignment

**Alignment** is the first cross-timeframe aggregation. It takes the four
Metrics Matrices for a single symbol (micro, fast, slow, macro) and computes
multi-timeframe (MTF) agreement — not indicator confluence, but timeframe
agreement. It answers: *how well do the timeframes agree on market direction?*

The Alignment output contains:
- `symbol`: the trading pair
- `timeframes_present`: 0–4 (how many TFs have data)
- Per-group MTF alignment scores (Trend, Momentum, Volume, Volatility) — mean of each group's scores across all active timeframes
- `mtf_overall_score`: weighted MTF bias in [-100, +100]
- `trend_agreement_pct`: % of timeframes agreeing on direction (same sign)
- `signal_cross_tf_count`: signals appearing in ≥2 timeframes
- Per-timeframe breakdown (each TF's scores, regime, active signals)

Alignment is computed by `compute_alignment()` in `shared/src/alignment.rs`.
Indicator confluence (within a single timeframe) lives in the Metrics Matrix
via MarketContext's local bias score.

**Condition**: Requires ≥2 timeframes with completed candles to produce
meaningful agreement. With 1 timeframe, Alignment is still defined but
agreement metrics are N/A.

### Level 5: Analysis

**Analysis** transforms structured observations and multi-timeframe
relationships into a complete interpretation of current market conditions.
It consumes the Metrics Matrices and Alignment Matrix. It answers: *given
everything currently observed, what is the complete interpretation of
this market?*

The Analysis output contains:
- `bias`: MarketBias (StrongBullish / Bullish / Neutral / Bearish / StrongBearish)
- `market_regime`: TrendingBull / TrendingBear / Range / Accumulation / Distribution / Expansion / Contraction / Transition
- `trend_assessment`: TrendQuality (Weak / Developing / Healthy / Strong / Exhausted)
- `momentum_assessment`: MomentumState (Increasing / Stable / Weakening / Exhausted / Reversing)
- `structure_assessment`: StructureState (Strong / Healthy / Weak / Broken / Unclear)
- `volatility_assessment`: VolatilityState (Compressed / Normal / Expanding / Extreme / Unstable)
- `volume_assessment`: VolumeState (Weak / Normal / Strong / Exceptional)
- `opportunity_analysis`: OpportunityType (TrendContinuation / Breakout / Pullback / MeanReversion / Reversal / NoClearOpportunity)
- `market_quality`: QualityLevel (Poor / Weak / Average / Good / Excellent)
- `market_interpretation`: human-readable summary
- `confidence`: 0.0–1.0
- `rationale`, `supporting_signals`, `contradicting_signals`

Analysis is computed by `derive_analysis()` in `shared/src/analysis.rs`.

**Key distinction**: This is pure **market interpretation**. It does not
evaluate risk, provide guidance, or recommend trading actions.

**Condition**: Requires an Alignment Matrix (≥2 timeframes).

### Level 6: Risk

**Risk** evaluates the level of uncertainty surrounding the current market
interpretation. It consumes the Analysis Matrix — risk is a property of an
interpretation, not of raw observations. You cannot evaluate how risky a
bullish trend is without first determining that there IS a bullish trend
and understanding its quality, regime, and structure.

The Risk output evaluates 9 dimensions, each with Score (0-100), Level
(VeryLow / Low / Moderate / High / Extreme), State (Stable / Increasing /
Elevated / Critical / Improving), Confidence (0-100%), and Evidence:
- `market_risk`: uncertainty from conflicting signals, weak structure, low confidence
- `volatility_risk`: danger from abnormal price movement (ATR, BBWP, Squeeze)
- `liquidity_risk`: quality of market participation (RVOL, VWAP, spread)
- `structure_risk`: uncertainty from weak/damaged price structure
- `momentum_risk`: vulnerability from exhausted/diverging momentum
- `signal_risk`: uncertainty from conflicting/unreliable signals
- `execution_risk`: practical difficulties (slippage, fast movement)
- `reward_risk`: opportunity quality vs environmental uncertainty ratio
- `overall_risk`: weighted aggregation of all dimensions

Risk is computed by `compute_risk()` in `shared/src/risk.rs`.

**Key distinction**: Risk is independent from market direction. A bullish
market is not automatically safe; a bearish market is not automatically
dangerous. Risk evaluates uncertainty, not direction.

**Condition**: Requires an Analysis Matrix.

### Level 7: Advisory

**Advisory** transforms complete market intelligence and risk assessment into
structured human-facing guidance. It consumes the Analysis Matrix and Risk
Matrix. It answers: *given the current market condition and associated risk,
what is the most reasonable interpretation and possible action direction?*

The Advisory output contains:
- `directional_guidance`: StrongLong / Long / Neutral / Short / StrongShort / AvoidDirectionalExposure
- `market_stance`: Aggressive / Constructive / Neutral / Cautious / Avoid
- `opportunity_classification`: TrendContinuation / Breakout / Pullback / MeanReversion / Reversal / NoClearOpportunity
- `strategy_environment`: TrendFollowing / Breakout / MeanReversion / HighVolatility / LowActivity / Unfavorable
- `entry_guidance`: Immediate / WaitForConfirmation / Pullback / Breakout / NoEntryContext
- `exit_guidance`: TrendWeakening / MomentumExhaustion / StructureBreakdown / RiskIncreasing / NoWarning
- `stop_loss_guidance`: StructureBased / VolatilityBased / ATRBased / SRBased / NoRecommendation
- `take_profit_guidance`: ResistanceBased / RRBased / VolatilityBased / TrailingMethod / NoRecommendation
- `confidence_assessment`: 0–100%
- `final_recommendation`: human-readable summary

Advisory is computed by `compute_advisory()` in `shared/src/advisory.rs`.

**Key distinction**: Advisory provides guidance, not autonomous decisions.
It explains and recommends; the final decision remains with the human trader.

**Condition**: Requires an Analysis Matrix + Risk Matrix.

### Level 8: Overview

**Overview** is the system-wide aggregation of all Advisory Matrices and
instance metadata. It provides a unified representation of the observed
market environment. It answers: *what is happening across the entire
monitored market?*

The Overview output contains:
- `global_market_bias`: StrongBullish / Bullish / Neutral / Bearish / StrongBearish / Mixed
- `market_breadth`: VeryWeak / Weak / Balanced / Positive / StrongPositive / Negative / StrongNegative
- `regime_distribution`: % of assets per regime
- `opportunity_distribution`: count per opportunity type + quality level
- `risk_distribution`: Low/Moderate/High % + RiskEnvironment classification
- `asset_ranking`: scored ranking (opportunity × confidence × alignment / risk)
- `market_synchronization`: HighlySynchronized / Synchronized / Mixed / Fragmented / HighlyFragmented
- `market_health`: Poor / Weak / Neutral / Healthy / Strong
- `global_summary`: human-readable overview

Overview is computed by `compute_overview()` in `shared/src/overview.rs`.

**Condition**: Requires at least one Advisory Matrix (one active instance).

The State contains:
- `instance_count`: number of active (symbol, timeframe) pipelines
- `active_symbols`: unique symbols being monitored
- `total_timeframes_active`: sum of active timeframes across all symbols
- `regime_distribution`: count per regime (TRENDING, RANGE, etc.)
- `global_bias_label`: overall system bias (mode of all symbol biases, or "Mixed")
- `per_symbol_summary`: one entry per symbol with bias, confidence, MTF score
- `active_signals_total`: total active signals across all instances

State is computed by `compute_state()` in `shared/src/state_matrix.rs`.

**Condition**: Requires at least one active instance with a completed candle.
Without data, State returns empty (instance_count = 0).

---

## The 12 Axes

Axes are the classification dimensions that cut across ontological levels.
Each axis answers a specific question about a Metric, Signal, or State.
Not every axis applies to every level — the table below shows which axes
are relevant at each level.

### Axis Definitions

#### 1. Entity Axis

| Property | Value |
|---|---|
| **Question** | What is being observed? |
| **Values** | `(symbol, timeframe)` tuple, e.g., `("BTC-USD", "fast180")` |
| **Applies to** | Entity, Metric, Signal |
| **Code location** | `ActivePair` in engine analyzer, pipeline identity |

#### 2. Category Axis (`IndicatorGroup`)

| Property | Value |
|---|---|
| **Question** | What market aspect does this measure? |
| **Values** | Trend, Momentum, Volume, Volatility, Structure, Regime, Institutional, DerivativesData |
| **Applies to** | Metric |
| **Code** | `shared::indicators::registry::IndicatorGroup` |

#### 3. Information Axis

| Property | Value |
|---|---|
| **Question** | How far removed from raw market data is this? |
| **Values** | **Observed** (raw price, volume, order book) → **Derived1** (single-pass transforms: RSI, MACD, ATR) → **Derived2** (multi-pass: divergence, squeeze momentum direction) → **Meta** (synthesis: MarketContext, confluence score) |
| **Applies to** | Metric, Signal, State |
| **Note** | Documentation-only axis. Not a code enum. Maps loosely onto IndicatorClass but measures information distance, not prediction timing. |

#### 4. Predictive Class Axis (`IndicatorClass`)

| Property | Value |
|---|---|
| **Question** | Does this indicator lead price or confirm it? |
| **Values** | Leading (anticipates price moves), Hybrid (mixed behavior), Lagging (confirms after the fact) |
| **Applies to** | Metric |
| **Code** | `shared::indicators::registry::IndicatorClass` |

#### 5. Source Axis

| Property | Value |
|---|---|
| **Question** | What market data feeds this indicator? |
| **Values** | **Price** (OHLC-derived), **Volume** (trade volume-derived), **OrderBook** (depth/spread-derived), **Derivatives** (open interest, funding rate), **Composite** (multi-source) |
| **Applies to** | Metric |
| **Derivation** | Inferred from `IndicatorGroup`: Trend/Momentum/Structure/Regime → Price; Volume → Volume; DerivativesData → Derivatives; Volatility/Institutional → Composite |

#### 6. Scale Axis

| Property | Value |
|---|---|
| **Question** | What is the numerical domain of this metric? |
| **Values** | **Bounded0to100** (RSI, Stochastic, Williams %R, MFI, Choppiness, BBWP), **UnboundedRatio** (MACD, OBV, CMF, AO, CCI, LinReg, Z-Score, Force Index), **PriceAbsolute** (EMA, Supertrend, VWAP, Bollinger, Donchian, Keltner, Ichimoku, Fibonacci, Pivots, PSAR, Hull MA, StdDev Channel, Volume Profile), **BooleanOnOff** (Squeeze on/off, TrendFlip), **PercentUnit** (HV, Funding Rate, Spread) |
| **Applies to** | Metric |
| **Code field** | `value_format` in `IndicatorMeta` (render hint, loosely maps) |

#### 7. Scoring Role Axis

| Property | Value |
|---|---|
| **Question** | How does this metric contribute to the overall score? |
| **Values** | **Directional** (signed [-1,1] contributor to weighted-mean scoring), **Gate** (non-directional multiplier/filter, never enters signed sum) |
| **Applies to** | Metric |
| **Code** | `IndicatorMeta::directional` (bool), 9 gates: adx, atr, bbwp, hv, volume, rvol, choppiness, funding_rate, spread |

#### 8. Signal Kind Axis (`SignalKind`)

| Property | Value |
|---|---|
| **Question** | What type of discrete event occurred? |
| **Values** | Divergence, Crossover, Threshold, Breakout, BandTouch, ZeroLineCross, CompressionRelease, LevelTest, TrendFlip, VolumeClimax, StackChange, PatternForming |
| **Applies to** | Signal |
| **Code** | `shared::indicators::normalized::SignalKind` (12 variants) |

#### 9. Signal Direction Axis (`SignalDirection`)

| Property | Value |
|---|---|
| **Question** | What is the directional bias of this signal? |
| **Values** | Bullish, Bearish, Neutral |
| **Applies to** | Signal |
| **Code** | `shared::indicators::normalized::SignalDirection` |

#### 10. Signal Status Axis (`SignalStatus`)

| Property | Value |
|---|---|
| **Question** | What is the lifecycle stage of this signal? |
| **Values** | **Potential** (freshly detected, unconfirmed), **Confirmed** (validated by secondary condition), **Active** (point-in-time event like breakout or threshold breach) |
| **Applies to** | Signal |
| **Code** | `shared::indicators::normalized::SignalStatus` |

#### 11. Confidence Axis

| Property | Value |
|---|---|
| **Question** | How reliable is this reading? |
| **Values** | 0.0 (no confidence) to 1.0 (maximum confidence). Base = `|normalized|`; boosted by confirmed signals, dampened by signal age. |
| **Applies to** | Metric, Signal |
| **Code** | `NormalizedIndicatorValue::confidence` (f64), `IndicatorSignal::strength` (f64) |

#### 12. Time Axis

| Property | Value |
|---|---|
| **Question** | At what temporal resolution is this metric computed? |
| **Values** | **micro60** (1-minute candles), **fast180** (3-minute), **slow300** (5-minute), **macro900** (15-minute) |
| **Applies to** | Entity, Metric, Signal |
| **Code** | Pipeline identity in `analyzer/mod.rs`, not a registry field |

---

### Axis × Level Applicability Matrix

| Axis | Entity | Metric | Signal | Alignment | Analysis | Risk | Advisory | Overview |
|---|---|---|---|---|---|---|---|---|
| Entity | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| Category | — | ✓ | — | — | — | — | — | — |
| Information | — | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Predictive Class | — | ✓ | — | — | — | — | — | — |
| Source | — | ✓ | — | — | — | — | — | — |
| Scale | — | ✓ | — | — | — | — | — | — |
| Scoring Role | — | ✓ | — | — | — | — | — | — |
| Signal Kind | — | — | ✓ | — | — | — | — | — |
| Signal Direction | — | — | ✓ | — | — | — | — | — |
| Signal Status | — | — | ✓ | — | — | — | — | — |
| Confidence | — | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Time | ✓ | ✓ | ✓ | ✓ | — | — | — | — |

---

## Relationship to Architecture Layers

The ontological levels map to the system's architectural layers:

| Ontological Level | Architecture Layer(s) | Code Location |
|---|---|---|
| Entity | Ingestion (WebSocket → CandleGenerator) | `engine/src/analyzer/mod.rs` (ActivePair, TimeframePipeline) |
| Metric | L1 Calculators, L2 Normalization | `shared/src/indicators/`, `shared/src/indicators/normalized/` |
| Signal | L2 Normalization+Signals | `shared/src/indicators/normalized/signals.rs` |
| Alignment | L2.5 Signal Correlation | `shared/src/alignment.rs`, `engine/src/analyzer/mod.rs` |
| Analysis | L4.5 Market Intelligence | `shared/src/analysis.rs`, `engine/src/analyzer/mod.rs` |
| Risk | L4.25 Risk Assessment | `shared/src/risk.rs`, `engine/src/analyzer/mod.rs` |
| Advisory | L4.75 Decision Guidance | `shared/src/advisory.rs`, `engine/src/analyzer/mod.rs` |
| Overview | L5.5 Market Synthesis | `shared/src/overview.rs`, `engine/src/analyzer/mod.rs` |

The axes defined here are the **metadata vocabulary** that describes indicators
in the registry (`shared/src/indicators/registry.rs`) and flows through every
`NormalizedIndicatorValue` and `IndicatorSignal` at runtime.

---

## Cross-References

- **Complete indicator reference**: [metrics-matrix-reference.md](metrics-matrix-reference.md) — 58 indicators × 12 axes × 101 signal declarations
- **Six-matrix architecture**: [monitor-matrices-reference.md](monitor-matrices-reference.md) — Metrics → Alignment → Analysis → Risk → Advisory → Overview
- **Master specification**: [indicator-system-master-spec.md](indicator-system-master-spec.md) — system layers, registry structure, scoring system
- **AI interpretation guide**: [indicators-guide.md](indicators-guide.md) — per-indicator signal thresholds and rules
- **System architecture**: [architecture.md](architecture.md) — deployment topology and data flow
- **User manual**: [user-manual.md](user-manual.md) — installation, configuration, usage
