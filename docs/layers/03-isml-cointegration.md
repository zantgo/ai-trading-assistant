# ISML-C / ISIL-C — Pair Cointegration Analysis

> **Cross-Layer Extension (ISM Layer 3 / ISIL Layer 6).**
> **Implementation Status: PLANNED** — Engle-Granger 2-step, Johansen test, OU half-life, pair spread analysis.
>
> **Parents:** [03-isml-structure-mapping.md](03-isml-structure-mapping.md) (structural context), [06-isil-statistical-intelligence.md](06-isil-statistical-intelligence.md) (statistical engine).

---

## Purpose

The existing ISIL Relationship Engine (Module E) computes pairwise Pearson correlations between indicators and returns. Correlation measures linear co-movement.

Cointegration answers a different question:

> **Do two price series share a long-run equilibrium relationship, such that deviations from this equilibrium are temporary and mean-reverting?**

While correlation tells you "do A and B move together?", cointegration tells you "if A and B diverge, will they come back together?" This is the statistical foundation of pairs trading and mean-reversion strategies.

**Why it matters:**
- Two assets can have zero correlation but be cointegrated (and vice versa)
- Cointegrated pairs provide mean-reversion trade signals: buy the underperformer, sell the overperformer
- The half-life of mean reversion tells you expected trade duration
- Spread z-scores provide entry/exit triggers

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| Price series per pair | L0 OHLCV buffers (1000 candles) | Log prices (ln(P)) per pair |
| Trading pairs list | Config `symbols` | String identifiers |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| Cointegration status per pair | `CointegrationResult { cointegrated: bool, hedge_ratio: f64, spread_mean: f64, half_life: f64 }` | ICSL (confluence context), IASL (structure analysis) |
| Spread z-score | f64 (current spread deviation from mean) | Frontend (pair trading signals) |
| Half-life of mean reversion | f64 (bars to halve deviation) | IEPL (expected trade duration) |
| Johansen rank + eigenvectors | `Vec<f64>` cointegrating vectors | Portfolio optimizer (long-run relationship weights) |

---

## Sub-Components

---

### A. Engle-Granger 2-Step Test

The simplest cointegration test for two assets. Computationally lightweight, no matrix library needed.

**Step 1: Estimate long-run relationship**
```
y_t = α + β·x_t + ε_t
```
OLS regression of one log-price series on the other. The residual ε_t is the "spread."

**Step 2: Test residuals for stationarity (Augmented Dickey-Fuller)**
```
Δε_t = γ·ε_{t-1} + Σ(δ_i·Δε_{t-i}) + u_t  (lags = 5)
Test H₀: γ = 0 (unit root = no cointegration)
ADF statistic = γ̂ / SE(γ̂)
```
If ADF statistic < critical value → reject H₀ → residuals are stationary → cointegrated.

**Critical values (MacKinnon 1994, 2-variable case):**
| Confidence | Critical Value |
|-----------|----------------|
| 90% | −3.04 |
| 95% | −3.34 |
| 99% | −3.96 |

**Hedge ratio:** β from the regression. For a $1 long in y, short β units of x.

**Spread z-score:**
```
z_score = (ε_current − με) / σε
```
- z > 2.0: y overvalued relative to x (short y, long x)
- z < −2.0: y undervalued relative to x (long y, short x)
- z near 0: pair is in equilibrium

---

### B. Johansen Test (Multi-Asset, requires nalgebra)

A more general test that can detect multiple cointegrating relationships among N > 2 assets.

**Procedure:**
1. Estimate VAR in levels: `X_t = A₁X_{t-1} + ... + A_kX_{t-k} + ε_t`
2. Transform to VECM form: `ΔX_t = ΠX_{t-1} + Σ(Γ_i·ΔX_{t-i}) + ε_t`
3. Eigenvalue decomposition of Π matrix (rank = number of cointegrating relationships)
4. Trace statistic: `λ_trace(r) = −T·Σ(ln(1−λ_i))` for i=r+1...n
5. Max-eigenvalue statistic: `λ_max(r) = −T·ln(1−λ_{r+1})`

**Output:**
```rust
JohansenResult {
    rank: usize,                        // number of cointegrating relationships
    eigenvalues: Vec<f64>,              // ordered eigenvalues
    trace_statistics: Vec<f64>,         // per rank
    max_eigenvalue_statistics: Vec<f64>,// per rank
    eigenvectors: Vec<Vec<f64>>,        // cointegrating vectors
    significant_at_95: Vec<bool>,       // per rank significance
}
```

