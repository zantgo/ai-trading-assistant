# Quantitative Trading Engine

A deterministic quantitative trading platform powered by Hyperliquid market data. 58 technical indicators, 25 statistical modules, multi-factor decision matrix, and paper trading execution — all without AI/LLM dependencies.

## Architecture

```
Hyperliquid WS → 5-TF Candle Aggregation → 58 Indicators (7 groups)
  → Regime Classification → Structure Mapping → Confluence Scoring
  → Decision Context (17 metrics) → Statistical Intelligence (25 modules)
  → Risk Management → Deterministic Decision Matrix → Execution → Performance
```

### 4 Domains

| Domain | Layers | Description |
|--------|--------|-------------|
| **Market Features** | ITIL, IRCL, ISML | 58 indicators, 5-regime classifier, S/R/SMC/VP/Fib mapping |
| **Quantitative Analysis** | ICSL, IDCL, ISIL, IRML | Confluence scoring, decision metrics, statistics (VaR, GARCH, EVT, MC), risk profiles |
| **Decision Matrix** | IDML | Weighted multi-factor engine — hard gates → composite score → directional action |
| **Trade Operations** | IEPL, IPEL, IMOL | TWAP/VWAP execution, paper trading, performance evaluation, active monitoring |

## Quick Start

```bash
# Build frontend
cd crates/frontend && npm install && npm run build

# Run engine
cd ../.. && cargo run

# Or use the management script
./manage.sh build && ./manage.sh run
```

Dashboard: `http://127.0.0.1:3000`

## Configuration

`config.toml` controls indicator windows, decision matrix weights, risk thresholds, and automation intervals.

## Testing

```bash
./manage.sh test-core     # Indicator math + serialization
./manage.sh test-engine   # DB, paper trading, server
./manage.sh test-ui       # Svelte 5 components
./manage.sh test          # Full suite (all 3)
```
