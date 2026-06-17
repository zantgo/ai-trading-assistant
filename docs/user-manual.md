# AI Trading Assistant — User Manual

## Overview

The AI Trading Assistant is a desktop market analysis tool that streams live cryptocurrency data from Hyperliquid, computes 10+ technical indicators in real time, and provides on-demand AI-powered trade recommendations via DeepSeek. It does **not** execute trades — it is a decision-support copilot for the human operator.

---

## Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| Rust (stable) | ≥ 1.80 | Compiles the engine binary |
| Node.js or Bun | latest LTS | Builds the Svelte 5 frontend |
| DeepSeek API key | — | Powers the AI assistant (optional; works without it using heuristics) |

---

## Installation & Build

A unified script is provided to automate build processes so you do not need to manually change directories.

```bash
# 1. Clone the repository and configure credentials
git clone <repo-url>
cd ai-trading-assistant

# 2. Set up your API credentials
cp .env.example .env
# Edit .env to add your DEEPSEEK_API_KEY

# 3. Execute the single-step build command
chmod +x manage.sh
./manage.sh build
```

This installs npm packages, compiles Svelte 5 files into production bundles, and
verifies the Rust binary structures.

---

## Configuration

Edit `config.toml` at the workspace root to adjust candle duration and indicator periods:

```toml
[candles]
duration_seconds = 5        # Length of each candlestick

[indicators]
ema_fast = 10               # EMA periods
ema_medium = 50
ema_slow = 100
ema_long = 200
rsi_period = 14
macd_fast = 12
macd_slow = 26
macd_signal = 9
adx_period = 14
atr_period = 14
squeeze_period = 20
```

Changes take effect on restart. The frontend reads settings from the engine via `GET /api/config` at page load.

An immutable fallback configuration is stored at `config.default.toml` to serve as a baseline reference. Always modify `config.toml` to adjust active indicator looks or symbols. The default configuration file will never be overwritten by the engine, nor will standard `clean` commands touch your active `config.toml` changes.

---

## LLM Setup (DeepSeek)

The engine reads API credentials from a `.env` file at the workspace root. The key is recommended but not mandatory — the engine starts regardless and falls back to local heuristic analysis when no valid key is present.

### Step 1 — Create `.env`

Copy the template and fill in your key:

```bash
cp .env.example .env
# Edit .env with your editor
```

`.env` contents:

```env
DEEPSEEK_API_KEY=sk-your-deepseek-api-key-here

# Optional overrides:
# DEEPSEEK_MODEL=deepseek-chat
# DEEPSEEK_BASE_URL=https://api.deepseek.com/v1
```

### Step 2 — Get a key

Create one at https://platform.deepseek.com/api_keys

### Environment Variables

| Variable | Required | Default |
|---|---|---|
| `DEEPSEEK_API_KEY` | **Yes** | — |
| `DEEPSEEK_BASE_URL` | No | `https://api.deepseek.com/v1` |
| `DEEPSEEK_MODEL` | No | `deepseek-chat` |

All variables go in the `.env` file, one per line. Do **not** export them in your shell — use the `.env` file instead.

### Startup validation

At startup the engine attempts a test call to the DeepSeek API to verify the key. If a valid key is found, the `api_key_configured` flag is set and the full multi-agent LLM pipeline becomes available for automated and on-demand analysis.

If the key is missing, empty, invalid, or rejected by the API, the engine prints a warning and continues to boot normally, falling back to local heuristic evaluation for all analysis requests:

```
⚠️  No .env file found: ...
   Create a .env file at the project root with: DEEPSEEK_API_KEY=sk-...
   The dashboard will run, but AI features require a valid key.
⚠️  No API key found. AI analysis will fall back to local heuristics. Configure via the UI config panel.
⚠️  API Key Validation Failed: ... You can configure it manually in the UI.
```

The engine does **not** exit or refuse to start without a key — it operates in heuristic-only mode until a valid key is provided via the dashboard Settings panel or by restarting with a corrected `.env` file.

---

## Running the Engine

The engine supports two execution modes: a full-featured web dashboard (GUI)
and an interactive terminal console (CLI).

### Web Dashboard (GUI Mode)

The web dashboard is the primary interface. It serves a Svelte 5 frontend with
real-time charts, AI analysis, paper trading, and risk management tools.

```bash
# Foreground with live log output
./manage.sh run

# Background with logs written to engine.log
./manage.sh run-silent
```

Once running, open **http://127.0.0.1:3000** in your browser.

