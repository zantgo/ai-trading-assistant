# AGENTS.md

This project is a **Trading Platform** — a complete quantitative trading system that ingests live cryptocurrency data from exchanges, computes 50 technical indicators across 4 configurable timeframes, synthesizes multi-timeframe market intelligence, evaluates execution policies, manages portfolio risk, and provides historical performance analytics. Built as a Cargo Workspace of 9 specialized, decoupled crates and a Svelte 5 dashboard.

## Project overview

The platform is organized around a **Two-Dimensional Architecture** — 5 specialized logical engines (DIE, MME, TAE, PME, PAE) across sequenced analytical layers. These logical engines are mapped onto 9 physical Rust crates so that "Engine" remains a logical term and the physical directories describe their engineering role.

| Logical Engine | Physical Crate(s) | Responsibility |
|---------------|-------------------|----------------|
| Data Infrastructure Engine (DIE) | `network-adapters` + `database-storage` | WebSocket / REST ingestion, candle reconstruction, NTP clock monitor, connection-quality tracker; SQLite schema, WAL telemetry logger, queries |
| Market Monitoring Engine (MME) | `market-analyzer` | 50 indicators across 4 timeframes, signals, multi-TF alignment, opportunity/risk scoring, decision support, market context synthesis |
| Trade Automation Engine (TAE) | `portfolio-supervisor` | Policy evaluation, position sizing, profile evaluation, trigger engine |
| Portfolio Management Engine (PME) | `portfolio-supervisor` + `api-gateway` | Instance lifecycle, session state, safety vetoes, capital/margin ledger, capital matrix veto |
| Performance Analytics Engine (PAE) | `performance-analytics` | Dashboard stats compilation, strategy optimizer, performance evaluator |
| (cross-cutting) | `core-domain` | Stateless DTOs (`MarketSnapshot`, `AnalysisMatrix`, etc.), JSON-RPC 2.0 transport, normalized value maps |
| (cross-cutting) | `config-models` | All `*Config` structs + `load_config()` / `load_instances()` readers |
| (cross-cutting) | `api-gateway` | Axum HTTP router, Axum `AppState`, WebSocket broadcast server, static asset serving |
| (cross-cutting) | `execution-daemon` | Headless CLI binary that wires everything together |

```
crates/
├── core-domain/            # Stateless DTOs, JSON-RPC schemas, shared types
├── config-models/          # All *Config structs + load_config() / load_instances()
├── market-analyzer/        # 50 indicators, multi-TF pipeline, decision support
├── database-storage/       # SQLite schema, migrations, WAL telemetry logger, queries
├── network-adapters/       # WS/REST clients, NTP clock monitor, candle reconstruction
├── portfolio-supervisor/   # Instances, sizing, safety vetoes, session, profile eval
├── performance-analytics/  # Stats compiler, strategy optimizer, perf evaluator
├── api-gateway/            # Axum router, WS broadcast, HTTP handlers, types
└── execution-daemon/       # main.rs: parses CLI, loads config, boots tasks, starts Axum
```

Frontend:

```
crates/frontend/           # Svelte 5 + Vite dashboard (served as static assets)
```

The unidirectional dependency graph (core-domain is a leaf; nothing in the workspace depends on the leaf except transitively):

```
                       [config-models]
                            ▲
                            │
                       [core-domain]
                       ▲   ▲   ▲   ▲
                       │   │   │   └─────────────────────┐
                       │   │   │                         │
               [market-analyzer]│                  [database-storage]
                                 │                       ▲   ▲
                                 │                       │   │
                                 │                       │   │
                        [network-adapters]               │   │
                                 ▲                       │   │
                                 │                       │   │
                                 └──────────── [portfolio-supervisor]
                                                         ▲
                                                         │
                                                 [performance-analytics]
                                                         ▲
                                                         │
                                                    [api-gateway]
                                                         ▲
                                                         │
                                                  [execution-daemon]
```

## Build & run

### Prerequisites
- Rust toolchain (stable)
- Node.js / Bun (for frontend)

