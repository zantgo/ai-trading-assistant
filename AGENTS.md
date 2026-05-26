# AGENTS.md

## Project overview
Rust workspace with 2 crates (`shared`, `engine`) and an embedded Svelte 5 frontend.

```
crates/shared/       — MarketSnapshot model, technical indicators (EMA, RSI, MACD, ADX, BB, Squeeze, ATR)
crates/engine/       — Binary: Hyperliquid WS ingestion, indicator pipeline, Axum web server, SQLite telemetry
crates/engine/frontend/ — Svelte 5 + Vite dashboard (served as static assets by the engine binary)
```

The README and docs reference `crates/mcp-server` and `crates/frontend` as separate crates — these do NOT exist yet.

## Build & run

### Prerequisites
- Rust toolchain (stable)
- Node.js / Bun (for frontend)

### Order matters
```bash
# 1. Build frontend (produces dist/)
cd crates/engine/frontend
npm install          # or: bun install
npm run build        # or: bun run build

# 2. Build & run engine from workspace root
cd ../..             # back to workspace root
cargo run
```

The engine binary reads `config.toml` from CWD at runtime. Run from the workspace root.

### Frontend dev mode
```bash
cd crates/engine/frontend
npm run dev          # Vite dev server
npm run check        # svelte-check + tsc typecheck
```

## Runtime details

- Server: `http://127.0.0.1:3000` (localhost only, not 0.0.0.0)
- WebSocket endpoint: `/ws` (serves `MarketSnapshot` JSON)
- Config API: `GET /api/config` (returns parsed `config.toml`)
- Database: SQLite, auto-created at `./telemetry.db` on startup
- Market data: Hyperliquid **Testnet** WebSocket (`wss://api.hyperliquid-testnet.xyz/ws`)
- Static assets served from `crates/engine/frontend/dist`

## Configuration

`config.toml` at workspace root controls indicator lookback windows and candle duration. Parsed at startup by `main.rs`. If missing, the engine panics.

## Testing

No tests exist yet. There is no CI, no lint configuration, no rustfmt.toml. When adding tests:
- Run a single crate's tests: `cargo test -p shared` or `cargo test -p engine`
- Run all tests: `cargo test` from workspace root

## Architecture notes

- The engine uses a multi-stage pipeline: WebSocket → channel → indicator analysis → broadcast → WebSocket to frontend
- `config.toml` is the single source of truth for indicator periods — both engine and frontend read it (frontend via `/api/config`)
- The Svelte frontend uses Svelte 5 runes (`$state`, `$effect`) — not Svelte 4 syntax
- Candle aggregation happens server-side; the broadcast includes both completed candle snapshots and "shadow" (real-time flickering) values
