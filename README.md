# ⚙️ DeX Trading Agent Engine

> **Autonomous AI trading orchestrator and data flywheel for Hyperliquid, engineered in Rust.**

The **DeX Trading Agent Engine** is a high-performance system designed to bridge Large Language Models (LLMs) with decentralized finance. Built on a modular Rust workspace, it captures high-resolution market telemetry, executes AI-driven reasoning, and persists outcomes for continuous model improvement.

## 🚀 Key Features
*   **High-Throughput Ingestion:** Async WebSocket engine powered by `Tokio`.
*   **Data Science Flywheel:** Telemetry pipeline mapping Market State → AI Inference → Execution Outcome.
*   **Model Context Protocol (MCP):** Exposes live trading tools directly to AI agents.
*   **Safety-First Design:** Deterministic risk guardrails for automated execution.
*   **Production-Ready:** Built with Rust for safety, speed, and memory efficiency.

## 🏗️ Architecture
The engine is structured as a modular Cargo Workspace:
- `crates/shared`: Core domain models and trading traits.
- `crates/engine`: Core trading logic, WebSocket clients, and exchange adapters.
- `crates/mcp-server`: MCP implementation for LLM integration.
- `crates/frontend`: Wasm-based visual verification dashboard.

---

## ⚠️ Disclaimer
This software is for **research and data science purposes only**. It involves high-risk financial instruments (perpetual futures). Do not trade with real capital until you have extensively tested the system in paper-trading environments.