### Order matters
```bash
# 1. Build frontend (produces dist/)
cd crates/frontend
npm install          # or: bun install
npm run build        # or: bun run build

# 2. Build & run engine from workspace root
cd ../..             # back to workspace root
cargo run --bin execution-daemon -- --web
```

The execution-daemon binary reads `config.toml` (legacy: `config.json`) from CWD at runtime. Run from the workspace root.

### Launch modes

| Command | Mode | Description |
|---|---|---|
| `./manage.sh run` | Web (GUI) | Foreground with live logs, dashboard at `http://127.0.0.1:3000` |
| `./manage.sh run-silent` | Web (GUI) | Background daemon, logs to `engine.log` |
| `./manage.sh stop` | — | Stop background engine instance |
| `./manage.sh status` | — | Check process uptime |

### Frontend dev mode
```bash
cd crates/frontend
npm run dev          # Vite dev server
npm run check        # svelte-check + tsc typecheck
```

## Runtime details

- Server: `http://127.0.0.1:3000` (localhost only, not 0.0.0.0)
- WebSocket endpoint: `/ws` (serves `MarketSnapshot` JSON)
- Config API: `GET /api/config` (returns parsed `config.json`)
- History API: `GET /api/history` (returns last 100 close prices)
- Connection Quality API: `GET /api/connection-quality?window=one_hour|six_hour|twenty_four_hour` (uptime, disconnect count, reconnect latency, score 0..100)
- Database: SQLite, auto-created at `./telemetry.db` on startup
- Market data: Hyperliquid WebSocket (`wss://api.hyperliquid.xyz/ws`) and Bitget WebSocket (`wss://ws.bitget.com/v2/ws/public`)
- Static assets served from `crates/frontend/dist`

### Connection Resilience & Quality

- **Reconnect policy**: `crates/network-adapters/src/adapters/resilience.rs` — exponential backoff (1s→30s, ±20% jitter) on WS disconnect; resilient to network crashes with auto-reconnect.
- **Candle reconstruction**: `crates/network-adapters/src/adapters/reconstruction.rs` — detects ingestion gaps on reconnect; ≥1m candles fetched from exchange REST historical, <1m candles synthesized via EMA/last-N closes. Reconstructed candles carry a `reconstructed: Some(ReconstructionMethod)` flag.
- **Clock drift**: `crates/network-adapters/src/clock_monitor.rs` — NTP polling enforces ≤50µs UTC drift budget; default warn loudly on breach, configurable to panic via `[clock_monitor].breach_action`.
- **Quality tracking**: `crates/network-adapters/src/connection_quality_tracker.rs` (in-memory) + `crates/database-storage/src/connection_quality_persistence/mod.rs` (DB writer) — rolling 1h/6h/24h windows with composite score formula: `0.5×uptime_pct + 30×(1 - min(disconnects/10, 1)) + 20×(1 - min(avg_reconnect_ms/5000, 1))`, clamped to 0..100.

## Configuration

`config.toml` at workspace root controls indicator lookback windows, candle duration, exchange endpoints, engine toggles, fee rates, leverage, safety thresholds, and all per-engine parameters. Parsed at startup by `crates/config-models/src/lib.rs::load_config()`. If missing, the daemon panics.

## Documentation

Full specification documents under `docs/`:

| Directory | Contents |
|-----------|----------|
| `docs/conceptual-foundations/` | Global architecture, ontology, data flow, timeframe model |
| `docs/engines/` | Per-engine overview + layer specifications (DIE, MME, TAE, PME, PAE) |
| `docs/matrices/` | Matrix schema contracts (Metrics, Alignment, Analysis, Opportunity, Risk, Decision, Overview, etc.) |
| `docs/integration-and-api/` | API gateway contract, database schema |
| `docs/ui-ux/` | Dashboard layout, component specifications |
| `docs/operations-and-compliance/` | Pre-trade risk controls, user manual, connection resilience, candle reconstruction, connection quality, clock monitor |

