# AI Trading Assistant — Documentation Hub

> Central navigation for the 10-layer institutional trading strategy documentation.

---

## 10-Layer Decision Pipeline

```
┌─────────────────────────────────────────────────────────────────────┐
│                        RAW MARKET DATA (L0)                         │
│              Hyperliquid WebSocket — OHLCV Candles                   │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L1 — ITIL: Institutional Technical Indicator Layer                  │
│ 51 indicators · 7 groups · 115 signal emissions · 12 SignalKinds    │
│ Transforms OHLCV → NormalizedIndicatorValue + signals[]             │
│ File: docs/layers/01-itil-technical-indicator.md                    │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ indicator values + signals
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L2 — IRCL: Institutional Regime Classification Layer                │
│ 5 regimes · 6 voting indicators · confidence + stability            │
│ Classifies: Trending | Compression | Expansion | Range | Transitional│
│ File: docs/layers/02-ircl-regime-classification.md                  │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ regime + confidence
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L3 — ISML: Institutional Structure Mapping Layer                    │
│ S/R engine · Fibonacci · Volume Profile · SMC (4 sub-engines)       │
│ Chart patterns · Level hierarchy · Structural integrity score       │
│ File: docs/layers/03-isml-structure-mapping.md                      │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ level map + integrity score
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L4 — ICSL: Institutional Confluence Scoring Layer                   │
│ 44 directional contributors · 7 non-directional gates · TF confirm  │
│ Synthesizes L1-L3 → weighted [-100,+100] confluence score           │
│ File: docs/layers/04-icsl-confluence-scoring.md                     │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ confluence + consensus + gates
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L5 — IDCL: Institutional Decision Context Layer                     │
│ 17 metrics: probability · consensus · range · volatility · risk     │
│ quality · reward/risk · stop · regime · trend · readiness           │
│ File: docs/layers/05-idcl-decision-context.md                       │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ decision metrics
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L6 — ISIL: Institutional Statistical Intelligence Layer             │
│ Modules A-F: Distribution · Probability · Confidence · Shape        │
│ Relationships · Monte Carlo + ML: learning · importance · anomaly   │
│ File: docs/layers/06-isil-statistical-intelligence.md               │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ statistical context
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L7 — IRML: Institutional Risk Management Layer                      │
│ 6 categories (A-F) · Position Risk Profile · Capital Allocation     │
│ Drawdown state machine · Consecutive loss engine · Adaptive R:R     │
│ Trade permission gate · Hard execution constraints                  │
│ File: docs/layers/07-irmL-risk-management.md                        │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ risk profile + permission + R:R
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L8 — IASL: Institutional AI Synthesis Layer                         │
│ Two-Agent Pipeline: Analyst (information preparation) → Trader      │
│ Analyst receives ALL L1-L7 data → institutional document            │
│ Trader receives document + position + IRML → final decision         │
│ File: docs/layers/08-iasl-ai-synthesis.md                           │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ trading decision + rationale
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L9 — IEPL: Institutional Execution Protocol Layer                   │
│ 3-layer entry · Fractional Slot Machine (4 FIFO slots)              │
│ Stop placement (7-level hierarchy) · TP tiering · Invalidation      │
│ Break-even trailing · Bracket order constraints                     │
│ File: docs/layers/09-iepl-execution-protocol.md                     │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ realized trade outcomes
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ L10 — IPEL: Institutional Performance Evaluation Layer              │
│ Trade journaling · Journal Agent · Performance metrics              │
│ Direction correctness · Historical Analyst · Regime breakdown       │
│ Adaptive learning → feedback to ICSL/IRML/IEPL                      │
│ File: docs/layers/10-ipel-performance-evaluation.md                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Layer Reference Table

| # | Initials | Full Name | Question Answered | File |
|---|----------|-----------|-------------------|------|
| 1 | ITIL | Institutional Technical Indicator Layer | What mathematical patterns exist in price data? | [01-itil-technical-indicator.md](layers/01-itil-technical-indicator.md) |
| 2 | IRCL | Institutional Regime Classification Layer | What kind of market are we trading in? | [02-ircl-regime-classification.md](layers/02-ircl-regime-classification.md) |
| 3 | ISML | Institutional Structure Mapping Layer | Where are the institutional zones and levels? | [03-isml-structure-mapping.md](layers/03-isml-structure-mapping.md) |
| 4 | ICSL | Institutional Confluence Scoring Layer | How much agreement exists across indicators? | [04-icsl-confluence-scoring.md](layers/04-icsl-confluence-scoring.md) |
| 5 | IDCL | Institutional Decision Context Layer | What do the numbers say probabilistically? | [05-idcl-decision-context.md](layers/05-idcl-decision-context.md) |
| 6 | ISIL | Institutional Statistical Intelligence Layer | How unusual is this statistically? | [06-isil-statistical-intelligence.md](layers/06-isil-statistical-intelligence.md) |
| 7 | IRML | Institutional Risk Management Layer | How much capital should be exposed? | [07-irmL-risk-management.md](layers/07-irmL-risk-management.md) |
| 8 | IASL | Institutional AI Synthesis Layer | What is the optimal trading action? | [08-iasl-ai-synthesis.md](layers/08-iasl-ai-synthesis.md) |
| 9 | IEPL | Institutional Execution Protocol Layer | How exactly to enter, manage, and exit? | [09-iepl-execution-protocol.md](layers/09-iepl-execution-protocol.md) |
| 10 | IPEL | Institutional Performance Evaluation Layer | How well did we do, and how to adapt? | [10-ipel-performance-evaluation.md](layers/10-ipel-performance-evaluation.md) |
| 11 | IMOL | Institutional Monitoring Layer | How is the active trade performing? | [11-imol-monitoring.md](layers/11-imol-monitoring.md) |

---

## Frontend Decision Flow (UI)

The 10-layer analytical pipeline feeds a 5-stage **frontend trade lifecycle** displayed in the dashboard:

```
SETUP ──► TRIGGER ──► CONFIRMATION ──► EXECUTION ──► MONITORING
   │          │            │              │              │
   ▼          ▼            ▼              ▼              ▼
