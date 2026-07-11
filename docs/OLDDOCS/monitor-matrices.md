# Monitor Matrices — Five-Stage Architecture

The Market Monitor transforms raw exchange data into market intelligence
through a five-stage pipeline:

```
INSTANCE LEVEL (per symbol × timeframe):
  Metrics Matrix (micro)  Metrics Matrix (fast)  Metrics Matrix (slow)  Metrics Matrix (macro)
        │                        │                        │                        │
        └────────────────────────┴────────────────────────┴────────────────────────┘
                                             │
SYMBOL LEVEL (per symbol):                  ▼
                                  Alignment Matrix (MTF agreement)
                                             │
                                             ▼
                                  Risk Matrix (market risk assessment)
                                             │
                                             ▼
                                  Analysis Matrix (market assessment)
                                             │
                                    All symbols' analyses
                                             │
SYSTEM LEVEL (all instances):               ▼
                                  State Matrix (global overview)

UI Panel Assignment:
  Metrics Matrix   → Metrics Panel    (TerminalMonitor.svelte)
  Alignment Matrix → Alignment Panel   (AlignmentPanel.svelte)
  Risk Matrix      → Risk Panel        (RiskPanel.svelte)
  Analysis Matrix  → Analysis Panel    (AnalysisPanel.svelte)
  State Matrix     → General Dashboard (GeneralDashboard.svelte)
```

---

## Stage 1: Metrics Matrix

### Definition

A **Metrics Matrix** is the complete analytical snapshot for a single
instance — one trading pair on one timeframe at one point in time. Every
running strategy instance owns one Metrics Matrix. It is the primary source
of truth consumed by higher architectural layers.

### Scope

```
One Metrics Matrix = One symbol × One timeframe × One snapshot
```

Examples: `BTCUSDT-1m`, `BTCUSDT-5m`, `ETHUSDT-15m`

### Components

The Metrics Matrix is composed of three logical sections:

**1. Indicators** — Continuous numerical models describing different aspects of the market.
58 indicators across 8 functional groups (Trend, Momentum, Volume, Volatility,
Structure, Regime, Institutional, DerivativesData).

**2. Signals** — Discrete events derived from one or more indicators. 12 SignalKind
variants (Divergence, Crossover, Threshold, Breakout, BandTouch, ZeroLineCross,
CompressionRelease, LevelTest, TrendFlip, VolumeClimax, StackChange, PatternForming).

**3. Derived Metrics** — Higher-level analytical summaries computed from indicators
and signals. Includes: MarketContext (trend, momentum, volatility, volume, liquidity,
regime, overall_score), local bias score (equal-weighted mean of all directional
indicators), trend quality, strategy recommendation.

### Indicator Evaluation Axes

Every indicator exposes multiple dimensions beyond its raw value:

| Axis | Description | Possible Values | Code Field |
|---|---|---|---|
| **Value** | Raw numerical output | Native indicator units | `NormalizedIndicatorValue.raw_value` |
| **State** | Human-readable interpretation | Bullish / Bearish / Neutral | `NormalizedIndicatorValue.state_label` |
| **Normalized** | Unified [-1,+1] scale | Continuous -1.0 to +1.0 | `NormalizedIndicatorValue.normalized` |
| **Confidence** | Estimated reliability | 0.0–1.0 | `NormalizedIndicatorValue.confidence` |
| **Direction** | Current movement | Rising / Falling / Flat | *deferred* |
| **Strength** | Measures intensity | Weak / Moderate / Strong / Extreme | *deferred* |
| **Market Regime** | Environment relevance | Trending / Ranging / Expansion / Compression / Transition | *deferred* |
| **Freshness** | How recently condition developed | New / Recent / Aging / Expired | *deferred* |
| **Quality** | Overall reading quality | Poor / Normal / Healthy / Excellent | *deferred* |

### Signal Evaluation Axes

