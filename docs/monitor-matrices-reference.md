# Monitor Matrices — Six-Stage Architecture Reference

The Market Monitor transforms raw exchange data into market intelligence
through a six-stage pipeline:

```
INSTANCE LEVEL (per symbol × timeframe):
  Metrics Matrix (micro)  Metrics Matrix (fast)  Metrics Matrix (slow)  Metrics Matrix (macro)
        │                        │                        │                        │
        └────────────────────────┴────────────────────────┴────────────────────────┘
                                             │
SYMBOL LEVEL (per symbol):                  ▼
                                  Alignment Matrix (MTF agreement — 10 dimensions)
                                             │
                                             ▼
                                  Analysis Matrix (market interpretation — 10 components)
                                             │
                                             ▼
                                  Risk Matrix (risk evaluation — 9 dimensions)
                                             │
                                             ▼
                                  Advisory Matrix (human-facing guidance — 10 components)
                                             │
                                    All symbols' advisory matrices
                                             │
SYSTEM LEVEL (all instances):               ▼
                                  Overview Matrix (global synthesis — 9 components)

UI Panel Assignment:
  Metrics Matrix   → Metrics Panel    (TerminalMonitor.svelte)
  Alignment Matrix → Alignment Panel   (AlignmentPanel.svelte)
  Analysis Matrix  → Analysis Panel    (AnalysisPanel.svelte)
  Risk Matrix      → Risk Panel        (RiskPanel.svelte)
  Advisory Matrix  → Advisory Panel    (AdvisoryPanel.svelte)
  Overview Matrix  → General Dashboard (GeneralDashboard.svelte)
```

---

## Stage 1: Metrics Matrix

### Definition

A **Metrics Matrix** is the foundational analytical component. It represents
a complete snapshot of a single trading symbol on a single timeframe. Each
running strategy instance owns one Metrics Matrix.

### Scope

```
One Metrics Matrix = One symbol × One timeframe × One snapshot

Examples:
  BTCUSDT-1m, BTCUSDT-5m, BTCUSDT-15m, BTCUSDT-1H, ETHUSDT-5m
```

### Components

The Metrics Matrix is composed of three logical sections:

**1. Indicators** — Continuous numerical models. 58 indicators across 8
functional groups (Trend, Momentum, Volume, Volatility, Structure, Regime,
Institutional, DerivativesData). Each indicator exposes: Value, State,
Normalized score, Confidence, Direction, Strength, Market Regime,
Freshness, Quality.

**2. Signals** — Discrete events derived from indicators. 12 SignalKind
variants. Each signal carries: Type, Direction, Status, Strength, Freshness,
Confirmation, Multi-Timeframe alignment, Confidence, Market Regime,
Risk Level, Priority.

**3. Features** — Reusable, high-level quantitative variables derived from
indicators and signals, organized into eight groups: Trend, Momentum,
Volatility, Volume, Structure, Market, Probability, and Confidence Features.
Examples: Trend Score, Momentum Acceleration, ATR Percentile, Relative Volume,
Breakout Pressure, Market Regime, Market Phase, Breakout/Continuation/Reversal/
Mean Reversion Probabilities, Overall Confidence.

### Local Confluence (Intra-Timeframe)

The Metrics Matrix computes a **local bias score** — the equal-weighted
arithmetic mean of all directional indicators:

```
local_bias = Σ(normalized) / count × 100   ([-100, +100])
```

This answers: _"What does THIS timeframe think?"_

### UI Panel: Metrics Panel

`TerminalMonitor.svelte` — 58 indicator rows with raw/normalized/state_label/
confidence, active signals with freshness badges, features section
(MarketContext: trend, momentum, volatility, volume, liquidity, regime, overall).

---

## Stage 2: Alignment Matrix

### Definition

