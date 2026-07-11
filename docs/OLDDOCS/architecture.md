# System Architecture

The Market Monitor implements a real-time market telemetry pipeline combining
multi-timeframe analysis, deterministic indicator computation, and interactive
charting. It is an observational tool — it does not execute trades.

For the formal ontology defining entities, metrics, signals, states, decisions,
and the 12 classification axes, see [ontology.md](ontology.md).
For the complete indicator reference, see [metrics-matrix.md](metrics-matrix.md).
For the Metrics → State → Decision matrix flow, see [monitor-matrices.md](monitor-matrices.md).

## Core Crate Topology

- `crates/shared`: Domain representations (`MarketSnapshot`), normalized exchange events, and 43+ functional indicator calculators (EMA, RSI, MACD, ADX, ATR, Bollinger, Squeeze, BBWP, +35 more) plus normalization engine, registry, and MarketContext synthesis.
- `crates/engine`: High-performance daemon maintaining live Hyperliquid/Bitget WebSocket connections, multi-timeframe candle aggregation, indicator pipeline, SQLite telemetry persistence, and the Axum HTTP/WS dashboard server.
- `crates/frontend`: Svelte 5 application providing interactive charting across 4 timeframes, real-time WebSocket snapshots, telemetry matrix, and MarketContext dashboard.

## Data Flow (Ontological Levels)

The architecture maps to the 5 ontological levels defined in [ontology.md](ontology.md):

```
Entity    → Ingestion Layer    (WebSocket → CandleGenerator)
Metric    → Calculator + Normalization Layers
Signal    → Normalization + Signal Derivation
State     → MarketContext Synthesis
Decision  → Confluence + Decision Matrix
```

## 5-Layer Architecture

### Layer 1 — Market Data
- **Exchange Data**: Hyperliquid & Bitget WebSocket ingestion (order book + trade streams)
- **Data Cache**: In-memory sliding window buffers for all timeframes
- **Candle Aggregator**: Generates candles from raw trade streams; aggregates macro candles (4h, 1d) from 1m closes
- **Normalization**: `SymbolMapper` translates exchange-specific symbols into normalized identifiers

### Layer 2 — Analysis
- **Indicator Engine**: 43+ calculator modules across 8 functional groups
- **Normalization Engine**: Maps raw values → unified [-1,+1] scale with state labels and signals
- **Derived Metrics**: MarketContext synthesis (trend/momentum/vol/vol/liquidity/regime per instance)
- **Market Structure Engine**: Swing highs/lows, S/R role tracking, chart pattern classification
- **Regime Detection**: Classifies market into Trending, Compression, Expansion, or Range

### Layer 3 — Market Intelligence Pipeline
- **Alignment Matrix**: Cross-timeframe MTF agreement (micro×fast×slow×macro) per symbol
- **Risk Matrix**: Market risk assessment (volatility, liquidity, trend, structural, signal reliability) + stop/target guidance
- **Analysis Matrix**: Market bias, confidence, trade readiness, preferred strategy, warnings, opportunity scores
- **State Matrix**: System-wide aggregation of all symbols and instances

### Layer 4 — Risk Analysis
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
