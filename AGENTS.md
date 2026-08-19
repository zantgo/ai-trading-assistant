# AGENTS.md

> **Single-operator local deployment.** This platform is built for one operator and their team — no clients, no multi-tenant/SaaS model. One workspace, one operator identity (`local`), no per-route authentication. All audit events carry `operator_id = "local"`.

This project is a **Trading Platform** — a quantitative trading system that ingests live cryptocurrency data from exchanges, computes 52 technical indicators across 4 configurable timeframes, synthesizes multi-timeframe market intelligence, evaluates trade setups, manages portfolio risk, and provides historical performance analytics. Built as a Cargo Workspace of 9 specialized, decoupled crates and a Svelte 5 dashboard.

> **Implementation status (v7.1 — roadmap complete).** All five engines are **implemented and production-ready**: DIE + MME end-to-end; TAE = v7 setup executor on the unified execution engine (`ExecutionBackend`: `PaperSimulation` default, `LiveBroker` + `BitgetLiveBroker` for live dispatch); PME = informational portfolio mirror (safety ladder live); PAE = live analytics + recorded-decision backtest with the full significance treatment (t-test, 10k Monte Carlo, α = 0.05, edge verdict). The roadmap and its verification checklist are closed; `./manage.sh test-doc` reports ALL CHECKS PASSED.

## Project overview

The platform is organized around a **Two-Dimensional Architecture** — 5 specialized logical engines (DIE, MME, TAE, PME, PAE) across sequenced analytical layers. These logical engines are mapped onto 9 physical Rust crates so that "Engine" remains a logical term and the physical directories describe their engineering role.

