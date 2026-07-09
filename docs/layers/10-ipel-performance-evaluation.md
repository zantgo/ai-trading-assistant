# IPEL — Institutional Performance Evaluation Layer

> **Layer 10 of 10 in the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: IN PROGRESS** — Performance evaluator and journal agent complete. Historical analyst and adaptive learning feedback loop in development.

---

## Purpose

The Institutional Performance Evaluation Layer (IPEL) answers the continuous improvement question:

> **How well did we perform, and how should the system adapt?**

IPEL closes the feedback loop. Every trade is journaled, every outcome is measured, every agent is scored, and every regime is analyzed. The output isn't just reports — it's actionable adjustment recommendations that feed back into ICSL (scoring weights), IRML (R:R calibration), and IEPL (position sizing). IPEL transforms the system from a static rule engine into an adaptive learning organism.

**IPEL evaluates, never executes. Recommendations are logged for human review — never auto-applied.**

**Code location:** `crates/engine/src/performance_evaluator.rs` (direction correctness), `crates/engine/src/historical_analyst.rs` (AI-driven historical analysis), `crates/engine/src/llm/agents.rs::run_journal_agent()` (post-trade audit), `crates/engine/src/db/queries/performance.rs` (metrics queries).

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| Completed trades | `paper_trades` table | Entry/exit price, size, leverage, PnL, fees, regime at entry, confluence score, exit reason |
| Trade telemetry | `trade_telemetry_history` table | Per-slot execution mirror with P&L, timestamp, exchange label |
| AnalystDocument | IASL Analyst Agent output | 8-section JSON (recorded at entry analysis time) |
| TraderDecision | IASL Trader Agent output | action + confidence + rationale (recorded at decision time) |
| Journal results | IASL Journal Agent output | final_analysis + execution_score per completed trade |
| Portfolio equity history | `portfolio_equity_history` table | Time-series equity for drawdown computation |
| Decision metrics snapshot | IDCL | trade_quality, trade_readiness, risk_level at entry time |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| Performance metrics dashboard | Win rate, profit factor, expectancy, Sharpe, max drawdown, recovery factor, avg R-multiple | Frontend (Performance Dashboard), IRML (R:R calibration) |
| Direction correctness scores | % correct at 1h/4h/24h horizons per symbol | Frontend, strategy optimization |
| Regime-specific breakdown | Per-regime metrics table | IRCL (gate tuning), ICSL (weight adjustment) |
| Historical Analyst recommendations | HistoricalRecommendation JSON | IRML (R:R adjustment), IEPL (sizing adjustment) |
| Agent performance scores | Per-agent accuracy tracking | IASL (prompt refinement) |
| Adaptive learning recommendations | Suggested indicator weight changes, threshold adjustments | ICSL (manual review), IRML (manual calibration) |

---

## Sub-Components

---

### A. Trade Journaling

Every completed trade is immutably recorded with full execution context. The journal serves as the ground-truth dataset for all performance analysis.

**Per-trade recorded fields:**

| Field | Source | Description |
|-------|--------|-------------|
| `symbol` | Config | Trading pair (e.g., "BTC") |
| `entry_price` | paper_trades | Weighted average entry across all filled slots |
| `exit_price` | paper_trades | Weighted average exit across all closed slots |
| `size` | paper_trades | Total position size in base units |
| `leverage` | paper_trades | Leverage used (fixed at 20× cross) |
| `direction` | paper_trades | LONG or SHORT |
| `realized_pnl` | paper_trades | Net P&L after fees and funding |
| `realized_pnl_pct` | paper_trades | P&L as percentage of allocated margin |
| `fees_commission` | paper_trades | Total commission fees (maker + taker) |
| `fees_funding` | paper_trades | Total funding rate payments |
| `regime_at_entry` | IRCL | Regime label at the time of entry analysis |
| `confluence_at_entry` | ICSL | Confluence score [-100,+100] at entry |
| `trade_quality_at_entry` | IDCL | trade_quality at entry |
| `trade_readiness_at_entry` | IDCL | trade_readiness at entry |
| `risk_level_at_entry` | IRML | overall_risk at entry |
| `rr_recommended_at_entry` | IRML | R_recommended from adaptive R:R engine |
| `analyst_document_json` | IASL | Full AnalystDocument JSON at entry analysis time |
| `trader_decision_json` | IASL | Full TraderDecision JSON at entry decision time |
| `exit_reason` | IEPL | TP1/TP2/TP3/SL hit / Opposite score / Decisive close / Structural breakdown / Manual |
| `entry_timestamp` | System | Unix epoch of first slot entry |
| `exit_timestamp` | System | Unix epoch of last slot exit |
| `hold_duration_secs` | Computed | exit_timestamp − entry_timestamp |

