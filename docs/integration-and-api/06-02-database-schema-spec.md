# Database Schema Specification

**Version:** 2.1 (now **22 Active Tables** — added `connection_quality_samples`, `liquidation_events`, `performance_matrix_snapshots`, `strategy_analytics_history`; previous version stated 18 Active Tables)
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
| Configuration source | **`config.json`** at workspace root (single source of truth; see [08-01-user-manual.md §5](../operations-and-compliance/08-01-user-manual.md)). The DB tables hold session-scoped local fallbacks; precedence is detailed in §3.0. |

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
- **Connection-quality retention:** `connection_quality_samples` rows older than 7 days are cleaned on startup and hourly.
- **Liquidation-event retention:** `liquidation_events` rows older than 90 days are cleaned on startup and hourly.
- **PAE retention:** `performance_matrix_snapshots` and `strategy_analytics_history` rows older than 365 days are cleaned on startup and daily.

---

## 3. Table Catalog (22 Active Tables)

### 3.0 Configuration Precedence

The single source of configuration truth is **`config.json`** at the workspace root, as documented in [08-01-user-manual.md §5](../operations-and-compliance/08-01-user-manual.md). The DB tables `risk_profiles` (§3.6) and `paper_balances` (§3.2) hold **session-scoped local fallbacks** for values not present in `config.json`.

**Precedence rules:**

1. **For values present in multiple sources** (most-recent override wins):
   - `paper_balances` row (`POST /api/instances/:id/config`) — instance-level override.
   - `risk_profiles` row (`POST /api/risk-profiles`) — profile-level override.
   - `config.json` — global default (loaded at startup; not auto-overwritten by GUI).

2. **For values present in only one source**: the existing value is used; no override.

3. **For values absent from all sources**: the engine falls back to hard-coded defaults.

The precedence is enforced by the engine loader at startup and verified on every `POST /api/config` round-trip. Note: `config.json` is **mutable** via explicit `POST /api/config` calls but is **not auto-overwritten** by routine GUI runtime edits (those go to `risk_profiles` or `paper_balances`).

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
| `trades_count` | INTEGER | Number of trades aggregated into the candle. |
| `reconstruction_method` | TEXT | Provenance flag for reconstructed candles. Values: `ExchangeHistorical` / `ExponentialMovingAverage` / `LinearInterpolation` / `Unavailable`. NULL for live candles. |
| `indicators_json` | TEXT | **Single JSON document** holding the entire indicator evaluation map — every indicator's `normalized`, `state_label`, `raw_value`, and nested `signals` array, plus structural series (Bollinger, ATR, VWAP, EMA ribbon, MACD, ADX, Squeeze, BBWP, S/R levels, Fibonacci levels). Mirrors the Metrics Matrix indicator block 1:1. |
| `liquidity_json` | TEXT | JSON-serialized `LiquidityFlow` snapshot (Phase 1, nullable). NULL when liquidity extension disabled. |
| `cluster_json` | TEXT | JSON-serialized `LiquidationClusterMatrix` snapshot (Phase 2, nullable, 5-min refresh). NULL when liquidity extension disabled. |
| `liquidity_signals_json` | TEXT | JSON-serialized `Vec<LiquiditySignal>` (Phase 3, per-snapshot, derived from `liquidity_json` + `cluster_json`). NULL when liquidity extension disabled. |

> **Precision boundary.** Columns storing financial values use SQLite REAL (IEEE 754 f64). The cold-path Decimal precision invariant is preserved at the Rust engine boundary (engine code converts REAL ↔ `rust_decimal::Decimal` on read/write); SQLite REAL is acceptable for **telemetry and historical analytics** per the architectural exception documented in [01-02-global-architecture.md §6.2](../conceptual-foundations/01-02-global-architecture.md). The Decimal precision guarantee applies to **active position math** (TAE/PME) which never reads from SQLite on the hot path.

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
Unified order management. Note: orders in `PRE_DISPATCH` state (held by Gate 5 slippage review, see [08-02-pre-trade-risk-controls.md §3.2](../operations-and-compliance/08-02-pre-trade-risk-controls.md)) are **not** persisted here — only post-exchange-acknowledgement orders are recorded.

| Column | Type |
|--------|------|
| `id` | SERIAL PK |
| `order_id` | TEXT (UNIQUE) | Exchange-assigned order ID (post-acknowledgement). |
| `client_order_id` | TEXT | Engine-assigned client order ID for reconciliation. |
| `symbol` | TEXT |
| `order_type` | TEXT (LIMIT / STOP / MARKET) |
| `direction` | TEXT (BUY / SELL) |
| `state` | TEXT (OPEN / FILLED / PARTIALLY_FILLED / CANCELED / REJECTED / EXPIRED) |
| `price` / `trigger_price` | REAL |
| `size` | REAL |
| `is_reduce_only` | INTEGER (0/1) |
| `is_emergency_liquidation` | INTEGER (0/1) | True for Hard Exit path orders (bypasses pre-trade gates). |
| `associated_position_id` | INTEGER |
| `created_at` | INTEGER |
| `acknowledged_at` | INTEGER | Exchange acknowledgement timestamp (used to enforce cancellation timing per [PME Layer 4 §4.2](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)). |