| Manage command | Underlying flag | Description |
|---|---|---|
| `./manage.sh run` | `cargo run -- --web` | Foreground, live logs |
| `./manage.sh run-silent` | `cargo run -- --web` (nohup) | Background daemon |
| `./manage.sh stop` | `kill` background PID | Graceful termination |
| `./manage.sh status` | Process check | Uptime + log size |

### CLI Console (Interactive Mode)

The CLI console provides a terminal-based interface for managing instances,
viewing telemetry, and chatting with the AI Director — no browser required.

```bash
./manage.sh cli
```

This launches the engine with `cargo run -- --cli`. Inside the console,
type `help` to see available commands and `quit` to exit.

**CLI Command Reference:**

| Command | Description |
|---|---|
| `add <BASE> <QUOTE>` | Create a new trading instance (e.g., `add BTC USDT`) |
| `pause <ID>` | Pause an instance, keeping open positions |
| `stop <ID>` | Stop an instance, closing all positions |
| `delete <ID>` | Permanently delete an instance |
| `list` / `ls` | List all active instances |
| `show <ID>` | Detailed instance view (telemetry + indicators) |
| `show <ID> charts` | Indicator summary for the instance |
| `show <ID> dashboard` | Performance metrics for the instance |
| `show <ID> trades` | Completed trade history for the instance |
| `watch <ID> [tf_secs]` | Real-time price stream (Ctrl+C to stop) |
| `dashboard` / `dash` | General system dashboard overview |
| `status` / `stat` | System heartbeat (pairs, tokens, DB health) |
| `config` | View global configuration |
| `safety <ID>` | Show safety/consecutive-loss status |
| `safety reset <ID>` | Reset the consecutive loss counter |
| `manual open <ID> <LONG\|SHORT>` | Open a manual paper position |
| `manual close <ID>` | Close a manual paper position |
| `chat <ID> <message>` | Send a message to the AI Director |
| `help` / `?` | Show the CLI command list |
| `quit` / `exit` / `q` | Graceful shutdown (closes all positions) |

### Full manage.sh Command Reference

The `manage.sh` script provides all operational commands for the workspace:

| Command | Description |
|---|---|
| `./manage.sh build` | Compile frontend + verify Rust workspace |
| `./manage.sh run` | Start engine in web mode (foreground) |
| `./manage.sh run-silent` | Start engine in web mode (background) |
| `./manage.sh cli` | Start engine in CLI interactive mode |
| `./manage.sh stop` | Stop background engine instance |
| `./manage.sh status` | Check if engine is running |
| `./manage.sh test` | Run all 5 test suites sequentially |
| `./manage.sh test-core` | Indicators + serialization (154 tests) |
| `./manage.sh test-engine` | DB + paper trading + server (69 tests) |
| `./manage.sh test-engine-full` | Engine suite including load test (70 tests) |
| `./manage.sh test-ui` | Svelte 5 components (24 tests) |
| `./manage.sh test-property` | Generative property tests (38 tests) |
| `./manage.sh test-correlation` | Pearson + drawdown (15 tests) |
| `./manage.sh test-e2e` | E2E analytical loop (2 tests) |
| `./manage.sh test-load` | Multi-pair load test (1 test) |
| `./manage.sh clean` | Delete build targets and temp files |
| `./manage.sh destroy` | Full reset (stop + clean + wipe DB) |
| `./manage.sh help` | Show this help reference |

Expected startup output:

```
⚙️ AI Trading Assistant: Loading Master Configuration...
✅ Configuration Loaded: System configured dynamically.
🔑 Validating DeepSeek API key... ✅ Key validated successfully.
🗄️  Initializing local SQLite telemetry database...
✅ Database Setup: Connected to local telemetry.db file and verified schema.
🌐 Web Server Setup: Visualizer Dashboard live at http://127.0.0.1:3000
```

### Total System Reset

To completely reset the application — clearing historical records, wiping the
telemetry database, and cleaning all workspace folders:

```bash
./manage.sh destroy
```

This terminates the engine, purges `telemetry.db` and its WAL files, and
restores `config.toml` from `config.default.toml`.

---

## Using the Dashboard

### Layout

The dashboard has two columns:

| Column | Content |
|---|---|
| **Left (main)** | Price chart, Volume, ADX, ATR, RSI, MACD, and Squeeze Momentum panels |
| **Right (sidebar)** | Settings overview, position selector, and AI Assistant controls |

Each chart panel can be toggled on/off via the header checkboxes and supports vertical resize by dragging the bottom edge.

