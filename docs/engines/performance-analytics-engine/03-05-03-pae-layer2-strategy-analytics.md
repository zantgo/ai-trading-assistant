# PAE Layer 2 — Strategy Analytics Layer

**Version:** 2.0
**Status:** Approved
**Engine:** Performance Analytics Engine (PAE)
**Layer:** 2 of 4
**Input Contract:** Trade Analytics Matrix (L1)
**Output Contract:** Strategy Analytics Matrix (grouped performance metrics + statistical significance)
**Purpose:** This document specifies the Strategy Analytics Layer — the strategy-level aggregation and significance testing system that groups closed trades by execution policy and computes mathematical significance against random market noise.

---

## 1. Purpose

The Strategy Analytics Layer determines whether the trading system generates a **statistically significant edge** over random chance. It groups trades by their originating execution policy, computes standard performance metrics, and runs Null Hypothesis Significance Testing (NHST) plus Monte Carlo sign-randomization.

```
[Trade Analytics Matrix] ──► STRATEGY ANALYTICS (L2) ──► [Strategy Analytics Matrix] ──► [Risk Analytics (L3)]
```

---

## 2. Strategy Analytics Matrix Schema

| Field | Type | Description |
|-------|------|-------------|
| `policy_id` | `string` | Originating execution policy. |
| `total_trades` | `u32` | Number of closed trades under this policy. |
| `win_count` | `u32` | Profitable trades (net PnL > 0). |
| `loss_count` | `u32` | Losing trades (net PnL < 0). |
| `win_rate` | `f64` | `win_count / total_trades`. |
| `gross_profit` | `Decimal` | Sum of all profitable trade net PnLs. |
| `gross_loss` | `Decimal` | Sum of all losing trade net PnLs (absolute). |
| `profit_factor` | `f64` | `gross_profit / gross_loss`. |
| `average_win` | `Decimal` | Mean net PnL of winning trades. |
| `average_loss` | `Decimal` | Mean net PnL of losing trades. |
| `avg_win_loss_ratio` | `f64` | `|average_win| / |average_loss|`. |
| `expectancy` | `Decimal` | Expected net return per trade: `(win_rate × avg_win) − ((1−win_rate) × avg_loss)`. |
| `slippage_overhead` | `f64` | Combined drag of slippage and fees as percentage of gross PnL. |
| `t_statistic` | `f64` | Student's T-Test statistic against $H_0$: $\mu = 0$. |
| `p_value` | `f64` | Probability of observing the strategy's returns if $H_0$ is true. |
| `p_mc` | `f64` | Monte Carlo empirical significance — fraction of sign-randomized samples whose mean return meets or exceeds actual performance. |
| `monte_carlo_runs` | `u32` | Number of Monte Carlo sign-randomization samples executed. |
| `is_significant` | `bool` | `true` if `p_value < 0.05` AND `p_mc < 0.05`. |

---

## 3. Statistical Significance Testing

### 3.1 Null Hypothesis ($H_0$)

$$H_0: \mu_{\text{returns}} = 0 \quad \text{(Strategy returns are indistinguishable from random noise)}$$

$$H_1: \mu_{\text{returns}} \neq 0 \quad \text{(Strategy generates a statistically significant edge)}$$

### 3.2 T-Statistic

$$t = \frac{\bar{x} - 0}{s / \sqrt{n}}$$

where $\bar{x}$ = mean trade return, $s$ = standard deviation of returns, $n$ = number of trades.

### 3.3 Monte Carlo Sign-Randomization Testing

The null hypothesis $H_0$ is that the strategy has **zero directional edge** — i.e., each trade's outcome is as likely to have been a win as a loss. This is tested by randomizing the **signs** of the realized returns, not their order. (Merely shuffling the order of a fixed set of PnL values leaves the sum and mean unchanged, so a permutation-of-order test would yield $p_{mc} = 1.0$ by construction and is invalid here.)

1. Collect the sequence of closed-trade net PnL values $\{x_1, \dots, x_n\}$.
2. Generate $N$ randomized samples (default: 10,000). For each sample, multiply every trade's PnL by an independent fair-coin sign $\varepsilon_i \in \{-1, +1\}$, producing $\{\varepsilon_1 x_1, \dots, \varepsilon_n x_n\}$.
3. For each randomized sample, compute the mean return.
4. Count randomized samples where the randomized mean ≥ the actual strategy mean.
5. $p_{mc} = \frac{\text{count}}{N}$

A low $p_{mc}$ (< 0.05) confirms the strategy's positive mean return is unlikely to arise from a zero-edge process (random win/loss direction on the same trade magnitudes).

---

## 4. Performance Classification

| Criteria | Classification |
|----------|---------------|
| `profit_factor > 2.0 AND win_rate > 50% AND p_value < 0.01 AND p_mc < 0.01` | **Strong Edge** |
| `profit_factor > 1.5 AND win_rate > 45% AND p_value < 0.05 AND p_mc < 0.05` | **Moderate Edge** |
| `profit_factor > 1.2 AND p_value < 0.10` | **Weak Edge** |
| `profit_factor < 1.0 OR p_value > 0.10` | **No Edge / Negative** |
| `total_trades < 30` | **Insufficient Data** |

---

## 5. Interaction with Regime Mapping

Strategy performance is segmented by the market regime active at trade entry (from MME Analysis Matrix). This segmentation feeds the [Performance Layer (L4)](03-05-05-pae-layer4-performance.md) for regime-compatibility mapping.

---

## 6. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Statistical rigor** | Both parametric (T-Test) and non-parametric (Monte Carlo) tests are applied. |
| **Minimum sample size** | NHST requires ≥ 30 trades; below this, significance fields are null with a warning. |
| **Deterministic randomization** | Monte Carlo sign-randomization uses a fixed random seed for reproducibility. |

---

## 7. Cross-References

- [PAE Overview](../performance-analytics-engine/03-05-01-pae-overview-spec.md) — Engine boundaries and scheduled tasks.
- [PAE Layer 1 — Trade Analytics](03-05-02-pae-layer1-trade-analytics.md) — Upstream data source.
- [PAE Layer 3 — Risk Analytics](03-05-04-pae-layer3-risk-analytics.md) — Drawdown and risk-adjusted metrics.
- [PAE Layer 4 — Performance](03-05-05-pae-layer4-performance.md) — Regime compatibility mapping.
- [Ontology — Performance Analytics](../../conceptual-foundations/01-01-ontology.md) — Conceptual definitions.