| Axis | Description | Code Field |
|---|---|---|
| **Signal Type** | What event occurred (12 kinds) | `IndicatorSignal.kind` |
| **Direction** | Bullish / Bearish / Neutral | `IndicatorSignal.direction` |
| **Status** | Lifecycle stage | `IndicatorSignal.status` (Potential→Confirmed, Active) |
| **Strength** | How significant (0–1) | `IndicatorSignal.strength` |
| **Freshness** | Bars since first appearance | `IndicatorSignal.age_bars` |
| **Confirmation** | Pending / Confirmed / Rejected | `IndicatorSignal.status` |
| **Multi-Timeframe** | Same signal across TFs | Alignment Matrix `signal_cross_tf_count` |
| **Confidence** | Signal-level reliability | *deferred* |
| **Market Regime** | Regime-appropriateness | *deferred* |
| **Risk Level** | Low / Medium / High | *deferred* |
| **Priority** | Critical / High / Medium / Low | *deferred* |

### Derived Metrics

| Metric | Description | Code Location |
|---|---|---|
| Market Regime | TRENDING / RANGE / EXPANSION / COMPRESSION | `MarketContext.regime` |
| Trend Score | Trend-group mean [-100,+100] | `MarketContext.trend.score` |
| Momentum Score | Momentum-group mean [-100,+100] | `MarketContext.momentum.score` |
| Volume Score | Volume magnitude | `MarketContext.volume.score` |
| Volatility Score | BBWP-derived | `MarketContext.volatility.score` |
| Liquidity State | VWAP + volume proxy | `MarketContext.liquidity` |
| Overall Confidence | Local bias weighted mean | `MarketContext.overall_score` |
| Trend Quality | ADX + EMA-derived | *deferred* |
| Strategy Recommendation | TrendFollowing/Breakout/Pullback/etc. | *deferred* |
| Trade Readiness | NotReady/Building/Ready/Confirmed/Late | Analysis Matrix |
| Market Phase | Accumulation/Markup/Distribution/Markdown | *deferred* |

### Intra-Timeframe Confluence (Local Bias)

The Metrics Matrix computes a **local bias score** — the equal-weighted
arithmetic mean of all directional indicators present in the snapshot:

```
local_bias = Σ(normalized) / count × 100   ([-100, +100])
```

This answers: _"What does THIS timeframe think?"_ It never looks at another
timeframe. It is the indicator-confluence within one Metrics Matrix.

### UI Panel: Metrics Panel

The **Metrics Panel** (`TerminalMonitor.svelte`) renders the Metrics Matrix for
a single instance. It shows:
- All 58 indicator rows with raw_value, normalized score, state_label, confidence
- Active signals per indicator with freshness badges
- Derived metrics section (MarketContext: trend, momentum, volatility, volume, liquidity, regime, overall)

---

## Stage 2: Alignment Matrix

### Definition

