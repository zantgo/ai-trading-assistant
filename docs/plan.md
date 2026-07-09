# Project Roadmap

> Implementation status and phased execution plan for the AI Trading Assistant.

---

## Phase Status

| Phase | Name | Status | Description |
|-------|------|--------|-------------|
| 0 | Registry + Signals Foundation | Complete | `registry.rs` with 51 IndicatorMeta entries; `SignalKind` enum (12 types); `NormalizedIndicatorValue.signals[]`; registry serialized in `/api/config` |
| 1A | Core Indicator Expansion (7 indicators) | Complete | Supertrend, Donchian, Keltner, OBV, CMF, MFI, HV calculators + normalizers + registry entries + charts + docs |
| 1B | Regime Indicators (4 indicators) | Complete | Aroon, Choppiness, LinReg Slope, Z-Score calculators + normalizers + charts + docs |
| 2 | Divergence + Signals Emission | Complete | Generalized DivergenceDetector across 8 oscillators; 6 new `*_divergence` keys; `signals[]` emitted for all 51 indicators; frontend badges + markers |
| 3 | Configurable Scoring + AI Agents | Complete | `ScoringConfig` per-indicator weight/enable; scoring UI; 2-agent AI pipeline (Analyst → Trader); guide sections for all indicators |
| 4 | DecisionContext | Complete | 17 quantitative metrics computed from normalized indicator map; attached to MarketSnapshot; 10 property tests |
| 5 | Statistical Intelligence (SIL) | Complete | 6 modules (A-F): Distribution, Probability, Confidence, Market Shape, Relationships, Monte Carlo; ML: online learning, feature importance, clustering, anomaly, regime classification; derived features |
| 6 | Risk Management (IRML) | Complete | 6 risk categories; position risk profile; capital allocation tiers; drawdown state machine; consecutive loss engine; adaptive Bayesian R:R engine; risk_events + rr_calibration tables |
| 7 | Execution Protocol | Complete | Fractional Dynamic Position Slot Machine (4 slots, FIFO); break-even trailing; decisive close invalidation; bracket order constraints; paper trading simulation |
| 8 | Real-Exchange Execution | In Progress | OrderManager, PositionReconciler, ExecutionGuard implemented (`execution/`); API handlers pending |
| 9 | Performance & Learning | In Progress | Performance evaluator (direction correctness); historical analyst (AI-driven); journal agent (post-trade audit); adaptive learning feedback loop |
| 10 | Documentation System | Complete | 10-layer institutional strategy docs; 51-indicator signal audit; unified naming convention; cross-reference infrastructure |
| 11 | Open Interest & Funding Rate | Complete | Live OI + funding WS subscriptions; 4 new indicators (open_interest, oi_delta, funding_rate, oi_price_divergence); config + frontend types |
| 12 | Order Book Depth Analysis | Complete | Full L2 depth processing; OFI, wall detection, depth-weighted mid, spread, depth_bias indicators; OrderBookAnalysis calculator |
| 13 | Backtesting Framework | Complete | Historical replay engine; Sharpe/Sortino/DSR metrics; walk-forward optimizer; `POST /api/backtest/run` endpoint |
| 14 | Portfolio Optimization | Complete | Kelly Criterion (full + fractional); Risk Parity (ERC + budgeted); PortfolioOptimizer integrating IRML win rates; 24 unit tests |
| 15 | Derivatives Data Indicators | Complete | Open Interest, OI Delta, Funding Rate, OI-Price Divergence indicators; WS integration; config + frontend |
| 16 | ISIL Advanced Risk Modeling | Planned | VaR/CVaR engine, GARCH(1,1) volatility forecasting, EVT (POT/GPD) tail risk, Spearman Information Coefficient — pure math in `statistics/` |
| 17 | TWAP/VWAP Execution Algorithms | Planned | Time-sliced execution (TWAP/VWAP/IS), scalping-optimized short-duration slices, algo progress tracking, integration with slot machine |
| 18 | Factor Model + Stress Testing | Planned | 1-factor alpha/beta decomposition, predefined stress scenarios (flash crash, vol spike, correlation breakdown), scenario P&L |
| 19 | Cointegration Framework | Planned | Engle-Granger 2-step + Johansen test, OU half-life, pair cointegration z-scores, cross-pair mean-reversion signals (adds nalgebra dep) |
| 20 | Markowitz Mean-Variance | Planned | Efficient frontier computation, tangency portfolio (max Sharpe), complement to existing Kelly + Risk Parity allocation engine |

