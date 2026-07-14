# Market Monitor Dashboard

> **High-performance market telemetry monitor for Hyperliquid and Bitget, built in Rust.**

The **Market Monitor** processes high-resolution exchange telemetry and transforms raw data into real-time technical indicator visualizations. It serves as a comprehensive market observation tool — it does **not** execute trades — computing 50 technical indicators across 8 functional groups (Trend, Momentum, Volume, Volatility, Structure, Regime, Institutional, Derivatives Data) with 100 signal-kind declarations across 12 SignalKind types (Divergence, Crossover, Threshold, Breakout, BandTouch, ZeroLineCross, CompressionRelease, LevelTest, TrendFlip, VolumeClimax, StackChange, PatternForming), 8 of which support bull/bear divergence detection. All computation runs in Rust, streaming live data to a Svelte 5 dashboard via WebSocket.

## Quick Start Workflow

The system provides a unified helper script (`manage.sh`) at the root level to simplify building, running, and testing the entire workspace.

### 1. Common Workflow Commands

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

## Workspace Structure
- `crates/shared`: Shared domain structures (`MarketSnapshot`) and technical indicator math engines (50 indicators across 8 groups).
- `crates/engine`: Ingestion engine, WebSocket clients (Hyperliquid + Bitget), SQLite persistence, and HTTP/WS server serving dashboard assets.
- `crates/frontend`: Svelte 5 dashboard with interactive charting, real-time data, and market analysis tools.

## Documentation

The complete institutional documentation set lives under [`docs/`](docs/), organized into conceptual foundations, matrix data contracts, per-engine specifications, integration/API references, and UI/UX layouts.

| Document | Audience | Description |
|---|---|---|
| **[Global Architecture](docs/conceptual-foundations/global-architecture.md)** | Developers | Two-dimensional framework: 5 engines x sequenced analytical layers |
| **[Ontology](docs/conceptual-foundations/ontology.md)** | Developers | Formal ontology: engines, layers, matrices, 12 evaluation axes |
| **[Systemic Data Flow](docs/conceptual-foundations/systemic-data-flow.md)** | Developers | Chronological data-flow sequences across all engines |
| **[Timeframe Model](docs/conceptual-foundations/timeframe-model.md)** | Developers | Configurable 4-tier timeframe model (micro/fast/slow/macro) |
| **[Matrices](docs/matrices/)** | Developers | Physical schemas and JSON contracts for all 11 matrices (MME + DIE) |
| **[Market Monitoring Engine](docs/engines/market-monitoring-engine/)** | Developers | 7 layer specs, indicator guide, signal guide, per-indicator specs |
| **[Trade Automation Engine](docs/engines/trade-automation-engine/)** | Developers | Policy + Execution layer specs, paper trading, execution policy spec |
| **[Portfolio Management Engine](docs/engines/portfolio-management-engine/)** | Developers | Position + Exposure + Capital + Portfolio layer specs |
| **[Performance Analytics Engine](docs/engines/performance-analytics-engine/)** | Developers | Trade + Strategy + Risk + Performance layer specs |
| **[Integration & API](docs/integration-and-api/api-gateway-contract.md)** | Developers | REST/WebSocket/JSON-RPC contracts and database schema |
| **[UI/UX](docs/ui-ux/ui-overview-spec.md)** | Frontend | Svelte 5 state management and dashboard layout specifications |
| **[AGENTS.md](AGENTS.md)** | Maintainers | Build instructions, runtime details, testing conventions |
