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

The system models market information as a pipeline of seven ontological levels:

```
Entity → Metric → Signal → Alignment → Risk → Analysis → State
  │        │        │        │          │       │         │
  └────────┴────────┴────────┴──────────┴───────┴─────────┘
                         described by AXES

Instance level        Symbol level           System level
(per symbol×TF)       (per symbol)          (all instances)
```

The levels ascend in aggregation scope: Entity and Metric are per-instance
(one symbol on one timeframe), Signal is per-instance, Alignment and Risk and
Analysis are per-symbol (cross-timeframe), and State is system-wide (all symbols).

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

### Level 5: Risk

**Risk** assesses the market risk environment for a single symbol by consuming
the Alignment Matrix. It does not know about portfolio or account state —
it only evaluates market-derived risk factors. It answers: *given the current
market conditions, how should risk be managed?*

The Risk output contains:
- `overall_market_risk`: VeryLow / Low / Moderate / High / Extreme
- `volatility_risk`: derived from ATR + BBWP + Squeeze state
- `liquidity_risk`: derived from volume + RVOL + VWAP + spread
- `trend_stability`: Weak / Developing / Healthy / Strong / Exhausted
- `structural_risk`: derived from S/R proximity + swing structure
- `signal_reliability`: Poor / Fair / Good / Excellent
- `suggested_stop_method`: ATR / SwingLow / Support / VWAP / Supertrend / StructureBased
- `suggested_stop_distance`: in ATR multiples (e.g., 1.8)
- `suggested_target_method`: Fibonacci / SwingHigh / ATRMultiple / Resistance
- `expected_rr`: estimated risk/reward ratio

Risk is computed by `compute_risk()` in `shared/src/risk.rs`.

**Condition**: Requires an Alignment Matrix (≥2 timeframes).

### Level 6: Analysis

**Analysis** is the final per-symbol synthesis layer. It consumes the
Alignment Matrix and Risk Matrix to produce a complete market assessment.
It answers: *given all available evidence, what is the complete technical
assessment for this symbol?*

```
Analysis = Bullish / Bearish / Neutral market bias
```

The Analysis output contains:
- `bias`: MarketBias (Bullish / Bearish / Neutral)
- `confidence`: 0.0–1.0
- `trade_readiness`: NotReady / Building / Ready / Confirmed / Late
- `preferred_strategy`: TrendFollowing / Breakout / Pullback / RangeTrading / MeanReversion / Scalping / NoTrade
- `market_quality`: Poor / Weak / Average / Good / Excellent
- `warnings`: list of active risk warnings
- `rationale`: human-readable explanation
- `supporting_signals` / `contradicting_signals`
- `opportunity_scores`: per-strategy 0–100 scores (trend, breakout, pullback, mean_reversion)

Analysis is computed by `derive_analysis()` in `shared/src/analysis.rs`.

**Key distinction**: This is a **market assessment**, not a trade execution
decision. The Market Monitor informs; it does not act.

**Condition**: Requires an Alignment Matrix + Risk Matrix (≥2 timeframes).

### Level 7: State

**State** is the system-wide aggregation of all active Analysis Matrices
and instance metadata. It provides the global dashboard overview. It answers:
*what is the state of the entire Market Monitor right now?*

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

| Axis | Entity | Metric | Signal | Alignment | Risk | Analysis | State |
|---|---|---|---|---|---|---|---|
| Entity | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| Category | — | ✓ | — | — | — | — | — |
| Information | — | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Predictive Class | — | ✓ | — | — | — | — | — |
| Source | — | ✓ | — | — | — | — | — |
| Scale | — | ✓ | — | — | — | — | — |
| Scoring Role | — | ✓ | — | — | — | — | — |
| Signal Kind | — | — | ✓ | — | — | — | — |
| Signal Direction | — | — | ✓ | — | — | — | — |
| Signal Status | — | — | ✓ | — | — | — | — |
| Confidence | — | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Time | ✓ | ✓ | ✓ | ✓ | — | — | — |

---

## Relationship to Architecture Layers

The ontological levels map to the system's architectural layers:

| Ontological Level | Architecture Layer(s) | Code Location |
|---|---|---|
| Entity | Ingestion (WebSocket → CandleGenerator) | `engine/src/analyzer/mod.rs` (ActivePair, TimeframePipeline) |
| Metric | L1 Calculators, L2 Normalization | `shared/src/indicators/`, `shared/src/indicators/normalized/` |
| Signal | L2 Normalization+Signals | `shared/src/indicators/normalized/signals.rs` |
| Alignment | L2.5 MTF Alignment | `shared/src/alignment.rs`, `engine/src/analyzer/mod.rs` |
| Risk | L4.25 Risk Assessment | `shared/src/risk.rs`, `engine/src/analyzer/mod.rs` |
| Analysis | Assembly (analysis derivation) | `shared/src/analysis.rs`, `engine/src/analyzer/mod.rs` |
| State | Assembly (system aggregation) | `shared/src/state_matrix.rs`, `engine/src/analyzer/mod.rs` |

The axes defined here are the **metadata vocabulary** that describes indicators
in the registry (`shared/src/indicators/registry.rs`) and flows through every
`NormalizedIndicatorValue` and `IndicatorSignal` at runtime.

---

## Cross-References

- **Complete indicator reference**: [metrics-matrix.md](metrics-matrix.md) — 58 indicators × 12 axes × 101 signal declarations
- **Five-matrix architecture**: [monitor-matrices.md](monitor-matrices.md) — Metrics Matrix → Alignment Matrix → Risk Matrix → Analysis Matrix → State Matrix
- **Master specification**: [indicator-system-master-spec.md](indicator-system-master-spec.md) — system layers, registry structure, scoring system
- **AI interpretation guide**: [indicators-guide.md](indicators-guide.md) — per-indicator signal thresholds and rules
- **System architecture**: [architecture.md](architecture.md) — deployment topology and data flow
- **User manual**: [user-manual.md](user-manual.md) — installation, configuration, usage
