# Database Schema Specification

**Version:** 2.2 (now **22 active live tables** + **1 deferred table (DB-05 `order_fills`)** listed for forward compatibility). The active set: `market_snapshots`, `individual_indicator_logs`, `user_trades`, `paper_balances`, `active_positions`, `position_slots`, `position_equity_snapshots`, `paper_trades`, `exchange_keys`, `decision_profiles`, `profile_indicators`, `risk_profiles`, `portfolio_equity_history`, `trade_telemetry_history`, `trade_learning_journal`, `saved_edges`, `edge_analytics_cache`, `support_resistance_levels`, `connection_quality_samples`, `liquidation_events`, `performance_matrix_snapshots`, `strategy_analytics_history`. This version adds full column tables for the previously stub-only entries (`position_slots`, `position_equity_snapshots`, `portfolio_equity_history`, `decision_profiles`, `profile_indicators`, `user_trades`, `exchange_keys`, `saved_edges`, `edge_analytics_cache`, `support_resistance_levels`, `trade_learning_journal`) and extends `active_positions` with `stop_loss_price`/`take_profit_price`, `paper_balances` with persistent safety state (`active_stance`/`starting_session_equity`/`peak_equity`/`cooldown_start_ms`), and renames the legacy `final_invalidation_level` to `invalidation_level`. See the §3 section expansions and migration footnotes.
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

## 3. Table Catalog (22 Live + 1 Deferred)

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
| `reconstruction_method` | TEXT | Provenance flag for reconstructed candles. Values: `ExchangeHistorical` / `ExponentialMovingAverage` / `LinearInterpolation` / `Unavailable`. NULL for live candles. **Note (v2.1 — naming alignment):** the in-memory Rust struct uses the field name `reconstructed: Option<ReconstructionMethod>` (see [02-06-market-data-matrix.md](../matrices/02-06-market-data-matrix.md) §3 and `crates/shared/src/normalized.rs::NormalizedCandle`); the database column is `reconstruction_method`. The persistence layer maps the Rust `reconstructed` field to the SQLite `reconstruction_method` column on insert/select. When a candle has no reconstruction provenance (live data), the Rust field is `None` (omitted on the wire via `skip_serializing_if`), and the SQLite column stores `NULL`. |
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
One active position per symbol. Field names **align with `03-04-02-pme-layer1-position.md §3`** (positions are written/read by the PME Position Layer).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `symbol` (UNIQUE) | TEXT | Trading pair key (e.g. `BTC-USDT`). |
| `direction` | TEXT | `LONG` / `SHORT`. |
| `entry_price` / `average_entry_price` | REAL / REAL | Entry reference and VWAP. |
| `size` | REAL | Current base-currency size. |
| `allocated_usd` | REAL | Margin allocated at entry. |
| `entry_timestamp` | INTEGER | Entry fill timestamp (ms since epoch). |
| `invalidation_level` | REAL | Structural invalidation price (`Decimal` semantics; stored as `REAL` for telemetry per §3.1 architectural exception). **Renamed from `final_invalidation_level` in v2.1** to align with the canonical `invalidation_level` name used by the Opportunity Matrix and Decision Matrix. Migration `2026XXXX03_rename_invalidation.sql` handles the destructive rename. |
| `stop_loss_price` | REAL | Active stop-loss price coordinate (`Decimal` semantics). **Persisted (DB-03)**: previously in-memory only; restored on cold-start so an engine restart does not open the user to unhedged exposure. |
| `take_profit_price` | REAL | Active take-profit price coordinate (`Decimal` semantics). **Persisted (DB-03)**: symmetric to `stop_loss_price`. |
| `target_profit_ratio` | REAL | Target R:R ratio (e.g. `2.5`). |
| `current_portions` | INTEGER | Active portion count (legacy 4-slot state). |
| `initial_allocated_margin` | REAL | Lifecycle capital tracker. |
| `realized_pnl_accumulator` | REAL | Lifecycle realized PnL. |

