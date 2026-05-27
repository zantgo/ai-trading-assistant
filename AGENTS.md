# AGENTS.md

This project is configured as a Cargo Workspace containing an ingestion daemon and a Svelte 5 trading dashboard. The goal is to act as an **AI Trading Assistant** that helps human operators make structured manual trade decisions.

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
- History API: `GET /api/history` (returns last 100 close prices)
- Analysis API: `POST /api/analyze` (accepts position + market data, returns structured assistant response)
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
- The local variable holding `getState()` must NOT be named `state` — it conflicts with the `$state` rune. Use `app` or `store` instead.

## Implementation Guidelines

When writing code to realize the AI Assistant workflow, adhere to the following setup instructions:

### 1. Svelte 5 UI Adjustments (`crates/engine/frontend/src/App.svelte`)
- Locate the sidebar component (`<aside class="sidebar-panel">`).
- Add a new input block for tracking the current position status:
  ```svelte
  <div class="position-selector">
    <span class="sub-title">Current Position:</span>
    <label>
      <input type="radio" bind:group={currentPosition} value="None" /> None
    </label>
    <label>
      <input type="radio" bind:group={currentPosition} value="Long" /> Long
    </label>
    <label>
      <input type="radio" bind:group={currentPosition} value="Short" /> Short
    </label>
  </div>
  ```
- Change the placeholder section inside the `"SIGNALS"` box to handle the structured response of the assistant:
  - Add an `"Request AI Assistant Analysis"` button.
  - Create a handler to send a POST request containing:
    1. The selected position (`currentPosition`).
    2. The last 100 historical prices.
    3. The current state parameters.
  - Implement a loading state showing progress as the sequential analysis runs (Trend Check -> Indicators -> Recommendation).

### 2. Rust Ingestion Cache (`crates/engine/src/analyzer.rs`)
- Introduce a sliding window buffer inside the analysis task (e.g., a `VecDeque<Decimal>`) capped at 100 items to store the closing prices of completed candles.
- Expose this vector via an Axum routing handler (`GET /api/history`).

### 3. Structured Assistant Prompt Template
When submitting payload parameters to your LLM, supply a system prompt designed to return JSON matching the following schema:
```json
{
  "trend_analysis": {
    "classification": "trending upwards | trending downwards | sideways",
    "structural_reasoning": "Brief description of the raw price actions observed in the last 100 steps."
  },
  "indicator_alignment": {
    "classification": "supportive | conflicting | neutral",
    "observation": "Brief detail on how key variables like Squeeze Momentum, MACD, and RSI match the trend."
  },
  "position_recommendation": {
    "action": "Hold | Close | Wait | Open Long | Open Short",
    "rationale": "Clear operational reasoning guiding the user on the optimal step given their position context."
  }
}
```

By keeping tasks manual, structured, and strictly advisory, the codebase retains its performance traits without introducing autonomous execution risks.
