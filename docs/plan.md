# Project Execution Plan: DeX AI Trading Assistant

This workspace uses a multi-stage approach to build an interactive trading visualizer and an on-demand AI decision assistant.

## Phase 1: Telemetry & Ingestion (Implemented)
- [x] Create shared domain schema (`MarketSnapshot`).
- [x] Build Hyperliquid Testnet WebSocket client for L2 order book parsing.
- [x] Program fast-path Rust indicators (EMA, SMA, ATR, RSI, MACD, ADX, Bollinger, Squeeze).

## Phase 2: Visual Terminal (Implemented)
- [x] Configure Axum to stream dynamic snapshots over `/ws`.
- [x] Create Svelte 5 responsive dashboard with synced lightweight-charts.
- [x] Implement dynamic configuration syncing from `config.toml`.

## Phase 3: Persistent Logging (Implemented)
- [x] Implement SQLite database initialization at `./telemetry.db`.
- [x] Save completed candle snapshots to database for evaluation.

## Phase 4: Interactive Assistant UI & Prompting
- [ ] Add interactive form inputs in the Svelte sidebar to toggle the current position status:
  * Radio options: `None`, `Long`, `Short`.
- [ ] Add a prominent action button: `"Request AI Assistant Analysis"`.
- [ ] Implement an engine buffer to cache the close prices of the last 100 intervals.
- [ ] Build a local API endpoint `POST /api/analyze` in the Rust engine to compile:
  * Current position status.
  * Historical close prices (array of 100 floats).
  * Current indicator snapshot.

## Phase 5: Structured AI Analysis Loop (MCP or Direct LLM Integration)
- [ ] Integrate LLM connectivity (e.g., via a local endpoint or an MCP Server).
- [ ] Define the sequential system prompt instructing the LLM to output a clean JSON structure:
  1. **Market Structure Evaluation:** Analyze the last 100 prices to determine the trend (`Upward`, `Downward`, `Sideways`).
  2. **Technical Alignment:** Match the computed indicators against the trend direction.
  3. **Strategic Recommendation:**
     * If position is `Long` or `Short` -> Recommendation: `Hold` or `Close` (with reasoning).
     * If position is `None` -> Recommendation: `Wait`, `Open Long`, or `Open Short` (with entry reasoning).

## Phase 6: Interactive Terminal Output
- [ ] Replace the mock "SIGNALS" placeholder in Svelte with a dynamic, multi-step progress UI.
- [ ] Display the AI assistant's structured response sequentially:
  * Show parsed trend classification.
  * Show indicator interpretation.
  * Highlight the final call-to-action block.

## Phase 7: Historical Performance Logging
- [ ] Save the assistant's structured recommendations to a database table (`assistant_records`).
- [ ] Provide simple review visualizers to cross-reference recommendations against future price outcomes.