#### `position_slots`
4-slot dynamic margin state machine for scaled entries (`03-04-03-pme-layer2-exposure.md`).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `position_id` (FK) | INTEGER | References `active_positions(id)`. `ON DELETE CASCADE`. |
| `symbol` | TEXT | Trading pair key. |
| `direction` | TEXT | `LONG` / `SHORT` (`CHECK (direction IN ('LONG','SHORT'))`). |
| `slot_index` | INTEGER | `0`–`3` (`CHECK (slot_index BETWEEN 0 AND 3)`). |
| `is_active` | INTEGER (0/1) | Active flag for the slot. |
| `entry_price` / `size` / `allocated_usd` | REAL | Per-slot entry data. |
| `realized_pnl` | REAL | Per-slot realized PnL. |
| `timestamp` | INTEGER | Slot open timestamp. |

**Indexes:**
- `idx_position_slots_active (position_id, slot_index)` partial — `WHERE is_active = 1`.
- `idx_position_slots_symbol (symbol, is_active)`.

#### `position_equity_snapshots`
Time-series equity valuations per symbol (per-position equity curve for GUI).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `symbol` | TEXT | Trading pair key. |
| `timestamp` | INTEGER | Snapshot time (ms since epoch). |
| `equity_value` | REAL | Mark-to-market equity including unrealized PnL. |
| `cash_balance` | REAL | Realized cash at snapshot. |
| `unrealized_pnl` | REAL | Open-position PnL at snapshot. |

**Indexes:**
- `idx_pos_equity_ts (symbol, timestamp ASC)`.

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
Per-symbol capital config + **persistent safety state**. `committed_margin`, `unrealized_pnl`, and `available_margin` are **derived metrics** computed on-demand from `active_positions` and `open_orders` — they are **not** persisted as columns. The startup recovery process recomputes them from the persisted `active_positions` and `open_orders` rows using the canonical formula in [PME Layer 3 §4.2](../engines/portfolio-management-engine/03-04-04-pme-layer3-capital.md).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `symbol` (UNIQUE) | TEXT | Trading pair key. |
| `initial_usd` | TEXT (Decimal-precision) | Starting capital allocated to this symbol. **Migrated from REAL to TEXT** for full `rust_decimal::Decimal` precision per migration `2026XXXX05_paper_balances_decimal.sql`. The Decimal precision invariant applies at the cold-path edge; SQLite TEXT round-trips exactly. |
| `current_cash` | TEXT (Decimal-precision) | **Absolute cash balance** = `initial_usd + SUM(paper_trades.realized_pnl)` (see DB-17 below). The column stores the absolute balance, not the realized-PnL relative metric — the conceptual seam is `realized_pnl = current_cash − initial_usd`. |
| `allocation_pct` | REAL | Fraction of total capital allocated to this symbol. |
| `auto_execute` | INTEGER (0/1) | Enable automated execution. |
| `auto_execute_intervals` | INTEGER | Number of timeframe intervals to evaluate per cycle. |
| `max_risk_pct` | TEXT (Decimal-precision) | Per-trade risk cap. |
| `leverage` | INTEGER | Maximum cross leverage. |
| `lookback_trades` | INTEGER | Rolling lookback for analytics. |
| `break_even_trail_enabled` | INTEGER (0/1) | Enable break-even trailing stop. (Added in migration `20260703000000_break_even_trail.sql`.) |
| `active_stance` | TEXT NOT NULL DEFAULT `'ACTIVE'` | Per-symbol authorization (`ACTIVE` / `CLOSE_ONLY` / `AVOID`). **Persisted (DB-07)** since migration `2026XXXX01_persistent_safety_state.sql` — previously in-memory only, which meant an engine restart reset the active safety veto to `default_stance` and bypassed cooldowns. |
| `starting_session_equity` | TEXT NOT NULL DEFAULT `'0'` | Equity recorded at the most recent session-reset boundary (operator-defined `session_reset_cron`, default `00:00 UTC`). Used for the `max_daily_drawdown_pct` warning. **Persisted (DB-04)**. |
| `peak_equity` | TEXT NOT NULL DEFAULT `'0'` | Trailing high-water mark. The 30 % drawdown veto evaluates `current_equity / peak_equity < 1 − drawdown_limit_pct`. **Persisted (DB-04)** — without this column the early-warning system cannot survive engine restart. |
| `cooldown_start_ms` | INTEGER | Timestamp (ms since epoch) at which a symbol entered the `SUSPENDED` cooldown state. **Persisted (DB-18)**. NULL when not on cooldown. |

