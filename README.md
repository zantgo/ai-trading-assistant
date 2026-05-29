# ⚙️ AI Trading Assistant Dashboard

> **High-performance market telemetry orchestrator and interactive AI decision assistant for Hyperliquid, built in Rust.**

The **AI Trading Assistant** is designed to process high-resolution decentralized finance telemetry and transform raw data into structured, actionable market analysis for human traders. Rather than trading autonomously, it serves as an interactive copilot, running technical indicator analysis in Rust and feeding data on-demand to LLMs via the Model Context Protocol (MCP) to provide structured guidance.

## 🚀 Quick Start Workflow

The system provides a unified helper script (`manage.sh`) at the root level to simplify building, running, and testing the entire workspace.

### 1. Setup Environment
Ensure your `.env` is configured at the workspace root:
```bash
cp .env.example .env
# Open .env and populate your DEEPSEEK_API_KEY
```

### 2. Common Workflow Commands

All key operations can be executed directly from the root directory:

```bash
# Make the manager script executable
chmod +x manage.sh

# 1. Install dependencies & build all components (Frontend + Backend)
./manage.sh build

# 2. Run the platform in the foreground (with live terminal logs)
./manage.sh run

# 3. Run the platform in the background (with silent logs writing to engine.log)
./manage.sh run-silent

# 4. Check status or stop the background execution
./manage.sh status
./manage.sh stop

# 5. Run all test suites (Rust unit/integrations + Svelte 5 Vitest)
./manage.sh test

# 6. Stop engine, clean builds, and permanently delete telemetry.db
./manage.sh destroy
```

Once running, navigate to http://127.0.0.1:3000 to access the dashboard.

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
