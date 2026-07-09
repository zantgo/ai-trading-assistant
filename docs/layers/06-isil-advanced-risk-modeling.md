# ISIL-A — Institutional Statistical Intelligence: Advanced Risk Modeling

> **Layer 6 Extension of the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: PLANNED** — VaR/CVaR, GARCH(1,1), EVT, and Spearman IC.
>
> **Parent:** [06-isil-statistical-intelligence.md](06-isil-statistical-intelligence.md) — Module A (Distribution) and Module D (Market Shape) extensions.

---

## Purpose

The existing ISIL computes distribution statistics (percentile, z-score), market shape (skewness, kurtosis, entropy), and empirical probabilities from historical data. These are descriptive — they answer "how unusual is this compared to the past?"

The Advanced Risk Modeling extension answers a deeper question:

> **What is the probabilistic maximum loss, what will volatility be next period, and how extreme are the tails beyond what history has observed?**

Four new sub-modules extend ISIL:

1. **VaR/CVaR Engine** — Probabilistic loss quantification (ISIL §A.2)
2. **GARCH(1,1) Volatility Forecasting** — Conditional volatility prediction (ISIL §D.2)
3. **EVT Tail Risk (Peaks-Over-Threshold)** — Extreme tail modeling beyond historical percentiles (ISIL §D.3)
4. **Spearman Information Coefficient** — Signal quality measurement (ISIL §B.2)

All operate on the existing return history already maintained by the `StatisticsEngine`.

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| Log returns history | ISIL `DistributionTracker` (metric index 1) | 5 rolling windows (20, 50, 100, 250, 500) |
| Confluence score history | ICSL per-candle output | f64 values per bar |
| Forward returns | Computed from price buffer | N-bar forward log returns (N = 5) |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| `var_95`, `var_99` | f64 (loss as positive %) | IRML (capital allocation), IASL (risk context), Frontend (StatisticalPanel) |
| `cvar_95`, `cvar_99` | f64 (expected loss beyond VaR %) | IRML (tail risk adjustment), Frontend |
| `garch_forecast_vol` | f64 (1-bar forecast %) | IEPL (dynamic stop width), IDCL (expected_volatility enhancement) |
| `garch_long_run_vol` | f64 (unconditional vol %) | IRML (regime-aware vol baseline) |
| `garch_persistence` | f64 (α+β, [0,1]) | Frontend (volatility regime indicator) |
| `evt_var_99` | f64 (EVT-based 99% VaR %) | IRML (superior tail estimate for crypto fat tails) |
| `evt_expected_shortfall_99` | f64 (EVT-based ES %) | IRML (worst-case capital reserve) |
| `evt_tail_index` | f64 (ξ, GPD shape parameter) | Frontend (tail heaviness indicator; ξ>0 = heavy-tailed, ξ<0 = bounded) |
| `evt_scale` | f64 (β, GPD scale parameter) | Frontend (tail dispersion) |
| `ic_spearman` | f64 ([-1,+1]) | IPEL (signal quality tracking) |
| `ic_rank` | f64 ([0,1]) | Frontend (confluence score reliability gauge) |

---

## Sub-Components

---

### A. VaR/CVaR Engine

**Purpose:** Quantify probabilistic maximum loss at specified confidence levels.

**Formula:**

```
Historical VaR α% = Percentile(returns, 1−α)    // negative tail mapped to positive loss
Historical CVaR α% = Mean(returns ≤ VaR_α)       // expected shortfall
```

The `RollingWindow` already exposes `sorted_values()` for precise percentile calculation.

**Output struct:**
```rust
VarCvarSummary {
    var_95: f64,   // "There is a 5% chance of losing ≥ X% in one period"
    var_99: f64,   // "There is a 1% chance of losing ≥ X% in one period"
    cvar_95: f64,  // "When the 5% worst case happens, average loss is X%"
    cvar_99: f64,  // "When the 1% worst case happens, average loss is X%"
}
```

**Interpretation:** VaR answers "how bad can it get?" CVaR answers "when it gets bad, how bad?" CVaR is always ≥ VaR and is a coherent risk measure (sub-additive).

**Code location:** `crates/shared/src/statistics/var.rs`

---

### B. GARCH(1,1) Volatility Forecasting

**Purpose:** Replace static historical volatility with a conditional, forward-looking volatility forecast that captures volatility clustering.

