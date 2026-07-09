# IRML-E — Institutional Risk Management: Stress Testing & Portfolio Optimization

> **Layer 7 Extension of the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: PLANNED** — Stress Testing Framework + Markowitz Mean-Variance Optimization.
>
> **Parent:** [07-irmL-risk-management.md](07-irmL-risk-management.md) — New sub-components.

---

## Purpose

The existing IRML provides 6 risk categories, capital allocation tiers, and a Bayesian R:R engine. Two extensions add further rigor:

> **Stress Testing:** "What happens to the portfolio in extreme, predefined adverse scenarios?"

> **Mean-Variance Optimization:** "What is the mathematically optimal allocation across trading pairs given their expected returns, volatilities, and correlations?"

Stress testing answers "what if?" — scenarios that historical data may not contain (black swans). MVO answers "how much should go where?" — portfolio-level allocation beyond Kelly and Risk Parity.

---

## Part 1: Stress Testing Framework

### Inputs

| Input | Source | Format |
|-------|--------|--------|
| Current positions | `paper_trades` DB | `ActivePaperPosition` per pair |
| Portfolio equity | `portfolio_equity_history` | Current HWM + equity |
| Historical returns | ISIL `DistributionTracker` | 500-bar rolling return windows per pair |
| Historical ATR | ITIL `atr` indicator | f64 per pair |
| Pair correlations | ISIL Module E | Pairwise Pearson correlation matrix |
| Funding rates | ITIL `funding_rate` indicator | f64 per pair |

### Sub-Component A: Scenario Definitions

Five predefined stress scenarios, each representing a distinct market crisis archetype:

| Scenario | Trigger | Price Shock | Vol Shock | Correlation | Description |
|----------|---------|-------------|-----------|-------------|-------------|
| **Flash Crash** | Instant −5σ move | 5× σ_daily in 1 minute | — | 0.95 all pairs | Liquidity cascade; all assets crash together |
| **Vol Spike** | Max historical ATR | ±2× ATR (direction of current position) | 3× current ATR | Normal | Sudden volatility expansion; stops gapped |
| **Correlation Breakdown** | All-or-nothing move | Each pair moves 3σ against position | 2× current vol | 0.95 | Diversification fails; all positions move together |
| **Trend Reversal** | Largest 5-bar reversal | Largest counter-trend move in 500-bar history | 2.5× current ATR | Normal | Sudden structural regime shift |
| **Funding Crisis** | Funding rate inversion | 3× current funding × position size | — | — | Funding payments spike against position direction |

### Sub-Component B: Scenario Computation

For each scenario and each position:

```
stress_pnl[scenario][pair] = position_size × direction × price_shock_pct × leverage
stress_margin[pair] = |stress_pnl[pair]| + current_margin[pair]
scenario_total_pnl = Σ(stress_pnl) + cumulative_funding × funding_shock
```

**Output per scenario:**
```
StressTestResult {
    scenario_name: "Flash Crash"
    total_portfolio_pnl: −$3,420.00     // total loss
    pnl_pct_equity: −13.7%              // loss as % of equity
    worst_pair: "BTC"                   // heaviest hit pair
    worst_pair_pnl: −$1,800.00
    margin_call: true                   // would margin be breached?
    liquidation: false                  // would position be liquidated?
    margin_buffer_pct: 12.3%            // remaining margin buffer after shock
}
```

### Sub-Component C: Aggregate Stress Score

```
stress_score = max(|scenario_pnl_pct|) across all scenarios
```

| Score | Level | Recommendation |
|-------|-------|----------------|
| < 5% | Low | No action needed |
| 5% – 15% | Moderate | Consider reducing correlated exposure |
| 15% – 30% | High | Reduce position sizes immediately |
| > 30% | Critical | Close riskiest position; review portfolio |

---

## Part 2: Markowitz Mean-Variance Optimization

### Purpose

The existing portfolio optimization uses **Kelly Criterion** (position sizing per trade given win rate and R:R) and **Risk Parity** (equal risk contribution across pairs). Markowitz MVO adds the classical framework:

> **Efficient Frontier:** For every target return level, find the minimum-volatility portfolio.

> **Tangency Portfolio:** The portfolio on the efficient frontier with the highest Sharpe ratio.

### Sub-Component D: Efficient Frontier Computation

**Inputs:**
- Returns matrix: `returns[n_assets][n_periods]`
- Expected returns vector: `μ = mean(returns)` per asset
- Covariance matrix: `Σ = Cov(returns)` (n_assets × n_assets)

