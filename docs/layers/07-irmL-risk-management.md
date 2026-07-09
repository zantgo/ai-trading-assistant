# IRML — Institutional Risk Management Layer

> **Layer 7 of 10 in the Institutional Trading Strategy Decision Pipeline.**
>
> **Purpose:** The IRML is a deterministic gatekeeper placed between analysis and execution. It evaluates 6 risk categories, produces a position risk profile, manages drawdown state, and computes an adaptive reward/risk ratio that guarantees positive long-run expectancy. Risk never depends on AI reasoning — the IRML is pure mathematics. The AI consumes the risk assessment; it never creates it.

## Institutional Specification for Quantitative Risk Control

### Version 1.0

> **Implementation Status: COMPLETE** — 6 categories, drawdown state machine, Bayesian R:R engine all implemented.
> Deterministic core: `crates/shared/src/risk/` (`object.rs`, `categories.rs`, `rr.rs`, `mod.rs`).
> Stateful engine: `crates/engine/src/risk_engine.rs`; persistence: `crates/engine/src/db/queries/risk.rs`
> + migration `20260709000000_irml_risk.sql`. Config: `[risk]` in `config.toml` (`RiskConfig`).
> AI payload: injected into the Trader agent (`services/analyzer.rs`, `automation.rs`, `llm/prompts.rs`).
> API: `GET /api/risk-profile`. Frontend: **Risk Management** tab under USER-CONTROLLED
> (`components/RiskProfilePanel.svelte`). Scope is per-pair; R:R is advisory; historical trades are
> backfilled into the R:R block ledger.

---

## 1. Purpose

The existing architecture already answers the *analytical* questions:

- **Indicators + Signal Engine** — What is the market doing?
- **Market Context** — What regime are we in? How strong is the trend?
- **Decision Context** — How likely is a direction? Is the setup favorable?
- **Statistical Intelligence Layer (SIL)** — How unusual is this, statistically?
- **AI Orchestrator (Analyst → Trader agents)** — Should the AI open or avoid a position?

The missing component is **not** another indicator or probability model.

The missing component is a dedicated **Institutional Risk Management Layer (IRML)**.

This layer answers a different, orthogonal set of questions:

- How much capital should be exposed?
- Is this trade worth taking *given our current state*?
- What is the maximum acceptable loss?
- Is the current market environment suitable for this strategy?
- Should exposure be reduced?
- Should trading be suspended entirely?
- **What reward/risk ratio must this trade clear to keep long-run expectancy positive?**

The IRML preserves capital *before* maximizing returns. It is a deterministic
**gatekeeper** placed between analysis and execution. Alpha generation and risk
management are independent systems working together — exactly as in professional
quantitative trading desks.

### 1.1 What is genuinely new vs. already computed

The IRML is largely an **aggregation and governance layer**. Most raw inputs
already exist; the IRML's novelty is (a) reframing them as *risk* rather than
*opportunity*, (b) adding persistent behavioral/drawdown state, and (c) the
**Adaptive Reward/Risk Recommendation Engine** (Section 12).

| Already exists | New in IRML |
|---|---|
| `MarketContext`, `DecisionContext`, `StatisticalContext` | Six unified Risk Categories (A–F) |
| `SafetyManager` (in-memory consecutive losses) | Persistent drawdown state machine + loss escalation |
| `RiskCalculation` (position sizing, R:R math) | Adaptive R:R *recommendation* from realized win rate |
| `PortfolioRiskState` (exposure, correlation) | Unified Position Risk Profile + Trade Permission |
| `paper_trades` DB (realized_pnl) | `rr_calibration` block accounting, expectancy tracking |

---

## 2. Design Philosophy

The IRML satisfies five principles.

### Principle 1 — Capital Preservation First

Capital preservation has absolute priority. A missed opportunity is acceptable;
an uncontrolled loss is not.

### Principle 2 — Risk Is Dynamic

Every decision depends on *current* market conditions and *current* system state,
never on hard-coded static thresholds alone. Thresholds are configurable anchors,
not fixed truths.

### Principle 3 — Risk Is Direction-Independent

A long and a short trade under identical conditions receive the **same** risk
evaluation. Risk scoring never inspects trade direction.

### Principle 4 — Every Recommendation Is Explainable