**Model:**
```
σ²_t = ω + α·ε²_{t-1} + β·σ²_{t-1}

Where:
  ε_t = r_t − μ     (return residual, μ = mean return)
  ω   = baseline variance
  α   = reaction to recent shock (ARCH term, "news impact")
  β   = persistence of past volatility (GARCH term, "memory")
  α+β = persistence parameter (< 1 for stationarity; ≈ 1 = long memory)
```

**Parameter estimation (method of moments):**
1. Compute unconditional variance: σ² = Var(r)
2. Compute autocorrelation of squared returns at lag 1: ρ₁ = Corr(ε²_t, ε²_{t-1})
3. Initial estimate: α = ρ₁, β = 1 − α, ω = σ² × (1 − α − β)
4. Refine iteratively (5 passes, adjusting α and β toward stationarity)

**Per-candle update:**
```
ε_new = r_new − μ
σ²_new = ω + α·ε²_current + β·σ²_current
```

**Multi-step forecast:**
```
σ²_{t+h} = ω / (1−α−β) + (α+β)^h × (σ²_t − ω/(1−α−β))
```
As h→∞, forecast converges to unconditional variance (long-run vol).

**Output struct:**
```rust
GarchForecast {
    current_vol: f64,       // σ_t, current period
    forecast_1bar: f64,     // E[σ_{t+1}]
    forecast_5bar: f64,     // E[σ_{t+5}]
    long_run_vol: f64,      // ω/(1−α−β), unconditional
    persistence: f64,       // α+β, near 1 = long memory
}
```

**Integration:** The GARCH forecast replaces/extends the existing HV-based `expected_volatility` in IDCL. IEPL dynamic stop multipliers can use GARCH forecast instead of raw ATR.

**Code location:** `crates/shared/src/statistics/garch.rs`

---

### C. EVT Tail Risk (Peaks-Over-Threshold)

**Purpose:** Historical percentiles (used in plain VaR) are biased for extremes — you cannot observe a 1-in-500 event from 500 data points. EVT fits a Generalized Pareto Distribution to tail exceedances, enabling extrapolation beyond observed extremes.

**Method: Peaks-Over-Threshold (POT)**
1. Select threshold `u` at specified percentile (default: 95th of negative returns)
2. Extract exceedances: `X_i = u − r_i` for all `r_i < u` (positive exceedances)
3. Fit GPD to exceedances via Probability-Weighted Moments (PWM)
4. Compute EVT-based VaR and Expected Shortfall

**GPD (Generalized Pareto Distribution):**
```
F(x) = 1 − (1 + ξ·x/β)^(−1/ξ)    for ξ ≠ 0
     = 1 − exp(−x/β)              for ξ = 0 (exponential)

ξ = shape (tail index):  ξ>0 = heavy-tailed (Pareto), ξ=0 = exponential, ξ<0 = bounded
β = scale
```

**PWM (Probability-Weighted Moments) Estimation:**
```
Method: b_r = E[X·F(X)^r]
For GPD with ξ > −1:
  b₀ = E[X]
  b₁ = E[X·(1−F(X))]
  ξ̂ = (b₀/b₁ − 2) / (−1)  =  2 − b₀/b₁  (simplified)
  β̂ = 2·b₀·b₁ / (b₀ − 2·b₁)
```

PWM is preferred over MLE because:
- Closed-form (no iterative optimizer)
- Robust for small samples
- Works for ξ ∈ (−1, 0.5] (covers most financial data)

**EVT-VaR formula:**
```
VaR_α = u + (β/ξ) × [(n/Nu × (1−α))^(−ξ) − 1]

Where:
  n  = total observations
  Nu = number of exceedances
```

**EVT-ES formula:**
```
ES_α = VaR_α / (1−ξ) + (β − ξ·u) / (1−ξ)
```

**Output struct:**
```rust
EvtTailMetrics {
    var_99: f64,                    // EVT-based 99% VaR
    expected_shortfall_99: f64,     // EVT-based 99% ES
    tail_index_xi: f64,             // ξ (shape)
    scale_beta: f64,                // β (scale)
    threshold: f64,                 // u (POT threshold)
    exceedance_count: usize,        // number of observations beyond threshold
}
```

**Why EVT matters for crypto:** Crypto returns have ξ > 0 (heavy-tailed). Historical VaR systematically underestimates risk. EVT extrapolates into the unobserved tail, producing more conservative and realistic risk estimates.

**Code location:** `crates/shared/src/statistics/evt.rs`

