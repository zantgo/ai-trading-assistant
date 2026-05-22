# Project Execution Plan: DeX Trading Agent Engine

This project is built using a **Cargo Workspace**. Each phase must be independently verifiable.

## Phase 1: Core Engine & Data Ingestion
- [ ] Initialize Cargo Workspace (crates: `engine`, `shared`).
- [ ] Implement Hyperliquid WebSocket client using `tokio` and `tungstenite`.
- [ ] Implement `MarketSnapshot` struct in `crates/shared`.
- [ ] Implement math module for Technical Indicators (RSI, MACD) in Rust.

## Phase 2: Frontend Sandbox (Wasm)
- [ ] Setup `Leptos` or `Dioxus` (Rust Wasm).
- [ ] Create WebSocket server in `engine` to stream data to frontend.
- [ ] Visualize real-time price and indicators.

## Phase 3: Telemetry & Database
- [ ] Docker Compose setup for PostgreSQL.
- [ ] SQLx schema definition for `MarketSnapshot`.
- [ ] Persistent logging pipeline.

## Phase 4: MCP & AI Integration
- [ ] Implement MCP Server using `mcp-sdk-rs`.
- [ ] Integrate OpenRouter API (DeepSeek/Qwen).
- [ ] Log "Chain-of-Thought" to Postgres.

## Phase 5: Paper Trading Engine
- [ ] Implement `PaperTradingEngine` state machine in Rust.
- [ ] Handle Buy/Sell/TP/SL execution logic.
- [ ] Log trade outcomes (PnL) to DB.

## Phase 6: Data Science Analysis
- [ ] Jupyter Notebook setup (Python).
- [ ] Correlation analysis (AI confidence vs. PnL).
- [ ] Data export to Parquet/CSV.

## Phase 7: Live Execution
- [ ] Implement cryptographic signing (EIP-712).
- [ ] Enforce deterministic Risk Guardrails.
- [ ] Live Mainnet deployment.
