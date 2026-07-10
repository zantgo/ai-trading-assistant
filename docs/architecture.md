# System Architecture

The Market Monitor implements a real-time market telemetry pipeline combining multi-timeframe analysis, deterministic indicator computation, and interactive charting.

## Core Crate Topology

- `crates/shared`: Domain representations (`MarketSnapshot`), normalized exchange events, and functional indicator calculations (EMA, RSI, MACD, ADX, ATR, Bollinger, Squeeze, BBWP, Fibonacci, Chart Patterns, Divergence, +25 more).
- `crates/engine`: High-performance daemon maintaining live Hyperliquid/Bitget WebSocket connections, multi-timeframe candle aggregation, SQLite telemetry persistence, and the Axum HTTP/WS dashboard server.
- `crates/frontend`: Svelte 5 application providing interactive charting, real-time WebSocket snapshots, decision scoring, commission calculator, and market analysis tools.

## 5-Layer Architecture

### Layer 1 — Market Data
- **Exchange Data**: Hyperliquid & Bitget WebSocket ingestion (order book + trade streams)
- **Data Cache**: In-memory sliding window buffers for all timeframes
- **Candle Aggregator**: Generates candles from raw trade streams; aggregates macro candles (4h, 1d) from 1m closes
- **Normalization**: `SymbolMapper` translates exchange-specific symbols into normalized identifiers

### Layer 2 — Analysis
- **Indicator Engine**: EMA(10/50/100/200), RSI(14), MACD(12/26/9), ADX(14) with DI+/DI- crossovers, ATR(14) with volatility regime, Bollinger Bands, Squeeze Momentum, BBWP(252/20), VWAP, Fibonacci retracement/extensions, Stochastic, ChandeMO, Supertrend, Keltner Channels, Donchian Channels, OBV, CMF, MFI, Historical Volatility, Aroon, Choppiness Index, Linear Regression Slope, Z-Score, RVOL
- **Market Structure Engine**: Swing highs/lows, S/R role tracking with flip detection, chart pattern classification
- **Volume Analysis**: RVOL (relative volume), average volume tracking, volume profile levels
- **Regime Detection Engine**: Classifies market into Trending, Compression, Expansion, or Range using ADX, BBWP, Squeeze state, and ATR regime

### Layer 3 — Decision Scoring
- **8-Factor Confluence Score**: RSI (10pt), RSI Divergence (20pt), MACD (10pt), MACD Divergence (10pt), Support/Resistance (10pt), Trend (20pt), 200EMA (10pt), Patterns (10pt)
- **Registry-Based Scoring**: Weighted confluence scoring across all 34 registered indicators with per-regime multipliers
- **Opposite-Signal Exit Evaluation**: Evaluates exit signals for existing positions
- **Momentum Bias**: Directional bias computed from indicator alignment

### Layer 4 — Risk & Commission Analysis
- **Risk Calculator**: `risk_calculator.rs` computes margin, liquidation price, position sizing, risk-reward ratios
- **Commission & Fee Projection**: `commission.rs` calculates maker/taker fees, dual-entry projections, viability gates
- **Profile-Based Configuration**: Decision profiles and risk profiles with per-indicator weight customization

### Layer 5 — Analytics & Dashboard
- **Performance Metrics**: Dashboard statistics with equity curves, daily activity, PnL analysis
- **Trade Journal**: Trade logging and journaling for manual trade tracking
- **Portfolio Monitoring**: Portfolio equity snapshots, risk state tracking

## Data Flow Diagram

```
+------------------+       Live WebSocket        +---------------+
|  Hyperliquid /   |  ======================>    |  Rust Engine  |
|     Bitget       |                             +---------------+
+------------------+                                     || Multi-Timeframe Pipeline
                                                         || Indicator Computation (34+)
                                                         || Regime Detection
                                                         || S/R Role Tracking
+------------------+      WebSocket (MarketSnapshot)  +---------------+
|   User Browser   |  <=============================> |  Axum Server  |
| (Svelte 5 Term)  |                             | (port 3000) |
+------------------+                             +---------------+
                                                         |
                                                 +---------------+
                                                 |   SQLite DB   |
                                                 | (telemetry.db)|
                                                 +---------------+
```

## Commission Module (`crates/engine/src/commission.rs`)

- **Fee Table Generator**: Computes minimum profit % needed to cover round-trip fees
- **Dual-Entry Projection**: Position metrics for two-entry strategies with Entry 1/2, SL 1/2, TP 1/2
- **Fee Breakdown**: Separates maker vs taker fees, per-entry commission, and funding costs
- **Viability Gate**: Determines whether `max_gain_net_after_fees > 0`

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | Serves Svelte 5 dashboard (static files from `crates/frontend/dist`) |
| `GET` | `/ws` | WebSocket streaming `MarketSnapshot` JSON at candle close |
| `GET` | `/api/config` | Returns parsed `config.toml` settings |
| `GET` | `/api/history` | Returns last 100 close prices |
| `GET` | `/api/session/status` | Returns active session state |
| `POST` | `/api/session/init` | Initialize monitoring session (exchange + currency) |
| `POST` | `/api/session/quit` | Gracefully shutdown all instances |
| `GET` | `/api/instances` | List all active instances |
| `POST` | `/api/instances` | Create a new instance |
| `GET` | `/api/monitor` | Live monitoring data |
| `GET` | `/api/dashboard/stats` | Aggregated dashboard statistics |

## Runtime Details

- Server: `http://127.0.0.1:3000` (localhost only)
- Market data: Hyperliquid (`wss://api.hyperliquid.xyz/ws`) and Bitget (`wss://ws.bitget.com/v2/ws/public`)
- SQLite auto-created at `./telemetry.db` on startup
- Configuration: `config.toml` at workspace root (panics if missing)