| Logical Engine | Physical Crate(s) | Responsibility | Status |
|---------------|-------------------|----------------|--------|
| Data Infrastructure Engine (DIE) | `network-adapters` + `database-storage` + `market-analyzer` (L2–L4) | WebSocket / REST ingestion, candle reconstruction, NTP clock monitor, connection-quality tracker; SQLite schema, WAL telemetry logger, queries; candle generation, quality validation, distribution (executes in `market-analyzer` for latency — logical ownership remains DIE's) | Implemented |
| Market Monitoring Engine (MME) | `market-analyzer` | 52 indicators across 4 timeframes, signals, multi-TF alignment, opportunity/risk scoring, decision support, market context synthesis; plus L1.5 (derivatives telemetry) and L2.5 (liquidity synthesis) fractional extension layers | Implemented |
| Trade Automation Engine (TAE) | `portfolio-supervisor` | Setup executor (4-TF top-setup aggregation, zone-midpoint geometry, lifecycle state machine, invalidation), unified execution engine + `ExecutionBackend` (`PaperSimulation` default; `LiveBroker` + `BitgetLiveBroker` live), risk sizing, STOP flatten; live `TradeAutomationDashboard` (`/api/instances/:id/automation`) | Implemented (paper default; live dispatch Hyperliquid + Bitget) |
| Portfolio Management Engine (PME) | `portfolio-supervisor` | Instance lifecycle, session state, safety-state ladder (WARN / CAUTIOUS / SUSPENDED / DRAWDOWN_STOP), capital/margin ledger, position/exposure/capital/portfolio layers; informational (read-only); live `PortfolioDashboard` (`/api/instances/:id/portfolio` + `/safety`) | Implemented (informational) |
| Performance Analytics Engine (PAE) | `performance-analytics` + `database-storage` | Dashboard stats compilation, strategy optimizer, performance evaluator, recorded-decision backtest runner (NHST: t-test, 10k Monte Carlo, α = 0.05, edge verdict); SQLite persistence for analytics tables; live backtest tab (`POST /api/backtest/run`) | Implemented |
| (cross-cutting) | `core-domain` | Stateless DTOs (`MarketSnapshot`, `AnalysisMatrix`, etc.), JSON-RPC 2.0 transport, normalized value maps | Implemented |
| (cross-cutting) | `config-models` | All `*Config` structs + `load_config()` / `load_instances()` readers | Implemented |
| (cross-cutting) | `api-gateway` | Axum HTTP router, Axum `AppState`, WebSocket broadcast server, static asset serving | Implemented |
| (cross-cutting) | `execution-daemon` | Headless CLI binary that wires everything together | Implemented |

```
crates/
├── core-domain/            # Stateless DTOs, JSON-RPC schemas, shared types
├── config-models/          # All *Config structs + load_config() / load_instances()
├── market-analyzer/        # 52 indicators, multi-TF pipeline, decision support
├── database-storage/       # SQLite schema, migrations, WAL telemetry logger, queries
├── network-adapters/       # WS/REST clients, NTP clock monitor, candle reconstruction, connection-quality tracker
├── portfolio-supervisor/   # PME+TAE: instances, sizing, exposure, capital, session, safety vetoes, profile eval
├── performance-analytics/  # Stats compiler, strategy optimizer, perf evaluator
├── api-gateway/            # Axum router, WS broadcast, HTTP handlers, types
└── execution-daemon/       # main.rs: parses CLI, loads config, boots tasks, starts Axum
```

Frontend:

```
ui/           # Svelte 5 + Vite dashboard (served as static assets)
```

The unidirectional dependency graph and the four cycle-breaking design decisions (MarketContext split, RegistryContext extraction, ConnectionQualityTracker split, paper_trading call-site removal) live in **`docs/conceptual-foundations/01-06-crate-layout-and-cycles.md`** — the canonical single source of truth for "where does X live?" and "why don't these two crates import each other?". That document also covers the test-suite topology and the dev-dependency exceptions.

## Build & run

### Prerequisites
- Rust toolchain (stable)
- Bun (for frontend)

### Order matters
```bash
# 1. Build frontend (produces dist/)
cd ui
bun install
bun run build

# 2. Build & run engine from workspace root
cd ../..             # back to workspace root
cargo run --bin execution-daemon -- --web
```

The execution-daemon binary reads `config.toml` from CWD at runtime. Run from the workspace root.

### Launch modes

| Command | Mode | Description |
|---|---|---|
| `./manage.sh run` | Web (GUI) | Foreground with live logs, dashboard at `http://127.0.0.1:3000` |
| `./manage.sh run-silent` | Web (GUI) | Background daemon, logs to `engine.log` |
| `./manage.sh run-cli` | CLI (terminal) | Interactive launch prompt → terminal monitor (`--mode cli`; observe-only, no web server) |
| `./manage.sh stop` | — | Stop background engine instance |
| `./manage.sh status` | — | Check process uptime |

## Session modes (Observe / Simulate / Execute)

The Launch Setup wizard (and the CLI launch prompt) offer three execution modes:

| UI | Backend `ExecutionMode` | Meaning |
|----|-------------------------|---------|
| **Observe** | `observe` | Market/signal monitoring only — the TAE setup executor never evaluates or dispatches orders. No capital, no credentials. |
| **Simulate** | `paper` | Simulated orders against paper capital (starting capital default per instance). |
| **Execute** | `live` | Real orders via `LiveBroker` / `BitgetLiveBroker`; requires an active encrypted exchange key. |

The mode is persisted per instance (`InstanceEntry.mode`) and mirrored at runtime on
`Instance::execution_mode` (the TAE loop gate in `execution-daemon/src/main.rs` skips fills +
setup evaluation for `observe`). The session default (`POST /api/session/init` + `set_session_defaults`)
applies to newly created instances; `POST /api/instances/:id/mode` toggles per instance.

### CLI ↔ GUI parity (observe mode)

The CLI terminal monitor and the GUI Market Overview panel render the **same server-computed
payload**: the L7 aggregation task produces `OverviewMatrix` + the v7.2 panel fields
(`hero`, `overview_rows`, `signal_quality`, `direction_distribution`, `market_health_dims`)
via `core_domain::overview_panel::build_overview_panel`; `GET /api/overview` (GUI) and
`run_terminal_monitor` (CLI) read the same object. One producer, one payload, two renderers —
the 13-check contract lives in `docs/conceptual-foundations/01-10-cli-gui-parity.md` and is
enforced by `test-doc` gate G18. Default TF ladder: registry fallback 60/180/ws-slow/ws-macro,
shared by CLI (`tf_ladder_defaults`) and the wizard (`/api/config`).

### Frontend dev mode
```bash
cd ui
bun run dev          # Vite dev server
bun run check        # svelte-check + tsc typecheck
```

## Runtime details

- Server: `http://127.0.0.1:3000` (localhost only, not 0.0.0.0)
- WebSocket endpoint: `/ws` (serves `MarketSnapshot` JSON)
- Config API: `GET /api/config` (returns parsed `config.toml`)
- History API: `GET /api/history?symbol=&timeframe_secs=&limit=` (default `100`, max `1000`; returns `{ symbol, prices[], candles[], indicator_history }`)
- Connection Quality API: `GET /api/connection-quality?instance_id=…&timeframe_secs=…&window=one_hour|six_hour|twenty_four_hour` (uptime, disconnect count, reconnect latency, score 0..100; when both `instance_id` and `timeframe_secs` are supplied returns per-scope; absent params return process-wide aggregate)
- Database: SQLite, auto-created at `./telemetry.db` on startup
- Market data: Hyperliquid WebSocket (`wss://api.hyperliquid.xyz/ws`) and Bitget WebSocket (`wss://ws.bitget.com/v2/ws/public`)
- Static assets served from `ui/dist`
- **Price-chart overlays** (toggle pills in `ChartToggles.svelte`, opt-in, both default `false`):
  - **LIQ HEATMAP** — `LiquidationHeatmapPrimitive` (`ui/src/lib/liquidationHeatmap.ts`) renders colored horizontal bands at liquidation cluster price zones, fed by `tf.cluster` (per-TF `LiquidationClusterMatrix` since v6.4.2; refreshed at each TF's own candle cadence; see `docs/engines/market-monitoring-engine/03-02-11-mme-liquidity-extension.md` and `docs/conceptual-foundations/01-05-liquidity-domain.md`).
  - **VOL PROFILE** — `VolumeProfilePrimitive` (`ui/src/lib/volumeProfile.ts`) renders a right-edge stacked buy/sell histogram with POC / VAH / VAL labels, fed by `tf.volumeProfile` (per-TF `VolumeProfileSnapshot`; see `docs/engines/market-monitoring-engine/03-02-13-mme-volume-profile-layer.md`). Static bin count from config `volume_profile_bins` (default 100); `num_bins` reports the non-empty bins after filtering.

### Connection Resilience & Quality

- **Reconnect policy**: `crates/network-adapters/src/adapters/resilience.rs` — exponential backoff (1s→30s, ±20% jitter) on WS disconnect; resilient to network crashes with auto-reconnect.
- **Candle reconstruction**: `crates/network-adapters/src/adapters/reconstruction.rs` — detects ingestion gaps on reconnect; ≥1m candles fetched from exchange REST historical, <1m candles synthesized via EMA/last-N closes. Reconstructed candles carry a `reconstructed: Some(ReconstructionMethod)` flag.
- **Clock drift**: `crates/network-adapters/src/clock_monitor.rs` — NTP polling enforces the configured UTC drift budget (`[clock_monitor] threshold_micros`, shipped default 10 ms); default warn loudly on breach, configurable to hard-stop via `[clock_monitor].breach_action = panic`.
- **Quality tracking**: `crates/network-adapters/src/connection_quality_tracker.rs` (in-memory windows + 60s persistence loop) — rolling 1h/6h/24h windows with composite score formula: `50×(uptime_pct/100) + 30×(1 - min(disconnects/10, 1)) + 20×(1 - min(avg_reconnect_ms/5000, 1)) - 5×min(data_loss_s/600, 1) - 5×min(reconstructed_candles/100, 1)`, clamped to 0..100. Connection quality is served live from the in-memory `ConnectionQualityRegistry`; historical samples are persisted to the `connection_quality_samples` table for future analytical queries.

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

## Testing (~2,355 tests across 5 stages)

| Suite | Command | Boundary | Tests | Runtime |
|-------|---------|----------|-------|---------|
| TEST-CORE | `./manage.sh test-core` | Pure math, indicators, serialization, liquidity module (`core-domain`, `market-analyzer`, `config-models`) | 831 | <3s |
| TEST-GOLDEN | `./manage.sh test-golden` | Golden-vector conformance (AUDIT-AIU Phase 10) | 24 | <5s |
| TEST-ENGINE | `./manage.sh test-engine` | DB, server, failover, liquidation e2e, performance analytics, network adapters, daemon (`database-storage`, `api-gateway`, `portfolio-supervisor`, `performance-analytics`, `network-adapters`, `execution-daemon`) | 331 | <10s |
| TEST-DOC | `./manage.sh test-doc` | Documentation corpus: file inventory, worked-example recomputation, grep-based consistency sweeps (`docs/`) | — | <5s |
| TEST-UI | `./manage.sh test-ui` | Svelte 5 runes, components, snapshots, LiquidityPanel (76 test files) | 1125 | ~150s |
| TEST-INDICATORS | `./manage.sh test-indicators` | Per-indicator pipeline e2e (37 candle-based) with terminal console reporting. Exercises calculator → normalizer → signal deriver → lifecycle builder across 4 market patterns. Catches duplicate `(label, kind)` signal pairs that would trigger `each_key_duplicate` in the UI, lifecycle regressions, value-map key collisions. | 44 | ~8s |
| All | `./manage.sh test` | Core → Golden → Indicators → Engine → UI sequentially (5 stages); `test-doc` runs at release time | 2,355 | <3 min |

### Liquidity Intelligence (Phases 0-4) test coverage

| Phase | Test file | Tests | Boundary |
|---|---|---|---|
| 0 | `crates/portfolio-supervisor/tests/phase0_derivatives.rs` | 11 | portfolio-supervisor |
| 1 | `crates/core-domain/tests/phase1_liquidity_flow.rs` + `crates/portfolio-supervisor/tests/phase1_liquidation_e2e.rs` | 15 + 1 | core + portfolio-supervisor |
| 2 | `crates/core-domain/tests/phase2_cluster_matrix.rs` | 14 | core |
| 3 | `crates/core-domain/tests/phase3_signals.rs` | 12 | core |
| 4 | `ui/src/components/LiquidityPanel.test.ts` | 5 | ui |
| **Total** | | **58** | |

### Specialized test selectors

| Command | Targets |
|---------|---------|
| `./manage.sh test-property` | Generative property tests (38 tests across 10 indicator modules) |
| `./manage.sh test-engine-full` | All engine tests including load/stress |

### Developer guidelines

- **Modifying indicators, Fibonacci, models** → `./manage.sh test-core` (fast, <3s)
- **Modifying DB schemas, server APIs** → `./manage.sh test-engine` (<10s)
- **Modifying Svelte 5 runes, components, charts** → `./manage.sh test-ui` (<10s)
- **Modifying normalizer / signal deriver / indicator soft-floor / close-only lifecycle** → `./manage.sh test-indicators` (~8s) — validates no duplicate `(label, kind)` signal pairs are emitted that would trigger `each_key_duplicate` in the frontend.
- **Pre-commit / PR validation** → `./manage.sh test` (full sequential run)

## Architecture notes

- The engine uses a multi-stage pipeline: WebSocket → channel → indicator analysis → broadcast → WebSocket to frontend
- `config.toml` is the single source of truth for all platform parameters — both engine and frontend read it (frontend via `/api/config`)
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