### Real-Time Data

Once connected, price and indicator panels update continuously via WebSocket. Charts show both completed candle data and real-time "shadow" values for the current candle.

The connection status dot in the header is blue when connected, grey when reconnecting.

---

## AI Assistant Workflow

The AI Assistant is in the right sidebar under **"AI ASSISTANT"**.

### Step 1 — Set your position

Select your current position using the radio buttons at the top of the AI ASSISTANT panel:

- **None** — You are not holding a position
- **Long** — You are long ETH-USD
- **Short** — You are short ETH-USD

### Step 2 — Request analysis

Click the **"Request AI Assistant Analysis"** button.

The button changes to **"Analyzing Market..."** and a progress indicator appears showing the three stages:

```
Trend Check → Indicators → Recommendation
```

### Step 3 — Read the results

The analysis returns in three sequential blocks:

#### 1. Price Action Trend
A colored badge showing the classification:
- **trending upwards** (green)
- **trending downwards** (red)
- **sideways** (amber)

Followed by a brief description of price action observed in the last 100 candles.

#### 2. Indicator Alignment
A colored badge showing whether technical indicators agree with the trend:
- **supportive** (green) — RSI, MACD, and Squeeze confirm the trend
- **conflicting** (red) — Indicators diverge from price action
- **neutral** (grey) — No clear signal

#### 3. Position Recommendation
A highlighted call-to-action and rationale:

| Your Position | Possible Actions |
|---|---|
| **Long** | Hold, Close |
| **Short** | Hold, Close |
| **None** | Wait, Open Long, Open Short |

The rationale explains *why* the action is recommended based on the confluence of trend, indicators, and your current position context.

---

## Commission & Fee Calculation

The Fee Projection tool (`💸 Fee Projection` tab) helps you determine whether a dual-entry trade is viable after accounting for exchange fees, funding costs, and leverage. It prevents you from executing trades where the net gains are negative.

### Fee Table

The **Fee Reference Table** shows, for any combination of leverage and capital, the minimum percentage profit your trade must achieve just to cover the round-trip (open + close) exchange fees:

| Exchange Fee % | Leverage | Capital ($) | Min % Profit to Cover Fees | Fees ($) |
|---|---|---|---|---|
| 0.06 (Taker) | 10x | 50 | 1.20% | 0.60 |
| 0.06 (Taker) | 20x | 50 | 2.40% | 1.20 |
| 0.06 (Taker) | 25x | 50 | 3.00% | 1.50 |
| 0.06 (Taker) | 40x | 50 | 4.80% | 2.40 |
| 0.06 (Taker) | 50x | 10 | 6.00% | 0.60 |

**Formula:** `Fees = Fee% × Capital × Leverage × 2` (×2 for round-trip open + close).  
`Min Profit % = Fees / Capital × 100`.

### Dual-Entry Projection

The tool calculates projections for a **two-entry** trading strategy:

| Parameter | Description |
|---|---|
| **Entry 1 / Entry 2** | Two price levels where you split your entry |
| **Stop Loss 1 / Stop Loss 2** | Per-entry stop-loss levels |
| **Take Profit 1 / Take Profit 2** | Per-entry take-profit targets |
| **Capital Split** | Percentage of capital allocated to Entry 1 (Entry 2 gets the remainder) |
| **Order Type** | Maker (limit order, lower fee) or Taker (market order, higher fee) |

### Projection Output

The tool returns:

- **Combined Position:** Weighted average entry, effective stop-loss/take-profit, total notional, total margin, total risk
- **Per-Entry Metrics:** Capital allocated, position size, notional value, margin required, risk amount, potential profit, fees, and net profit for each entry
- **Fee Breakdown:** Maker vs taker fee rates, per-entry commission, total commission, funding costs, and the minimum profit % needed to break even
- **Scenario Projections:** Maximum gain (gross and net), maximum loss (gross and net), required price move %
- **Viability Gate:** A yes/no decision — if the maximum net gain after fees is negative or zero, the trade is flagged as **NOT VIABLE** and should not be executed

### Configuration

Exchange fee rates are configured in `config.toml`:

```toml
[fees]
maker_fee_pct = 0.02    # 0.02% for limit orders
taker_fee_pct = 0.06    # 0.06% for market orders
```

Per-profile commission overrides are stored in each Risk Profile (via the `🛡️ Risk Management` tab).