The **Alignment Matrix** aggregates the four Metrics Matrices (micro, fast,
slow, macro) for a single symbol to measure **timeframe agreement** — not
indicator confluence (which lives in the Metrics Matrix's local bias score).

This is NOT the same as the per-instance local bias inside each Metrics Matrix.
The Alignment Matrix measures cross-timeframe agreement — how well micro,
fast, slow, and macro agree on direction.

### Structure

```rust
AlignmentMatrix {
    symbol: String,
    timeframes_present: u8,             // 0–4
    mtf_trend_alignment: f64,           // [-1, +1] mean agreement of Trend group
    mtf_momentum_alignment: f64,        // [-1, +1] mean agreement of Momentum group
    mtf_volume_alignment: f64,          // [-1, +1] mean agreement of Volume group
    mtf_volatility_alignment: f64,      // [-1, +1] mean agreement of Volatility group
    mtf_overall_score: f64,             // [-100, +100] weighted MTF bias
    mtf_overall_label: String,          // STRONG_BULL_MTF / NEUTRAL_MTF / etc.
    timeframe_alignments: Vec<TfAlignmentInfo>,
    signal_cross_tf_count: u32,
    trend_agreement_pct: f64,
}
```

### Computation

1. For each active timeframe, compute a MarketContext from its Metrics Matrix
2. Per-dimension (Trend, Momentum, Volume, Volatility): average the scores
   across all present timeframes
3. Compute `trend_agreement_pct`: % of timeframes where overall_score has same sign
4. Weighted overall: higher timeframes (slow, macro) get proportionally more weight

### Condition

Requires ≥2 timeframes. With 1 timeframe, Alignment Matrix is defined but marked
as insufficient (agreement is N/A).

### UI Panel: Alignment Panel

The **Alignment Panel** (`AlignmentPanel.svelte`) displays:
- MTF alignment gauge per group (Trend, Momentum, Volume, Volatility)
- Timeframe breakdown table
- Trend agreement % visual
- Cross-TF signal count

---

## Stage 3: Risk Matrix

### Definition

The **Risk Matrix** evaluates market-derived risk factors for a single symbol
by consuming the Alignment Matrix and per-timeframe Metrics. It answers:
*given the current market conditions, how should risk be managed?*

It does **not** know about portfolio state, account balance, or position size.
It only understands the market.

### Structure

```rust
RiskMatrix {
    symbol: String,
    overall_market_risk: RiskLevel,       // VeryLow / Low / Moderate / High / Extreme
    volatility_risk: RiskLevel,
    liquidity_risk: RiskLevel,
    trend_stability: TrendStability,      // Weak / Developing / Healthy / Strong / Exhausted
    structural_risk: RiskLevel,
    signal_reliability: SignalReliability, // Poor / Fair / Good / Excellent
    suggested_stop_method: StopMethod,    // ATR / SwingLow / Support / VWAP / Supertrend / StructureBased
    suggested_stop_distance: f64,         // in ATR multiples (e.g., 1.8)
    suggested_target_method: TargetMethod,// Fibonacci / SwingHigh / ATRMultiple / Resistance / Support
    expected_rr: f64,                     // estimated risk/reward ratio
}
```

### Inputs

| Risk Factor | Indicators Used |
|---|---|
| **Volatility Risk** | ATR, BBWP, Bollinger Width, Squeeze state |
| **Liquidity Risk** | Volume, RVOL, VWAP, spread, depth bias |
| **Trend Stability** | ADX, EMA Ribbon, Supertrend, Aroon, Choppiness |
| **Structural Risk** | S/R proximity, Fibonacci levels, swing structure, pivot points |
| **Signal Reliability** | Signal agreement %, freshness, confirmation states |
| **Stop Distance** | Highest of: ATR × multiplier, nearest support/swing-low |
| **Target Method** | Based on regime: Fib extensions (trending), S/R levels (ranging) |
| **Expected RR** | (nearest target distance) / (suggested stop distance) |

### Condition

Requires an Alignment Matrix (≥2 timeframes).

### UI Panel: Risk Panel

The **Risk Panel** (`RiskPanel.svelte`) displays:
- Overall market risk gauge (colored bar: green → yellow → red)
- Risk breakdown cards: volatility, liquidity, trend stability, structural, signal reliability
- Suggested stop method + distance display
- Suggested target method + expected RR bar

---

## Stage 4: Analysis Matrix

### Definition

The **Analysis Matrix** is the highest analytical layer for a single symbol.
It consumes the Alignment Matrix and Risk Matrix to produce a complete,
explainable market assessment for discretionary traders.

It answers: _"Given the complete multi-timeframe analysis, what is the current
market assessment, how risky is this market, and how should a trader approach it?"_

It does **not** make trading decisions or execute trades.

### Structure

```rust
AnalysisMatrix {
    symbol: String,
    bias: MarketBias,              // Bullish / Bearish / Neutral
    confidence: f64,               // 0.0–1.0
    trade_readiness: TradeReadiness, // NotReady / Building / Ready / Confirmed / Late
    preferred_strategy: Strategy,  // TrendFollowing / Breakout / Pullback / RangeTrading / MeanReversion / Scalping / NoTrade
    market_quality: MarketQuality, // Poor / Weak / Average / Good / Excellent
    warnings: Vec<String>,
    rationale: String,
    supporting_signals: Vec<String>,
    contradicting_signals: Vec<String>,
    timeframes_considered: u8,
    opportunity_scores: OpportunityScores,
}
```

### Derivation

- **Bias**: `mtf_overall_score > 20` → Bullish, `< -20` → Bearish, else Neutral
- **Confidence**: base = |score|/100, modified by trend agreement, cross-TF signals, TF count
- **Trade Readiness**: derived from confidence + signal freshness + market quality
- **Preferred Strategy**: rule-based mapping from regime + bias + volatility
  - Strong trend + high ADX → TrendFollowing
  - Compression + squeeze release → Breakout
  - Trending but overextended → Pullback
  - Ranging + high choppiness → RangeTrading
  - Extreme BBWP → MeanReversion
- **Market Quality**: aggregate of signal reliability + trend stability + liquidity
- **Warnings**: low liquidity, extreme volatility, conflicting signals, diverging timeframes, false breakout risk

### Condition

Requires Alignment Matrix + Risk Matrix (≥2 timeframes).

### UI Panel: Analysis Panel

The **Analysis Panel** (`AnalysisPanel.svelte`) displays:
- Large bias badge (green BULLISH / red BEARISH / grey NEUTRAL)
- Confidence bar
- Trade readiness indicator
- Preferred strategy recommendation
- Market quality gauge
- Active warnings list
- Rationale text
- Supporting/contradicting signals
- Per-strategy opportunity scores

---

## Stage 5: State Matrix

### Definition

The **State Matrix** is the system-wide aggregation. It collects all Analysis
Matrices and instance metadata to produce a global dashboard summary.

### Structure

```rust
StateMatrix {
    instance_count: u32,
    active_symbols: Vec<String>,
    total_timeframes_active: u32,
    regime_distribution: HashMap<String, u32>,
    global_bias_label: String,
    per_symbol_summary: Vec<SymbolSummary>,
    active_signals_total: u32,
}
```

### UI Panel: General Dashboard

The **General Dashboard** (`GeneralDashboard.svelte`) displays:
- Active instance count + symbols list
- Regime distribution
- Per-symbol analysis summary cards (bias, confidence, MTF score, regime)
- Global bias label
- Active signals count

---

## Complete Data Flow

```
EXCHANGE WEBSOCKETS
  │
  ▼
CANDLE GENERATOR (micro/fast/slow/macro)
  │
  ▼
RAW INDICATORS + NORMALIZATION
  │ NormalizedIndicatorValue map (58 entries per snapshot)
  ▼
══════════════════════════════════════════
  ▼
METRICS MATRIX                     ← Stage 1
  │ Per instance: indicators + signals + derived metrics + local bias
  │ → Metrics Panel
  ▼
ALIGNMENT MATRIX                   ← Stage 2
  │ Per symbol: timeframe agreement (not indicator confluence)
  │ → Alignment Panel
  ▼
RISK MATRIX                        ← Stage 3
  │ Per symbol: market risk assessment + stop/target guidance
  │ → Risk Panel
  ▼
ANALYSIS MATRIX                    ← Stage 4
  │ Per symbol: bias, confidence, strategy, warnings, opportunity scores
  │ → Analysis Panel
  ▼
STATE MATRIX                       ← Stage 5
  │ System-wide: all symbols aggregated
  │ → General Dashboard
══════════════════════════════════════════
```

---

## Cross-References

- **Formal ontology and axes**: [ontology.md](ontology.md)
- **Complete indicator reference**: [metrics-matrix.md](metrics-matrix.md)
- **Master specification**: [indicator-system-master-spec.md](indicator-system-master-spec.md)
- **Alignment implementation**: `crates/shared/src/alignment.rs`
- **Risk implementation**: `crates/shared/src/risk.rs`
- **Analysis implementation**: `crates/shared/src/analysis.rs`
- **State Matrix implementation**: `crates/shared/src/state_matrix.rs`
