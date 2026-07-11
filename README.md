# Market Monitor Dashboard

> **High-performance market telemetry monitor for Hyperliquid and Bitget, built in Rust.**

The **Market Monitor** processes high-resolution exchange telemetry and transforms raw data into real-time technical indicator visualizations. It serves as a comprehensive market observation tool — it does **not** execute trades — computing 43+ technical indicators (58 registry entries with 101 signal-type declarations) in Rust and streaming live data to a Svelte 5 dashboard via WebSocket.

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
- `crates/shared`: Shared domain structures (`MarketSnapshot`) and technical indicator math engines (34+ indicators).
- `crates/engine`: Ingestion engine, WebSocket clients (Hyperliquid + Bitget), SQLite persistence, and HTTP/WS server serving dashboard assets.
- `crates/frontend`: Svelte 5 dashboard with interactive charting, real-time data, and market analysis tools.

## Documentation

| Document | Audience | Description |
|---|---|---|
| **[User Manual](docs/user-manual.md)** | End Users | Installation, configuration, dashboard usage, troubleshooting |
| **[Architecture](docs/architecture.md)** | Developers | System topology, data-flow diagrams, indicator pipeline |
| **[Ontology](docs/ontology.md)** | Developers | Formal ontology: 5 ontological levels, 12 classification axes |
| **[Metrics Matrix](docs/metrics-matrix-reference.md)** | Developers | Complete 58-indicator × 12-signal reference table |
| **[Monitor Matrices](docs/monitor-matrices-reference.md)** | Developers | Metrics → State → Decision three-stage architecture |
| **[Indicator Spec](docs/indicator-system-master-spec.md)** | Developers | Master specification: layers, registry, scoring, per-indicator specs |
| **[AGENTS.md](AGENTS.md)** | AI Agents | Build instructions, runtime details, testing conventions |
