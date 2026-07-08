# System Architecture

The AI Trading Assistant implements a 7-layer institutional trading framework combining multi-timeframe analysis, multi-agent LLM orchestration, and disciplined risk management.

## Core Crate Topology

- `crates/shared`: Domain representations (`MarketSnapshot`), normalized exchange events, and functional indicator calculations (EMA, RSI, MACD, ADX, ATR, Bollinger, Squeeze, BBWP, Fibonacci, Chart Patterns, Divergence).
- `crates/engine`: High-performance daemon maintaining live Hyperliquid WebSocket connections, multi-timeframe candle aggregation, SQLite telemetry persistence, paper trading simulation, and the Axum HTTP/WS dashboard server.
- `crates/frontend`: Svelte 5 application providing interactive charting, real-time WebSocket snapshots, AI analysis triggers, paper trading controls, commission calculator, and performance analytics.

## 7-Layer Architecture

### Layer 1 — Market Data (Implemented)
- **Exchange Data**: Hyperliquid WebSocket ingestion (order book + trade streams)
- **Data Cache**: In-memory sliding window buffers for all 5 timeframes
- **Candle Aggregator**: Generates 15s, 1m, 5m, 15m, and 1h candles from raw trade streams; additionally aggregates 4h and 1d macro candles from 1m closes
- **Normalization**: `SymbolMapper` translates exchange-specific symbols into normalized identifiers

### Layer 2 — Analysis (Implemented)
- **Indicator Engine**: 51 indicators across 7 groups (10 Trend, 7 Momentum + 4 divergence keys, 7 Volume + 3 divergence keys, 6 Volatility + 1 divergence key, 5 Structure, 4 Regime, 4 Institutional). All driven by the unified registry manifest. Each indicator flows through: calculator → normalizer → signal derivation → scoring. Every indicator produces a `NormalizedIndicatorValue { raw_value, normalized[-1,+1], state_label, values{}, signals[], confidence }`.
- **Market Structure Engine**: Swing highs/lows, S/R role tracking with flip detection, chart pattern classification (triangles, wedges, channels), 29-species candlestick pattern recognition with geometric→context→confirmation pipeline, Smart Money Concepts (BOS/CHoCH, liquidity sweeps, fair value gaps, order blocks).
- **Volume Analysis**: RVOL, Volume Profile (POC/VAH/VAL via OHLCV binning, 100-bar window), Anchored VWAP (daily/weekly/monthly/swing-anchored).
- **Regime Detection Engine**: Classifies market into Trending, Compression, Expansion, or Range using ADX, BBWP, Squeeze state, Choppiness Index, and ATR regime. 7 non-directional gates act as confluence multipliers.

### Layer 3 — Agent Layer (Implemented)
Specialized LLM agents running in a sequential two-agent pipeline via `run_analyst_agent()` → `run_trader_agent()`:

- **Analyst Agent:** Receives ALL deterministic data (51 indicator DTOs, DecisionContext, MarketContext, S/R levels, price history) and produces an 8-section institutional market document (market_summary, trend/momentum/volatility/volume/structure indicators, active_signals, confluence_summary). Zero trading decisions — purely descriptive.
- **Trader Agent:** Receives the Analyst Document + current position + IRML risk profile. Makes strict rule-bound decisions from 5 actions (Hold, Close, Wait, Open Long, Open Short) with confidence score and operational rationale.
- **Journal Agent:** Post-trade audit producing retrospective analysis and execution score (0-10).
- **Chat Agent:** Conversational interface for user queries about market conditions.

### Layer 4 — Orchestration (Implemented)
- **Pipeline Orchestration**: `services/analyzer.rs` orchestrates the two-agent cycle: builds indicator DTOs → compiles DecisionContext + MarketContext → runs Analyst Agent → builds IRML risk profile → runs Trader Agent → spawns background updates
- **Orchestration Cycle**: Configurable interval via `automation.interval_seconds` (default: 900s)
- **Decision Context**: Assembled from analyst document, active positions, IRML risk boundaries
- **Heuristic Fallback**: When no LLM API key is configured, a local 100-point confluence scoring model (trend 30/volatility 25/momentum 20/structure 25) gates decisions by regime, volume confirmation, and minimum score thresholds

### Layer 5 — Risk Management (Partially Implemented)
- **Per-Trade Risk (Implemented)**: `risk_calculator.rs` computes margin, liquidation price, position sizing; 8-factor scoring model with ADX regime gates, RVOL volume gates, and BBWP volatility adjustments
- **Portfolio Risk (New)**: `portfolio_risk.rs` validates new positions against daily drawdown limits, total portfolio exposure caps, single-pair concentration limits, and cross-pair Pearson correlations
- **Paper Trading (Implemented)**: Simulated execution with balance tracking, break-even trailing, opposite-signal exit, and decisive-close invalidation

