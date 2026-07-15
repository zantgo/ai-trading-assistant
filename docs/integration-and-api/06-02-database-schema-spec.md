# Database Schema Specification

**Version:** 2.0
**Status:** Approved
**Purpose:** This document specifies the SQLite database schema — all persistent tables, indexes, WAL configuration, and migration strategy for the Trading Platform's shared telemetry store.

---

## 1. Database Technology

| Property | Value |
|----------|-------|
| Engine | SQLite (auto-created at `./telemetry.db` on first launch) |
| WAL mode | Enabled (Write-Ahead Logging) |
| Synchronous level | `NORMAL` |
| Connection pool | `sqlx` SQLite pool |
| Migration strategy | Versioned `.sql` files applied sequentially by `sqlx::migrate!()` |

---

## 2. WAL & Retention Policy

```
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
```

- **Busy timeout:** 5 seconds.
- **Shadow-snapshot exclusion:** The telemetry logger rejects any `MarketSnapshot` with `is_completed == false` before writing; only finalized candle closes are persisted (see §3.1.1).
- **Snapshot purge:** Completed snapshots older than 7 days are cleaned on startup and hourly by the telemetry logger.
- **Equity purge:** Portfolio equity history records older than 30 days are cleaned.

---

## 3. Table Catalog (18 Active Tables)

### 3.1 Core Telemetry

#### `market_snapshots`
The primary time-series table — **one row per completed candle** (see §3.1.1 for the shadow-snapshot exclusion). Market/quote fields are stored as native columns; the full indicator evaluation map is stored as a single structured JSON document, avoiding per-indicator column migrations as the registry grows.

| Column | Type | Description |
|--------|------|-------------|
| `id` | SERIAL PK | Auto-increment. |
| `exchange` | TEXT | Hyperliquid / Bitget. |
| `symbol` | TEXT | Unified internal symbol. |
| `timeframe_secs` | INTEGER | Candle duration. |
| `timestamp` | INTEGER | Candle close (Unix epoch). |
| `mid_price` / `bid_price` / `ask_price` | REAL | Quote snapshot. |
| `open` / `high` / `low` / `close` | REAL | OHLC. |
| `volume` / `average_volume` | REAL | Volume. |
| `indicators_json` | TEXT | **Single JSON document** holding the entire indicator evaluation map — every indicator's `normalized`, `state_label`, `raw_value`, and nested `signals` array, plus structural series (Bollinger, ATR, VWAP, EMA ribbon, MACD, ADX, Squeeze, BBWP, S/R levels, Fibonacci levels). Mirrors the Metrics Matrix indicator block 1:1. |

**Rationale:** A single JSON document decouples the persistence schema from the indicator registry. Adding, removing, or re-phasing an indicator requires **no SQLite migration** — only the serializer changes. This trades a small read-time parse cost for zero schema churn across the 50-entry (and growing) registry.

**Indexes:**
- `(symbol, timeframe_secs, timestamp DESC)` — primary time-series lookup.
- ML feature-vector access uses **SQLite JSON1 expression indexes** over hot keys, e.g. `CREATE INDEX idx_ms_rsi ON market_snapshots (json_extract(indicators_json, '$.rsi.normalized'))`. New feature indexes are added without altering the table structure.

#### 3.1.1 Shadow Snapshot Exclusion

The telemetry logger **persists only finalized candle closes**. Any incoming `MarketSnapshot` with `is_completed == false` (a real-time "shadow" flicker snapshot streamed on every tick for live UI updates) is **rejected before the write path** and never reaches disk. This guarantees `market_snapshots` holds exactly one row per completed candle and prevents tick-cadence write-lock contention and unbounded database growth. The `is_completed` flag itself is not stored — every persisted row is, by construction, a completed candle.

---

### 3.2 Position & Trading State

#### `active_positions`
One active position per symbol.

| Column | Type |
|--------|------|
| `symbol` (UNIQUE) | TEXT |
| `direction` | TEXT |
| `entry_price` / `average_entry_price` | REAL |
| `size` | REAL |
| `allocated_usd` | REAL |
| `entry_timestamp` | INTEGER |
| `final_invalidation_level` / `target_profit_ratio` | REAL |
| `current_portions` | INTEGER |
| `initial_allocated_margin` | REAL |
| `realized_pnl_accumulator` | REAL |

#### `position_slots`
4-slot dynamic margin state machine for scaled entries.