The system always states *why* exposure is reduced or increased. Every risk object
carries provenance: which inputs, which percentile, how many observations.

### Principle 5 — Deterministic Engine

Risk never depends on AI reasoning. The IRML is pure mathematics. The AI **consumes**
the risk assessment; it never **creates** it. If the LLM is unconfigured the system
reports an error — it must never fall back to an isolated offline decision-maker
(consistent with the project's Single-Pipeline Hybrid Model, no-fallback rule).

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
Statistical Intelligence Layer (distribution, probabilities, confidence)
        │
        ▼
┌───────────────────────────────────────────────┐
│   INSTITUTIONAL RISK MANAGEMENT LAYER (IRML)   │  ◄── deterministic gatekeeper
│   • 6 Risk Categories                          │
│   • Position Risk Profile                      │
│   • Capital Allocation Engine                  │
│   • Drawdown / Loss-Streak State Machines      │
│   • Adaptive R:R Recommendation Engine         │
│   • Trade Permission                           │
└───────────────────────────────────────────────┘
        │  (structured risk profile, read-only)
        ▼
AI Orchestrator (Analyst → Trader agents)
        │  (decides WITHIN risk boundaries)
        ▼
Execution Engine (paper_trading / order matcher)
        │  (ENFORCES hard constraints)
        ▼
Realized trade outcomes ──► feeds back into Behavioral Risk + R:R calibration
```

The AI receives the complete risk profile. The execution engine enforces the
hard constraints. Outcomes feed back into Behavioral Risk and the R:R engine,
closing the loop.

### 3.1 Placement in the codebase (proposed)

- **Compute module:** `crates/shared/src/risk/mod.rs` (pure, deterministic, testable
  under TEST-CORE) — mirrors `market_context.rs`, `decision_context.rs`,
  `statistics/`.
- **Stateful/persistent engine:** `crates/engine/src/risk_engine.rs` — owns the
  drawdown and loss-streak state machines and the R:R calibrator; consumes
  `SafetyManager`, `PortfolioRiskState`, and the `paper_trades` table.
- **Attachment point:** a `RiskProfile` is attached to each `MarketSnapshot`
  alongside `MarketContext` / `DecisionContext` / `StatisticalContext` in
  `analyzer/mod.rs`, and injected into the Trader agent payload in
  `services/analyzer.rs` and `automation.rs`.

The IRML is a **read-only enrichment layer**: it never mutates indicators,
`DecisionContext`, `MarketContext`, or `StatisticalContext`.

---

## 4. Responsibilities

The layer has six primary responsibilities.

1. **Exposure Control** — Increased / Normal / Reduced / Minimal / Zero.
2. **Position Sizing** — max allocation, max portfolio exposure, recommended trade
   size, scaling factor. *The engine determines acceptable exposure percentages,
   not exchange contract quantities.*
3. **Loss Protection** — stop quality, expected downside, drawdown probability,
   stop robustness.
4. **Portfolio Protection** — consecutive losses, daily/weekly drawdown, current
   risk state.
5. **Market Safety** — whether the market itself is safe enough to trade.
6. **Trading Permission** — the final gate: Allowed / Restricted / High Caution /
   Suspended / Emergency Stop.

---

## 5. Risk Categories

The engine produces six **independent** categories. All are scored on `[0, 1]`
where `0` = negligible risk and `1` = extreme risk. All are direction-independent.

### A. Market Risk — *How dangerous is the market right now?*

| Input | Source |
|---|---|
| Volatility level | `MarketContext.volatility`, ATR, HV |
| Trend instability | `DecisionContext.trend_persistence`, `MarketContext.trend` |
| Compression / expansion | Squeeze state, BBWP, `MarketContext.regime` |
| Choppiness | Choppiness Index, `DecisionContext.consensus` |
| Structural uncertainty | `StatisticalContext` entropy / market_predictability |

Answers: Is this market predictable? Is price behaving normally? Should exposure
be reduced?

### B. Structural Risk — *Confidence in market structure*

| Input | Source |
|---|---|
| Order Blocks / SMC | SmartMoney indicator |
| Support / Resistance | `sr_engine.rs` (`SrRoleTracker`), `server/math.rs` |
| VWAP / Anchored VWAP | VWAP, AnchoredVWAP indicators |
| Volume Profile | VolumeProfile indicator |
| Trend alignment | EMA stack, `MarketContext.trend` |
| Recommended stop presence | `DecisionContext.recommended_stop` |

Answers: Is there a logical stop? Is structure clean? Is price trapped between
major levels?

### C. Momentum Risk — *Directional stability*

| Input | Source |
|---|---|
| Divergence | 6 divergence detectors (RSI/MACD/OBV/CMF/MFI/AO) |
| Momentum weakening | MACD histogram slope, Squeeze momentum decay |
| Trend persistence | `DecisionContext.trend_persistence` |
| Consensus decay | `DecisionContext.consensus` trend over recent snapshots |

Answers: Is momentum fading? Is the trend becoming unstable?

### D. Volatility Risk — *Abnormal movement*

| Input | Source |
|---|---|
| ATR (absolute & percentile) | ATR + SIL percentile |
| Historical Volatility | HV + SIL percentile |
| BBWP | BBWP indicator |
| Squeeze | Squeeze Momentum |
| Standard deviation | ZScore / StdDevChannel, SIL distribution |

Answers: Can normal stops survive? Is volatility statistically extreme?

### E. Liquidity Risk (OHLC proxy) — *Execution difficulty*

Only OHLCV is available, so liquidity is estimated from candle geometry over the
history buffer:

| Input | Definition |
|---|---|
| Large-candle frequency | fraction of recent candles with range > k·ATR |
| Gap frequency | fraction of candles with open ≠ prior close beyond tolerance |
| Wick intensity | mean (upper+lower wick) / total range |
| Price rejection | frequency of long-wick rejection candles |
| Candle instability | variance of candle range over the window |

Answers: How difficult may execution become?

### F. Behavioral Risk — *System health*

| Input | Source |
|---|---|
| Consecutive losses | `SafetyManager.consecutive_losses` |
| Recovery state | Drawdown state machine (Section 9) |
| Recent performance | `paper_trades` (realized_pnl over lookback) |
| Current confidence | rolling win rate / expectancy (Section 12) |

Answers: Is the system overtrading? Should it step back and protect capital?

---

## 6. Risk Objects

Every category exposes a uniform **Risk Object**. This is the atomic unit of the
IRML and mirrors the SIL's provenance-rich style.

| Field | Type | Meaning |
|---|---|---|
| `score` | f64 `[0,1]` | Current risk magnitude |
| `confidence` | f64 `[0,1]` | How much evidence supports the score |
| `historical_percentile` | f64 `[0,100]` | Where the score sits vs. its own history |
| `trend` | enum | `Increasing` / `Stable` / `Decreasing` |
| `level` | enum | 7-level band (Section 14) |
| `explanation` | String | Human-readable justification |

**Example**

```
Volatility Risk
  score                 0.72
  historical_percentile 94
  trend                 Increasing
  confidence            High
  level                 High
  explanation           "ATR expanding; HV elevated; BBWP above historical average"
```

---

## 7. Position Risk Profile

Each candidate trade receives a complete profile aggregating the six categories.

```
Trade Risk
  Overall Risk        Moderate  (0.41)
  Market Risk         Low
  Structural Risk     Very Low
  Momentum Risk       Medium
  Volatility Risk     High
  Liquidity Risk      Low
  Behavioral Risk     Very Low
  Overall Confidence  88%
```

### 7.1 Aggregation formula

Overall risk is a **confidence-weighted, penalty-biased** aggregate — not a plain
mean — so a single extreme category cannot be hidden by calm ones:

```
weighted_mean = Σ(scoreᵢ · confᵢ · wᵢ) / Σ(confᵢ · wᵢ)
overall_risk  = max( weighted_mean , λ · max_i(scoreᵢ) )
```

- `wᵢ` — configurable per-category weights (`[risk.category_weights]`).
- `λ` — worst-case bias factor (default `0.5`): even if the weighted mean is low,
  the overall risk is pulled toward the single worst category. This encodes
  Principle 1 (capital preservation first).
- `Overall Confidence` = confidence-weighted mean of category confidences.

The AI reasons from this structured profile rather than a single opaque number.

---

## 8. Capital Allocation Engine

Instead of a fixed rule (`always risk 2%`), the IRML maps `overall_risk` and
Trade Permission to an **adaptive exposure tier**. Outputs remain **percentage
based**, never exchange-specific.

| Tier | Condition (default) | Scaling factor |
|---|---|---|
| Maximum Exposure | overall_risk ≤ 0.20 and Permission = Allowed | 1.00 × base |
| Normal Exposure | 0.20 < risk ≤ 0.40 | 0.75 × base |
| Reduced Exposure | 0.40 < risk ≤ 0.60 or High Caution | 0.50 × base |
| Minimal Exposure | 0.60 < risk ≤ 0.80 or Restricted | 0.25 × base |
| No Exposure | risk > 0.80 or Suspended/Emergency | 0.00 |

`base` is the pair's configured `base_allocation_pct` (from `[scoring]`). The
engine outputs `recommended_allocation_pct = tier_factor × base`, plus
`max_allocation_pct` and `max_portfolio_exposure_pct` (from `PortfolioRiskState`).
It does **not** compute contract quantities — that remains in `paper_trading.rs`.

---

## 9. Drawdown Protection State Machine

Monitors current / daily / weekly / max-historical drawdown and recovery progress,
sourced from `portfolio_equity_history` and `PortfolioRiskState`.

**States and transitions (default thresholds, all configurable):**

```
        equity high-water mark
                │
             NORMAL ──────────────► drawdown > 5%  ──► RECOVERY
                ▲                                          │
                │  recovered to HWM                        │ drawdown > 10%
                │                                          ▼
             RECOVERY ◄──────── recover 50% ─────────  DEFENSIVE
                                                           │ drawdown > 20%
                                                           ▼
                                                        CRITICAL
                                                           │ drawdown > configured max
                                                           ▼
                                                        SHUTDOWN
```

| State | Exposure effect |
|---|---|
| Normal | full allocation permitted |
| Recovery | exposure capped at Reduced tier |
| Defensive | exposure capped at Minimal tier |
| Critical | new positions blocked; manage-only |
| Shutdown | Emergency Stop; all new trading suspended |

Recovery progress = `(current_equity − trough_equity) / (HWM − trough_equity)`.

---

## 10. Consecutive Loss Engine

Tracks losing/winning streaks and escalates risk. Backed by `SafetyManager`
(currently in-memory `AtomicU32`) plus a persistent `risk_events` record so state
survives restarts.

| Streak | Escalation (default) |
|---|---|
| 3 consecutive losses | Reduce exposure → High Caution |
| 5 consecutive losses | Restricted (Minimal exposure) |
| 7 consecutive losses | Trading Suspended |

Thresholds map to the existing `[safety]` config
(`consecutive_loss_caution`, `consecutive_loss_dropout`) and are extended with a
`consecutive_loss_suspend` field. Winning streaks may *relax* caution one level at
a time (never faster than it was applied — hysteresis, Section 14). Tracks:
current losing streak, current winning streak, loss severity (mean loss size),
recovery progress, risk escalation level.

---

## 11. Opportunity vs Risk

Every opportunity is paired with its risk score. Institutional systems **compare**
the two rather than evaluating either in isolation.

```
Trade Opportunity  91%   Trade Risk  82%   →  Recommendation: WAIT
Trade Opportunity  78%   Trade Risk  18%   →  Recommendation: OPEN
```

- **Opportunity** is sourced from existing analysis: `DecisionContext.trade_quality`
  / `trade_readiness` / `confluence` blended with SIL breakout/continuation
  probabilities.
- **Recommendation** is derived from the opportunity-minus-risk margin **and** the
  Adaptive R:R gate (Section 12): a trade is only "OPEN"-eligible when its expected
  R:R clears the recommended threshold *and* opportunity exceeds risk by a
  configurable margin.

This is advisory input to the AI, not an execution command.

---

## 12. Adaptive Reward/Risk Recommendation Engine

This is the mathematical core requested for the IRML. The recommended reward/risk
ratio is **not fixed** — it adapts to the system's realized win rate so that
long-run expectancy stays positive.

Throughout, the ratio is written `1 : R`, where **`1` = risk (max loss per trade)**
and **`R` = reward multiple (target profit as a multiple of the risk)**. `R = 2`
means "risk 1 to make 2" (i.e. 1:2).

### 12.1 Expectancy and the breakeven anchor

For a strategy with win probability `W` (loss probability `1 − W`), risking one
unit to win `R` units, per-trade expectancy is:

```
E = W · R − (1 − W) · 1
```

Setting `E = 0` gives the **breakeven reward multiple** — the minimum `R` required
just to avoid losing money:

```
R_breakeven = (1 − W) / W
```

This single formula reproduces every scenario in the specification:

| Win rate W | R_breakeven = (1−W)/W | Ratio | Interpretation |
|---|---|---|---|
| 0.50 (5/10) | 0.50/0.50 = **1.00** | 1:1.00 | Neutral anchor ✓ |
| 0.70 (7/10) | 0.30/0.70 = **0.43** | 1:0.43 | Below 1:1 — small wins suffice ✓ |
| 0.60 (6/10) | 0.40/0.60 = 0.67 | 1:0.67 | |
| 0.40 (4/10) | 0.60/0.40 = 1.50 | 1:1.50 | |
| 0.30 (3/10) | 0.70/0.30 = **2.33** | 1:2.33 | Above 1:2 — few wins must pay a lot ✓ |
| 0.20 (2/10) | 0.80/0.20 = 4.00 | 1:4.00 | |

The 50% → 1:1, 70% → ~1:0.5, and 30% → >1:2 examples from the specification are all
exactly consistent with `R_breakeven = (1 − W) / W`.

### 12.2 Recommended ratio (positive-expectancy margin)

Breakeven is not the goal — *positive* expectancy is. The engine multiplies the
breakeven ratio by a **safety margin `k > 1`** (config `[risk].rr_safety_margin`,
default `1.25`):

```
R_recommended = k · (1 − W) / W
```

With `k = 1.25` and `W = 0.30`, `R_recommended = 1.25 × 2.33 = 2.92 ≈ 1:3`, matching
the spec's "1:3 or even 1:4" guidance for weak win rates. The resulting per-trade
expectancy is provably positive:

```
E = W · (k · (1−W)/W) − (1−W) = (k − 1)(1 − W) > 0   for all k > 1, W < 1
```

So expectancy grows with the margin `k` and with the loss rate `(1 − W)` — the
engine automatically demands more reward exactly when wins are rare.

### 12.3 Robust win-rate estimation (Bayesian Beta smoothing)

Recomputing `W` from a raw block of 10 trades is statistically unstable: a single
unlucky 2/10 block would swing the recommendation violently. The engine therefore
uses a **Beta-smoothed** estimate that starts *exactly* at the 50% baseline and
updates smoothly as evidence accumulates.

The prior encodes the specification's "5 wins out of 10" neutral assumption as
`Beta(α₀, β₀)` with `α₀ = β₀ = 5`. After observing `wins` and `losses`, the
posterior mean is:

```
W_est = (α₀ + wins) / (α₀ + β₀ + wins + losses)
      = (5 + wins) / (10 + wins + losses)
```

| Observed | W_est | R_breakeven | R_recommended (k=1.25) |
|---|---|---|---|
| 0 trades | 5/10 = 0.500 | 1.00 | 1.25 (≈1:1.25) |
| 7W / 3L | 12/20 = 0.600 | 0.67 | 0.83 (≈1:0.83) |
| 3W / 7L | 8/20 = 0.400 | 1.50 | 1.88 (≈1:1.9) |
| 70W / 30L | 75/110 = 0.682 | 0.47 | 0.58 (≈1:0.58) |
| 30W / 70L | 35/110 = 0.318 | 2.14 | 2.68 (≈1:2.7) |

Key properties:

- **Block 0 starts at exactly 1:1** (the required neutral anchor).
- Early blocks stay conservative near 1:1; the estimate only becomes extreme once a
  large, consistent sample justifies it — preventing whipsaw.
- As `wins + losses → ∞`, `W_est` converges to the true empirical win rate, so the
  breakeven/recommended ratios converge to the raw-block values of §12.1.

The evaluation window may be the whole history or a rolling window of the last `N`
trades (config `[risk].rr_lookback_trades`); either way the Beta prior is applied,
so behavior degrades gracefully on small samples.

### 12.4 Recommendation confidence

Confidence in the R:R recommendation comes directly from the Beta posterior
variance:

```
Var(W) = α·β / ((α + β)² · (α + β + 1)),   α = 5 + wins,  β = 5 + losses
confidence = 1 − 2·√Var(W)     (clamped to [0,1])
```

Few observations → wide posterior → low confidence → recommendation stays near the
1:1 anchor. Many observations → tight posterior → high confidence → recommendation
tracks realized performance. (An equivalent Wilson score interval may be reported
for display.)

### 12.5 Per-block recalibration and net-positive proof

The specification requires recalibration "after every full cycle of 10 trades" and
a positive net result per block. Define a **block** as 10 completed trades. After
each block the engine records `(wins, losses, mean_win, mean_loss)` to
`rr_calibration` and re-derives `W_est` and `R_recommended`.

Provided each executed trade clears `R ≥ R_recommended` and the realized win rate
of the block is at least `W_est`, the block's net expectancy is:

```
E_block = 10 · (k − 1)(1 − W_est) > 0
```

Worked example (weak regime, `W_est = 0.30`, `k = 1.25`, risk unit = 1):

```
R_recommended = 1.25 × (0.70/0.30) = 2.92
Wins:   3 × 2.92 = +8.76
Losses: 7 × 1.00 = −7.00
Net block P&L = +1.76 units  (positive, as required)
```

Worked example (strong regime, `W_est = 0.70`, `k = 1.25`):

```
R_recommended = 1.25 × (0.30/0.70) = 0.54
Wins:   7 × 0.54 = +3.78
Losses: 3 × 1.00 = −3.00
Net block P&L = +0.78 units  (positive)
```

The engine becomes **more conservative (higher R demanded) when the win rate is
low**, and **more aggressive (lower R accepted) when the win rate is high** — always
targeting positive net expectancy per block. This `R_recommended` is passed to the
Trader agent as a data-driven, context-aware risk parameter (Section 13).

---

## 13. AI Integration

The AI receives **structured risk**, not instructions. The IRML injects the
following JSON block into the Trader agent's user payload (alongside the existing
Analyst Document). Schema:

```json
{
  "overall_risk": 0.28,
  "overall_confidence": 0.88,
  "categories": {
    "market":     { "score": 0.20, "level": "Low",       "confidence": 0.9, "percentile": 42, "trend": "Stable" },
    "structural": { "score": 0.10, "level": "Very Low",  "confidence": 0.9, "percentile": 15, "trend": "Stable" },
    "momentum":   { "score": 0.45, "level": "Medium",    "confidence": 0.7, "percentile": 60, "trend": "Increasing" },
    "volatility": { "score": 0.62, "level": "High",      "confidence": 0.8, "percentile": 88, "trend": "Increasing" },
    "liquidity":  { "score": 0.18, "level": "Low",       "confidence": 0.6, "percentile": 30, "trend": "Stable" },
    "behavioral": { "score": 0.05, "level": "Very Low",  "confidence": 0.9, "percentile": 10, "trend": "Stable" }
  },
  "drawdown_state": "Normal",
  "exposure_recommendation": "Normal",
  "recommended_allocation_pct": 3.75,
  "opportunity_score": 0.78,
  "reward_risk": {
    "win_rate_estimate": 0.50,
    "breakeven_ratio": 1.00,
    "recommended_ratio": 1.25,
    "confidence": 0.41,
    "sample_size": 0
  },
  "trade_permission": "Allowed",
  "explanation": "Volatility elevated (88th pct) but structure clean and streak healthy."
}
```

The AI decides **within** these boundaries. The execution engine **enforces** them.

---

## 14. Risk Transitions

Rather than binary Safe/Unsafe, the IRML uses a **7-level gradual scale** for every
risk band:

```
Very Safe → Safe → Normal → Elevated → High → Critical → Emergency
```

To avoid unstable flapping around thresholds, transitions use **hysteresis**: the
threshold to *enter* a more dangerous level is lower than the threshold to *leave*
it. Default band edges (on `[0,1]`):

| Level | Enter when score ≥ | Exit when score < |
|---|---|---|
| Very Safe | 0.00 | — |
| Safe | 0.15 | 0.10 |
| Normal | 0.30 | 0.25 |
| Elevated | 0.45 | 0.40 |
| High | 0.60 | 0.55 |
| Critical | 0.75 | 0.70 |
| Emergency | 0.90 | 0.85 |

The 0.05 gap between enter/exit edges is the hysteresis margin
(config `[risk].transition_hysteresis`).

---

## 15. Trade Lifecycle Risk

Risk evolves during a trade. The IRML continuously re-evaluates the profile across
lifecycle stages:

```
Entry → Position Building → Active Position → Profit Protection → Exit → Post Trade
```

- **Entry / Building** — gate on Trade Permission + R:R recommendation.
- **Active** — monitor Volatility & Momentum risk for deterioration.
- **Profit Protection** — tighten stop guidance as unrealized profit grows
  (integrates with `break_even_trail_enabled`).
- **Post Trade** — feed realized outcome into Behavioral Risk (Section 5F) and the
  R:R calibrator (Section 12.5).

---

## 16. Execution Constraints

The IRML defines **immutable hard limits**. These are enforced by the execution
engine (`paper_trading.rs`, `portfolio_risk.rs`) and cannot be overridden by the AI:

| Constraint | Source |
|---|---|
| Maximum simultaneous exposure | `PortfolioRiskState.max_total_exposure_pct` |
| Maximum correlated exposure | `PortfolioRiskState` pairwise Pearson limit |
| Maximum daily loss | `[risk].max_daily_loss_pct` |
| Maximum drawdown | `[safety].capital_drawdown_pct` |
| Maximum trade duration | `[risk].max_trade_duration_secs` |
| Minimum reward/risk | `R_recommended` (Section 12) |
| Minimum trade quality | `[risk].min_trade_quality` vs `DecisionContext.trade_quality` |
| Minimum statistical confidence | `[risk].min_confidence` vs SIL confidence |
| Maximum volatility percentile | `[risk].max_volatility_percentile` |

If any hard limit is breached, Trade Permission drops to at least **Restricted** and
the violated constraint is named in the explanation.

---

## 17. Explainability

Every restriction explains itself. Nothing may appear arbitrary.

```
Trading Restricted
  Historical volatility   96th percentile
  Trend persistence       0.32
  Consensus               48%
  Expected drawdown       High
  Recent losses           3
  Exposure reduced        50%
```

Each Risk Object's `explanation` string and the top-level `explanation` are
assembled from the specific inputs that drove the score, following the SIL's
provenance convention (window, observation count, percentile).

---

## 18. Performance Monitoring

Long-term system-health metrics. These are **descriptive** — they support
continuous evaluation of the strategy and calibrate the R:R engine; they do **not**
directly override individual trade decisions.

| Metric | Definition |
|---|---|
| Win rate | wins / total (Beta-smoothed for R:R, §12.3) |
| Average gain | mean realized_pnl of winning trades |
| Average loss | mean realized_pnl of losing trades |
| Profit factor | Σ gains / |Σ losses| |
| Expectancy | `W·avg_gain − (1−W)·avg_loss` |
| Avg reward/risk achieved | mean realized reward multiple |
| Drawdown history | from `portfolio_equity_history` |
| Recovery factor | net profit / max drawdown |
| Risk-adjusted return | Sharpe / Sortino on realized trade returns |
| Trade frequency | trades per unit time |
| Opportunity acceptance rate | opened / signalled |
| Opportunity rejection rate | rejected / signalled |

Data sources: `paper_trades`, `trade_telemetry_history`, `portfolio_equity_history`.

---

## 19. Proposed Data Model & Configuration

### 19.1 New `[risk]` config section (config.toml)

```toml
[risk]
# Aggregation
category_weights   = { market = 1.0, structural = 1.0, momentum = 1.0, volatility = 1.2, liquidity = 0.8, behavioral = 1.0 }
worst_case_lambda  = 0.5          # Section 7.1
transition_hysteresis = 0.05      # Section 14

# Adaptive Reward/Risk engine (Section 12)
rr_prior_wins      = 5            # Beta prior α₀ (baseline "5 of 10")
rr_prior_losses    = 5            # Beta prior β₀
rr_safety_margin   = 1.25         # k > 1, guarantees positive expectancy
rr_block_size      = 10           # trades per recalibration block
rr_lookback_trades = 0            # 0 = full history; N = rolling window

# Hard execution constraints (Section 16)
max_daily_loss_pct        = 5.0
max_trade_duration_secs   = 86400
min_trade_quality         = 0.4
min_confidence            = 0.5
max_volatility_percentile = 95.0
```

Parsed into a `RiskConfig` struct in `crates/engine/src/config/models.rs`, added to
`AppConfig`. Backward compatible via `#[serde(default)]`.

### 19.2 New tables (SQLite migration)

```sql
-- Persistent risk state so streaks/drawdown survive restarts
CREATE TABLE risk_events (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key       TEXT NOT NULL,
    timestamp      INTEGER NOT NULL,
    overall_risk   REAL NOT NULL,
    drawdown_state TEXT NOT NULL,
    permission     TEXT NOT NULL,
    losing_streak  INTEGER NOT NULL,
    winning_streak INTEGER NOT NULL,
    explanation    TEXT
);

-- Per-block R:R calibration ledger (Section 12.5)
CREATE TABLE rr_calibration (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key          TEXT NOT NULL,
    block_index       INTEGER NOT NULL,
    wins              INTEGER NOT NULL,
    losses            INTEGER NOT NULL,
    win_rate_estimate REAL NOT NULL,   -- Beta-smoothed W_est
    breakeven_ratio   REAL NOT NULL,   -- (1-W)/W
    recommended_ratio REAL NOT NULL,   -- k·(1-W)/W
    confidence        REAL NOT NULL,
    net_block_pnl     REAL NOT NULL,
    timestamp         INTEGER NOT NULL
);
```

Win/loss classification derives from `paper_trades.realized_pnl` (`> 0` = win),
consistent with the existing schema — no new win/loss column is required.

---

## 20. Frontend — Risk Management Panel

Risk Management is a **dedicated standalone panel** accessible from the GENERAL mode
(via the "Risk Management" tab) and the USER-CONTROLLED mode. It is **not** a decision
pipeline stage — risk management determines *how* to trade, not *whether*.

The panel has two sub-tabs:

### Calculator Tab
Pre-trade position sizing calculator with live ATR-based stop computation:
- Risk profile selection (capital, max risk %, leverage, commission, funding rate)
- Direction toggle (LONG / SHORT)
- Entry price, ATR-based stop (multiplier × live ATR from market data), Take Profit
- Max daily loss, max allocation % per trade
- Calculated outputs: risk capital, position size, notional, margin, liquidation price, R:R, net PnL

### Risk Profile Tab
Institutional IRML dashboard (read-only server-generated analysis):
- Six risk category breakdown (Market, Structural, Momentum, Volatility, Liquidity, Behavioral)
- Overall risk gauge with permission badge
- Exposure tier and recommended allocation
- Adaptive R:R recommendation (Beta-smoothed win rate, breakeven ratio)
- Opportunity vs Risk comparison
- R:R block history table

Data flows: `GET /api/risk-profile?pair_key=...` for IRML data,
`POST /api/risk/calculate` for position sizing,
live ATR from pair's `microTerm.latestSnapshot.atr_14` via WebSocket.

---

## 21. Expected Outcome

The IRML transforms the system from making isolated trade decisions into operating
under a comprehensive risk framework. Rather than relying on fixed stop-losses or
static position sizing, it continuously evaluates market conditions, structural
integrity, statistical uncertainty, and the system's own recent performance to
determine acceptable exposure, trading permission, and the reward/risk ratio each
trade must clear to keep long-run expectancy positive. It is a deterministic,
transparent, explainable gatekeeper between analysis and execution — mirroring
professional quantitative desks where alpha generation and risk management are
independent systems working together to produce disciplined, capital-preserving
decisions.

---

## 22. Extensions

| Extension | File | Description |
|-----------|------|-------------|
| **Stress Testing** | [07-irmL-stress-testing-mvo.md](07-irmL-stress-testing-mvo.md) §Part 1 | 5 predefined stress scenarios (Flash Crash, Vol Spike, Correlation Breakdown, Trend Reversal, Funding Crisis) |
| **Markowitz MVO** | [07-irmL-stress-testing-mvo.md](07-irmL-stress-testing-mvo.md) §Part 2 | Efficient frontier, tangency portfolio, mean-variance allocation alongside existing Kelly + Risk Parity |
| **VaR/CVaR Consumption** | [06-isil-advanced-risk-modeling.md](06-isil-advanced-risk-modeling.md) | IRML consumes VaR and CVaR for capital allocation guardrails |
| **EVT Tail Risk** | [06-isil-advanced-risk-modeling.md](06-isil-advanced-risk-modeling.md) | IRML consumes EVT-based VaR/ES for superior crypto tail risk estimation |
