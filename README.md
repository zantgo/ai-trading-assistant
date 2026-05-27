# ⚙️ DeX AI Trading Assistant Dashboard

> **High-performance market telemetry orchestrator and interactive AI decision assistant for Hyperliquid, built in Rust.**

The **DeX AI Trading Assistant** is designed to process high-resolution decentralized finance telemetry and transform raw data into structured, actionable market analysis for human traders. Rather than trading autonomously, it serves as an interactive copilot, running technical indicator analysis in Rust and feeding data on-demand to LLMs via the Model Context Protocol (MCP) to provide structured guidance.

## 🚀 Core Assistant Workflow
1. **Live Telemetry & Indicators:** High-frequency WebSocket updates compute EMAs, RSI, Squeeze, MACD, ADX, Bollinger Bands, ATR, and VWAP in Rust.
2. **On-Demand Assistant Analysis:** A visual interface lets you input your current position state (`None`, `Long`, or `Short`) and request an AI review.
3. **Structured Sequential AI Logic:** When triggered, the assistant feeds historical price arrays (last 100 intervals) and indicator parameters to the AI model to perform a multi-stage analysis:
   * **Stage 1 (Price Action Trend):** Categorizes the market trend (`trending upwards`, `trending downwards`, or `sideways`).
   * **Stage 2 (Indicator Validation):** Matches the trend against the mathematical indicators.
   * **Stage 3 (Recommendation Engine):** Melds position context and market evaluation to output a deterministic trade action (`Hold`, `Close`, `Wait`, `Open Long`, or `Open Short`).

## 🏗️ Workspace Structure
- `crates/shared`: Shared domain structures (`MarketSnapshot`) and technical indicator math engines.
- `crates/engine`: Ingestion engine, WebSocket client, SQLite persistence, and HTTP/WS server serving dashboard assets.
- `crates/mcp-server` *(Planned Integration)*: MCP adapter translating telemetry history and position context into structured AI prompt parameters.

## 📚 Documentation

| Document | Audience | Description |
|---|---|---|
| **[User Manual](docs/user-manual.md)** | End Users | Installation, configuration, LLM setup, dashboard usage, AI assistant workflow, troubleshooting |
| **[Architecture](docs/architecture.md)** | Developers | System topology, data-flow diagrams, on-demand assistant loop, structured reasoning sequence |
| **[Project Plan](docs/plan.md)** | Maintainers | Phased execution roadmap with implementation status for each milestone |
| **[AGENTS.md](AGENTS.md)** | AI Agents | Build instructions, runtime details, testing conventions, implementation guidelines for LLM-based contributors |

---

## ⚠️ Disclaimer
This system is an information tool for **research and educational purposes only**. It does not execute trades automatically. All financial execution remains the sole responsibility of the user.
