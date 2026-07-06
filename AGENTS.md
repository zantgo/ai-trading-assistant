# AGENTS.md

This project is configured as a Cargo Workspace containing an ingestion daemon and a Svelte 5 trading dashboard. The goal is to act as an **AI Trading Assistant** that helps human operators make structured manual trade decisions.

## Project overview
Rust workspace with 2 crates (`shared`, `engine`) and a Svelte 5 frontend.

```
crates/shared/       — MarketSnapshot model, technical indicators (EMA, RSI, MACD, ADX, BB, Squeeze, ATR)
crates/engine/       — Binary: Hyperliquid WS ingestion, indicator pipeline, Axum web server, SQLite telemetry
crates/frontend/ — Svelte 5 + Vite dashboard (served as static assets by the engine binary)
```

The README and docs reference `crates/mcp-server` and `crates/frontend` as separate crates — these do NOT exist yet.

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
cargo run
```

The engine binary reads `config.toml` from CWD at runtime. Run from the workspace root.

### Launch modes

| Command | Mode | Description |
|---|---|---|
| `./manage.sh run` | Web (GUI) | Foreground with live logs, dashboard at `http://127.0.0.1:3000` |
| `./manage.sh run-silent` | Web (GUI) | Background daemon, logs to `engine.log` |
| `./manage.sh cli` | CLI | Interactive terminal console with instance management, watch, and chat |
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
- Config API: `GET /api/config` (returns parsed `config.toml`)
- History API: `GET /api/history` (returns last 100 close prices)
- Analysis API: `POST /api/analyze` (accepts position + market data, returns structured assistant response)
- Database: SQLite, auto-created at `./telemetry.db` on startup
- Market data: Hyperliquid **Testnet** WebSocket (`wss://api.hyperliquid-testnet.xyz/ws`)
- Static assets served from `crates/frontend/dist`

- Cost Estimate API: `GET /api/cost-estimate?pair_key=Hyperliquid-BTC` (returns projected + actual token costs)
- Token tracking is per-pair; each LLM API call accumulates prompt/completion token usage attributed to its pair key
- Cost config is global (`[costs]` in `config.toml`) but projections are computed per-pair based on each pair's automation interval

## Configuration

`config.toml` at workspace root controls indicator lookback windows, candle duration, AI token pricing, and automation intervals. Parsed at startup by `main.rs`. If missing, the engine panics.

### Token Cost Tracking

The `[costs]` section configures per-1M-token pricing for your LLM provider:

```toml
[costs]
price_per_1m_input_tokens = 0.27   # DeepSeek default
price_per_1m_output_tokens = 1.10  # DeepSeek default
```

- Each LLM API call (phase-1 indicator agents, phase-2 orchestrator, chat, journal agent) tracks actual `prompt_tokens` and `completion_tokens` from the API `usage` response.
- Token usage is accumulated per-pair in a `TokenTracker` (thread-safe `Arc<Mutex<HashMap>>`).
- The frontend "💰 Token Costs" tab displays projected daily/weekly/monthly costs based on the pair's automation interval plus actual tracked usage.
- The settings panel includes a cost calculator where you can update the per-1M-token prices.

**Cost formula:**
- Tokens per analysis run: 35 phase-1 agents × ~1,536 tokens + 1 phase-2 orchestrator × ~3,072 tokens ≈ 57K tokens
- Runs per day = 86,400 ÷ interval_seconds
- Daily cost = (input_tokens/1M × input_price) + (output_tokens/1M × output_price)
- Weekly = daily × 7, Monthly = daily × 30

## Testing (248 tests across 3 boundaries)

| Suite | Command | Boundary | Tests | Runtime |
|-------|---------|----------|-------|---------|
| TEST-CORE | `./manage.sh test-core` | Pure math, indicators, serialization | 154 | <3s |
| TEST-ENGINE | `./manage.sh test-engine` | DB, paper trading, server, failover | 69 | <5s |
| TEST-UI | `./manage.sh test-ui` | Svelte 5 runes, components, snapshots | 24 | <10s |
| All | `./manage.sh test` | Core → Engine → UI sequentially | 248 | <18s |

### Specialized test selectors

| Command | Targets |
|---------|---------|
| `./manage.sh test-property` | Generative property tests (38 tests across 10 indicator modules) |
| `./manage.sh test-correlation` | Pearson correlation, sliding window, drawdown (15 tests) |
| `./manage.sh test-e2e` | End-to-end analytical loop + history endpoint (2 tests) |
| `./manage.sh test-engine-full` | All engine tests including load/stress (70 tests) |
| `./manage.sh test-load` | Multi-pair load test only (1 test, requires --ignored flag) |

### Developer guidelines

- **Modifying indicators, Fibonacci, models** → `./manage.sh test-core` (fast, <3s)
- **Modifying DB schemas, paper trading, server APIs** → `./manage.sh test-engine` (<5s)
- **Modifying Svelte 5 runes, components, charts** → `./manage.sh test-ui` (<10s)
- **Pre-commit / PR validation** → `./manage.sh test` (full sequential run)

### CI integration

See `.github/workflows/ci.yml` — 3-stage sequential pipeline:
1. Stage 1: Core verification (`test-core`)
2. Stage 2: Frontend verification (`test-ui`)
3. Stage 3: Engine integration (`test-engine`)

## Architecture notes

- The engine uses a multi-stage pipeline: WebSocket → channel → indicator analysis → broadcast → WebSocket to frontend
- `config.toml` is the single source of truth for indicator periods — both engine and frontend read it (frontend via `/api/config`)
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

## Implementation Guidelines

When writing code to realize the AI Assistant workflow, adhere to the following setup instructions:

### 1. Svelte 5 UI Adjustments (`crates/frontend/src/App.svelte`)
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

## Unified Deterministic-AI Hybrid Architecture

To maintain high performance and prevent logic fragmentation, this system strictly uses a **Single-Pipeline Hybrid Model**.

1. **NO FALLBACK SYSTEMS:** Deterministic indicators (RVOL, BBWP, ATR, S/R role-reversals, and the 100-point confluence score) are computed exclusively in Rust as Layer 1 & 2 "Ground Truth" data.
2. **AI-ALWAYS DECISIONS:** This computed data is passed directly into the system prompts and user payloads of the Layer 3 AI Domain Agents and Layer 4 Master Orchestrator.
3. **STRATEGIC REASONING:** The LLM's role is to perform cognitive synthesis, risk adjustments, and contextual memory lookups — NOT to perform redundant mathematical calculations. If the LLM is unconfigured, the system reports an error; it must never fallback to an isolated offline decision maker.