#### `paper_balances`
Per-symbol capital config. `committed_margin`, `unrealized_pnl`, and `available_margin` are **derived metrics** computed on-demand from `active_positions` and `open_orders` — they are **not** persisted as columns. The startup recovery process recomputes them from the persisted `active_positions` and `open_orders` rows using the canonical formula in [PME Layer 3 §4.2](../engines/portfolio-management-engine/03-04-04-pme-layer3-capital.md).

| Column | Type | Description |
|--------|------|-------------|
| `symbol` (UNIQUE) | TEXT | Trading pair key. |
| `initial_usd` | REAL | Starting capital allocated to this symbol. |
| `current_cash` | REAL | Current cash balance for this symbol (realized only; derived metrics are computed on-demand). |
| `allocation_pct` | REAL | Fraction of total capital allocated to this symbol. |
| `auto_execute` | INTEGER (0/1) | Enable automated execution. |
| `auto_execute_intervals` | INTEGER | Number of timeframe intervals to evaluate per cycle. |
| `max_risk_pct` | REAL | Per-trade risk cap (fraction). |
| `leverage` | INTEGER | Maximum cross leverage. |
| `default_stance` | TEXT | Operator-configured default stance (`ACTIVE` / `CLOSE_ONLY` / `AVOID`), restored on veto release. |
| `consecutive_losses` | INTEGER DEFAULT 0 | Per-symbol loss streak (used by `SUSPENDED` state trigger). |
| `lookback_trades` | INTEGER | Rolling lookback for analytics. |
| `break_even_trail_enabled` | INTEGER (0/1) | Enable break-even trailing stop. |
| `leverage_mode` | TEXT | Cross / Isolated. |
| `leverage_cap` | REAL | Maximum allowed leverage. |
| `atr_leverage_multiplier` | REAL | ATR-based dynamic leverage factor. |

#### `paper_trades`
Closed paper-trade history.

| Column | Type |
|--------|------|
| `id` | SERIAL PK |
| `symbol` / `direction` | TEXT / TEXT |
| `entry_price` / `exit_price` / `size` | REAL / REAL / REAL |
| `realized_pnl` / `roi_pct` | REAL / REAL |
| `entry_timestamp` / `exit_timestamp` | INTEGER / INTEGER |
| `flat_trade` | INTEGER (0/1) | `1` if `|gross_pnl| == 0` (used to suppress division-by-zero in `profit_factor` and `fee_efficiency`). |
| `trigger` | TEXT | Originating policy/trigger identifier. |

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
| `commission_pct` / `spread` | REAL / REAL |

> **Note.** `funding_rate_8h` is **not** stored in `risk_profiles`. The funding rate is a global venue property read from `config.json` `fees.funding_rate_8h` (default 0.01 % per 8 hours). See [03-03-05-tae-paper-trading-spec.md §4](../engines/trade-automation-engine/03-03-05-tae-paper-trading-spec.md) for the authoritative source.

---

### 3.4 Performance & Observability

#### `portfolio_equity_history`
Equity snapshots (60 s cadence). Index on `timestamp DESC`. 30-day retention.

#### `trade_telemetry_history`
Automated trade telemetry.

| Column | Type |
|--------|------|
| `id` | SERIAL PK |
| `exchange` / `symbol` / `direction` | TEXT / TEXT / TEXT |
| `entry_timestamp` / `exit_timestamp` | INTEGER / INTEGER |
| `entry_price` / `exit_price` / `size` | REAL / REAL / REAL |
| `commission_fees` / `funding_fees` / `realized_pnl` | REAL / REAL / REAL |
| `roi_percentage` | REAL |
| `flat_trade` | INTEGER (0/1) | `1` if `|gross_pnl| == 0` (used to suppress division-by-zero in `profit_factor` and `fee_efficiency`). |
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
Caches S/R levels per symbol (UNIQUE on `symbol`).

---

### 3.6 Extensions (Phase 3+ — Liquidity, Connection Quality, PAE)

#### `connection_quality_samples`
Time-series connection-quality reports. Written every 60 seconds by `crates/engine/src/connection_quality.rs` background task.

| Column | Type |
|--------|------|
| `id` | SERIAL PK |
| `timestamp_ms` | INTEGER (NOT NULL) |
| `window` | TEXT (NOT NULL) — `ONE_HOUR` \| `SIX_HOUR` \| `TWENTY_FOUR_HOUR` |
| `uptime_pct` | REAL (NOT NULL) |
| `disconnect_count` | INTEGER (NOT NULL) |
| `avg_reconnect_ms` | REAL (NOT NULL) |
| `total_data_loss_secs` | INTEGER (NOT NULL) |
| `reconstructed_candles` | INTEGER (NOT NULL) |
| `score` | REAL (NOT NULL) |