### Layer 6 — Execution (Paper Only)
- **Paper Trading Simulation**: Virtual order matching against live prices, PnL tracking, trade journal logging
- **Real-Exchange Execution (Planned — Phase 8)**: Order Manager, Position Tracker, and Execution Monitor for Hyperliquid API integration

### Layer 7 — Analytics & Learning (Partially Implemented)
- **Performance Evaluator**: Checks direction correctness at 1h/4h/24h horizons against future price outcomes
- **Trade Journal**: `paper_trades`, `trade_telemetry_history`, `trade_learning_journal` tables with agent-scored post-trade analysis
- **Strategy Optimizer (New)**: Periodic per-regime performance analysis computing Profit Factor, Win Rate, and Avg R-Multiple; generates allocation bias and threshold adjustment recommendations
- **Continuous Learning**: Adaptive reweighting of indicators, threshold adjustments, and position sizing optimization (recommendations logged for review)

## Data Flow Diagram

```
+------------------+       Live Websocket       +---------------+
|   Hyperliquid    |  ======================>   |  Rust Engine  |
+------------------+                            +---------------+
                                                        || 5-Timeframe Pipeline
                                                        || Indicator Computation
                                                        || Cache last 100 closes
                                                        || Serves HTTP API & WS
+------------------+      Trigger Analysis      +---------------+
|   User Browser   |  ---------------------->   |  AI Assistant |
| (Svelte 5 Term)  |  <======================   |   (LLM)       |
+------------------+    Structured Response     +---------------+
         |                                               |
         |        +---------------------------+          |
         +------->|  Portfolio Risk Engine    |<---------+
                  |  Strategy Optimizer       |
                  |  Performance Evaluator    |
                  +---------------------------+
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
| `GET` | `/api/history` | Returns last 100 close prices for the default symbol |
| `POST` | `/api/analyze` | Accepts position + market data, returns structured assistant response |
| `GET` | `/api/cost-estimate` | Token cost projections per pair |
| `POST` | `/api/config/set-key` | Configure DeepSeek API key |
| `POST` | `/api/config/add-pair` | Add a new trading pair at runtime |
| `DELETE` | `/api/config/remove-pair` | Remove a trading pair |
| `GET` | `/api/paper/balance` | Query paper trading balance |
| `POST` | `/api/paper/configure` | Configure paper trading parameters |

## Decision Support Layer

- **DecisionContext**: Read-only quantitative metrics computed per snapshot from the full indicator map. Zero new indicators, zero state.
  - `P(bullish)` / `P(bearish)` — weighted probabilistic vote (confidence × |normalized| × registry weight)
  - `Consensus` — fraction of indicators agreeing on direction. <0.55 = fragmented market.
  - `Expected Range` — 1/5/20 bar ranges, √N scaled, regime-adjusted (trending widens, choppy narrows)
  - `Expected Volatility` — forward-looking annualized σ (HV + squeeze coil/BBWP/ATR expansion signals)
  - `Confluence` — registry-weighted directional score [−100,+100]
  - Attached to every `MarketSnapshot`; auto-transported via WebSocket + REST `/api/history`

## Frontend Trade Lifecycle (5-Stage Flow)

The dashboard presents a clean 5-stage decision pipeline:

```
SETUP → TRIGGER → CONFIRMATION → EXECUTION → MONITORING
```

| Stage | Indicator Groups | Synthesis |
|-------|-----------------|-----------|
| Setup | Trend, Regime, Structure | — |
| Trigger | Momentum, Price Action, Breakouts | — |
| Confirmation | Volume, Trend Strength, Volatility, Smart Money, Order Flow | — |
| Execution | — | Confluence, Decision Context, AI Synthesis |
| Monitoring | — | Active trades, exit signals, scale/TP/SL, PnL tracking |

**Risk Management (IRML)** is a dedicated standalone panel (General mode > Risk Management) providing:
position sizing, ATR stops, stop loss, take profit, max risk, max daily loss, max allocation,
and leverage. Risk boundaries feed into Execution and Monitoring stages but do not form a
pipeline stage themselves.

## Runtime Details

- Server: `http://127.0.0.1:3000` (localhost only)
- Market data: Hyperliquid WebSocket (`wss://api.hyperliquid.xyz/ws`)
- SQLite auto-created at `./telemetry.db` on startup
- Configuration: `config.toml` at workspace root (panics if missing)