Start at `docs/README.md` for a guided reading order.

## Testing (481 tests across 3 boundaries)

| Suite | Command | Boundary | Tests | Runtime |
|-------|---------|----------|-------|---------|
| TEST-CORE | `./manage.sh test-core` | Pure math, indicators, serialization, liquidity module (`core-domain`, `market-analyzer`, `config-models`) | ~280 | <3s |
| TEST-ENGINE | `./manage.sh test-engine` | DB, server, failover, liquidation e2e, performance analytics, network adapters, daemon (`database-storage`, `api-gateway`, `portfolio-supervisor`, `performance-analytics`, `network-adapters`, `execution-daemon`) | ~177 | <10s |
| TEST-UI | `./manage.sh test-ui` | Svelte 5 runes, components, snapshots, LiquidityPanel | 24 | <10s |
| All | `./manage.sh test` | Core → Engine → UI sequentially | 481 | <20s |

### Liquidity Intelligence (Phases 0-4) test coverage

| Phase | Test file | Tests | Boundary |
|---|---|---|---|
| 0 | `crates/engine/tests/phase0_derivatives.rs` | 11 | engine |
| 1 | `crates/shared/tests/phase1_liquidity_flow.rs` + `crates/engine/tests/phase1_liquidation_e2e.rs` | 15 + 1 | core + engine |
| 2 | `crates/shared/tests/phase2_cluster_matrix.rs` | 14 | core |
| 3 | `crates/shared/tests/phase3_signals.rs` | 10 | core |
| 4 | `crates/frontend/src/components/LiquidityPanel.test.ts` | 5 | ui |
| **Total** | | **56** | |

### Specialized test selectors

| Command | Targets |
|---------|---------|
| `./manage.sh test-property` | Generative property tests (38 tests across 10 indicator modules) |
| `./manage.sh test-engine-full` | All engine tests including load/stress |

### Developer guidelines

- **Modifying indicators, Fibonacci, models** → `./manage.sh test-core` (fast, <3s)
- **Modifying DB schemas, server APIs** → `./manage.sh test-engine` (<5s)
- **Modifying Svelte 5 runes, components, charts** → `./manage.sh test-ui` (<10s)
- **Pre-commit / PR validation** → `./manage.sh test` (full sequential run)

## Architecture notes

- The engine uses a multi-stage pipeline: WebSocket → channel → indicator analysis → broadcast → WebSocket to frontend
- `config.json` is the single source of truth for all platform parameters — both engine and frontend read it (frontend via `/api/config`)
- The Svelte frontend uses Svelte 5 runes (`$state`, `$effect`) — not Svelte 4 syntax
- Candle aggregation happens server-side; the broadcast includes both completed candle snapshots and "shadow" (real-time flickering) values
- The local variable holding `getState()` must NOT be named `state` — it conflicts with the `$state` rune. Use `app` or `store` instead.

## Frontend CSS Management

Every Svelte component with custom styles must follow the **Scoped CSS Modules** pattern:

1. **Extraction:** Remove the `<style>` block from the `.svelte` file entirely and move it into a companion `[ComponentName].module.css` file in the same directory.
2. **Import:** In the `<script>` block, add `import styles from './[ComponentName].module.css';`.
3. **Binding:** Map CSS classes to elements using `class={styles.className}` syntax. For conditional classes use template literals: `class="{styles.baseClass} {condition ? styles.active : ''}"`.
4. **Naming:** CSS class names use kebab-case (`.welcome-card`). The Vite config maps these to `camelCaseOnly`, so reference them as `styles.welcomeCard`.
5. **Exception:** Chart-only components (AtrChart, RsiChart, MacdChart, SqueezeChart, VolumeChart, AdxChart) that only render a raw canvas via Lightweight Charts with a minimal wrapper style (`.chart-container { width:100%; height:100% }`) do not need companion stylesheets.
6. **Line limit:** No single source file (`.svelte`, `.ts`, `.css`) may exceed 1000 lines of code.
