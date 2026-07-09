# ISIL — Institutional Statistical Intelligence Layer

> **Layer 6 of 10 in the Institutional Trading Strategy Decision Pipeline.**
>
> **Implementation Status: COMPLETE** — All 6 modules (A-F), ML layer, derived features, Kalman filter implemented.
>
> **Purpose:** Transform historical OHLC and indicator data into statistical knowledge. Answer "How unusual is this?" rather than "What is the value?" Six modules (A-F) plus ML layer provide distribution context, empirical probabilities, confidence intervals, market shape analysis, cross-feature relationships, and Monte Carlo simulation.
>
> **Inputs:** ITIL (indicator history buffers), IDCL (decision metrics for enrichment), OHLCV price history from L0.
>
> **Outputs:** StatisticalContext with 50+ fields → IRML (risk percentile inputs, anomaly detection), IDCL (confidence enrichment), IASL (statistical summary in Analyst context).

## Institutional Specification for Quantitative Decision Support

### Version 1.0

---

## 1. Purpose

The current architecture already contains:

- Market data ingestion (Hyperliquid WebSocket)
- 51-indicator engine with normalization pipeline
- Signal engine (~250 discrete signals)
- Market Context (regime classification, trend/momentum/volatility dimensions)
- Decision Context (17 quantitative decision-support metrics)
- AI reasoning layer (5 domain agents + master orchestrator)

The missing component is not another indicator engine.

The missing component is a **Statistical Intelligence Layer (SIL)**.

This layer transforms historical OHLC information into statistical knowledge.

Instead of asking

> "What is the ATR?"

it answers

> "How unusual is today's ATR compared with the last 500 candles?"

Instead of

> "Price is above VWAP"

it answers

> "There is only a 7% historical probability that price extends another ATR after reaching this condition."

This layer provides objective statistical evidence instead of additional technical indicators.

---

## 2. Design Philosophy

The SIL satisfies five principles.

### Principle 1 — No New Market Data

Only use Open, High, Low, Close, and Volume from existing ingestion. No new exchange feeds, no external datasets, no off-chain oracles.

### Principle 2 — No Duplicate Indicators

Everything must derive from price history, returns, and existing indicator history. Never create another RSI-like indicator. The 51-indicator normalized map is the sole source of derived features.

### Principle 3 — Pure Mathematics

Every value must be deterministic. No AI. No heuristic scoring. No manual tuning parameters. Statistical quantities follow standard mathematical definitions.

### Principle 4 — Explainability

Every statistical value should answer **Why?** instead of "Because the algorithm says so." Each output includes provenance: which rolling window, which distribution, how many observations, and what percentile supports the claim.

### Principle 5 — Independent Layer

The SIL must not modify indicators, DecisionContext, MarketContext, or any existing struct. It is a read-only enrichment layer that attaches statistical context to the existing data flow.

---

## 3. Architecture

```
Market Data (WebSocket)
    │
    ▼
51-Indicator Engine (deterministic calculators)
    │
    ▼
Normalization Pipeline (NormalizedIndicatorValue map)
    │
    ▼
Signal Engine (~250 structured signals)
    │
    ▼
Market Context (regime, trend, momentum, volatility, volume, liquidity)
    │
    ▼
Decision Context (17 quantitative metrics)
    │
    ▼
┌──────────────────────────────────┐
│ Statistical Intelligence Layer   │  ← NEW
│ Modules A–F + ML + Derived       │
│ Enriches snapshot, does NOT       │
│ modify anything above this line   │
└──────────────────────────────────┘
    │
    ▼
AI Orchestrator (domain agents + master synthesis)
    │
    ▼
Trading Decision (advisory, manual execution)
```

### Data Flow

1. Each `MarketSnapshot` already carries `indicators` (51-item normalized map), `context` (MarketContext), and `decision_context` (DecisionContext).
2. The SIL consumes price history (from the `TimeframePipeline` OHLCV buffer), returns history (computed from close prices), and indicator history (already computed per-candle).
3. The SIL produces a `StatisticalContext` struct, attached as `statistical_context: Option<StatisticalContext>` on `MarketSnapshot`.
4. The snapshot is broadcast via WebSocket to the frontend and persisted to SQLite telemetry.
5. The AI orchestrator receives a compact `StatisticalContextSummary` in the `DeterministicTelemetry`, enriching every indicator reading with distribution context.

### Crate Location

