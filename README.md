# ⚙️ AI Trading Assistant Dashboard

> **High-performance market telemetry orchestrator and interactive AI decision assistant for Hyperliquid, built in Rust. 51 indicators, 10-layer institutional strategy pipeline, two-agent AI synthesis.**

The **AI Trading Assistant** is an institutional-grade trading decision support system that streams live Hyperliquid market data, computes 51 technical indicators across 5 timeframes, and synthesizes all analysis through a two-agent AI pipeline (Analyst → Trader) powered by DeepSeek. It does **not** execute trades autonomously — it is a copilot for human operators.

## 🚀 Quick Start Workflow

The system provides a unified helper script (`manage.sh`) at the root level.

```bash
chmod +x manage.sh
cp .env.example .env        # Add your DEEPSEEK_API_KEY
./manage.sh build            # Build frontend + backend
./manage.sh run              # Start at http://127.0.0.1:3000
```

## 🏗️ Workspace Structure

- `crates/shared`: Domain models (`MarketSnapshot`), 51 indicator calculators, DecisionContext, MarketContext, Statistical Intelligence (SIL), risk objects
- `crates/engine`: Ingestion daemon, WebSocket client, SQLite persistence, Axum HTTP/WS server, paper trading, two-agent LLM pipeline, automation
- `crates/frontend`: Svelte 5 dashboard with real-time charts, AI analysis, paper trading controls, risk management, performance analytics

## 📚 Documentation — 10-Layer Decision Pipeline

| # | Layer | Document |
|---|-------|----------|
| | **[Index](docs/index.md)** | Full navigation hub with ASCII pipeline diagram |
| 1 | ITIL — Institutional Technical Indicator Layer | [layers/01-itil-technical-indicator.md](docs/layers/01-itil-technical-indicator.md) |
| 2 | IRCL — Institutional Regime Classification Layer | [layers/02-ircl-regime-classification.md](docs/layers/02-ircl-regime-classification.md) |
| 3 | ISML — Institutional Structure Mapping Layer | [layers/03-isml-structure-mapping.md](docs/layers/03-isml-structure-mapping.md) |
| 4 | ICSL — Institutional Confluence Scoring Layer | [layers/04-icsl-confluence-scoring.md](docs/layers/04-icsl-confluence-scoring.md) |
| 5 | IDCL — Institutional Decision Context Layer | [layers/05-idcl-decision-context.md](docs/layers/05-idcl-decision-context.md) |
| 6 | ISIL — Institutional Statistical Intelligence Layer | [layers/06-isil-statistical-intelligence.md](docs/layers/06-isil-statistical-intelligence.md) |
| 7 | IRML — Institutional Risk Management Layer | [layers/07-irmL-risk-management.md](docs/layers/07-irmL-risk-management.md) |
| 8 | IASL — Institutional AI Synthesis Layer | [layers/08-iasl-ai-synthesis.md](docs/layers/08-iasl-ai-synthesis.md) |
| 9 | IEPL — Institutional Execution Protocol Layer | [layers/09-iepl-execution-protocol.md](docs/layers/09-iepl-execution-protocol.md) |
| 10 | IPEL — Institutional Performance Evaluation Layer | [layers/10-ipel-performance-evaluation.md](docs/layers/10-ipel-performance-evaluation.md) |

**Top-Level Documents:**

| Document | Audience | Description |
|----------|----------|-------------|
| [Institutional Unified Strategy Framework](docs/institutional-unified-strategy-framework.md) | Traders + AI | Complete trading methodology with layer cross-references |
| [User Manual](docs/user-manual.md) | End Users | Installation, configuration, dashboard usage, troubleshooting |
| [Architecture](docs/architecture.md) | Developers | System topology, data-flow, crate structure |
| [Design System](docs/design.md) | Frontend | Grayscale monochrome dark mode specification |
| [Glossary](docs/glossary.md) | All | 80+ institutional trading terms |
| [Project Plan](docs/plan.md) | Maintainers | Phased roadmap |
| [Indicator Index](docs/indicators/index.md) | All | 51 indicators across 7 groups with signal types |
| [AGENTS.md](AGENTS.md) | AI Agents | Build instructions, testing, implementation guidelines |

## 🔑 Key Metrics

| Metric | Count |
|--------|-------|
| Indicators | 51 (44 directional, 7 non-directional gates) |
| Signal Types | 12 SignalKinds |
| Signal Emissions | 115 per candle |
| AI Pipeline | 2-agent (Analyst → Trader) |
| Strategy Layers | 10 |
| Timeframes | 5 (15s / 1m / 5m / 15m / 1h + 1h/4h structural) |
| Test Suites | 248 tests (core + engine + UI) |

---

## ⚠️ Disclaimer
This system is an information tool for **research and educational purposes only**. It does not execute trades automatically. All financial execution remains the sole responsibility of the user.
