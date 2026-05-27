# DeX AI Trading Assistant — User Manual

## Overview

The DeX AI Trading Assistant is a desktop market analysis tool that streams live cryptocurrency data from Hyperliquid, computes 10+ technical indicators in real time, and provides on-demand AI-powered trade recommendations via DeepSeek. It does **not** execute trades — it is a decision-support copilot for the human operator.

---

## Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| Rust (stable) | ≥ 1.80 | Compiles the engine binary |
| Node.js or Bun | latest LTS | Builds the Svelte 5 frontend |
| DeepSeek API key | — | Powers the AI assistant (optional; works without it using heuristics) |

---

## Installation & Build

```bash
# 1. Clone the repository
git clone <repo-url>
cd dex-trading-agent-engine

# 2. Build the frontend (produces crates/engine/frontend/dist/)
cd crates/engine/frontend
npm install          # or: bun install
npm run build        # or: bun run build

# 3. Build & run the engine from the workspace root
cd ../..             # back to workspace root
cargo run
```

The engine binary reads `config.toml` from the current working directory. Always run `cargo run` from the workspace root.

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

---

## LLM Setup (DeepSeek)

Set one or more environment variables before starting the engine:

| Variable | Required | Default |
|---|---|---|
| `DEEPSEEK_API_KEY` | Yes * | *(none)* |
| `DEEPSEEK_BASE_URL` | No | `https://api.deepseek.com/v1` |
| `DEEPSEEK_MODEL` | No | `deepseek-chat` |

\* If `DEEPSEEK_API_KEY` is not set or is empty, the assistant falls back to a local heuristic engine that classifies trends using quartile comparisons. The heuristic is deterministic and has no network dependency.

**Examples:**

```bash
# Basic usage
export DEEPSEEK_API_KEY="sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

# Custom model
export DEEPSEEK_API_KEY="sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
export DEEPSEEK_MODEL="deepseek-reasoner"

# Self-hosted / proxy
export DEEPSEEK_API_KEY="sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
export DEEPSEEK_BASE_URL="https://your-proxy.example.com/v1"
```

The engine prints a startup message confirming whether the LLM client initialized:

```
🤖 LLM Integration: DeepSeek API client initialized.
```
or
```
ℹ️  LLM Integration: No DEEPSEEK_API_KEY found. Using heuristic analysis fallback.
```

---

## Running the Engine

```bash
# From workspace root, after building the frontend
cargo run
```

Expected output:

```
⚙️ DeX AI Trading Assistant: Loading Master Configuration...
✅ Configuration Loaded: System configured dynamically.
🤖 LLM Integration: DeepSeek API client initialized.
🗄️  Initializing local SQLite telemetry database...
✅ Database Setup: Connected to local telemetry.db file and verified schema.
🌐 Web Server Setup: Visualizer Dashboard live at http://127.0.0.1:3000
```

Open **http://127.0.0.1:3000** in your browser.

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

### When using heuristics (no API key)

The fallback engine uses fixed rules:
1. **Trend**: Compares average price in the first 25 candles vs. the last 25 candles. A change > ±0.5% is considered trending; otherwise sideways.
2. **Indicators**: Scores each indicator as supportive or conflicting based on direction relative to the trend.
3. **Recommendation**: Simple truth table mapping (Position × Trend × Indicator Alignment → Action).

The heuristic is fast, deterministic, and requires no network.

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

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| Engine panics at startup | Missing `config.toml` | Ensure `config.toml` exists in the workspace root |
| Frontend shows blank page | `dist/` not built | Run `npm run build` inside `crates/engine/frontend` |
| Charts stuck at initial values | No WebSocket connection | Verify engine is running and port 3000 is not blocked |
| AI Assistant returns heuristic results | `DEEPSEEK_API_KEY` not set | Export the environment variable and restart |
| "LLM API returned 401" | Invalid API key | Check the key at https://platform.deepseek.com/api_keys |
| "Failed to parse LLM JSON output" | Model returned non-JSON | Falls back to heuristics automatically; check logs for raw content |
| Port 3000 already in use | Another process bound to 3000 | Kill the existing process or change the port in `main.rs` |

---

## Disclaimer

This system is an information tool for **research and educational purposes only**. It does not execute trades automatically. All financial execution remains the sole responsibility of the user. Past analysis does not guarantee future results.