**Telemetry mirroring:** Each closed slot is also mirrored to `trade_telemetry_history` with the exchange label "PAPER" within the same database transaction. This allows the asynchronous Journal Agent to audit the trade without violating referential integrity.

---

### B. Journal Agent — Post-Trade Audit

After each completed trade, the Journal Agent performs an AI-driven retrospective analysis. This is separate from the performance metrics — the Journal Agent evaluates **process adherence**, not just outcomes.

**Input:** Structured trade context JSON containing entry conditions, indicator state at entry, analyst/trader outputs, exit conditions, P&L, and hold duration.

**Output:**
```json
{
  "final_analysis": "3-4 sentence retrospective covering key mistakes or successes. Identifies whether the trade followed the protocol, whether entries were timed well, whether exits were disciplined.",
  "execution_score": 7.5
}
```

**Scoring guidelines (0-10):**

| Score | Label | Criteria |
|-------|-------|----------|
| 9.0 – 10.0 | Exemplary | Perfect process adherence. Entry at optimal zone. Disciplined exit. All gates respected. |
| 7.0 – 8.9 | Good | Minor deviations from protocol. Slightly early/late on entry. Overall sound decision-making. |
| 5.0 – 6.9 | Adequate | Some process violations. Entry without full confluence. Late exit. Salvageable outcome. |
| 3.0 – 4.9 | Poor | Significant protocol violations. FOMO entry. Trading against regime. Poor stop placement. |
| 0.0 – 2.9 | Reckless | Complete disregard for process. Counter-trend entry. No stop. Revenge trading patterns. |

**Key principle:** A losing trade with excellent discipline can score high (8+). A winning trade entered recklessly should score low (<4). The score measures process quality, not P&L.

**Common mistakes detected:**
- FOMO entries (entering before candle confirmation)
- Trading against macro trend (structural EMA stack violation)
- Failing to wait for RVOL confirmation on breakouts
- Improper scale-in pacing (all slots filled too quickly)
- Moving stop-losses away from price (emotional intervention)
- Over-sizing beyond IRML allocation recommendation

**Code location:** `crates/engine/src/llm/agents.rs::run_journal_agent()`, temperature 0.2, max_tokens 512.

---

### C. Performance Metrics

A comprehensive set of institutional performance metrics is computed continuously from the trade journal.

**Core metrics:**

| Metric | Formula | Interpretation |
|--------|---------|----------------|
| **Win Rate** | W = (α₀ + wins) / (α₀ + β₀ + wins + losses) | Beta-smoothed with prior α₀=β₀=5 (50% baseline). Avoids unstable estimates on small samples. Feeds IRML R:R engine. |
| **Profit Factor** | Σ gains / |Σ losses| | >1.5 = strong. <1.0 = losing system. |
| **Expectancy** | E = W × avg_gain − (1−W) × avg_loss | Positive = system has edge. Per-trade expected value in dollars. |
| **Sharpe Ratio** | (μ_return − r_f) / σ_return | Risk-adjusted return. >1.0 adequate, >2.0 excellent. |
| **Sortino Ratio** | (μ_return − r_f) / σ_downside | Like Sharpe but penalizes only downside deviation. More relevant for trading. |
| **Max Drawdown** | max(HWM − equity) / HWM | From portfolio_equity_history. Must stay below IRML configured limit. |
| **Recovery Factor** | Net profit / max drawdown | How quickly the system recovers from drawdowns. >2.0 = robust. |
| **Avg R-Multiple** | Mean(realized_PnL / initial_risk) | Normalized return per unit of risk. >0.3 = positive edge. |
| **Avg Hold Duration** | Mean(hold_duration_secs) | Per-regime breakdown identifies regime-specific optimal hold times. |
| **Trade Frequency** | trades / time_unit | Too high → overtrading. Too low → missing opportunities. |
| **Opportunity Acceptance Rate** | opened / signalled | What % of IASL "Open" signals result in actual entries? |
| **Opportunity Rejection Rate** | rejected / signalled | What % of signals are rejected by IRML gates or user override? |

