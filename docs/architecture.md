# System Architecture

## Workspace Topology
- `crates/shared`: Defines the domain entities (`MarketSnapshot`, `Trade`, `Indicator`). Shared by Engine and Frontend.
- `crates/engine`: High-performance async daemon. Contains the WebSocket logic, math modules, and DEX adapters.
- `crates/frontend`: WebAssembly UI for live monitoring.
- `crates/mcp-server`: MCP protocol implementation to interact with AI models.

## The Hexagonal Pattern (Multi-DEX)
We use Rust **Traits** to define the `DexExchange` port. This allows the engine to swap Hyperliquid for Drift or Uniswap simply by switching a configuration trait.

## Telemetry Pipeline
Every tick follows a transactional flow:
1. Stream (Hyperliquid L1)
2. Log (Postgres Snapshot)
3. Reason (AI Agent)
4. Execute (Trade)
5. Audit (Result logging)
