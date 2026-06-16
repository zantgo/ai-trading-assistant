# Project Execution Plan: AI Trading Assistant

This workspace uses a multi-stage approach to build an interactive trading visualizer and an on-demand AI decision assistant based on the Institutional Multi-Timeframe Momentum & Volatility Framework (v3.0).

## Phase 1: Telemetry & Ingestion (Implemented)
- [x] Create shared domain schema (`MarketSnapshot`).
- [x] Build Hyperliquid WebSocket client for L2 order book parsing.
- [x] Program fast-path Rust indicators (EMA, ATR, RSI, MACD, ADX, Bollinger, Squeeze).

## Phase 2: Visual Terminal (Implemented)
- [x] Configure Axum to stream dynamic snapshots over `/ws`.
- [x] Create Svelte 5 responsive dashboard with synced lightweight-charts.
- [x] Implement dynamic configuration syncing from `config.toml`.

## Phase 3: Persistent Logging (Implemented)
- [x] Implement SQLite database initialization at `./telemetry.db`.
- [x] Save completed candle snapshots to database for evaluation.

## Phase 4: Interactive Assistant UI & Prompting (Implemented)
- [x] Add position status radio inputs (`None`, `Long`, `Short`).
- [x] Add "Request AI Assistant Analysis" button.
- [x] Implement engine buffer caching last 100 close prices.
- [x] Build `POST /api/analyze` endpoint.

## Phase 5: Structured AI Analysis Loop (Implemented)
- [x] Integrate DeepSeek LLM connectivity.
- [x] Define multi-agent pipeline (Trend, Volatility, Structure, Risk, Position agents).
- [x] Master Orchestrator synthesis with decision memory and trade history buffers.

## Phase 6: Interactive Terminal Output (Implemented)
- [x] Multi-step progress UI in Svelte.
- [x] Display structured response: trend classification → indicator interpretation → recommendation.

## Phase 7: Historical Performance Logging (Implemented)
- [x] Save recommendations to `master_assistant_records`.
- [x] Performance evaluator checking direction correctness at 1h/4h/24h horizons.

## Phase 7.5: Framework Alignment & Infrastructure (In Progress)
- [x] Wire candle aggregator for 4h/1d macro candles from 1m closes.
- [x] Align heuristic fallback with 100-point confluence scoring, regime gating, and volume confirmation.
- [x] Update architecture documentation to 7-layer v3.0 specification.
- [ ] Migrate config.toml from legacy `[scoring]` section to framework-defined institution thresholds.

## Phase 8: Portfolio Risk Engine (Implemented)
- [x] `portfolio_risk.rs` module with daily drawdown limits, total exposure caps, single-pair concentration, and Pearson correlation checks.
- [x] Integration into automation loop for pre-trade validation.
- [x] `pair_close_histories` shared state for cross-pair correlation computation.

## Phase 9: Strategy Optimization Engine (Implemented)
- [x] `strategy_optimizer.rs` periodic evaluation loop (1h interval).
- [x] Per-regime performance analysis: Win Rate, Profit Factor, Avg R-Multiple.
- [x] Allocation bias and threshold adjustment recommendations.
- [x] Reports logged to `agent_thought_logs` table.

## Phase 10: Real-Exchange Execution Layer (Planned)
- [ ] Hyperliquid API order routing integration.
- [ ] Order Manager: market orders, limit orders, stop orders.
- [ ] Position Tracker: entry price, size, leverage, SL/TP.
- [ ] Execution Monitor: filled orders, partial fills, failed orders, slippage.
- [ ] Exchange credential encryption using AES-256-GCM.

## Phase 11: Dynamic Strategy Re-weighting (Planned)
- [ ] Automatic indicator weight adjustment based on per-regime performance.
- [ ] Confidence threshold adaptation from completed trade outcomes.
- [ ] Position sizing optimization from historical R-multiple distributions.
- [ ] All adjustments require Master Orchestrator review before adoption.

## Phase 12: Multi-Asset Portfolio Management (Planned)
- [ ] Sector concentration monitoring.
- [ ] Correlation-adjusted position sizing across pairs.
- [ ] Portfolio-level drawdown circuit breakers.
- [ ] Walk-forward validation and Monte Carlo simulation.