---

## Documentation Roadmap (10-Layer Decision Pipeline)

| # | Layer | File | Status |
|---|-------|------|--------|
| 1 | ITIL — Institutional Technical Indicator Layer | `docs/layers/01-itil-technical-indicator.md` | Complete |
| 2 | IRCL — Institutional Regime Classification Layer | `docs/layers/02-ircl-regime-classification.md` | Complete |
| 3 | ISML — Institutional Structure Mapping Layer | `docs/layers/03-isml-structure-mapping.md` | Complete |
| 4 | ICSL — Institutional Confluence Scoring Layer | `docs/layers/04-icsl-confluence-scoring.md` | Complete |
| 5 | IDCL — Institutional Decision Context Layer | `docs/layers/05-idcl-decision-context.md` | Complete |
| 6 | ISIL — Institutional Statistical Intelligence Layer | `docs/layers/06-isil-statistical-intelligence.md` | Complete |
| 7 | IRML — Institutional Risk Management Layer | `docs/layers/07-irmL-risk-management.md` | Complete |
| 8 | IASL — Institutional AI Synthesis Layer | `docs/layers/08-iasl-ai-synthesis.md` | Complete |
| 9 | IEPL — Institutional Execution Protocol Layer | `docs/layers/09-iepl-execution-protocol.md` | Complete |
| 10 | IPEL — Institutional Performance Evaluation Layer | `docs/layers/10-ipel-performance-evaluation.md` | In Progress |

## Layer Extension Documents

| # | Document | Layer | Description |
|---|----------|-------|-------------|
| 6e | ISIL Advanced Risk Modeling | ISIL | VaR/CVaR, GARCH volatility forecasting, EVT tail risk, Spearman IC |
| 7e | IRML Portfolio Optimization | IRML | Markowitz MVO, efficient frontier, stress testing framework |
| 9e | IEPL Algorithmic Execution | IEPL | TWAP, VWAP, Implementation Shortfall execution algorithms |
| 10e | IPEL Factor Attribution | IPEL | Alpha/beta decomposition, factor performance attribution |
| 3e | ISML Cointegration | ISML/ISIL | Pair cointegration analysis, Johansen test, OU mean-reversion |

## New Phases (Post-10-Layer Completion)

| Phase | Name | Status | Description |
|-------|------|--------|-------------|
| 11 | Open Interest & Funding Rate | Complete | Live OI and funding rate WS ingestion, OI delta indicator, funding rate gate |
| 12 | Order Book Depth Analysis | Complete | Full L2 depth processing, OFI, wall detection, depth-weighted mid-price |
| 13 | Backtesting Framework | Complete | Historical replay engine, walk-forward optimization, deflated Sharpe ratio |
| 14 | Real-Exchange Execution (Phase 8) | Complete (engine) | OrderManager, PositionReconciler, ExecutionGuard; API handlers pending |
| 15 | Portfolio Optimization | Complete | Kelly Criterion sizing, Risk Parity allocation, integrated with IRML |
| 16 | ISIL Advanced Risk Modeling | Planned | VaR/CVaR, GARCH(1,1) volatility forecast, EVT tail risk, Spearman IC |
| 17 | TWAP/VWAP Execution Algorithms | Planned | Time-sliced execution, VWAP tracking, scalping-optimized durations |
| 18 | Factor Model + Stress Testing | Planned | Alpha/beta, scenario P&L, covariance-based stress tests |
| 19 | Cointegration Framework | Planned | Engle-Granger, Johansen, OU half-life (adds nalgebra dep) |
| 20 | Markowitz Mean-Variance | Planned | Efficient frontier, tangency portfolio, MVO allocation |

---

## Deferred

| Item | Notes |
|------|-------|
| Ichimoku Cloud client-side displacement shift | Full rendering deferred |
| Pivot Points prior-day OHLC tracker | Deferred |
| Divergence connecting-lines (chart overlay) | Signal points recorded, line rendering deferred |
| Real-exchange Hyperliquid order execution | Phase 8 |
| MCP server adapter | Planned integration, `crates/mcp-server` not yet created |