---

### D. Spearman Information Coefficient

**Purpose:** Measure whether trading signals (confluence score, indicator values) actually contain predictive information about future returns.

**Definition:**
```
Spearman IC = Corr(rank(signal_t), rank(forward_return_{t+N}))
```
Ranks are used instead of raw values to be robust to outliers and non-linear relationships.

**Rolling tracker:**
```rust
IcTracker {
    predictions_history: VecDeque<f64>,  // last 50 confluence scores
    outcomes_history: VecDeque<f64>,     // last 50 forward 5-bar returns
    lookback: usize,                     // 50
}
```

**Output:**
```rust
IcMetrics {
    spearman_ic: f64,        // rank correlation [-1, +1]
    rank: f64,               // |spearman_ic| normalized to [0, 1]
    significance: f64,       // approximate p-value (t-statistic based)
}
```

**Interpretation:**
| IC Range | Signal Quality |
|----------|---------------|
| > 0.10 | Strong predictive power (rare) |
| 0.05 – 0.10 | Good predictive power |
| 0.02 – 0.05 | Weak but non-zero |
| < 0.02 | Essentially random |
| Negative | Counter-predictive — signal points wrong way |

**Usage in IPEL:** Declining IC triggers a warning in the adaptive learning feedback loop — the system's signals are losing predictive power and weights may need recalibration.

**Code location:** `crates/shared/src/statistics/information_coeff.rs`

---

## Integration

### Feeds Into
- **IRML (Layer 7)** — VaR/CVaR feeds capital allocation guardrails. EVT-VaR provides superior tail risk estimate. GARCH forecast replaces static ATR for dynamic stop placement.
- **IDCL (Layer 5)** — GARCH forecast enhances `expected_volatility`. EVT-VaR provides better risk context than simple percentile.
- **IEPL (Layer 9)** — GARCH forecast for dynamic ATR multiplier selection in stop placement hierarchy.
- **IPEL (Layer 10)** — IC tracking feeds adaptive learning feedback loop.
- **IASL (Layer 8)** — All metrics injected into Analyst Agent for narrative context ("Volatility is elevated: GARCH forecasts 3.2% vs. long-run 2.1%").

### Receives From
- **ISIL (Layer 6)** — Log returns from `DistributionTracker`, rolling window infrastructure, per-candle pipeline entry point (`advance_ext`).

### Cross-References
- [ISIL: §A Distribution Statistics](06-isil-statistical-intelligence.md) — Data source for all modules
- [IRML: §7 Position Risk Profile](../layers/07-irmL-risk-management.md) — Consumer of VaR and EVT tail metrics
- [IRML: §12 Adaptive R:R Engine](../layers/07-irmL-risk-management.md) — Related probabilistic risk framework
- [IDCL: §Expected Volatility](../layers/05-idcl-decision-context.md) — Consumer of GARCH forecast
- [IEPL: §C Stop-Loss Placement](../layers/09-iepl-execution-protocol.md) — Consumer of GARCH-based dynamic stop sizing

---

## Configuration

New fields in `config.toml [statistics]` section:

```toml
[statistics]
# VaR/CVaR (Phase 16)
var_confidence_levels = [0.95, 0.99]

# GARCH (Phase 16)
garch_enabled = true
garch_estimation_window = 252
garch_max_iterations = 5

# EVT (Phase 16)
evt_enabled = true
evt_threshold_percentile = 0.95

# Information Coefficient (Phase 16)
ic_enabled = true
ic_lookback = 50
ic_forward_bars = 5
```

---

## Verification

| Test | Verifies |
|------|----------|
| `test_var_95_exceeds_var_99` | VaR monotonicity (higher confidence = larger loss) |
| `test_cvar_exceeds_var` | CVaR always ≥ VaR |
| `test_garch_stationary` | α+β < 1 for stationary process |
| `test_garch_forecast_converges` | Multi-step forecast converges to unconditional vol |
| `test_evt_heavy_tail` | ξ > 0 for heavy-tailed returns (crypto) |
| `test_evt_var_exceeds_historical` | EVT-VaR > historical VaR for heavy tails |
| `test_evt_pwm_positive_scale` | GPD scale parameter β > 0 |
| `test_ic_symmetry` | IC(a,b) = −IC(b,a) |
| `test_ic_perfect_correlation` | Perfect linear signal → IC = ±1 |
| `test_ic_independent` | Random signal → IC ≈ 0 |