#### `paper_trades`
Closed paper-trade history.

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `symbol` / `direction` | TEXT / TEXT | Trading pair / LONG or SHORT. |
| `entry_price` / `exit_price` / `size` | REAL / REAL / REAL | Entry/exit fill and base-currency size. |
| `realized_pnl` / `roi_pct` | REAL / REAL | Net PnL after fees / ROI percentage. `roi_pct` is the canonical key (not `roi_percentage`); see DB-20 below. |
| `entry_timestamp` / `exit_timestamp` | INTEGER / INTEGER | Fill timestamps (ms since epoch). |
| `flat_trade` | INTEGER (0/1) | `1` if `|gross_pnl| == 0` (used to suppress division-by-zero in `profit_factor` and `fee_efficiency`). |
| `trigger` | TEXT | Originating policy/trigger identifier. |
| `hold_time_seconds` | INTEGER | Duration of the trade (seconds from entry fill to final exit fill). Persisted per PAE Layer 1 requirement (DB-09). |
| `execution_slippage` | REAL | Target-vs-actual execution slippage (bps). Persisted per PAE Layer 1 (DB-09). |
| `mfe` / `mae` | REAL / REAL | Maximum Favorable / Adverse Excursion. Persisted per PAE Layer 1 (DB-09). |
| `exit_reason` | TEXT | Exit cause (e.g. `STOP_HIT`, `TARGET_HIT`, `INVERSE_SIGNAL`, `DRAWDOWN_STOP`). Persisted per PAE Layer 1 (DB-09). |

---

### 3.3 Decision & Intelligence

#### `decision_profiles`
Scoring profiles.

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `profile_name` (UNIQUE) | TEXT | Display key. |
| `long_threshold` | INTEGER | Threshold above which a directional bias is LONG. Default `40`. |
| `short_threshold` | INTEGER | Threshold below which a directional bias is SHORT. Default `-40`. |

#### `profile_indicators`
Per-profile indicator weight rules (FK → `decision_profiles` ON DELETE CASCADE).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `profile_id` (FK) | INTEGER | References `decision_profiles(id)`. |
| `indicator_name` | TEXT | Display key (e.g. `RSI (Oversold/Overbought)`). |
| `weight` | INTEGER | Multiplier. Default `10`. |
| `override_status` | TEXT | `NONE` / `OVERRIDE` / `EXCLUDE`. Default `'NONE'`. |

#### `risk_profiles`
Risk management configuration. **All Decimal fields migrated to TEXT** in `20260715200000_risk_profiles_decimal.sql` for full `rust_decimal::Decimal` precision.

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `profile_name` (UNIQUE) | TEXT | Profile display key. |
| `capital` | TEXT (Decimal-precision) | Allocated capital. Default `'1000'`. |
| `max_risk_pct` | TEXT (Decimal-precision) | Per-trade risk cap. Default `'2'`. |
| `leverage` | INTEGER | Cross-leverage multiplier. Default `20`. |
| `commission_pct` | TEXT (Decimal-precision) | Commission rate. Default `'0.06'` (0.06 %). |
| `funding_rate_8h` | TEXT (Decimal-precision) | Funding rate per 8-hour window (the canonical Decimal-precision column, intentionally added to this spec — `06-02 §3.3` previously listed the column in prose but omitted it from the table). Default `'0'`. |
| `spread` | TEXT (Decimal-precision) | Spread cost basis. Default `'0'`. |

> **Note (v2.1 — clarification).** `funding_rate_8h` **is** stored in `risk_profiles` (column type `TEXT` for full `rust_decimal::Decimal` precision after migration `20260715200000_risk_profiles_decimal.sql`). The runtime uses the per-profile value when present, falling back to the global `config.json` `fees.funding_rate_8h` (default `0.01` = 0.01 % per 8 hours) when the profile value is `0` or unset. **Canonical unit.** Like `risk_per_trade_pct`, `funding_rate_8h` is a **percent float** at the wire/config boundary and is divided by 100 to obtain a fraction inside the engine for accrual computation. See [03-03-05-tae-paper-trading-spec.md §4](../engines/trade-automation-engine/03-03-05-tae-paper-trading-spec.md) and `crates/engine/src/risk_calculator.rs::compute_risk_from_profile` (line 187) for the authoritative consumer.