#### `position_equity_snapshots`
Time-series equity valuations per symbol.

#### `open_orders`
Unified order management.

| Column | Type |
|--------|------|
| `symbol` | TEXT |
| `order_type` | TEXT (LIMIT / STOP) |
| `direction` | TEXT (BUY / SELL) |
| `price` / `trigger_price` | REAL |
| `size` | REAL |
| `is_reduce_only` | INTEGER |
| `associated_position_id` | INTEGER |
| `created_at` | INTEGER |

#### `paper_balances`
Per-symbol capital config.

| Column | Type |
|--------|------|
| `symbol` (UNIQUE) | TEXT |
| `initial_usd` / `current_cash` | REAL |
| `allocation_pct` | REAL |
| `auto_execute` / `auto_execute_intervals` | INTEGER / INTEGER |
| `max_risk_pct` / `leverage` | REAL / INTEGER |
| `lookback_trades` | INTEGER |
| `break_even_trail_enabled` | INTEGER |
| `leverage_mode` / `leverage_cap` / `atr_leverage_multiplier` | TEXT / REAL / REAL |

#### `paper_trades`
Closed paper-trade history.

| Column | Type |
|--------|------|
| `symbol` / `direction` | TEXT / TEXT |
| `entry_price` / `exit_price` / `size` | REAL / REAL / REAL |
| `realized_pnl` / `roi_pct` | REAL / REAL |
| `entry_timestamp` / `exit_timestamp` | INTEGER / INTEGER |
| `trigger` | TEXT |

---

### 3.3 Decision & Intelligence

#### `decision_profiles`
Scoring profiles.

| Column | Type |
|--------|------|
| `profile_name` (UNIQUE) | TEXT |
| `long_threshold` / `short_threshold` | REAL |

#### `profile_indicators`
Per-profile indicator weight rules (FK → `decision_profiles` ON DELETE CASCADE).

#### `risk_profiles`
Risk management configuration.

| Column | Type |
|--------|------|
| `profile_name` (UNIQUE) | TEXT |
| `capital` / `max_risk_pct` / `leverage` | REAL / REAL / INTEGER |
| `commission_pct` / `funding_rate_8h` / `spread` | REAL / REAL / REAL |

---

### 3.4 Performance & Observability

#### `portfolio_equity_history`
Equity snapshots (60 s cadence). Index on `timestamp DESC`. 30-day retention.

#### `trade_telemetry_history`
Automated trade telemetry.

| Column | Type |
|--------|------|
| `exchange` / `symbol` / `direction` | TEXT / TEXT / TEXT |
| `entry_timestamp` / `exit_timestamp` | INTEGER / INTEGER |
| `entry_price` / `exit_price` / `size` | REAL / REAL / REAL |
| `commission_fees` / `funding_fees` / `realized_pnl` | REAL / REAL / REAL |
| `roi_percentage` | REAL |
| `trigger_source` | TEXT |

#### `trade_learning_journal`
Human-annotated trade journal (FK → `trade_telemetry_history`).

---


### 3.5 Strategy Configuration

#### `saved_edges`
Edge strategy persistence.

| Column | Type |
|--------|------|
| `name` (UNIQUE) | TEXT |
| `pair_key` | TEXT |
| `description` | TEXT |
| `config_payload` | TEXT (JSON) |

#### `edge_analytics_cache`
Cached analytics (PK = `edge_id`, FK CASCADE). Historical metrics, Monte Carlo paths, bootstrap results.

#### `user_trades`
User-logged trade outcomes.

#### `exchange_keys`
Encrypted API credentials (AES-256-GCM, master key from `EXCHANGE_SECRET_KEY` env var).

#### `support_resistance_levels`
Cached S/R levels per symbol (UNIQUE on `symbol`).

---

## 4. Seeding

A default `'Default'` decision profile (long_threshold=40, short_threshold=−40) with 7 indicator rules and a default `'Risk Profile'` ($1,000 capital, 2% max risk, 20× leverage) are seeded on startup.

---

## 5. Cross-References

- [API Gateway Contract](06-01-api-gateway-contract.md) — API surface.
- [Systemic Data Flow](../conceptual-foundations/01-03-systemic-data-flow.md) — Data flows.
- [PAE Overview](../engines/performance-analytics-engine/03-05-01-pae-overview-spec.md) — Performance database consumption.
- [TAE Paper Trading](../engines/trade-automation-engine/03-03-05-tae-paper-trading-spec.md) — Paper tables.
