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

The engine reads API credentials from a `.env` file at the workspace root. **This file is required** — the engine will refuse to start without a valid key.

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

At startup the engine makes a test call to the DeepSeek API to verify the key. If the key is missing, empty, invalid, or rejected by the API, the engine prints an error and exits immediately:

```
❌ Failed to load .env file: ...
❌ LLM Setup Error: DEEPSEEK_API_KEY not found in .env file...
❌ API Key Validation Failed: DeepSeek API rejected the key (HTTP 401)...
```

The engine will **not** start without a valid key. There is no offline / heuristic-only mode.

---

## Running the Engine

You can run the engine using two different log profiles depending on whether you
are actively debugging or letting it run in the background:

### Profile A: Live Diagnostic Logging (Foreground)

If you are developing or want to see live market data updates and API analysis
traces printed directly to your console:

```bash
./manage.sh run
```

### Profile B: Silent execution (Background)

If you want to keep the assistant active without keeping your terminal window
open:

```bash
# Starts the process silently and saves logs to engine.log
./manage.sh run-silent

# Check process uptime and file sizes
./manage.sh status

# Gracefully terminate execution
./manage.sh stop
```

Open **http://127.0.0.1:3000** in your browser once the engine is running.

Expected startup output:

```
⚙️ AI Trading Assistant: Loading Master Configuration...
✅ Configuration Loaded: System configured dynamically.
🔑 Validating DeepSeek API key... ✅ Key validated successfully.
🗄️  Initializing local SQLite telemetry database...
✅ Database Setup: Connected to local telemetry.db file and verified schema.
🌐 Web Server Setup: Visualizer Dashboard live at http://127.0.0.1:3000
```

### Total System Reset (Purging Database and Builds)

If you need to completely reset the application, clear your historical assistant records, wipe the telemetry database, and clean all workspace folders:

```bash
# Safely terminates engine, purges db, and overwrites config.toml from config.default.toml
./manage.sh destroy
```

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

Both Rust and Frontend test suites can be executed simultaneously or
individually using the command helper:

```bash
# Run both Svelte 5 and Rust testing engines
./manage.sh test

# Run Rust unit and database integration tests only
./manage.sh test-rust

# Run Svelte 5 state machine unit tests only
./manage.sh test-ui
```

---

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| Engine panics at startup | Missing `config.toml` | Ensure `config.toml` exists in the workspace root |
| "Failed to load .env file" | No `.env` file | Copy `.env.example` to `.env` and fill in your key |
| "DEEPSEEK_API_KEY not found" | `.env` exists but key is commented or missing | Add `DEEPSEEK_API_KEY=sk-...` to `.env` |
| "API Key Validation Failed (HTTP 401)" | Invalid or expired API key | Check your key at https://platform.deepseek.com/api_keys |
| Frontend shows blank page | `dist/` not built | Run `npm run build` inside `crates/engine/frontend` |
| Charts stuck at initial values | No WebSocket connection | Verify engine is running and port 3000 is not blocked |
| "Failed to parse LLM JSON output" | Model returned non-JSON | Falls back to heuristics automatically; check logs for raw content |
| Port 3000 already in use | Another process bound to 3000 | Kill the existing process or change the port in `main.rs` |

---

## Disclaimer

This system is an information tool for **research and educational purposes only**. It does not execute trades automatically. All financial execution remains the sole responsibility of the user. Past analysis does not guarantee future results.