**Indexes:**
- `idx_cq_window_time (window, timestamp_ms)`

Retention: 7 days. See [08-05-connection-quality.md](../operations-and-compliance/08-05-connection-quality.md) §Persistence for the source-of-truth behaviour and rolling-window semantics.

#### `liquidation_events`
Raw per-trade liquidation events ingested from the exchange WebSocket (Phase 1 of the Liquidity Intelligence extension). Source-of-truth for `LiquidityFlow.per_event` aggregation and for the `cascade_risk_index` computation.

| Column | Type | Description |
|--------|------|-------------|
| `id` | SERIAL PK | Auto-increment. |
| `exchange` | TEXT (NOT NULL) | Originating venue. |
| `symbol` | TEXT (NOT NULL) | Instrument key. |
| `side` | TEXT (NOT NULL) | `Long` \| `Short`. |
| `notional` | REAL (NOT NULL) | Liquidation notional in quote currency. |
| `price` | REAL (NOT NULL) | Trade price at the moment of liquidation. |
| `timestamp_ms` | INTEGER (NOT NULL) | Exchange event timestamp. |
| `received_ms` | INTEGER (NOT NULL) | Local ingest timestamp. |

**Indexes:**
- `idx_liq_sym_time (symbol, timestamp_ms)`
- `idx_liq_exchange_time (exchange, timestamp_ms)`

Retention: 90 days per [01-05-liquidity-domain.md §Configuration](../conceptual-foundations/01-05-liquidity-domain.md). See [02-12-liquidity-matrix.md §Schema](../matrices/02-12-liquidity-matrix.md) for the in-memory `LiquidityFlow` aggregation contract.

### 3.7 PAE Persistence Tables

#### `performance_matrix_snapshots`
Snapshot of the [Performance Matrix](../engines/performance-analytics-engine/03-05-05-pae-layer4-performance.md) at scheduled cadence (default 300 s). Used by the GUI for retroactive visualization and by the headless CLI for after-action review.

| Column | Type | Description |
|--------|------|-------------|
| `id` | SERIAL PK | Auto-increment. |
| `timestamp_ms` | INTEGER (NOT NULL) | Snapshot write time. |
| `window_start_ms` | INTEGER (NOT NULL) | First trade timestamp included. |
| `window_end_ms` | INTEGER (NOT NULL) | Last trade timestamp included. |
| `regime_compatibility_json` | TEXT (NOT NULL) | Serialized Regime Compatibility Matrix. |
| `system_metrics_json` | TEXT (NOT NULL) | Serialized Sharpe / Sortino / drawdown summary. |
| `trade_count` | INTEGER (NOT NULL) | Trade count over the window. |

**Indexes:** `idx_pms_time (timestamp_ms)`

Retention: 365 days.

#### `strategy_analytics_history`
Statistical-significance history per execution policy (`policy_id`).

| Column | Type | Description |
|--------|------|-------------|
| `id` | SERIAL PK | Auto-increment. |
| `policy_id` | TEXT (NOT NULL) | FK — originating execution policy. |
| `timestamp_ms` | INTEGER (NOT NULL) | Computation time. |
| `window_trade_count` | INTEGER (NOT NULL) | Trade count in the analysis window. |
| `win_rate` | REAL | Win-rate ∈ [0, 1]. |
| `profit_factor` | REAL | gross_profit / gross_loss. |
| `expectancy` | REAL | `(win_rate × avg_win) − ((1−win_rate) × avg_loss)`. |
| `sharpe` | REAL | Sharpe ratio over the window. |
| `sortino` | REAL | Sortino ratio over the window. |
| `t_statistic` | REAL | T-Statistic (one-tailed positive test). |
| `p_value` | REAL | P-value from one-tailed t-distribution. |
| `p_mc` | REAL | Monte Carlo sign-randomization empirical probability. |
| `is_significant` | INTEGER (0/1) | `1` iff `p_value < 0.05` AND `p_mc < 0.05`. |
| `monte_carlo_runs` | INTEGER (NOT NULL) | MC sample count (default 10 000). |

**Indexes:** `idx_sah_policy_time (policy_id, timestamp_ms)`

Retention: 365 days. See [03-05-03-pae-layer2-strategy-analytics.md §2](../engines/performance-analytics-engine/03-05-03-pae-layer2-strategy-analytics.md) for field provenance.

---

## 4. Seeding

A default `'Default'` decision profile (long_threshold=40, short_threshold=−40) with 7 indicator rules and a default `'Risk Profile'` ($1,000 capital, 2% max risk, 20× leverage) are seeded on startup.

---

## 5. Cross-References

- [API Gateway Contract](06-01-api-gateway-contract.md) — API surface.
- [Systemic Data Flow](../conceptual-foundations/01-03-systemic-data-flow.md) — Data flows.
- [PAE Overview](../engines/performance-analytics-engine/03-05-01-pae-overview-spec.md) — Performance database consumption.
- [TAE Paper Trading](../engines/trade-automation-engine/03-03-05-tae-paper-trading-spec.md) — Paper tables.
