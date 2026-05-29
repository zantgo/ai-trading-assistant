# System Architecture

The AI Trading Assistant acts as a data pipeline and UI cockpit designed for structured analysis, passing clean market data arrays and user context directly to LLMs on-demand.

## Core Topology
- `crates/shared`: Domain representations (`MarketSnapshot`) and functional indicator calculations.
- `crates/engine`: High-performance daemon that maintains live WebSocket connections, aggregates telemetry, caches historical sequences, and hosts the visual dashboard.
- `crates/engine/frontend`: A lightweight Svelte 5 application providing interactive layout panels, real-time charting, and manual AI analysis triggers.

```
+------------------+       Live Websocket       +---------------+
|   Hyperliquid    |  ======================>   |  Rust Engine  |
+------------------+                            +---------------+
                                                        || Caches last 100 closes
                                                        || Serves HTTP API & WS
                                                        \/
+------------------+      Trigger Analysis      +---------------+
|   User Browser   |  ---------------------->   |  AI Assistant |
| (Svelte 5 Term)  |  <======================   |   (LLM/MCP)   |
+------------------+    Structured Response     +---------------+
```

## System Operations: The On-Demand Assistant Loop

Rather than using automated trade-signing pipelines, the data-flow focuses on structured, transactional prompting:

### 1. Ingestion & Historical Cache
The Rust engine receives high-frequency market updates. It keeps a sliding buffer of the last 100 candles (closes, highs, lows) alongside computed indicator states.

### 2. Svelte State Synthesis
The user defines their operational context through the frontend panel:
- Position State: `None` | `Long` | `Short`

### 3. Trigger & Request Assembly
When the operator clicks "Request AI Assistant Analysis", an API request is assembled containing:
```json
{
  "position": "Long",
  "historical_prices": [3124.50, 3125.10, 3122.90, "... 100 items"],
  "indicators": {
    "rsi": 42.50,
    "squeeze_on": true,
    "macd_histogram": -0.45
  }
}
```

### 4. Structured Reasoning Sequence
The AI analyzes the data in a deterministic chain of logic, enforced by system instructions:
- **Price Action Trend Resolution:** Inspects the raw price vector to evaluate structure (`upward`, `downward`, `sideways`).
- **Indicator Merging:** Compares structural movement against secondary indicator features.
- **Position Mitigation:** Evaluates context options:
  * Long + Upward/Strong Indicators = `Hold`
  * Long + Downward/Weak Indicators = `Close`
  * Short + Downward/Strong Indicators = `Hold`
  * Short + Upward/Weak Indicators = `Close`
  * None + Upward/Strong Indicators = `Open Long`
  * None + Downward/Strong Indicators = `Open Short`
  * None + Sideways/Unclear Indicators = `Wait`

### 5. Render
The output is returned as structured JSON, rendering the sequential logical breakdown on the dashboard for the trader's final manual execution.