---

### 3.4 Performance & Observability

#### `portfolio_equity_history`
Equity snapshots (60 s cadence). Index on `timestamp DESC`. 30-day retention.

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `timestamp` | INTEGER (NOT NULL) | Snapshot time (ms since epoch). |
| `total_value` | REAL | Total portfolio mark-to-market equity. |
| `cash_balance` | REAL | Realized cash across all symbols (`SUM(paper_balances.current_cash)`). |
| `unrealized_pnl` | REAL | Sum of open-position unrealized PnL. |

**Indexes:**
- `idx_equity_history_timestamp_desc (timestamp DESC)`.

#### `trade_telemetry_history`
Automated trade telemetry (aggregated closed-trade facts).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `exchange` / `symbol` / `direction` | TEXT / TEXT / TEXT | Originating exchange / pair / side. |
| `entry_timestamp` / `exit_timestamp` | INTEGER / INTEGER | Fill timestamps. |
| `entry_price` / `exit_price` / `size` | REAL / REAL / REAL | Entry/exit fills and base-currency size. |
| `commission_fees` / `funding_fees` / `realized_pnl` | REAL / REAL / REAL | Cost components and net PnL. |
| `roi_pct` | REAL | ROI percentage — **canonical key is `roi_pct`** (not `roi_percentage`); see DB-20 below. |
| `flat_trade` | INTEGER (0/1) | `1` if `|gross_pnl| == 0`. |
| `trigger_source` | TEXT | Originating policy/trigger identifier. |
| `hold_time_seconds` | INTEGER | Trade duration (DB-09). |
| `execution_slippage` | REAL | Per-trade slippage (DB-09). |
| `mfe` / `mae` | REAL / REAL | MFE / MAE (DB-09). |
| `exit_reason` | TEXT | Exit cause (DB-09). |

#### `trade_learning_journal`
Human-annotated trade journal (FK → `trade_telemetry_history`).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `trade_id` (FK) | INTEGER | References `trade_telemetry_history(id)`. |
| `entry_date` / `exit_date` | TEXT / TEXT | ISO-8601 dates. |
| `asset` / `direction` | TEXT / TEXT | Symbol and side. |
| `entry_reason` | TEXT | Operator's thesis rationale. |
| `roi_percentage` | REAL | Companion ROI percentage. *(The journal uses `roi_percentage` for legacy reasons and is retained here; the canonical term across `paper_trades` and `trade_telemetry_history` is `roi_pct`. Roadmap: align this column in a future migration.)* |
| `final_analysis` | TEXT | Operator's post-trade assessment. |
| `execution_score` | REAL | 0–10 self-evaluation. |
| `human_notes` | TEXT | Free-form notes. |
| `created_at` | TEXT | Default `datetime('now')`. |

**Indexes:**
- `idx_journal_lookup (asset, execution_score DESC)`.

---


### 3.5 Strategy Configuration

#### `saved_edges`
Edge strategy persistence.

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `name` (UNIQUE) | TEXT | Human-readable edge name. |
| `pair_key` | TEXT | Trading pair key. |
| `description` | TEXT | Free-form summary. |
| `config_payload` | TEXT (NOT NULL) | JSON-serialized policy configuration. |
| `created_at` | TEXT (NOT NULL) | Default `datetime('now')`. |

**Indexes:**
- `idx_saved_edges_name (name)`.

#### `edge_analytics_cache`
Cached analytics per `saved_edges` row (PK = `edge_id`, FK CASCADE).

| Column | Type | Description |
|--------|------|-------------|
| `edge_id` (PK, FK) | INTEGER | References `saved_edges(id)`. `ON DELETE CASCADE`. |
| `historical_metrics` | TEXT (NOT NULL) | JSON-serialized metrics. |
| `monte_carlo_paths` | TEXT (NOT NULL) | JSON-serialized Monte Carlo distribution. |
| `bootstrap_results` | TEXT (NOT NULL) | JSON-serialized bootstrap statistics. |
| `generated_at` | TEXT (NOT NULL) | Default `datetime('now')`. |