Trend      Momentum     Volume         Confluence     Trade Mgmt
Regime     Price Action Trend Strength Decision       Scale In/Out
Structure  Breakouts    Volatility     Context         Trailing Stop
                        Smart Money                   Partial TP
                        Order Flow                    Exit Signals
```

| Stage | Question | Source | Populated By |
|-------|----------|--------|-------------|
| **Setup** | Is the market tradable? | L1+L2+L3 | Indicators: Trend, Regime, Structure groups |
| **Trigger** | Did an opportunity appear? | L1 | Indicators: Momentum, Price Action, Breakouts |
| **Confirmation** | Is the probability high enough? | L1+L4 | Indicators: Volume, Trend Strength, Volatility, Smart Money, Order Flow |
| **Execution** | Can I enter efficiently? | L4+L5+L8 | Confluence score + Decision Context + AI Synthesis |
| **Monitoring** | How is the active trade performing? | L9+L10+L7 | Active positions, scale slots, exit signals, trailing stop, PnL tracking |

> **Risk Management (IRML)** is a dedicated standalone panel — not a pipeline stage. It handles position sizing, ATR stops, stop loss, take profit, max risk, max daily loss, max allocation, and leverage. Accessed via the GENERAL mode > Risk Management tab. The IRML feeds risk boundaries into both the Execution stage and the Monitoring panel.

---

## Top-Level Documents

| Document | Audience | Description |
|----------|----------|-------------|
| [Institutional Unified Strategy Framework](institutional-unified-strategy-framework.md) | Traders + AI | Complete trading methodology with layer cross-reference table |
| [Architecture](architecture.md) | Developers | System topology, data-flow, crate structure, 7-layer software architecture |
| [Design System](design.md) | Frontend | Grayscale monochrome dark mode design specification |
| [Indicator System Master Spec](indicator-system-master-spec.md) | Developers | Registry manifest, normalized value model, scoring system, phase checklist |
| [Indicators Guide (AI Rulebook)](indicators-guide.md) | LLM / AI Agents | Condensed reference: signal thresholds, AI input schemas, rules per indicator |
| [User Manual](user-manual.md) | End Users | Installation, configuration, dashboard usage, troubleshooting |
| [Fractional Slot Machine](fractional-dynamic-position-slot-machine.md) | Developers | Position lifecycle engine: 4-slot FIFO, cycle capital math, bracket constraints |
| [Glossary](glossary.md) | All | 80+ terms: SMC, Volume Profile, indicators, statistics, risk, execution |
| [Project Plan](plan.md) | Maintainers | Phased roadmap with implementation status |
| [Commission & Fees](commission.md) | Developers | Fee modeling, funding rate decay, viability gating |
| [Stats Compiler](stats-compiler.md) | Developers | Dashboard statistics aggregation, performance analytics |
| [Trigger Engine](trigger-engine.md) | Developers | Trigger dispatch, signal relay, automation hooks |

---

## Indicator Documentation (51 Registry Entries)

| Group | Count | Index | Doc Path |
|-------|-------|-------|----------|
| Trend | 10 | [Group Table](indicators/index.md#trend) | `docs/indicators/` |
| Momentum | 11 | [Group Table](indicators/index.md#momentum) | `docs/indicators/` |
| Volume | 10 | [Group Table](indicators/index.md#volume) | `docs/indicators/` |
| Volatility | 7 | [Group Table](indicators/index.md#volatility) | `docs/indicators/` |
| Structure | 5 | [Group Table](indicators/index.md#structure) | `docs/indicators/` |
| Regime | 4 | [Group Table](indicators/index.md#regime) | `docs/indicators/` |
| Institutional | 4 | [Group Table](indicators/index.md#institutional) | `docs/indicators/` |
| Derivatives Data | 4 | [Group Table](indicators/index.md#derivatives) | `docs/indicators/` |

> **Note:** The Derivatives Data group (Open Interest, OI Delta, Funding Rate, OI-Price Divergence) is planned for Phase 11.

---

## Data Flow Summary

```
Hyperliquid WS → Candle Aggregator (5 TFs)
  → ITIL (51 calculators + normalizers + 115 signal emissions)
    → IRCL (6-indicator regime vote)
    → ISML (S/R/Fib/SMC/Volume Profile/Patterns)
    → ICSL (44-fold weighted directional Σ, 7 gates)
      → IDCL (17 probabilistic metrics)
      → ISIL (6-module statistical enrichment)
      → IRML (6-category risk gatekeeper + R:R)
        → IASL (Analyst: full institutional document → Trader: decision)
          → IEPL (3-layer entry, 4-slot machine, TP/SL tiering, invalidation)
            → IPEL (journaling, metrics, learning feedback)
              → Feedback to ICSL weights, IRML R:R, IEPL sizing
```

---

## Quick Start

```bash
# Read the strategy overview
cat docs/institutional-unified-strategy-framework.md

# Drill into a specific layer
cat docs/layers/01-itil-technical-indicator.md

# Look up an indicator signal
cat docs/indicators/rsi.md

# Understand a term
grep -A2 "BOS" docs/glossary.md
```
