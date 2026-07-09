# IPEL-E — Institutional Performance Evaluation: Factor Attribution

> **Layer 10 Extension of the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: PLANNED** — 1-factor alpha/beta decomposition, rolling factor exposures.
>
> **Parent:** [10-ipel-performance-evaluation.md](10-ipel-performance-evaluation.md) — New sub-component H.

---

## Purpose

The existing IPEL computes performance metrics (Win Rate, Profit Factor, Sharpe, Sortino) and tracks direction correctness. These metrics measure **absolute** performance.

Factor attribution answers a deeper question:

> **How much of the P&L came from market exposure (beta), and how much came from skill (alpha)?**

This decomposition is essential for:
- Separating luck from skill (was the profit from a rising market, or from good trades?)
- Understanding when to reduce exposure (high beta in a bearish regime = risk)
- Evaluating signal quality independently of market direction
- Attribution reporting for strategy refinement

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| Trade-level returns | `paper_trades` DB | Realized P&L % per trade |
| Market returns (BTC as proxy) | ISIL log returns from BTC pair | f64 return per bar |
| Confluence scores at entry | ICSL | f64 per trade entry |
| Trade direction | `paper_trades` | LONG/SHORT |
| Regime at entry time | IRCL | Regime label |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| `alpha_annualized` | f64 (% annualized excess return) | Frontend (PerformanceDashboard), IPEL (adaptive learning) |
| `beta_to_btc` | f64 (sensitivity to market factor) | IRML (exposure calibration), Frontend |
| `r_squared` | f64 ([0,1] goodness of fit) | Frontend (model quality) |
| `attribution_ratio` | f64 (alpha_pnl / total_pnl) | Frontend (% of P&L from skill) |
| `rolling_beta` | Vec<f64> (trailing 100-bar beta) | Frontend (beta stability chart) |

---

## Sub-Components

---

### H. Factor Attribution Engine

#### H.1 1-Factor Market Model

The simplest and most interpretable factor model:

```
r_i = α + β·r_m + ε

Where:
  r_i  = asset/strategy return series
  r_m  = market factor return (BTC as crypto market proxy)
  α    = Jensen's alpha (excess return unexplained by market)
  β    = market exposure (sensitivity to bitcoin moves)
  ε    = idiosyncratic return (strategy-specific)
```

**OLS Estimation:**
```
β̂ = Cov(r_i, r_m) / Var(r_m)
α̂ = mean(r_i) − β̂·mean(r_m)
σ_ε = stddev(residuals)
R² = 1 − Var(ε) / Var(r_i)
```

No matrix library needed — single-variable OLS is closed-form.

#### H.2 Performance Attribution

```
Total PnL = Market PnL + Alpha PnL

Market PnL = β × cumulative_market_return × direction_at_entry
Alpha PnL  = Total PnL − Market PnL

Attribution Ratio = Alpha PnL / Total PnL
```

**Interpretation:**
| Attribution Ratio | Meaning |
|-------------------|---------|
| > 0.80 | Strategy is highly alpha-driven — skill dominates |
| 0.50 – 0.80 | Balanced — both market and skill contribute |
| 0.20 – 0.50 | Market-driven — most P&L explained by BTC direction |
| < 0.20 | Beta proxy — strategy IS the market |
| Negative | Alpha is negative — strategy underperforms market beta |

#### H.3 Rolling Factor Exposures

Beta is not constant — it varies with regime, volatility, and market conditions. Rolling estimation:

```
Rolling β_t = Cov(r_i[t−100:t], r_m[t−100:t]) / Var(r_m[t−100:t])
```

A stable β near 0.0 means the strategy is market-neutral. A β that oscillates between +1 and −1 means the strategy is regime-sensitive. A β consistently > 0.5 means the strategy is long-biased.

#### H.4 Regime-Conditional Alpha

Alpha decomposed by IRCL regime:

```
α_trending  = mean(r_i − β·r_m) for trades opened in Trending regime
α_range     = mean(r_i − β·r_m) for trades opened in Range regime
...
```

**Usage:** Identifies which market regimes the strategy adds genuine alpha in. If α > 0 in Trending but α < 0 in Range, this confirms the IRCL gate logic (trend-following trades in Trending are profitable; mean-reversion in Range needs improvement).

#### H.5 Signal Quality Decomposition

```
Signal Alpha = mean(r_i for trades where sign(confluence) == sign(realized_return))
Noise Alpha  = mean(r_i for trades where sign(confluence) ≠ sign(realized_return))
Signal Ratio  = Signal Alpha / (Signal Alpha + |Noise Alpha|)
```

When Signal Alpha dominates, the confluence scoring model is working. When Noise Alpha dominates, the signals are essentially random.

---

## Integration

### Feeds Into
- **IPEL §G (Adaptive Learning)** — Declining alpha or attribution ratio triggers weight recalibration recommendations
- **IRML §12 (R:R Engine)** — β informs market-dependent R:R requirements (higher β = higher correlation risk)
- **IRML §8 (Capital Allocation)** — High β strategies may warrant reduced allocation in bearish BTC regime
- **Frontend** — Alpha/beta chart in `PerformanceDashboard.svelte`

### Receives From
- **ISIL (Layer 6)** — Return data, BTC pair returns for factor computation
- **IPEL** — Trade P&L data from trade journal
- **ICSL (Layer 4)** — Confluence scores for signal quality decomposition

### Cross-References
- [IPEL: §C Performance Metrics](10-ipel-performance-evaluation.md) — Complement to standard metrics
- [IPEL: §F Regime Breakdown](10-ipel-performance-evaluation.md) — Feeds regime-conditional alpha
- [IPEL: §G Adaptive Learning](10-ipel-performance-evaluation.md) — Consumer of factor attribution insights
- [IRML: §12 Adaptive R:R](07-irmL-risk-management.md) — R:R adjustments from beta exposure

---

## Configuration

```toml
[statistics]
factor_model_enabled = true
factor_market_proxy = "BTC"          # which pair serves as market factor
factor_rolling_window = 100          # bars for rolling beta estimation
```

---

## Verification

| Test | Verifies |
|------|----------|
| `test_beta_market_neutral` | Strategy with β=0 correlation has β̂ ≈ 0 |
| `test_beta_perfect_correlation` | Strategy that perfectly tracks BTC has β̂ ≈ 1, α̂ ≈ 0 |
| `test_alpha_significance` | t-statistic for α computed correctly |
| `test_r_squared_bounds` | R² ∈ [0, 1] |
| `test_attribution_ratio_bounds` | Attribution ratio near 0 for pure beta, near 1 for pure alpha |
| `test_rolling_beta_monotonic` | Rolling beta tracks regime changes |
| `test_regime_alpha_decomposition` | Per-regime alpha computed correctly |