#### `user_trades`
User-logged trade outcomes (operator-entered journal-style records).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `timestamp` | INTEGER (NOT NULL) | Submission time. |
| `symbol` | TEXT (NOT NULL) | Trading pair. |
| `direction` | TEXT (NOT NULL) | `LONG` / `SHORT`. |
| `outcome` | TEXT (NOT NULL) | Operator-supplied outcome tag. |
| `risk_multiplier` | REAL (NOT NULL) | Operator multiplier on baseline risk. |
| `reward_multiplier` | REAL (NOT NULL) | Operator multiplier on baseline reward. |

#### `exchange_keys`
Encrypted API credentials (AES-256-GCM, master key from `EXCHANGE_SECRET_KEY` env var).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `exchange` | TEXT (NOT NULL) | `Hyperliquid` / `Bitget`. |
| `account_name` | TEXT (NOT NULL) | Operator label. |
| `api_key` | TEXT (NOT NULL) | Encrypted. |
| `api_secret` | TEXT (NOT NULL) | Encrypted. |
| `passphrase` | TEXT (NOT NULL, default `''`) | Some exchanges require. |
| `referred_uid` | TEXT (NOT NULL, default `''`) | Referral UID where required. |
| `is_active` | INTEGER (NOT NULL, default `0`) | Active flag. |
| `last_sync_timestamp` | INTEGER | Last successful sync (ms since epoch). |

#### `support_resistance_levels`
Caches S/R levels per symbol (UNIQUE on `symbol`).

| Column | Type | Description |
|--------|------|-------------|
| `id` (PK) | INTEGER | Auto-increment. |
| `symbol` (UNIQUE) | TEXT (NOT NULL) | Trading pair. |
| `s1` / `s2` / `s3` | REAL | Support levels. |
| `r1` / `r2` / `r3` | REAL | Resistance levels. |
| `calculated_at` | INTEGER (NOT NULL) | Last calculation timestamp. |

#### `order_fills`
> **Status: deferred (DB-05).** Per-fill table is **not** currently implemented — PAE trade reconstruction currently operates on aggregate `paper_trades` / `trade_telemetry_history` only. Future migration `2026XXXXX_order_fills.sql` to add: `fill_id` (PK), `order_id`, `trade_id`, `parent_position_id`, `exchange`, `symbol`, `side`, `price` (TEXT, Decimal), `size` (TEXT, Decimal), `fee` (TEXT, Decimal), `fee_currency`, `is_maker` (INTEGER), `timestamp_ms` (INTEGER). Defined here for forward compatibility — see [03-05-02-pae-layer1-trade-analytics.md §3, §4](../engines/performance-analytics-engine/03-05-02-pae-layer1-trade-analytics.md) for the analytical requirement.

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
Raw per-trade liquidation events ingested from the exchange WebSocket (Phase 1 of the Liquidity Intelligence extension). Source-of-truth for the per-candle `LiquidityFlow` aggregate (see [02-12-liquidity-matrix.md §Schema](../matrices/02-12-liquidity-matrix.md) for the in-memory fields: `long_liquidations_usd`, `short_liquidations_usd`, `net_liquidation_usd`, `event_count`, `largest_event_usd`, `largest_event_price`, `largest_event_side`, `cascade_state`, `cascade_intensity`). The `cascade_risk_index` computation is tracked under [01-05-liquidity-domain.md §Open questions — Canonical deferred-work tracker](../conceptual-foundations/01-05-liquidity-domain.md).

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
| `expectancy` | REAL | `(win_rate × avg_win) − ((1−win_rate) × avg_loss)`, where `average_loss` is stored as a **positive magnitude** (the mean absolute value of losing-trade PnLs). See [03-05-03-pae-layer2-strategy-analytics.md §2.1](../engines/performance-analytics-engine/03-05-03-pae-layer2-strategy-analytics.md) for the canonical sign convention. |
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