```
crates/shared/src/statistics/     ← NEW module (pure math, no I/O)
├── mod.rs                        Module declarations, config structs
├── types.rs                      Shared types and enums
├── rolling_window.rs             RollingWindow<T> generic incremental buffer
├── distribution.rs               Module A: Distribution Statistics
├── probability.rs                Module B: Empirical Probability Engine
├── bayesian.rs                   Bayesian updating (Beta-Binomial conjugate)
├── confidence.rs                 Module C: Confidence & Prediction Intervals
├── market_shape.rs               Module D: Market Distribution Engine
├── relationship.rs               Module E: Relationship Engine
├── monte_carlo.rs                Module F: Monte Carlo Price-Path Simulation
├── online_learning.rs            Online learning (incremental frequency tables)
├── feature_importance.rs         Feature importance (mutual information)
├── clustering.rs                 Regime discovery (online k-means)
├── anomaly.rs                    Anomaly detection (Mahalanobis distance)
├── regime_classifier.rs          Statistical regime classification
├── derived_features.rs           Section 5: Decision-support derived features
├── statistical_object.rs         StatisticValue struct
└── statistical_context.rs        StatisticalContext output envelope

crates/shared/src/models.rs       → Add statistical_context field to MarketSnapshot
crates/engine/src/analyzer/mod.rs → Hook SIL computation after DecisionContext
crates/engine/src/server/telemetry.rs → Enrich DeterministicTelemetry with SIL
crates/engine/src/llm/prompts.rs  → Update orchestrator/system prompts
crates/engine/src/server/pipeline.rs → Enrich agent context with stats
crates/frontend/                  → SIL panel, indicator value enrichment
```

---

## 4. Categories — Six Independent Modules

---

### Module A: Distribution Statistics

**Purpose**: Describe where the current market lies inside its historical distribution.

**Outputs per metric** (price, returns, ATR, RSI, BBWP, squeeze momentum, volume, RVOL, ADX):