**Computation (grid search over return targets):**
```
For each target return r*:
  1. Solve: min w′Σw  subject to  w′μ = r*,  w′1 = 1,  w ≥ 0
  2. For 2-10 assets without a QP solver:
     - Use the two-fund separation theorem
     - The minimum-variance frontier is spanned by any two frontier portfolios
     - Compute: w_g = Σ⁻¹·1 / (1′Σ⁻¹·1)  (global minimum variance)
     - Compute: w_h = Σ⁻¹·μ / (1′Σ⁻¹·μ)  (maximum return)
     - Any frontier portfolio: w = α·w_g + (1−α)·w_h
  3. Grid α from 0 to 1 for n_points evenly spaced portfolios
```

**Matrix operations** require `nalgebra` (added as dependency in Phase 19).

**Output:**
```rust
EfficientFrontier {
    points: Vec<FrontierPoint>,     // efficient frontier curve
    min_variance_portfolio: (f64, f64, Vec<f64>),  // (return, vol, weights)
    tangency_portfolio: Option<(f64, f64, f64, Vec<f64>)>, // (return, vol, sharpe, weights)
    current_allocation: (f64, f64, f64),  // (return, vol, sharpe) of current portfolio
}
```

### Sub-Component E: Integration with Existing Allocation

The IRML allocation engine (`portfolio_optimizer.rs`) gains a new allocation method:

| Method | Tunable | Formula | Best For |
|--------|---------|---------|----------|
| `static` | No | Equal weight per pair | Baseline |
| `kelly_risk_parity` | Yes (kelly_fraction) | Risk Parity weights × Kelly fraction per pair | Adaptive sizing with risk budgeting |
| `mean_variance` | Yes (risk_aversion λ) | w = Σ⁻¹·(μ − r_f·1) / (λ·1′Σ⁻¹·1) | Optimal theoretical allocation |

The method is selected in `config.toml`:

```toml
[portfolio]
allocation_method = "mean_variance"    # or "kelly_risk_parity" or "static"
risk_free_rate = 0.02                  # for Sharpe ratio
risk_aversion = 2.0                    # λ — higher = more conservative
mv_rebalance_interval_bars = 100       # recompute frontier every 100 candles
```

### Sub-Component F: Constraints

Standard portfolio constraints enforced:

| Constraint | Default | Description |
|-----------|---------|-------------|
| `∑w_i = 1` | Mandatory | Fully invested |
| `w_i ≥ 0` | Mandatory | Long-only (no shorting in this context) |
| `w_i ≤ max_allocation` | 0.40 | No single pair > 40% |
| `σ_p ≤ max_portfolio_vol` | 0.30 | Portfolio vol capped at 30% annualized |
| `|w_current − w_target| ≤ turnover_limit` | 0.20 | Max 20% turnover per rebalance |

---

## Integration

### Feeds Into
- **IRML §8 (Capital Allocation)** — Stress score adjusts exposure tier. MVO weights compete with Kelly weights.
- **IRML §16 (Execution Constraints)** — Stress test results feed hard limit violations.
- **Frontend** — Stress test panel in `RiskManagementPanel.svelte`. Efficient frontier chart in same panel.
- **IPEL (Layer 10)** — Stress test results logged in performance journal.

### Receives From
- **ISIL (Layer 6)** — Return history, covariance, correlations.
- **IRML** — Current positions, allocation, exposure tier.

### Cross-References
- [IRML: §8 Capital Allocation Engine](07-irmL-risk-management.md) — Consumer of stress score and MVO weights
- [IRML: §16 Execution Constraints](07-irmL-risk-management.md) — Hard limits informed by stress results
- [ISIL: §E Relationship Engine](../layers/06-isil-statistical-intelligence.md) — Covariance and correlation data source

---

## Configuration

```toml
[stress_test]
enabled = true
scenarios = ["flash_crash", "vol_spike", "correlation_breakdown", "trend_reversal", "funding_crisis"]
flash_crash_sigma = 5.0
vol_spike_atr_mult = 3.0
trend_reversal_bars = 5

[portfolio]
allocation_method = "kelly_risk_parity"
risk_free_rate = 0.02
risk_aversion = 2.0
mv_rebalance_interval_bars = 100
mv_max_single_allocation = 0.40
mv_max_portfolio_vol = 0.30
mv_turnover_limit = 0.20
```

---

## Verification

| Test | Verifies |
|------|----------|
| `test_flash_crash_pnl_negative` | Stress P&L is negative for long-biased portfolio |
| `test_margin_call_detected` | Margin breach flagged correctly |
| `test_vol_spike_respects_position_direction` | Shock direction opposes current position |
| `test_efficient_frontier_monotonic` | Higher return → higher volatility (non-decreasing) |
| `test_tangency_max_sharpe` | Tangency portfolio has highest Sharpe among frontier points |
| `test_mvo_weights_sum_to_one` | Portfolio weights sum to 1 |
| `test_mvo_long_only` | All weights ≥ 0 |
| `test_mvo_turnover_constrained` | Rebalance respects turnover limit |
