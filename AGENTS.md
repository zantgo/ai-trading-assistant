# AGENTS.md

This project is configured as a Cargo Workspace containing an ingestion daemon and a Svelte 5 trading dashboard. The goal is a **Quantitative Trading Engine** for structured manual trade decisions using deterministic statistical models.

## Project overview
Rust workspace with 2 crates (`shared`, `engine`) and a Svelte 5 frontend.

```
crates/shared/       — MarketSnapshot model, technical indicators (EMA, RSI, MACD, ADX, BB, Squeeze, ATR), statistics
crates/engine/       — Binary: Hyperliquid WS ingestion, indicator pipeline, Axum web server, SQLite telemetry
crates/frontend/ — Svelte 5 + Vite dashboard (served as static assets by the engine binary)
```

## Build & run

### Prerequisites
- Rust toolchain (stable)
- Node.js / Bun (for frontend)

### Order matters
```bash
# 1. Build frontend (produces dist/)
cd crates/frontend
npm install
npm run build

# 2. Build & run engine from workspace root
cd ../..
cargo run
```

The engine binary reads `config.toml` from CWD at runtime. Run from the workspace root.

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

- Server: `http://127.0.0.1:3000` (localhost only)
- WebSocket endpoint: `/ws` (serves `MarketSnapshot` JSON)
- Config API: `GET /api/config` (returns parsed `config.toml`)
- History API: `GET /api/history` (returns last 100 close prices)
- Decision API: `POST /api/decision` (accepts market data, returns deterministic decision)
- Database: SQLite, auto-created at `./telemetry.db` on startup
- Market data: Hyperliquid WebSocket
- Static assets served from `crates/frontend/dist`

## Configuration

`config.toml` at workspace root controls indicator lookback windows, candle duration, decision matrix weights, risk thresholds, and automation intervals. Parsed at startup by `main.rs`.

### Decision Matrix

The `[decision]` section configures the deterministic weighted multi-factor engine:

```toml
[decision]
exit_opposite_threshold = 60.0
anomaly_threshold = 0.85
max_signal_age_bars = 5
w_confluence = 0.25
w_readiness = 0.20
w_quality = 0.15
w_safety = 0.15
w_trend = 0.10
w_regime_conf = 0.10
w_breakout = 0.05
regime_mult_trending = 1.0
regime_mult_expansion = 0.9
regime_mult_range = 0.7
regime_mult_compression = 0.5
regime_mult_transitional = 0.0
open_threshold_trending = 0.55
open_threshold_expansion = 0.60
open_threshold_range = 0.70
open_threshold_compression = 999
```

## Testing

| Suite | Command | Boundary | Tests |
|-------|---------|----------|-------|
| TEST-CORE | `./manage.sh test-core` | Pure math, indicators, serialization | 154 |
| TEST-ENGINE | `./manage.sh test-engine` | DB, paper trading, server, failover | ~67 |
| TEST-UI | `./manage.sh test-ui` | Svelte 5 runes, components, snapshots | ~11 |
| All | `./manage.sh test` | Core → Engine → UI sequentially | ~232 |

## Architecture notes

- The engine uses a multi-stage pipeline: WebSocket → channel → indicator analysis → broadcast → WebSocket to frontend
- `config.toml` is the single source of truth for indicator periods — both engine and frontend read it (frontend via `/api/config`)
- The Svelte frontend uses Svelte 5 runes (`$state`, `$effect`)
- Candle aggregation happens server-side
- The local variable holding `getState()` must NOT be named `state` — it conflicts with the `$state` rune. Use `app` or `store` instead.
- All decisions are deterministic — no LLM/AI calls in the pipeline

## Documentation Structure

The project uses an institutional trading strategy documentation system under `docs/layers/`. Each layer follows a consistent format (Purpose → Inputs → Outputs → Sub-Components → Integration).

| Layer | Initials | File |
|-------|----------|------|
| 1 — Technical Indicator | ITIL | `docs/layers/01-itil-technical-indicator.md` |
| 2 — Regime Classification | IRCL | `docs/layers/02-ircl-regime-classification.md` |
| 3 — Structure Mapping | ISML | `docs/layers/03-isml-structure-mapping.md` |
| 4 — Confluence Scoring | ICSL | `docs/layers/04-icsl-confluence-scoring.md` |
| 5 — Decision Context | IDCL | `docs/layers/05-idcl-decision-context.md` |
| 6 — Statistical Intelligence | ISIL | `docs/layers/06-isil-statistical-intelligence.md` |
| 7 — Risk Management | IRML | `docs/layers/07-irmL-risk-management.md` |
| 8 — Decision Matrix | IDML | `docs/layers/08-idml-decision-matrix.md` |
| 9 — Execution Protocol | IEPL | `docs/layers/09-iepl-execution-protocol.md` |
| 10 — Performance Evaluation | IPEL | `docs/layers/10-ipel-performance-evaluation.md` |

## Frontend CSS Management

Every Svelte component with custom styles must follow the **Scoped CSS Modules** pattern. Chart-only components that only render a raw canvas via Lightweight Charts with a minimal wrapper style do not need companion stylesheets. No single source file may exceed 1000 lines.