| Statistic | Definition |
|-----------|------------|
| Rolling mean | Arithmetic mean over window |
| Median | 50th percentile |
| Variance | σ² (Welford's incremental algorithm) |
| Standard deviation | σ |
| Percentile | Rank of current value in historical distribution |
| Z-score | (current - mean) / σ |
| Interquartile range (IQR) | Q3 - Q1 |
| Median absolute deviation (MAD) | Median(|xi - median|) |

**Questions answered**:
- Is today's volatility unusual?
- Is today's trend exceptional?
- Is price statistically stretched?
- Is RSI historically extreme?
- Is volume anomalous for this time of day?

**Multi-window support**: All statistics computed independently for 20, 50, 100, 250, and 500-candle windows. This avoids overfitting to one market speed.

---

### Module B: Probability Engine

**Purpose**: Estimate empirical probabilities from historical frequencies. Unlike indicators, these values come from counting — not formulas.

**Probabilities tracked**:

| Probability | Definition | Observation |
|-------------|------------|-------------|
| P(trend_continuation) | Fraction of times price remained above/below EMA_medium for N bars after crossing | N = 5 bars |
| P(mean_reversion) | Fraction of times price returned to EMA within N bars after deviation > 1.5σ | N = 5 bars |
| P(breakout_success) | Fraction of BB/SR breaks where price extended > 0.5 ATR in break direction | At candle close |
| P(reversal) | Fraction of RSI > 70 followed by > 0.5 ATR pullback within 5 bars | Or RSI < 30 for bullish |
| P(atr_expansion) | Fraction of times ATR increased next bar when BBWP > 80 | Conditional |
| P(squeeze_release_direction) | Direction count after squeeze releases | Bullish vs bearish split |
| P(volatility_expansion) | P(next bar ATR > current ATR \| current BBWP level) | Conditional |
| P(close_above_ema) | Empirical frequency by deviation distance from EMA | Bucketed by σ |
| P(stop_before_target) | Given distance to target vs stop, fraction where stop hit first | From candle history |

Each probability returns a tuple: `(probability, observation_count, confidence_in_estimate)`.

**Bayesian Updating** (Section 10): Each probability tracks a Beta-Binomial conjugate model:
- Prior: Beta(α=1, β=1) — uniform, equivalent to one pseudo-observation each way
- Update: after each candle, observe whether the event occurred
- Posterior mean: (α + successes) / (α + β + trials)
- 95% HDI: Highest Density Interval of the posterior distribution
- P(p > threshold): Posterior probability that the true probability exceeds a given threshold

This means probabilities become more confident as observations accumulate, and rapidly adjust when market regime shifts (old observations decay).

---

### Module C: Confidence Engine

**Purpose**: Quantify uncertainty. Every prediction includes confidence, uncertainty, and reliability.

**Outputs**:

| Output | Definition |
|--------|------------|
| Prediction interval (68%) | 16th–84th percentile of historical return distribution |
| Prediction interval (95%) | 2.5th–97.5th percentile |
| Prediction interval (99%) | 0.5th–99.5th percentile |
| Bootstrap confidence | 95% CI for any metric via resampling (1000 iterations) |
| Historical reliability | For each probability: "how often was a similar estimate followed by the predicted outcome?" |
| Confidence score | Composite [0, 100]: 35% PI tightness + 25% bootstrap CI + 25% reliability + 15% observation count |

**Example**: Instead of "Bullish Probability: 84%", the AI receives:

```
Bullish Probability: 84%
Confidence: 91%
Historical reliability: 88%
Bootstrap 95% CI: [76%, 92%]
```

---

### Module D: Market Distribution Engine

**Purpose**: Describe the shape of the market distribution.

**Outputs** over rolling returns window:

| Metric | Definition | Interpretation |
|--------|------------|----------------|
| Skewness | Third standardized moment | > 0.5 = bullish tail risk. < -0.5 = bearish tail risk |
| Kurtosis (excess) | Fourth standardized moment - 3 | > 2 = fat tails (chaotic). > 3 = extreme |
| Entropy | Shannon entropy of histogram (20 bins), normalized [0, 1] | > 0.7 = random, unpredictable. < 0.3 = highly structured |
| Distribution symmetry | \|skewness\| / (\|skewness\| + 1) | Near 1 = asymmetric, near 0 = symmetric |
| Tail risk | 99th percentile loss / mean loss | > 2 = extreme tail risk |
| Volatility percentile | HV's percentile in historical HV distribution | > 90 = explosive. < 10 = unusually calm |
| Compression percentile | BBWP's percentile in historical BBWP | > 90 = coiling energy |

**Market shape classification**:

| Label | Condition |
|-------|-----------|
| `normal` | \|skewness\| < 0.5, kurtosis < 2, entropy < 0.6 |
| `compressed` | BBWP_percentile > 90 OR squeeze_on |
| `explosive` | ATR_percentile > 90 AND (squeeze_release OR BBWP < 10) |
| `chaotic` | kurtosis > 3 OR tail_risk > 2σ |
| `rare` | price_z_score > 2.5 OR volatility_z_score > 2.5 |

---

### Module E: Relationship Engine

**Purpose**: Describe relationships between features. Detect when 50 indicators are all saying the same thing.

**Outputs** (rolling Pearson correlation, incremental Welford's method):

| Metric | Definition |
|--------|------------|
| RSI × MACD correlation | Momentum alignment across oscillators |
| ATR × BBWP correlation | Volatility consistency |
| OBV × Price correlation | Volume confirmation |
| RSI × StochK correlation | Oscillator redundancy |
| EMA_50 × EMA_200 | Trend alignment |
| ADX × Choppiness | Trend vs noise |
| Feature agreement | Weighted fraction of indicator pairs with same-direction signal |
| Indicator redundancy | 1 - (effective rank / total directional indicators) |
| Consensus stability | 1 - stddev(consensus metric over window) |
| Trend consistency | Autocorrelation of directional_bias at lag 1 |
| Momentum consistency | Inter-correlation of RSI, MACD, Stoch outcomes |

---

### Module F: Monte Carlo Engine (Async)

**Purpose**: Simulate possible future price paths using historical distribution. Not prediction — simulation.

**Design**: Runs asynchronously on a background tokio task. Triggered on each candle close if `config.statistics.monte_carlo_enabled = true`. Results from the most recently completed simulation are served; the current candle is never blocked waiting for Monte Carlo.

**Algorithm** (naive mode — no Kalman):
1. Build empirical joint distribution of (return, volume_factor, atr_factor) tuples from rolling history
2. Resample with replacement: N paths × M steps forward
3. Price evolves: `P[t+1] = P[t] × exp(return_sample × atr_factor)`
4. For each path, track maximum drawdown, maximum favorable excursion, and final price
5. Compute summary statistics across all N paths

**Algorithm** (Kalman drift-aware mode — `kalman_enabled = true`):
1. A 1D Kalman filter (local linear trend) runs per candle on log-price, producing a filtered **drift** estimate (expected per-bar log-return) and a **residual** series (raw return − drift = pure noise)
2. Build empirical distribution of residuals (de-drifted returns) from rolling history
3. For each MC step: `P[t+1] = P[t] × exp((drift + noise_sample) / 100.0)`
4. The drift provides directional bias; residuals provide stochastic dispersion
5. Same target/stop/drawdown tracking as naive mode

**Outputs**:

| Output | Definition |
|--------|------------|
| P(target_hit) | Fraction of paths reaching target before stop |
| P(stop_hit) | Fraction reaching stop before target |
| max_drawdown_95 | 95th percentile of path drawdowns |
| max_favorable_excursion_95 | 95th percentile of favorable movement |
| expected_movement | Mean final price deviation (signed) |
| best_case | 95th percentile of final outcomes |
| worst_case | 5th percentile of final outcomes |
| median_outcome | 50th percentile |
| confidence_95_range | (5th, 95th) percentile interval |

**Configuration** (default: naive mode disabled, Kalman enabled):

```toml
[statistics]
monte_carlo_enabled = false
monte_carlo_paths = 1000
monte_carlo_steps = 50
monte_carlo_target_atr_mult = 2.0
monte_carlo_stop_atr_mult = 1.5
monte_carlo_interval_seconds = 300

# Kalman drift estimation (runs per-candle, feeds MC)
kalman_enabled = true
kalman_process_noise = 0.00001
kalman_measurement_noise = 0.001
kalman_residual_window = 100
```

---

### Module F.2: Kalman Filter Drift Estimation

**Purpose**: Decompose the log-price series into signal (drift/trend slope) and noise (residuals). The Kalman-estimated drift replaces naive mean-zero resampling in the Monte Carlo engine, producing directionally-aware path simulations.

**Model**: 1D local linear trend Kalman filter.

- **State vector**: `x = [level (log-price), slope (per-bar log-return)]`
- **Measurement**: observed log-price at each candle close
- **Process noise Q**: allows the state (especially slope) to evolve over time — higher values make the filter more responsive to recent changes
- **Measurement noise R**: how much of each observed price is "noise" — higher values make the filter smoother

**Per-candle update**: `predict → measure → correct` cycle. The filter incrementally refines its estimate of the current drift (slope component of the state vector).

**Outputs**:

| Output | Definition | Range |
|--------|------------|-------|
| `kalman_drift` | Filtered per-bar log-return (% per bar), annualized | ±50% typical |
| `kalman_noise_vol` | Rolling standard deviation of residuals (return − drift) | 0–10% typical |
| `kalman_trend_strength` | Signal-to-noise ratio: \|drift\| / noise_vol | 0–5 typical; > 1 = trending |

**Why residuals matter**: Residuals are the de-drifted return series. They have lower variance than raw returns because the directional component has been stripped out. Resampling from residuals + adding back the Kalman drift produces MC paths that:
- Have correct directional bias (trending up → paths trend up)
- Have realistic dispersion (the residual distribution captures real market noise)
- Converge to random walk when drift ≈ 0 (ranging market)

**Computational**: O(1) per candle (two 2×2 matrix operations). No lookup tables. Runs inline in `StatisticsEngine::advance()` — unlike MC which runs on a background task.

---

## 5. Machine Learning Layer (Section 10)

Without external datasets, machine learning remains lightweight and incremental.

### 5.1 Online Learning

**File**: `online_learning.rs`

After each completed candle, update all empirical frequency tables:

```rust
pub struct OnlineLearner {
    event_counts: HashMap<EventKey, (usize, usize)>,  // (occurred, total)
    feature_outcome_history: VecDeque<(Vec<f64>, f64)>, // rolling (features, next_return)
    regime_history: VecDeque<(String, f64)>,             // rolling (regime, forward_return)
}
```

Events tracked: trend_continuation, breakout_success, reversal, atr_expansion, squeeze_release, stop_hit, target_hit.

### 5.2 Feature Importance

**File**: `feature_importance.rs`

Uses incremental mutual information estimation:
- For each indicator feature (normalized value), discretize into 3 bins (high > 0.3, neutral [-0.3, 0.3], low < -0.3)
- Compute MI with forward N-bar returns (N = 5)
- Rank indicators by MI. Top N (default 5) are "most predictive for this market regime"

```rust
pub struct FeatureImportanceTracker {
    pub scores: Vec<(String, f64)>,  // (indicator_key, importance_score)
    pub top_n: usize,
}
```

### 5.3 Clustering — Regime Discovery

**File**: `clustering.rs`

Online k-means with 5 centroids mapping to regime archetypes:
- **Trending Up**: high ADX, positive directional_bias, low chop, returns > 0
- **Trending Down**: high ADX, negative directional_bias, low chop, returns < 0
- **Ranging**: low ADX, high chop, low momentum magnitude, entropy > 0.6
- **Volatile**: high ATR_percentile, high RVOL, high kurtosis
- **Compressed**: high BBWP_percentile, low ATR, squeeze_on
- **Transition**: regime changed within last N bars (detected via rolling max change)

```rust
pub struct RegimeClusterer {
    pub centroids: Vec<Vec<f64>>,   // 5-6 centroids in feature space
    pub labels: Vec<String>,        // human-readable labels
    pub current_regime: String,
    pub regime_stability: f64,      // bars_since_change / window_size
}
```

### 5.4 Anomaly Detection

**File**: `anomaly.rs`

For each metric in a multivariate feature vector, compute Mahalanobis distance from the historical mean:
- Build incremental covariance matrix (Welford's method for multivariate)
- Anomaly score = 1 - exp(-distance² / threshold)
- Score > 0.8 = anomalous market. > 0.95 = extreme outlier

```rust
pub struct AnomalyDetector {
    pub aggregate_score: f64,                      // [0, 1]
    pub per_metric_scores: HashMap<String, f64>,
    pub top_anomaly_reason: String,                // "ATR is 3.2σ above mean"
}
```

### 5.5 Statistical Regime Classification

**File**: `regime_classifier.rs`

Differs from `MarketContext.regime` (which is indicator-based). This uses distribution statistics:
- **Trending**: returns_skewness × direction > 0.3 AND trend_persistence > 0.6
- **Ranging**: autocorrelation < 0.1 AND chop > 60 AND ADX < 20
- **Volatile**: returns_kurtosis > 2 AND ATR_percentile > 85
- **Compressed**: BBWP_percentile > 85 AND ATR_percentile < 20
- **Transition**: regime changed within last 20 bars

**Avoided ML approaches**: deep learning, neural networks, transformers, random forests, gradient boosting — these require labels and much larger datasets.

---

## 6. Derived Features (Section 5)

Instead of exposing raw mathematics, create decision-support features that become AI inputs:

| Feature | Formula | Range | Interpretation |
|---------|---------|-------|----------------|
| Market Stretch Score | price_z_score × volatility_percentile / 100 | [-1, +1] | +1 = extremely stretched bullish. -1 = stretched bearish |
| Trend Reliability | trend_persistence × consensus_stability × (1 - entropy) | [0, 1] | How reliable is the current trend signal? |
| Momentum Stability | 1 - stddev([rsi_norm, macd_norm, stoch_norm]) | [0, 1] | Are momentum indicators consistent? |
| Volatility Shock Prob | 1 - CDF(current_ATR \| historical_ATR_distribution) | [0, 1] | Probability that current volatility is a shock |
| Compression Probability | P(BBWP > 85th %ile within next 5 bars) | [0, 1] | Probability of imminent compression |
| Expansion Probability | P(BBWP < 15th %ile within next 5 bars) | [0, 1] | Probability of imminent expansion |
| Reversal Probability | Bayesian posterior for reversal event | [0, 1] | From Bayesian tracker |
| Continuation Probability | Bayesian posterior for continuation event | [0, 1] | From Bayesian tracker |
| Breakout Confidence | RVOL_score × (1 - P_false_breakout) × (1 - redundancy) | [0, 1] | > 0.7 = genuine breakout likely |
| Trend Confidence | trend_reliability × bayesian_posterior_mean | [0, 1] | Combined trend conviction |
| Risk Confidence | 1 - (tail_risk × anomaly_score) | [0, 1] | > 0.8 = normal risk. < 0.3 = elevated risk |
| Expected Opportunity | expected_movement × P_target_hit - E_drawdown × P_stop_hit | [-1, +1] | Positive = asymmetric edge |
| Market Predictability | 1 - entropy | [0, 1] | > 0.7 = highly predictable. < 0.3 = essentially random |
| Kalman Trend Strength | \|drift\| / noise_vol | [0, +∞) | > 1 = trend-dominated. < 0.3 = noise-dominated |

---

## 7. Historical Windows

Everything exists on five rolling horizons, mapped to the existing multi-timeframe architecture:

| Window | Timeframe | Horizon | Use Case |
|--------|-----------|---------|----------|
| 20 | Micro (60s) | 20 minutes | Short-term noise, scalp-level statistical extremes |
| 50 | Fast (180s) | 2.5 hours | Intraday rhythm, session-level distribution |
| 100 | Slow (300s) | ~8 hours | Session-scale, single-day profile |
| 250 | Macro (900s) | ~62 hours | Multi-day patterns, weekly cycle |
| 500 | Macro (900s) | ~5 days | Weekly distribution, regime transitions |

Each `RollingWindow` tracks one metric x one window size. The `DistributionTracker` holds 5 windows per metric and exposes multi-horizon statistics.

---

## 8. Statistical Objects

Every statistic exposes a consistent `StatisticValue` struct:

```rust
pub struct StatisticValue {
    pub current: f64,          // Current raw value
    pub mean: f64,             // Rolling mean over window
    pub stddev: f64,           // Rolling standard deviation
    pub percentile: f64,       // [0, 100] — rank in historical distribution
    pub z_score: f64,          // (current - mean) / stddev
    pub confidence: f64,       // [0, 1] — 1 / (1 + cv), where cv = stddev / mean
    pub trend: String,         // "increasing", "decreasing", "stable"
}
```

Instead of:

```
ATR: 1.82
```

The system produces:

```
ATR
  current:    1.82
  mean:       1.20
  percentile: 93
  z-score:    2.11
  trend:      increasing
  confidence: high
```

---

## 9. `StatisticalContext` — Complete Output Envelope

```rust
pub struct StatisticalContext {
    // ── Module A: Distribution ──
    pub price_stats: StatisticValue,
    pub return_stats: StatisticValue,
    pub atr_stats: StatisticValue,
    pub rsi_stats: StatisticValue,
    pub bbwp_stats: StatisticValue,

    // ── Module D: Market Shape ──
    pub skewness: f64,
    pub kurtosis: f64,
    pub entropy: f64,
    pub tail_risk: f64,
    pub distribution_symmetry: f64,
    pub market_shape_label: String,      // "normal"|"compressed"|"explosive"|"chaotic"|"rare"
    pub volatility_percentile: f64,
    pub compression_percentile: f64,

    // ── Module B: Probabilities ──
    pub trend_continuation_prob: f64,
    pub mean_reversion_prob: f64,
    pub breakout_success_prob: f64,
    pub reversal_prob: f64,
    pub atr_expansion_prob: f64,
    pub squeeze_release_prob: f64,
    pub volatility_expansion_prob: f64,
    pub stop_before_target_prob: f64,
    pub observation_counts: HashMap<String, usize>,

    // ── Module C: Confidence ──
    pub prediction_interval_68: (f64, f64),
    pub prediction_interval_95: (f64, f64),
    pub prediction_interval_99: (f64, f64),
    pub bootstrap_confidence_95: (f64, f64),
    pub historical_reliability: f64,
    pub confidence_score: f64,           // [0, 100]

    // ── Module E: Relationships ──
    pub feature_agreement: f64,          // [0, 1]
    pub indicator_redundancy: f64,       // [0, 1]
    pub consensus_stability: f64,        // [0, 1]
    pub trend_consistency: f64,          // [-1, 1] autocorrelation
    pub momentum_consistency: f64,       // [0, 1]

    // ── Module F: Monte Carlo ──
    pub mc_target_hit_prob: f64,
    pub mc_stop_hit_prob: f64,
    pub mc_max_drawdown_95: f64,
    pub mc_max_favorable_excursion_95: f64,
    pub mc_expected_movement: f64,
    pub mc_best_case: f64,
    pub mc_worst_case: f64,
    pub mc_median_outcome: f64,
    pub mc_confidence_95_range: (f64, f64),

    // ── Module F.2: Kalman Drift ──
    pub kalman_drift: f64,              // annualized filtered drift (%)
    pub kalman_noise_vol: f64,          // residual volatility
    pub kalman_trend_strength: f64,     // signal-to-noise ratio |drift| / noise_vol

    // ── ML Layer ──
    pub regime_label: String,                        // statistical regime classification
    pub regime_stability: f64,                       // [0, 1]
    pub anomaly_score: f64,                          // [0, 1] — 1 = extreme anomaly
    pub top_anomaly_reason: String,
    pub top_predictive_indicators: Vec<(String, f64)>, // (name, importance)
    pub bayesian_posteriors: HashMap<String, (f64, f64, f64)>, // event → (mean, hdi_low, hdi_high)

    // ── Derived Features ──
    pub market_stretch_score: f64,      // [-1, +1]
    pub trend_reliability: f64,         // [0, 1]
    pub momentum_stability: f64,        // [0, 1]
    pub volatility_shock_prob: f64,     // [0, 1]
    pub compression_probability: f64,   // [0, 1]
    pub expansion_probability: f64,     // [0, 1]
    pub breakout_confidence: f64,       // [0, 1]
    pub trend_confidence: f64,          // [0, 1]
    pub risk_confidence: f64,           // [0, 1]
    pub expected_opportunity: f64,      // [-1, +1]
    pub market_predictability: f64,     // [0, 1]
}
```

---

## 10. Transport & Integration

### 10.1 MarketSnapshot

`MarketSnapshot` gains a new field:

```rust
pub statistical_context: Option<StatisticalContext>,
```

This auto-serializes through the existing JSON blob pipeline:
- WebSocket broadcast to frontend at `/ws`
- SQLite telemetry persistence
- `/api/history` REST endpoint

### 10.2 AI Orchestrator Integration

The `DeterministicTelemetry` struct gains a `StatisticalContextSummary`:

```rust
pub struct StatisticalContextSummary {
    pub price_percentile: f64,
    pub volatility_percentile: f64,
    pub market_shape: String,
    pub anomaly_score: f64,
    pub top_anomaly: String,
    pub trend_reliability: f64,
    pub breakout_confidence: f64,
    pub reversal_prob: f64,
    pub continuation_prob: f64,
    pub confidence_95_range: (f64, f64),
    pub market_predictability: f64,
    pub expected_opportunity: f64,
    pub top_predictors: Vec<(String, f64)>,
    pub risk_confidence: f64,
    pub kalman_drift: f64,
    pub kalman_noise_vol: f64,
    pub kalman_trend_strength: f64,
}
```

The master orchestrator prompt gains a `STATISTICAL CONTEXT` section that instructs the LLM to interpret distribution statistics as context for indicator readings.

### 10.3 Per-Indicator Enrichment

When building agent context in Phase 1 (pipeline.rs), each indicator reading includes its statistical profile:

```
RSI
  raw: 72
  percentile: 96
  z-score: 2.3
  historically exceeded: 4% of observations
  momentum persistence: high
  mean reversion probability: 68%
```

---

## 11. Configuration

New `[statistics]` section in `config.toml`:

```toml
[statistics]
enabled = true
windows = [20, 50, 100, 250, 500]
probability_min_observations = 30
probability_forward_bars = 5

# Monte Carlo (async, disabled by default)
monte_carlo_enabled = false
monte_carlo_paths = 1000
monte_carlo_steps = 50
monte_carlo_target_atr_mult = 2.0
monte_carlo_stop_atr_mult = 1.5
monte_carlo_interval_seconds = 300

# Kalman drift estimation (per-candle, feeds MC when enabled)
kalman_enabled = true
kalman_process_noise = 0.00001
kalman_measurement_noise = 0.001
kalman_residual_window = 100

# Online learning
online_learning_enabled = true
feature_importance_top_n = 5
clustering_regimes = 5
anomaly_threshold = 0.8

# Bayesian priors
bayesian_prior_alpha = 1.0
bayesian_prior_beta = 1.0
```

---

## 12. Computational Constraints (Principle 11)

Every computation follows these constraints:

| Constraint | Implementation |
|------------|---------------|
| Incremental | All statistics update in O(1) per candle (Welford's algorithm) |
| Streaming | VecDeque<T> — push front, pop back when at capacity |
| Rolling | Five independent window sizes per metric |
| Deterministic | Fixed seed for Monte Carlo when configured. No external entropy for stats. |
| Constant-memory | Each RollingWindow is bounded (capacity × sizeof(T)). No unbounded growth. |
| No expensive recomputation | Mean/variance/covariance/correlation are incremental. Percentile/median require O(n) sort per query only. |
| Async where needed | Monte Carlo runs on background task. Main pipeline never blocks. |

**Memory budget** (per timeframe):
- 5 windows × 12 metrics × 500 entries × 8 bytes = ~240KB for raw values
- Plus ancillary covariance/correlation matrices: ~20KB
- Total per timeframe: ~300KB. × 4 timeframes per pair: ~1.2MB per trading pair.

---

## 13. Explainability Standard (Principle 4)

Every value must answer **Why?** through composition:

```
Expansion Probability: 82%

Why?
  ATR percentile:          91 (extremely elevated)
  BBWP percentile:         97 (near-maximum compression)
  Historical squeeze releases: 81% of past releases at similar ATR/BBWP profiles
  Current volatility:      2.3 standard deviations above rolling mean
  Market shape:            compressed → expansion imminent
```

The `StatisticalContext` struct includes all sub-components. The frontend and AI orchestrator can decompose any derived feature into its constituent inputs for full explainability.

---

## 14. DecisionContext Integration

DecisionContext remains unchanged. The SIL enriches it:

**Before (current)**:
```
Trade Readiness: 82%
```

**After (enhanced)**:
```
Trade Readiness:        82%
  Historical confidence: 91%
  Bootstrap confidence:  88%
  Expected range:        2.4%
  95% interval:          1.8–3.0%
  Continuation probability: 71%
  Maximum expected drawdown: 1.2%
  Market predictability: 0.78
  Risk confidence:       0.85
```

---

## 15. Expected Benefits

Implementing the Statistical Intelligence Layer shifts the system from a collection of technical indicators to a quantitative decision engine:

- **Probabilistic context** instead of directional signals: every recommendation includes "How likely is this outcome, and how confident are we?"
- **Confidence estimates**: distinguish high-conviction setups from noisy signals
- **Historical rarity**: flag unusual market conditions ("This RSI has only been seen 4% of the time")
- **Uncertainty measurements**: prediction intervals, bootstrap CIs, historical reliability
- **Simulated outcomes**: Monte Carlo provides "What if?" scenarios without needing labels
- **Bayesian updating**: probabilities become more accurate as observations accumulate
- **No new data**: relies solely on the existing OHLCV feed — no external datasets required
- **AI improvement**: richer statistical context improves the LLM's ability to justify decisions
- **Disciplined risk**: market shape, anomaly detection, and risk confidence support sizing
- **Deterministic and efficient**: incremental, constant-memory, suitable for real-time execution

---

## 16. Testing Strategy

| Suite | Command | Tests | Coverage |
|-------|---------|-------|----------|
| Core property | `./manage.sh test-core` | Rolling window invariants, distribution monotonicity, probability convergence, Monte Carlo reproducibility | Phase 1–6 |
| Engine integration | `./manage.sh test-engine` | SIL pipeline produces context for completed candles, snapshot serialization round-trip | Phase 1 |
| E2E | `./manage.sh test-e2e` | Full analytical loop with SIL enrichment, history endpoint returns StatisticalContext | Phase 9 |
| UI | `./manage.sh test-ui` | SIL panel renders, indicator enrichment displays | Phase 10 |

**Property test designs**:
- Rolling window: mean recovery from uniform distribution, variance bounded by empirical
- Distribution: percentile monotonicity (x > y → %ile(x) > %ile(y)), z-score N(0,1) for Gaussian data
- Probability: convergence rate with increasing sample size, Bayesian posterior shrinks toward truth
- Monte Carlo: seed reproducibility, path count equals configured N

---

## 17. Files Modified vs. Files Created

| Action | File | Change |
|--------|------|--------|
| NEW | `crates/shared/src/statistics/kalman.rs` | Kalman filter drift estimation (1D local linear trend) |
| EDIT | `crates/shared/src/lib.rs` | Add `pub mod statistics;` |
| EDIT | `crates/shared/src/models.rs` | Add `statistical_context: Option<StatisticalContext>` to `MarketSnapshot` |
| EDIT | `crates/engine/src/config/models.rs` | Add `StatisticsConfig` struct |
| EDIT | `crates/engine/src/config/mod.rs` | Parse `[statistics]` section |
| EDIT | `crates/engine/src/analyzer/mod.rs` | Hook `StatisticsEngine::advance()` after `DecisionContext::compute()` |
| EDIT | `crates/engine/src/server/types.rs` | Add `StatisticalContextSummary` to `DeterministicTelemetry` and `MonitorResponse` |
| EDIT | `crates/engine/src/server/telemetry.rs` | Compile SIL summary into telemetry |
| EDIT | `crates/engine/src/llm/prompts.rs` | Add `STATISTICAL CONTEXT` section to orchestrator prompts |
| EDIT | `crates/engine/src/server/pipeline.rs` | Enrich agent context with distribution stats |
| EDIT | `config.toml` | Add `[statistics]` section |
| NEW | `crates/frontend/src/components/StatisticalPanel.svelte` | SIL dashboard panel |
| EDIT | `crates/frontend/src/types.ts` | Add `StatisticalContext` TypeScript interface |
| EDIT | `crates/frontend/src/App.svelte` | Add SIL tab to workspace navigation |

**Files NOT modified** (Principle 5 compliance):
- `crates/shared/src/indicators/` — zero changes to any indicator calculator or normalizer
- `crates/shared/src/decision_context.rs` — zero changes
- `crates/shared/src/market_context.rs` — zero changes
- `crates/shared/src/indicators/registry.rs` — zero new registry entries

---

## 18. Rollout Checklist

- [x] Phase 0: Design documentation (this file)
- [x] Phase 1: Foundation — `RollingWindow`, Module A (Distribution Statistics), `StatisticalContext`, pipeline hook
- [x] Phase 2: Module B (Probability Engine) + `BayesianTracker`
- [x] Phase 3: Module C (Confidence Engine)
- [x] Phase 4: Module D (Market Distribution Engine)
- [x] Phase 5: Module E (Relationship Engine)
- [x] Phase 6: Module F (Monte Carlo Engine, async)
- [x] Phase 6b: Module F.2 (Kalman Filter Drift Estimation, per-candle incremental)
- [x] Phase 7: Machine Learning (online learning, feature importance, clustering, anomaly, regime)
- [x] Phase 8: Derived Features
- [x] Phase 9: AI Integration (telemetry enrichment, prompt updates, agent context enrichment)
- [x] Phase 10: Frontend (SIL panel, indicator enrichment, `/api/statistics` endpoint)
- [x] Testing: core property tests, engine integration, E2E, UI
- [x] Config: `config.toml` `[statistics]` section populated

---

## 19. Advanced Risk Modeling Extension

The ISIL Advanced Risk Modeling extension adds four sub-modules:

| Module | File | Description |
|--------|------|-------------|
| **VaR/CVaR** | `crates/shared/src/statistics/var.rs` | Historical Value at Risk and Conditional VaR at 95/99% confidence |
| **GARCH(1,1)** | `crates/shared/src/statistics/garch.rs` | Conditional volatility forecasting capturing volatility clustering |
| **EVT (POT/GPD)** | `crates/shared/src/statistics/evt.rs` | Extreme Value Theory tail risk via Peaks-Over-Threshold + Generalized Pareto Distribution |
| **Information Coefficient** | `crates/shared/src/statistics/information_coeff.rs` | Spearman rank correlation between signals and forward returns |

See [ISIL Advanced Risk Modeling](06-isil-advanced-risk-modeling.md) for full specification.