**Data sources:** `paper_trades`, `trade_telemetry_history`, `portfolio_equity_history`.

---

### D. Direction Correctness Analysis

Separates signal quality from execution quality. A trade can have perfect execution (scored 10 by Journal Agent) but still lose because the direction was wrong — or vice versa.

**Evaluation horizons:**

| Horizon | Timeframe | Question |
|---------|-----------|----------|
| **1 hour** | Short-term | Did the market move in the predicted direction within 1 hour of the analysis? |
| **4 hours** | Medium-term | Was the directional bias correct over a half-session? |
| **24 hours** | Full session | Did the structural analysis hold over a full trading day? |

**Computation:**
```
direction_correct_1h = (sign(price_1h_later − price_at_analysis) == sign(predicted_direction))
```

Where `predicted_direction` = +1 for "Open Long" or "Bullish bias", −1 for "Open Short" or "Bearish bias".

**Per-agent tracking:** Direction correctness is tracked per agent. The Analyst Agent's `market_summary` directional bias and the Trader Agent's `action` are evaluated independently:
- Analyst bias correct at 1h but Trader chose "Wait" = Analyst signal good, Trader too conservative
- Analyst bias wrong but Trader chose "Wait" = Trader correctly rejected a bad signal
- Both correct at all 3 horizons = exceptional alignment

---

### E. Historical Analyst

An AI-driven periodic analysis of completed trades provides strategic-level insights that raw metrics cannot capture.

**Trigger:** After every N completed trades (configurable `sweep_interval_trades`, default 10), the Historical Analyst runs a comprehensive retrospective.

**Input:** Aggregated trade statistics from the last N trades + full trade journal entries.

**Output:**
```json
{
  "symbol": "BTC",
  "generated_at": "2026-07-08T12:00:00Z",
  "trades_analyzed": 10,
  "win_rate": 0.60,
  "avg_risk_reward": 1.8,
  "avg_hold_time_minutes": 45.2,
  "profit_factor": 1.75,
  "suggested_rr_adjustment": 1.5,
  "suggested_position_sizing_pct": 2.0,
  "regime_analysis": "Strongest performance in Trending regime (4 trades, 75% win rate, PF 2.3). Weakest in Range regime (2 trades, 0% win rate). Recommend reducing allocation to 0.5% in Range until trend-following filter improved.",
  "key_improvements": "1. Two exits triggered by premature decisive close — tolerance buffer may need widening from 0.2% to 0.3%. 2. L3 entries (Golden Pocket) consistently profitable (3/3 wins) — consider increasing allocation weight for L3. 3. Entries during RVOL < 1.5 account for 3 of 4 losses — stricter volume gate recommended.",
  "risk_recommendation": "Current drawdown state: Normal. Max drawdown this period: 4.2%. IRML exposure tier appropriate. No risk escalation needed."
}
```

**Code location:** `crates/engine/src/historical_analyst.rs`.

---

### F. Regime-Specific Performance Breakdown

Every performance metric is disaggregated by IRCL regime. This reveals where the system excels and where it struggles.

**Breakdown table (example):**

| Regime | Trades | Win Rate | Profit Factor | Avg R | Avg Hold (min) |
|--------|--------|----------|---------------|-------|-----------------|
| Trending | 15 | 0.73 | 2.10 | +1.4 | 62 |
| Compression | 5 | 0.40 | 0.85 | −0.3 | 28 |
| Expansion | 8 | 0.75 | 2.50 | +1.8 | 35 |
| Range | 6 | 0.33 | 0.60 | −0.8 | 41 |
| Transitional | 2 | 0.00 | 0.00 | −1.0 | 12 |

**Actionable insights from regime breakdown:**
- **Trending strong, Range weak** → Increase ICSL trend group weight in Range regime, reduce allocation to Minimal in Range (already configured via IRCL)
- **Compression underperforming** → Breakout detection may be premature. Increase squeeze_min_duration from 5 to 7 in config
- **Expansion excellent** → Consider increasing position sizing tier from Normal to Maximum when regime confidence >0.85
- **Transitional zero wins** → IRCL gate is working correctly (blocks trades). No change needed.

These insights feed into ICSL (scoring weight adjustments) and IEPL (sizing tier adjustments).

---