The **Alignment Matrix** compares, correlates, and evaluates relationships
between multiple independent Metrics Matrices for the same asset across
different timeframes. It measures **timeframe agreement** — not indicator
confluence (which lives in each Metrics Matrix's local bias score).

```
Question: "How consistently do different timeframes describe the same market?"
```

### Structure

```rust
AlignmentDimension {
    score: f64,          // 0-100% alignment
    state: AlignState,   // Bullish / Bearish / Neutral / Mixed
    confidence: f64,     // 0-100%
}

AlignmentMatrix {
    symbol: String,
    timeframes_present: u8,
    dimensions: Vec<AlignmentDimension>,  // 10 dimensions
    // Top-level summaries (preserved for backward compat)
    mtf_trend_alignment: f64,
    mtf_momentum_alignment: f64,
    mtf_volume_alignment: f64,
    mtf_volatility_alignment: f64,
    mtf_overall_score: f64,
    mtf_overall_label: String,
    timeframe_alignments: Vec<TfAlignmentInfo>,
    signal_cross_tf_count: u32,
    trend_agreement_pct: f64,
}
```

### 10 Alignment Dimensions

| # | Dimension | What It Measures | Source Metrics |
|---|---|---|---|
| 1 | Trend Alignment | Do TFs agree on direction? | EMA, VWAP, Supertrend, Trend Score |
| 2 | Momentum Alignment | Do TFs agree on pressure? | RSI, MACD, Stochastic, Momentum Score |
| 3 | Volume Alignment | Does participation confirm? | RVOL, OBV, CMF, Liquidity State |
| 4 | Volatility Alignment | Are vol conditions consistent? | ATR, Bollinger Width, BBWP, Squeeze |
| 5 | Structure Alignment | Do TFs describe compatible structures? | S/R levels, Swings, Fibonacci |
| 6 | Signal Alignment | Do market events reinforce? | Breakouts, Pullbacks, Retests, Divergences |
| 7 | Regime Alignment | Do TFs identify same environment? | Trending/Ranging/Expansion/Contraction |
| 8 | Confidence Alignment | Is analytical confidence consistent? | MarketContext per-TF confidence |
| 9 | Liquidity Alignment | Are liquidity conditions consistent? | RVOL, VWAP, volume profile |
| 10 | Opportunity Alignment | Do TFs agree about opportunity? | Local bias + regime per TF |

### Condition

Requires ≥2 timeframes. With 1 TF, Alignment is defined but marked N/A.

### UI Panel: Alignment Panel

`AlignmentPanel.svelte` — 10 alignment dimension gauge bars, timeframe
breakdown table, trend agreement % visual, cross-TF signal count.

---

## Stage 3: Analysis Matrix

### Definition

The **Analysis Matrix** transforms structured observations and multi-timeframe
relationships into a complete interpretation of current market conditions.
It represents the transition from "Market Observation" to "Market Understanding."

```
Question: "Given everything currently observed, what is the complete
           interpretation of this market?"
```

### Structure

```rust
AnalysisMatrix {
    symbol: String,
    bias: MarketBias,                     // StrongBullish/Bullish/Neutral/Bearish/StrongBearish
    confidence: f64,                       // 0.0–1.0
    market_regime: MarketRegime,           // TrendingBull/TrendingBear/Range/Accumulation/Distribution/Expansion/Contraction/Transition
    trend_assessment: TrendAssessment,     // Quality: Weak/Developing/Healthy/Strong/Exhausted
    momentum_assessment: MomentumAssessment, // Increasing/Stable/Weakening/Exhausted/Reversing
    structure_assessment: StructureAssessment, // Strong/Healthy/Weak/Broken/Unclear
    volatility_assessment: VolatilityAssessment, // Compressed/Normal/Expanding/Extreme/Unstable
    volume_assessment: VolumeAssessment,   // Weak/Normal/Strong/Exceptional
    opportunity_analysis: OpportunityAnalysis, // TrendContinuation/Breakout/Pullback/MeanReversion/Reversal/NoClear
    market_quality: MarketQuality,         // Poor/Weak/Average/Good/Excellent
    market_interpretation: String,         // human-readable summary
    rationale: String,
    supporting_signals: Vec<String>,
    contradicting_signals: Vec<String>,
    timeframes_considered: u8,
}
```

### Derivation

- **Bias**: from Alignment `mtf_overall_score` (±20 thresholds)
- **Regime**: from Metrics MarketContext (TRENDING/RANGE/EXPANSION/COMPRESSION)
- **Trend Quality**: from ADX + EMA stack state
- **Momentum**: from MACD histogram trend + RSI slope
- **Structure**: from S/R label state + proximity
- **Volatility**: from BBWP percentile
- **Volume**: from RVOL magnitude
- **Opportunity**: rule-based: strong trend+bullish=bullish trend; compression+squeeze=breakout; range=mean reversion
- **Quality**: aggregate of alignment confidence, signal freshness, liquidity
- **Interpretation**: generated from all component states

### Condition

Requires Alignment Matrix (≥2 timeframes). Consumes both Metrics Matrices
and Alignment Matrix.

### UI Panel: Analysis Panel

`AnalysisPanel.svelte` — Bias badge, confidence bar, regime label, trend/
momentum/structure/volatility/volume assessment cards, opportunity analysis,
market quality gauge, market interpretation text, supporting/contradicting
signals, rationale.

---

## Stage 4: Risk Matrix

### Definition

The **Risk Matrix** evaluates the level of uncertainty surrounding the
current market interpretation. Risk is a property of an interpretation,
not of raw observations. It consumes the Analysis Matrix — you cannot
evaluate how risky a bullish trend is without first determining that
there IS a bullish trend.

```
Question: "How dangerous is the current market interpretation?"
```

Risk is independent from market direction. A bullish market can be high risk.
A bearish market can be low risk.

### Structure

```rust
RiskDimension {
    score: f64,          // 0-100 (higher = riskier)
    level: RiskLevel,    // VeryLow/Low/Moderate/High/Extreme
    state: RiskState,    // Stable/Increasing/Elevated/Critical/Improving
    confidence: f64,     // 0-100%
    evidence: Vec<String>,
}

RiskMatrix {
    symbol: String,
    market_risk: RiskDimension,         // conflicting signals, weak structure, low confidence
    volatility_risk: RiskDimension,     // ATR magnitude, BBWP extreme, Bollinger width, Squeeze
    liquidity_risk: RiskDimension,      // RVOL, VWAP participation, spread, depth bias
    structure_risk: RiskDimension,      // broken S/R, unclear levels, excessive distance
    momentum_risk: RiskDimension,       // exhausted/diverging momentum, rapid change rate
    signal_risk: RiskDimension,         // conflicting signals, low reliability
    execution_risk: RiskDimension,      // wide spread, fast movement, slippage potential
    reward_risk: RiskDimension,         // opportunity quality vs environmental uncertainty
    overall_risk: RiskDimension,        // weighted aggregation of all 8 dimensions
}
```

### 9 Risk Dimensions

| # | Dimension | What It Measures | Inputs |
|---|---|---|---|
| 1 | Market Risk | General uncertainty | Signal conflict, weak structure, low confidence |
| 2 | Volatility Risk | Abnormal price movement | ATR, BBWP, Bollinger Width, Squeeze state |
| 3 | Liquidity Risk | Participation quality | RVOL, VWAP, spread, depth bias, volume profile |
| 4 | Structure Risk | Price structure uncertainty | S/R proximity, swing structure, Fibonacci |
| 5 | Momentum Risk | Momentum vulnerability | Exhausted MACD, diverging RSI, rapid Stoch change |
| 6 | Signal Risk | Signal conflict | Opposing signals, low reliability score |
| 7 | Execution Risk | Practical difficulties | Wide spread, fast price movement, low participation |
| 8 | Reward Risk | Opportunity vs uncertainty | Analysis.opportunity × confidence / risk |
| 9 | Overall Risk | Combined risk assessment | Weighted aggregation of all 8 dimensions |

### Condition

Requires an Analysis Matrix. Risk consumes the market interpretation,
not raw indicators.

### UI Panel: Risk Panel

`RiskPanel.svelte` — 9 risk dimension cards each with score gauge, level badge,
state label, confidence %. Risk profile summary. Risk warnings list.

---

## Stage 5: Advisory Matrix

### Definition

The **Advisory Matrix** transforms complete market intelligence and risk
assessment into structured human-facing guidance. It consumes the Analysis
Matrix and Risk Matrix to provide an explainable recommendation framework.

```
Question: "Given the current market condition and associated risk, what is
           the most reasonable interpretation and possible action direction?"
```

It does not execute trades. It explains and recommends.

### Structure

```rust
AdvisoryMatrix {
    symbol: String,
    directional_guidance: DirectionalGuidance,       // StrongLong/Long/Neutral/Short/StrongShort/Avoid
    market_stance: MarketStance,                     // Aggressive/Constructive/Neutral/Cautious/Avoid
    opportunity_classification: OpportunityClass,    // TrendContinuation/Breakout/Pullback/MeanReversion/Reversal/NoClear
    strategy_environment: StrategyEnvironment,       // TrendFollowing/Breakout/MeanReversion/HighVol/LowActivity/Unfavorable
    entry_guidance: EntryGuidance,                   // Immediate/WaitForConfirmation/Pullback/Breakout/NoEntryContext
    exit_guidance: ExitGuidance,                     // TrendWeakening/MomentumExhaustion/StructureBreakdown/RiskIncreasing/NoWarning
    stop_loss_guidance: StopLossGuidance,            // StructureBased/VolatilityBased/ATRBased/SRBased/NoRecommendation
    take_profit_guidance: TakeProfitGuidance,        // ResistanceBased/RRBased/VolatilityBased/TrailingMethod/NoRecommendation
    confidence_assessment: f64,                      // 0-100%
    final_recommendation: String,                    // human-readable
}
```

### Derivation

- **Directional Guidance**: from Analysis.bias × Risk.overall
- **Market Stance**: from Analysis.quality × Risk.overall
- **Opportunity Classification**: from Analysis.opportunity
- **Strategy Environment**: from Analysis.regime
- **Entry Guidance**: from Alignment trend_agreement + Analysis.momentum
- **Exit Guidance**: from Analysis.momentum + Risk.overall
- **Stop Loss**: from Risk.volatility + Analysis.structure
- **Take Profit**: from Analysis.structure + Risk.reward
- **Confidence**: from Analysis.confidence × (1 - Risk.overall/100)
- **Final Recommendation**: generated from all components

### Condition

Requires Analysis Matrix + Risk Matrix.

### UI Panel: Advisory Panel

`AdvisoryPanel.svelte` — Directional guidance badge, market stance label,
opportunity classification, strategy environment, entry/exit guidance,
stop loss/take profit guidance, confidence bar, final recommendation text.

---

## Stage 6: Overview Matrix

### Definition

The **Overview Matrix** is the final aggregation stage. It combines the
complete analytical state of all monitored assets into a unified representation
of the observed market environment. It consumes all Advisory Matrices.

```
Question: "What is happening across the entire monitored market?"
```

### Structure

```rust
OverviewMatrix {
    global_market_bias: GlobalBias,              // StrongBullish/Bullish/Neutral/Bearish/StrongBearish/Mixed
    market_breadth: MarketBreadth,               // VeryWeak/Weak/Balanced/Positive/StrongPositive/Negative/StrongNegative
    regime_distribution: HashMap<String, f64>,   // Regime → % of assets
    opportunity_distribution: HashMap<String, u32>, // Opportunity type → count + Quality
    risk_distribution: RiskDistribution,         // Low/Moderate/High % + RiskEnvironment label
    asset_ranking: Vec<AssetRank>,               // scored per asset
    market_synchronization: SyncLevel,           // HighlySynchronized/Synchronized/Mixed/Fragmented/HighlyFragmented
    market_health: HealthLevel,                  // Poor/Weak/Neutral/Healthy/Strong
    global_summary: String,                      // human-readable
    instance_count: u32,
    active_symbols: Vec<String>,
}
```

### 9 Components

| # | Component | What It Measures |
|---|---|---|
| 1 | Global Market Bias | Aggregated directional bias from all Advisory Matrices |
| 2 | Market Breadth | % of assets showing positive bias vs negative |
| 3 | Regime Distribution | Breakdown of regimes across all assets |
| 4 | Opportunity Distribution | Count per opportunity type + quality level |
| 5 | Risk Distribution | Low/Moderate/High % + Risk Environment class |
| 6 | Asset Ranking | Scored: opportunity × confidence × alignment / risk |
| 7 | Market Synchronization | How unified is market behavior across assets |
| 8 | Market Health | Aggregate of quality, breadth, synchronization |
| 9 | Global Summary | Human-readable overview |

### Condition

Requires at least one Advisory Matrix.

### UI Panel: General Dashboard

`GeneralDashboard.svelte` — Global bias badge, market breadth bar, regime
distribution pie/bars, asset ranking table, synchronization level, market
health gauge, per-symbol advisory summary cards, global summary text.

---

## Cross-References

- **Formal ontology and axes**: [ontology.md](ontology.md)
- **Complete indicator reference**: [metrics-matrix-reference.md](metrics-matrix-reference.md)
- **Master specification**: [indicator-system-master-spec.md](indicator-system-master-spec.md)
- **Implementation**: `shared/src/alignment.rs`, `analysis.rs`, `risk.rs`, `advisory.rs`, `overview.rs`