### API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/risk/fee-table?order_type=taker` | Returns the fee reference table |
| `POST` | `/api/risk/commission-projection` | Full dual-entry projection |

`POST /api/risk/commission-projection` request body:

```json
{
  "profile_id": 1,
  "direction": "LONG",
  "entry_1": 3120.0,
  "entry_2": 3100.0,
  "stop_loss_1": 3080.0,
  "stop_loss_2": 3070.0,
  "take_profit_1": 3180.0,
  "take_profit_2": 3220.0,
  "capital_entry_1_pct": 50,
  "order_type": "taker"
}
```

---

## Understanding the Analysis Logic

### When using DeepSeek (LLM mode)

The engine sends a structured prompt containing:
- System instructions defining the trading assistant role and JSON schema
- The last 100 closing prices
- Current indicator values (RSI, Squeeze, MACD Histogram, ADX, EMA Fast, EMA Slow)
- Your selected position

DeepSeek performs the multi-stage reasoning and returns valid JSON. The analysis is conversational, adaptive, and considers nuance in the data.

### Runtime fallback

If the DeepSeek API is temporarily unreachable during an analysis request, the engine falls back to a local heuristic that uses fixed rules (quartile trend comparison, indicator scoring, truth-table recommendation). The response format is identical.

---

## Database & Telemetry

The engine automatically creates `telemetry.db` in the workspace root on first run. It contains two tables:

| Table | Contents |
|---|---|
| `market_snapshots` | Every completed candle with all OHLCV and indicator values |
| `assistant_records` | Every AI analysis request and its structured result, with timestamp |

You can inspect the database with any SQLite client:

```bash
sqlite3 telemetry.db "SELECT * FROM assistant_records ORDER BY id DESC LIMIT 5;"
```

---

## API Endpoints

The engine exposes these HTTP endpoints on `127.0.0.1:3000`:

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/config` | Current `config.toml` as JSON |
| `GET` | `/api/history` | Last 100 closing prices |
| `POST` | `/api/analyze` | Submit position + data for AI analysis |
| `GET` | `/ws` | WebSocket upgrade for live `MarketSnapshot` stream |

The `POST /api/analyze` request body:

```json
{
  "position": "Long",
  "historical_prices": [3124.50, 3125.10, "..."],
  "indicators": {
    "rsi": 42.50,
    "squeeze_on": true,
    "macd_histogram": -0.45,
    "adx": 25.3,
    "ema_fast": 3120.1,
    "ema_slow": 3100.5
  }
}
```

---

## Testing

Test suites are organized by architectural boundary using the command helper:

```bash
# Run all test suites sequentially (core → engine → ui)
./manage.sh test

# Fast math indicators & serialization (154 tests, <3s)
./manage.sh test-core

# Database, paper trading, server integration (69 tests, <5s)
./manage.sh test-engine

# Full engine suite including load/stress test (70 tests)
./manage.sh test-engine-full

# Svelte 5 component & state tests (24 tests)
./manage.sh test-ui

# Generative property tests across all 12 indicators (38 tests)
./manage.sh test-property

# Pearson correlation + drawdown validation (15 tests)
./manage.sh test-correlation

# End-to-end analytical loop (2 tests)
./manage.sh test-e2e

# Multi-pair load/stress test (1 test)
./manage.sh test-load
```

---

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| Engine panics at startup | Missing `config.toml` | Ensure `config.toml` exists in the workspace root |
| "Failed to load .env file" | No `.env` file found; engine continues with heuristics | Copy `.env.example` to `.env` and add your key, or use the Settings panel |
| "DEEPSEEK_API_KEY not found" | `.env` exists but key is empty or commented; engine continues with heuristics | Add `DEEPSEEK_API_KEY=sk-...` to `.env` or configure via Settings panel |
| "API Key Validation Failed (HTTP 401)" | Invalid or expired API key; engine continues with heuristics | Check your key at https://platform.deepseek.com/api_keys |
| Frontend shows blank page | `dist/` not built | Run `npm run build` inside `crates/frontend` |
| Charts stuck at initial values | No WebSocket connection | Verify engine is running and port 3000 is not blocked |
| "Failed to parse LLM JSON output" | Model returned non-JSON | Falls back to heuristics automatically; check logs for raw content |
| Port 3000 already in use | Another process bound to 3000 | Kill the existing process or change the port in `main.rs` |

---

## Disclaimer

This system is an information tool for **research and educational purposes only**. It does not execute trades automatically. All financial execution remains the sole responsibility of the user. Past analysis does not guarantee future results.