### G. Adaptive Learning Feedback Loop

IPEL generates concrete adjustment recommendations that close the feedback loop back into the strategy layers. All recommendations are **logged for human review — never auto-applied.**

**Feedback pathways:**

| From IPEL Metric | Adjustment | Target Layer | Mechanism |
|-----------------|------------|-------------|-----------|
| Declining Win Rate | Increase R:R recommended ratio via Bayesian posterior | IRML §12 | `rr_recommended` increased; trades must clear higher bar |
| Regime-specific underperformance | Adjust per-regime indicator group weights | ICSL §F | `ScoringConfig.regime_weight_multipliers` updated |
| High RVOL < 1.5 loss rate | Tighten volume confirmation gate | ICSL §C | RVOL threshold raised from 1.0 to 1.2 for entry |
| Excessive drawdown | Reduce allocation tier | IEPL §A | `max_allocation_pct` reduced per IRML exposure tier |
| Consistently missed TP2 | TP2 target too aggressive | IEPL §D | Switch from 1.618 to 1.272 extension for TP2 in non-Expansion regimes |
| L3 entries highly profitable | Increase L3 allocation weight | IEPL §A | L3 allocation increased from 33% to 40% |
| Journal Agent scores declining | Process discipline eroding | IASL | Flag in Trader Agent prompt: "Recent trades show declining process adherence" |
| Short hold times with negative P&L | Premature exits | IEPL §E | Increase decisive close tolerance from 0.2% to 0.3% |

**Recommendation log format:**
```json
{
  "timestamp": 1751923200,
  "metric": "regime_range_win_rate",
  "current_value": 0.33,
  "threshold": 0.45,
  "recommended_action": "reduce_range_allocation",
  "current_setting": 0.50,
  "recommended_setting": 0.25,
  "rationale": "Range regime win rate (33%) below acceptable threshold (45%) over last 20 trades. Reducing max allocation from 50% to 25% until strategy adaptation improves Range performance.",
  "auto_applied": false,
  "reviewed": false
}
```

**Bayesian prior updating:** As trades accumulate, the Beta prior for IRML's R:R engine evolves:
```
α_post = α_prior + observed_wins
β_post = β_prior + observed_losses
```
The prior starts at Beta(5,5) = 50% neutral. After 100 trades with 60 wins:
```
α_post = 5 + 60 = 65
β_post = 5 + 40 = 45
W_est = 65 / (65+45) = 0.591
```
This smooth, evidence-driven update ensures the R:R recommendation adapts proportionally to accumulated data without overreacting to short-term variance.

---

## Integration

### Feeds Into (Feedback Loop)
- **ICSL (Layer 4)** — Regime-specific weight adjustments, volume gate threshold tuning
- **IRML (Layer 7)** — Bayesian R:R prior updates, drawdown limit calibration
- **IEPL (Layer 9)** — Position sizing tier adjustments, stop tolerance tuning, TP target recalibration
- **IASL (Layer 8)** — Agent prompt refinement based on performance scoring
- **IRCL (Layer 2)** — Regime detection threshold adjustments

### Receives From
- **IEPL (Layer 9)** — Completed trade records (entry, exit, P&L, fees, exit reason)
- **IASL (Layer 8)** — AnalystDocument + TraderDecision at analysis time
- **IRML (Layer 7)** — Risk metrics at entry (overall_risk, permission, R:R)
- **IDCL (Layer 5)** — Decision metrics at entry (trade_quality, trade_readiness)

### Cross-References
- [IRML: §12 Adaptive R:R Engine](../layers/07-irmL-risk-management.md) — How IPEL win rate feeds Bayesian R:R calibration
- [ICSL: §F Regime-Aware Weights](../layers/04-icsl-confluence-scoring.md) — How regime-specific performance adjusts scoring weights
- [IEPL: §A Entry Protocol](../layers/09-iepl-execution-protocol.md) — How sizing adjustments affect entry allocation
- [IASL: §B Analyst Agent](../layers/08-iasl-ai-synthesis.md) — How agent performance scoring feeds prompt refinement
- [Historical Analyst](../institutional-unified-strategy-framework.md) — AI-driven periodic trade review
- [IPEL Factor Attribution](../layers/10-ipel-factor-attribution.md) — Alpha/beta decomposition, rolling factor exposures, regime-conditional alpha, signal quality decomposition
