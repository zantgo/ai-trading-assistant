# Market Monitor — User Manual

## Overview

The Market Monitor is a desktop market analysis tool that streams live cryptocurrency data from Hyperliquid and Bitget, computes 34+ technical indicators in real time, and provides an interactive dashboard for market observation. It is a monitoring and analysis tool — it does **not** execute trades.

---

## Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| Rust (stable) | >= 1.80 | Compiles the engine binary |
| Node.js or Bun | latest LTS | Builds the Svelte 5 frontend |

---

## Installation & Build

A unified script is provided to automate build processes so you do not need to manually change directories.

```bash
# 1. Clone the repository
git clone <repo-url>
cd market-monitor

# 2. Execute the single-step build command
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
duration_seconds = 60        # Length of each candlestick

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

## Running the Engine

The engine runs as a web dashboard (GUI mode) that serves a Svelte 5 frontend with
real-time charts, decision scoring, commission calculator, and market analysis tools.

### Web Dashboard (GUI Mode)

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

### Full manage.sh Command Reference

The `manage.sh` script provides all operational commands for the workspace:

| Command | Description |
|---|---|
| `./manage.sh build` | Compile frontend + verify Rust workspace |
| `./manage.sh run` | Start engine in web mode (foreground) |
| `./manage.sh run-silent` | Start engine in web mode (background) |
| `./manage.sh stop` | Stop background engine instance |
| `./manage.sh status` | Check if engine is running |
| `./manage.sh test` | Run all test suites sequentially |
| `./manage.sh test-core` | Indicators + serialization |
| `./manage.sh test-engine` | DB + server |
| `./manage.sh test-engine-full` | Engine suite including load test |
| `./manage.sh test-ui` | Svelte 5 components |
| `./manage.sh test-property` | Generative property tests |
| `./manage.sh clean` | Delete build targets and temp files |
| `./manage.sh destroy` | Full reset (stop + clean + wipe DB) |
| `./manage.sh help` | Show this help reference |

Expected startup output:

```
⚙️ Market Monitor: Loading Master Configuration...
✅ Configuration Loaded: System configured dynamically.
🗄️  Initializing local SQLite telemetry database...
✅ Database Setup: Connected to local telemetry.db file and verified schema.
🌐 Web Server Setup: Dashboard live at http://127.0.0.1:3000
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

The dashboard provides multiple views accessible via the top navigation tabs:

| Tab | Content |
|---|---|
| **Dashboard** | General overview with UTC time and portfolio performance curve |
| **Instances** | Manage monitored trading pairs (add/remove/status) |
| **Settings** | General configuration |

Within each instance, sub-tabs provide:

| Sub-Tab | Content |
|---|---|
| **Live Terminal** | Price chart, Volume, ADX, ATR, RSI, MACD, Squeeze Momentum, and 20+ additional indicator panels |
| **Terminal Monitor** | Multi-timeframe analysis with market context and regime classification |
| **Decision Trading** | Market bias analysis with equal-weighted indicator confluence, momentum bias display |
| **Fee Projection** | Commission calculator with fee tables, dual-entry projections, and viability checks |
| **Performance Metrics** | Dashboard statistics, equity curves, daily activity |
| **Trade Audit** | Trade analytics and audit tools |
| **Trade Ledger** | Trade history and journaling |
| **Decision HUD** | Live decision observability and market snapshot data |
| **Timeframe Settings** | Configure candle duration and indicator periods per timeframe |
| **Workspace Settings** | Per-instance configuration and automation settings |

### Real-Time Data

Once connected, price and indicator panels update continuously via WebSocket. Charts show both completed candle data and real-time "shadow" values for the current candle.

The connection status dot in the header is blue when connected, grey when reconnecting.

---

## Decision Trading — 8-Factor Scoring

The Decision Trading panel provides a confluence-based market scoring system:

The 8 factors scored are:
1. **RSI** (10 points) — Oversold/overbought evaluation
2. **RSI Divergence** (20 points) — Confirmed divergence detection
3. **MACD** (10 points) — Crossover and momentum direction
4. **MACD Divergence** (10 points) — Histogram divergence
5. **Support/Resistance** (10 points) — Price proximity to key levels
6. **Trend** (20 points) — EMA alignment and ADX strength
7. **200EMA** (10 points) — Long-term trend relative to price
8. **Patterns** (10 points) — Chart pattern signals

Each indicator can be weighted and enabled/disabled per profile. Regime multipliers (Compression, Trending, Range, Expansion) further adjust indicator weights based on current market conditions.

---

## Commission & Fee Calculation

The Fee Projection tool (`💸 Fee Projection` tab) helps you determine whether a dual-entry trade is viable after accounting for exchange fees, funding costs, and leverage.

### Fee Table

The **Fee Reference Table** shows, for any combination of leverage and capital, the minimum percentage profit your trade must achieve just to cover the round-trip (open + close) exchange fees.

**Formula:** `Fees = Fee% x Capital x Leverage x 2` (x2 for round-trip open + close).
`Min Profit % = Fees / Capital x 100`.

### Dual-Entry Projection

The tool calculates projections for a **two-entry** strategy:

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
- **Viability Gate:** A yes/no decision — if the maximum net gain after fees is negative or zero, the trade is flagged as **NOT VIABLE**

### Configuration

Exchange fee rates are configured in `config.toml`:

```toml
[fees]
maker_fee_pct = 0.02    # 0.02% for limit orders
taker_fee_pct = 0.06    # 0.06% for market orders
```

### API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/risk/fee-table?order_type=taker` | Returns the fee reference table |
| `POST` | `/api/risk/commission-projection` | Full dual-entry projection |

---

## Database & Telemetry

The engine automatically creates `telemetry.db` in the workspace root on first run. It persists market snapshots with all OHLCV and indicator values for historical reference.

You can inspect the database with any SQLite client:

```bash
sqlite3 telemetry.db ".tables"
```

---

## API Endpoints

The engine exposes these HTTP endpoints on `127.0.0.1:3000`:

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/config` | Current `config.toml` as JSON |
| `GET` | `/api/history` | Last 100 closing prices |
| `GET` | `/ws` | WebSocket upgrade for live `MarketSnapshot` stream |
| `GET` | `/api/session/status` | Active session state |
| `POST` | `/api/session/init` | Initialize session with exchange + currency |
| `POST` | `/api/session/quit` | Graceful shutdown |

---

## Testing

Test suites are organized by architectural boundary using the command helper:

```bash
# Run all test suites sequentially
./manage.sh test

# Fast math indicators & serialization
./manage.sh test-core

# Database, server integration
./manage.sh test-engine

# Full engine suite including load/stress test
./manage.sh test-engine-full

# Svelte 5 component & state tests
./manage.sh test-ui

# Generative property tests across all 12 indicators
./manage.sh test-property
```

---

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| Engine panics at startup | Missing `config.toml` | Ensure `config.toml` exists in the workspace root |
| Frontend shows blank page | `dist/` not built | Run `npm run build` inside `crates/frontend` |
| Charts stuck at initial values | No WebSocket connection | Verify engine is running and port 3000 is not blocked |
| Port 3000 already in use | Another process bound to 3000 | Kill the existing process or change the port in `main.rs` |

---

## Disclaimer

This system is an information tool for **research and educational purposes only**. It does not execute trades automatically. All financial decisions remain the sole responsibility of the user.