**Requires `nalgebra`** for eigenvalue decomposition (`SymmetricEigen`). Pure Rust, zero C dependencies.

---

### C. Ornstein-Uhlenbeck Half-Life

Mean-reverting spread follows an Ornstein-Uhlenbeck process:

```
dS_t = θ·(μ − S_t)·dt + σ·dW_t
```

Where θ is the speed of mean reversion. The half-life is:

```
half_life = ln(2) / θ = ln(2) / |slope of ε_{t-1} in ADF regression|
```

**Interpretation:**
| Half-Life (bars) | Mean-Reversion Speed | Suitability |
|-----------------|---------------------|-------------|
| < 5 | Very fast | Scalping pairs |
| 5 – 20 | Fast | Intraday pairs |
| 20 – 50 | Moderate | Swing pairs |
| 50 – 100 | Slow | Position pairs |
| > 100 | Too slow / likely spurious | Not tradeable |

**Trading application:**
- Enter when z-score crosses ±2.0
- Expected hold time ≈ half_life × 2 (entry to reversion)
- Exit when z-score returns to 0 (or crosses opposite threshold for reversal signals)

---

### D. Multi-Pair Cointegration Tracker

Since cointegration is inherently cross-pair (not per-pair like most statistics), the tracker lives at the engine level:

```rust
CointegrationTracker {
    pair_combinations: Vec<(String, String)>,          // all pair combinations
    eg_results: HashMap<(String, String), EgResult>,   // cached Engle-Granger results
    johansen_result: Option<JohansenResult>,           // cached Johansen (all pairs)
    last_update_candle: u64,
    update_interval: 100,                               // recompute every 100 candles
}
```

Recalculated every 100 candles (configurable). Results cached and served via API.

---

## Integration

### Feeds Into
- **ICSL (Layer 4)** — Cointegration status provides additional confluence context (pairs trading signal)
- **IASL (Layer 8)** — Analyst Agent receives cointegration summary for structure analysis
- **IEPL (Layer 9)** — Half-life informs expected trade duration; z-score provides entry timing
- **IRML (Layer 7)** — Pairs trade risk incorporates hedge ratio for net exposure calculation
- **Frontend** — Pair spread chart with z-score bands in StatisticalPanel

### Receives From
- **L0 (Market Data)** — Price series per pair from OHLCV buffers
- **ISIL (Layer 6)** — Returns data for residual diagnostics

### Cross-References
- [ISIL: §E Relationship Engine](../layers/06-isil-statistical-intelligence.md) — Correlation computation (complementary to cointegration)
- [IEPL: §A Entry Protocol](../layers/09-iepl-execution-protocol.md) — Cointegration z-score as entry trigger
- [IRML: §8 Capital Allocation](../layers/07-irmL-risk-management.md) — Hedge ratio for net exposure

---

## Configuration

```toml
[cointegration]
enabled = false                     # disabled by default (cross-pair, computationally heavier)
test_method = "engle_granger"       # "engle_granger" | "johansen"
update_interval_bars = 100          # recompute every 100 candles
adf_max_lags = 5                    # ADF lag order
zscore_entry_threshold = 2.0        # |z| > 2.0 = entry signal
zscore_exit_threshold = 0.5         # |z| < 0.5 = exit
min_half_life_bars = 5              # ignore pairs with half-life < 5 (noise)
max_half_life_bars = 200            # ignore pairs with half-life > 200 (spurious)
```

---

## Verification

| Test | Verifies |
|------|----------|
| `test_eg_cointegrated_synthetic` | Engle-Granger detects cointegration in known cointegrated series |
| `test_eg_random_walk_not_cointegrated` | Independent random walks fail cointegration test |
| `test_hedge_ratio_recovery` | β estimated correctly from known relationship |
| `test_half_life_known_ou` | Half-life matches known OU process parameter |
| `test_zscore_stationary` | Spread z-score oscillates around 0 for cointegrated pair |
| `test_johansen_rank_recovery` | Johansen correctly identifies number of cointegrating vectors |
| `test_eigenvalue_ordering` | Eigenvalues correctly ordered descending |
