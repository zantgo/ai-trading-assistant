# Database Schema Specification

**Version:** 6.9 (2026-08-04) — see docs/CHANGELOG.md for the canonical version history.

**Status:** Specified — target of record

**This catalog is the current target schema (version per README §Feature Status).** Per-table implementation status is tracked in README §Feature Status.
**Purpose:** This document specifies the SQLite database schema — all persistent tables, indexes, WAL configuration, and migration strategy for the Trading Platform's shared telemetry store.

**Active tables (26):** `market_snapshots`, `open_orders`, `user_trades`, `paper_balances`, `active_positions`, `position_slots`, `position_equity_snapshots`, `paper_trades`, `exchange_keys`, `decision_profiles`, `profile_indicators`, `risk_profiles`, `portfolio_equity_history`, `trade_telemetry_history`, `trade_learning_journal`, `saved_edges`, `edge_analytics_cache`, `support_resistance_levels`, `connection_quality_samples`, `liquidation_events`, `performance_matrix_snapshots`, `strategy_analytics_history`, **`order_fills`** (B-6 — activated in v4.0), **`risk_control_events`** (B-5 — added in v4.0), **`instance_lifecycle`** + **`instance_lifecycle_events`** (IL-13 — added in v6.2; see [03-03-06 §5](../engines/trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md)).

**Deferred (forward-compatibility only):** none.

---

## 1. Database Engine & Configuration

| Property | Value |
|---|---|
| Engine | **SQLite** with WAL (`journal_mode = WAL`), `synchronous = NORMAL`, `foreign_keys = ON` |
| Driver | `rusqlite` (bundled) |
| File | `./telemetry.db` at workspace root (`telemetry.db` local to CLI engine instance; cloud-headless setups use `rsync`, `scp`, or cloud-synced volumes) |
| Migration | `crates/database-storage/migrations/ (sqlx::migrate! consumes at build-time)` — applied on engine startup via `apply_pending_migrations()` |

### 1.1 Type conventions

| Logical Type | SQLite DDL |
|---|---|
| Auto-increment ID | `INTEGER PRIMARY KEY AUTOINCREMENT` |
| Timestamp (epoch ms) | `INTEGER NOT NULL` |
| Timestamp (epoch sec) | `INTEGER NOT NULL` |
| `Decimal` (price, size, capital) | `TEXT NOT NULL` — serialized via `rust_decimal::Decimal::to_string()`. Per-column `GLOB` CHECKs are retired in all tables migrated since v4.0; validation is enforced at the Rust `Decimal::from_str()` type boundary. Named exceptions that retain GLOB CHECKs until their next migration: the 13 legacy tables (§3.11) and `funding_rate_8h` (§9 item 7). |
| Nullable value (optional Decimal) | `TEXT` — distinguishes `NULL` (inherit global) from the canonical pattern (e.g. `"0"` = disable). Per-column `GLOB` CHECKs retired except in the named exceptions above; validation at the Rust type boundary. |
| Boolean | `INTEGER NOT NULL CHECK (value IN (0, 1))` |
| JSON value | `TEXT NOT NULL CHECK (json_valid(value))` |
| JSON array | `TEXT NOT NULL CHECK (json_valid(value))` (always JSON, even for empty — the platform distinguishes JSON-empty from key-omitted via `json_valid` + presence) |
| JSON value (nullable) | `TEXT` (no CHECK) |
| String (enum union) | `TEXT NOT NULL CHECK (value IN (...))` |
| Foreign key | `INTEGER REFERENCES other_table(id) ON DELETE …` (only on the few FKs that exist — most references are denormalized config identifiers) |

All `Decimal` fields are serialized via `rust_decimal::Decimal::to_string()` and deserialized via `Decimal::from_str()`. The wire-format docs in [`02-07-metrics-matrix.md`](../matrices/02-07-metrics-matrix.md), [`02-09-overview-matrix.md`](../matrices/02-09-overview-matrix.md), and [`06-01-api-gateway-contract.md §2.10`](06-01-api-gateway-contract.md) mirror the same convention.

### 1.2 Schema-version invariant

Schema versions are tracked per-table in this catalog text: each table's section (§3.N) records its schema version and migration notes in prose — there is no physical `_schema_version` row or column in the DDL. Migrations bump the documented per-table version when the schema for that table changes. The engine refuses to start if the database `user_version` does not match the schema-version compatibility window — see §9.

---

## 2. Indexes & Query Performance

Indexes are created on each table for the query patterns the engine actually uses:

| Index | Columns | Use |
|---|---|---|
| `idx_market_snapshots_pair_time` | `(pair_key, timeframe_secs, timestamp DESC)` | Replay history fetch |
| `idx_market_snapshots_completed` | `(pair_key, timeframe_secs, timestamp DESC) WHERE is_completed = 1` | MME pipeline (only completed snapshots) |
| `idx_market_snapshots_reconstructed` | `(pair_key, timeframe_secs, reconstruction_method) WHERE reconstructed = 1` | Reconstructed-candle audit |
| `idx_open_orders_state` | `(state, instance_id, created_at)` | Live order lifecycle queries |
| `idx_position_slots_position_slot` | `(position_id, slot_index)` | Scaled Entry reconstruction |
| `idx_exchange_keys_exchange` | `(exchange)` | Key lookup by venue |
| `idx_rce_instance_gate_time` | `(instance_id, gate_id, timestamp_ms DESC)` | Gate-rejection audit dashboards |
| `idx_rce_operator_time` | `(operator_id, timestamp_ms DESC)` | Override-history audit (`operator_id = "local"`) |
| `idx_order_fills_trade` | `(trade_id)` | Per-fill PAE reconstruction |
| `idx_order_fills_order` | `(order_id)` | Per-order fill chain |
| `idx_cq_pair_timeframe_window_time` | `(pair_key, timeframe_secs, window, timestamp_ms DESC)` | Connection-quality queries (per-instance × per-timeframe window filter) |
| `idx_lifecycle_events_instance_time` | `(instance_id, timestamp_ms DESC)` | Lifecycle transition audit queries |

---

## 3. Active Table Catalog

Tables are grouped by ownership. Each entry shows the canonical schema (DDL-style), invariants, and migration notes. The `id` column is `INTEGER PRIMARY KEY AUTOINCREMENT` unless explicitly noted.

### 3.1 `market_snapshots` — MME telemetry persistence (storage owned by DIE; content produced by MME)

The primary time-series table — one row per completed candle, paired with the rolled-up MME matrix outputs that ride the WS `MarketSnapshot`. Rows are written only for completed candles; the `is_completed` column is retained for forward compatibility with shadow persistence (always `1` today), and the partial index `idx_market_snapshots_completed` is defensive.

```sql
CREATE TABLE IF NOT EXISTS market_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key TEXT NOT NULL,
    timeframe_secs INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    is_completed INTEGER NOT NULL DEFAULT 1,
    exchange TEXT NOT NULL,
    mid_price TEXT NOT NULL,
    bid_price TEXT NOT NULL,
    ask_price TEXT NOT NULL,
    bid_size TEXT NOT NULL,
    ask_size TEXT NOT NULL,
    funding_rate TEXT,
    open TEXT NOT NULL,
    high TEXT NOT NULL,
    low TEXT NOT NULL,
    close TEXT NOT NULL,
    volume TEXT NOT NULL,
    average_volume TEXT,
    open_interest TEXT,
    oi_delta_1h TEXT,
    prev_day_px TEXT,
    mark_price TEXT,
    index_price TEXT,
    mark_index_spread_pct REAL,
    reconstructed INTEGER NOT NULL DEFAULT 0,
    reconstruction_method TEXT CHECK (reconstruction_method IS NULL OR reconstruction_method IN ('EXCHANGE_HISTORICAL','EXPONENTIAL_MOVING_AVERAGE','LINEAR_EXTRAPOLATION','UNAVAILABLE')),
    indicators_json TEXT NOT NULL CHECK (json_valid(indicators_json)),
    liquidity_json TEXT CHECK (liquidity_json IS NULL OR json_valid(liquidity_json)),
    cluster_json TEXT CHECK (cluster_json IS NULL OR json_valid(cluster_json)),
    liquidity_signals_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(liquidity_signals_json)),
    alignment_json TEXT CHECK (alignment_json IS NULL OR json_valid(alignment_json)),
    analysis_json TEXT CHECK (analysis_json IS NULL OR json_valid(analysis_json)),
    risk_json TEXT CHECK (risk_json IS NULL OR json_valid(risk_json)),
    advisory_json TEXT CHECK (advisory_json IS NULL OR json_valid(advisory_json)),
    opportunity_json TEXT CHECK (opportunity_json IS NULL OR json_valid(opportunity_json)),
    metrics_config_json TEXT CHECK (metrics_config_json IS NULL OR json_valid(metrics_config_json)),
    decision_context_json TEXT CHECK (decision_context_json IS NULL OR json_valid(decision_context_json)),
    context_json TEXT CHECK (context_json IS NULL OR json_valid(context_json)),
    statistical_context_json TEXT CHECK (statistical_context_json IS NULL OR json_valid(statistical_context_json)),
    risk_profile_json TEXT CHECK (risk_profile_json IS NULL OR json_valid(risk_profile_json))
);
CREATE INDEX IF NOT EXISTS idx_market_snapshots_pair_time ON market_snapshots(pair_key, timeframe_secs, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_market_snapshots_completed ON market_snapshots(pair_key, timeframe_secs, timestamp DESC) WHERE is_completed = 1;
CREATE INDEX IF NOT EXISTS idx_market_snapshots_reconstructed ON market_snapshots(pair_key, timeframe_secs, reconstruction_method) WHERE reconstructed = 1;
```

**Persistence rule.** `market_snapshots` stores the **core candle + MME matrix fields** that are needed for replay and historical backtesting. Fields that are local to the live WS broadcast envelope (e.g. live shadow values that flicker until the next completed candle, the §A.7 `cascade_risk_index` placeholder) are **not persisted** to keep the table tight. The canonical wire contract is in [`02-07-metrics-matrix.md §2.1`](../matrices/02-07-metrics-matrix.md); live-only fields are computed on the wire and recomputed by the MME during replay. The wire and persistent contracts are deliberately scoped — see `docs/CHANGELOG.md` for the resolution history.

**Book-state provenance.** `mid_price`, `bid_price`, `ask_price`, and `average_volume` are sourced from the AssetContext/book channels (not from the candle close itself) and may lag the close.

**Liquidity signals serialization.** `liquidity_signals_json` is **always serialized** as a JSON array (never omitted via `skip_serializing_if`). An empty signal set produces `"[]"`. This policy is matched by the live `/ws` payload and by [`02-07-metrics-matrix.md §2.1`](../matrices/02-07-metrics-matrix.md).

### 3.2 `open_orders` — TAE order lifecycle (canonical vocabulary)

Persistent record of every order state from `PENDING` onwards. Matched to the canonical Execution Matrix lifecycle in [`03-03-03-tae-layer2-execution.md §4`](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md):

```sql
CREATE TABLE IF NOT EXISTS open_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id TEXT NOT NULL UNIQUE,
    instance_id TEXT NOT NULL,
    pair_key TEXT NOT NULL,
    side TEXT NOT NULL CHECK (side IN ('LONG', 'SHORT')),
    state TEXT NOT NULL CHECK (state IN ('PENDING','SUBMITTED','OPEN','PARTIALLY_FILLED','CLOSED','REJECTED','CANCELLED')),
    requested_size TEXT NOT NULL,
    filled_size TEXT NOT NULL DEFAULT '0',
    entry_price TEXT,
    stop_loss_price TEXT,
    take_profit_price TEXT,
    invalidation_level TEXT,
    is_reduce_only INTEGER NOT NULL DEFAULT 0,
    is_emergency_liquidation INTEGER NOT NULL DEFAULT 0 CHECK (is_emergency_liquidation IN (0, 1)),
    slippage_bps INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    acknowledged_at INTEGER,
    filled_at INTEGER,
    close_reason TEXT CHECK (close_reason IS NULL OR close_reason IN ('STOP_LOSS','TAKE_PROFIT','SIGNAL_EXIT','MANUAL','VETO','TIMEOUT','EMERGENCY_LIQUIDATION'))
);
CREATE INDEX IF NOT EXISTS idx_open_orders_state ON open_orders(state, instance_id, created_at);
```

**Per `03-03-03-tae-layer2-execution.md §4`:** `PRE_DISPATCH` orders are held in process memory only and are **never** persisted to `open_orders`. The `risk_control_events` table (§3.10) is the persistent audit trail for every held order; the `/api/pre-dispatch/*` resource ([`06-01 §2.9`](06-01-api-gateway-contract.md)) is the operator surface.

The `close_reason` vocabulary is canonical: `STOP_LOSS`, `TAKE_PROFIT`, `SIGNAL_EXIT`, `MANUAL`, `VETO`, `TIMEOUT`, `EMERGENCY_LIQUIDATION`. The PAE contract consumes this exact enum without aliasing.

**Emergency-liquidation flag.** `is_emergency_liquidation` is persisted for audit and replay: `close_reason = 'EMERGENCY_LIQUIDATION'` is written at close, while the flag covers the pre-close lifecycle (set when the order is dispatched as an emergency liquidation, e.g. by the `/stop` flatten path).

### 3.3 `user_trades` — operator-created manual trades

```sql
CREATE TABLE IF NOT EXISTS user_trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('LONG', 'SHORT')),
    outcome TEXT NOT NULL CHECK (outcome IN ('WIN', 'LOSS', 'BREAKEVEN', 'OPEN')),
    risk_multiplier TEXT NOT NULL,
    reward_multiplier TEXT NOT NULL,
    opened_at INTEGER NOT NULL,
    closed_at INTEGER,
    notes TEXT
);
```

### 3.4 `paper_balances` — persistent per-instance paper balance

```sql
CREATE TABLE IF NOT EXISTS paper_balances (
    instance_id TEXT PRIMARY KEY,
    balance TEXT NOT NULL,
    initial_balance TEXT NOT NULL,
    starting_session_equity TEXT NOT NULL,
    peak_equity TEXT NOT NULL,
    cooldown_start_ms INTEGER,
    active_stance TEXT NOT NULL DEFAULT 'ACTIVE' CHECK (active_stance IN ('ACTIVE','CLOSE_ONLY','AVOID')),
    safety_state TEXT NOT NULL DEFAULT 'NORMAL' CHECK (safety_state IN ('NORMAL','WARN','CAUTIOUS','SUSPENDED','DRAWDOWN_STOP')),
    consecutive_losses INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);
```

**Persistence semantics.** `active_stance` (per-symbol authorization: `ACTIVE`, `CLOSE_ONLY`, `AVOID`) and the account-level `safety_state` (`NORMAL`, `WARN`, `CAUTIOUS`, `SUSPENDED`, `DRAWDOWN_STOP`) are both persisted; `consecutive_losses` and `cooldown_start_ms` complete the safety-state reconstruction set. The PME reconstructs the safety-state machine on engine restart from these columns deterministically. See [`03-04-05-pme-layer4-portfolio.md §3`](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) and the `AUDIT-V4-046` resolution in `docs/CHANGELOG.md`.

### 3.5 `active_positions` — PME Position Matrix

Persistent per-symbol position state, full Position Matrix schema:

```sql
CREATE TABLE IF NOT EXISTS active_positions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    instance_id TEXT NOT NULL,
    pair_key TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('LONG', 'SHORT')),
    entry_price TEXT NOT NULL,
    average_entry_price TEXT NOT NULL,
    position_size TEXT NOT NULL,
    invalidation_level TEXT,
    stop_loss_price TEXT,
    take_profit_price TEXT,
    current_price TEXT,
    unrealized_pnl TEXT,
    roi_pct TEXT NOT NULL,
    opened_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

The `invalidation_level` field is canonical across L4 Opportunity Matrix, L6 Decision Matrix, and this Position Matrix. `roi_pct` is the canonical field; the legacy export alias (retired name recorded in `docs/CHANGELOG.md`) is deprecated — removal tracked as AUDIT-V4-044, target v6.9 (see [`06-01-api-gateway-contract.md §2.7`](06-01-api-gateway-contract.md)).

### 3.6 `position_slots` — scaled-entry reconciliation

```sql
CREATE TABLE IF NOT EXISTS position_slots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL REFERENCES active_positions(id) ON DELETE CASCADE,
    slot_index INTEGER NOT NULL,
    size TEXT NOT NULL,
    price TEXT NOT NULL,
    filled_at INTEGER NOT NULL,
    UNIQUE (position_id, slot_index)
);
CREATE INDEX IF NOT EXISTS idx_position_slots_position_slot ON position_slots(position_id, slot_index);
```

### 3.7 `paper_trades` and `order_fills` — PAE trade reconstruction (full per-fill, activated)

Two tables together support the PAE contract:

```sql
CREATE TABLE IF NOT EXISTS paper_trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trade_id TEXT NOT NULL UNIQUE,
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL,
    exit_reason TEXT NOT NULL CHECK (exit_reason IN ('STOP_LOSS','TAKE_PROFIT','SIGNAL_EXIT','MANUAL','VETO','TIMEOUT','EMERGENCY_LIQUIDATION')),
    hold_seconds INTEGER NOT NULL,
    gross_pnl TEXT NOT NULL,
    net_pnl TEXT NOT NULL,
    roi_pct TEXT NOT NULL,
    open_time INTEGER NOT NULL,
    close_time INTEGER NOT NULL,
    MFE TEXT NOT NULL,
    MAE TEXT NOT NULL,
    entry_vwap TEXT NOT NULL,
    exit_vwap TEXT NOT NULL,
    flat_trade INTEGER NOT NULL DEFAULT 0
);
```

```sql
CREATE TABLE IF NOT EXISTS order_fills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id TEXT NOT NULL,
    trade_id TEXT NOT NULL,
    fill_index INTEGER NOT NULL,
    side TEXT NOT NULL CHECK (side IN ('LONG', 'SHORT')),
    fill_type TEXT NOT NULL CHECK (fill_type IN ('ENTRY', 'EXIT')),
    filled_size TEXT NOT NULL,
    fill_price TEXT NOT NULL,
    target_price TEXT,
    fill_slippage_bps INTEGER NOT NULL DEFAULT 0,
    fee_currency TEXT NOT NULL CHECK (fee_currency IN ('MAKER', 'TAKER')),
    fee_paid TEXT NOT NULL,
    funding_accrued TEXT NOT NULL DEFAULT '0',
    filled_at INTEGER NOT NULL,
    UNIQUE (trade_id, fill_index)
);
CREATE INDEX IF NOT EXISTS idx_order_fills_trade ON order_fills(trade_id);
CREATE INDEX IF NOT EXISTS idx_order_fills_order ON order_fills(order_id);
```

`order_fills` is **active in v4.0** (B-6). The PAE contract (`03-05-02 §3`) is now **complete per-fill attribution**: MFE/MAE, slippage (per-fill `target_price - fill_price`), fee attribution, and volume-weighted average entry/exit are all computed from the per-fill rows. See [`03-05-02-pae-layer1-trade-analytics.md`](../engines/performance-analytics-engine/03-05-02-pae-layer1-trade-analytics.md) §3.

### 3.8 `exchange_keys` — encrypted API credentials (AES-256-GCM)

```sql
CREATE TABLE IF NOT EXISTS exchange_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_id TEXT NOT NULL UNIQUE,
    exchange TEXT NOT NULL,
    encrypted_api_key BLOB NOT NULL,
    encrypted_api_secret BLOB NOT NULL,
    encrypted_passphrase BLOB,
    encryption_nonce BLOB NOT NULL,
    encryption_algorithm TEXT NOT NULL DEFAULT 'AES-256-GCM' CHECK (encryption_algorithm = 'AES-256-GCM'),
    created_at INTEGER NOT NULL,
    last_rotated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_exchange_keys_exchange ON exchange_keys(exchange);
```

> **Encrypted credentials only.** `config.toml` holds no secret material — all API keys, secrets, and passphrases live in this table, encrypted at rest with `EXCHANGE_SECRET_KEY` (master key from environment variable). The contract is in [`06-01-api-gateway-contract.md §2.10`](06-01-api-gateway-contract.md).

### 3.9 `connection_quality_samples` — per-instance uptime telemetry

Instance- and timeframe-scoped (one row per `(pair_key, timeframe_secs, window, timestamp_ms)`). v6.0 makes this table the **single canonical home** for connection-quality persistence. The `connection_quality_events` table that appeared in earlier code (referenced from `crates/database-storage/src/connection_quality_persistence/mod.rs`) is **not** part of the active schema and that module is removed in v6.0; see `08-05-connection-quality.md` for the unified per-instance model.

```sql
CREATE TABLE IF NOT EXISTS connection_quality_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key TEXT NOT NULL,
    timeframe_secs INTEGER NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    window TEXT NOT NULL CHECK (window IN ('ONE_HOUR', 'SIX_HOUR', 'TWENTY_FOUR_HOUR')),
    uptime_pct REAL NOT NULL,
    disconnect_count INTEGER NOT NULL,
    avg_reconnect_ms REAL NOT NULL,
    total_data_loss_secs INTEGER NOT NULL,
    reconstructed_candles INTEGER NOT NULL,
    score REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_cq_pair_timeframe_window_time ON connection_quality_samples(pair_key, timeframe_secs, window, timestamp_ms DESC);
```

The persistence loop in `crates/network-adapters/src/connection_quality_tracker.rs::run_persistence_loop` writes one row per (tracker × window) every 60 seconds; there is one tracker per `(pair_key, timeframe_secs)` pair, so a workspace with `N` symbols and a 4-tier ladder yields up to `4 × N` trackers, each producing 3 rows per 60s tick (one per window).

### 3.10 `risk_control_events` — gate-rejection and override audit (new in v4.0)

Every pre-trade gate failure (Gates 0–7; Gate 0 is the lifecycle gate added in v6.2 per [03-03-06 IL-05](../engines/trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md)) and every operator override is logged with the local-operator identity, gate id, decision, prior state, resulting state, and a retention timestamp:

```sql
CREATE TABLE IF NOT EXISTS risk_control_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    policy_id TEXT,
    instance_id TEXT NOT NULL,
    gate_id INTEGER NOT NULL,
    decision TEXT NOT NULL CHECK (decision IN ('BLOCK', 'HELD_FOR_REVIEW', 'CLIP_AND_CONTINUE', 'OVERRIDE')),
    reason TEXT NOT NULL,
    requested_disposition TEXT NOT NULL,
    operator_id TEXT NOT NULL DEFAULT 'local',
    prior_state TEXT,
    resulting_state TEXT,
    pre_dispatch_order_id TEXT,
    timestamp_ms INTEGER NOT NULL,
    retention_until_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_rce_instance_gate_time ON risk_control_events(instance_id, gate_id, timestamp_ms DESC);
CREATE INDEX IF NOT EXISTS idx_rce_operator_time ON risk_control_events(operator_id, timestamp_ms DESC);
```

`operator_id = 'local'` is the fixed identity in v4.0 (per the local-only authentication model in [`06-01 §1`](06-01-api-gateway-contract.md)); `'anonymous'` remains available by convention for cases where the API layer forwards without an explicit identity (not currently surfaced). The column carries no CHECK — plain `TEXT NOT NULL DEFAULT 'local'` — forward-compatible with caller-supplied identity (AUDIT-V4-076). Caller-supplied identity via `X-Operator-Id` is deferred (AUDIT-V4-076, Unscheduled).

### 3.11 — 3.26 Remaining tables

The remaining tables: 13 retain their pre-v4.0 schemas (with the two v4.0 updates below); `instance_lifecycle` and `instance_lifecycle_events` were added in v6.2 and are detailed in §3.25/§3.26. Sections §3.N map to table N of 26; §3.7 covers `paper_trades` and `order_fills` together.

- `decimal_value` columns in these 13 legacy tables retain the `CHECK (value GLOB ...)` constraint pattern (named exception per §1.1) until their next migration.
- Foreign keys are added only where the relationship is genuinely relational (most references are denormalized config identifiers like `policy_id`/`instance_id` strings and are not FKs).

The unchanged tables:

- `position_equity_snapshots` — per-position mark-to-market history.
- `paper_trades` — see §3.7.
- `portfolio_equity_history` — account-level equity snapshot history.
- `decision_profiles`, `profile_indicators`, `risk_profiles`, `saved_edges`, `edge_analytics_cache` — profile management; the `policy_id` field on `strategy_analytics_history` references a config string key, not a relational FK (see §3.5 above).
- `trade_telemetry_history`, `trade_learning_journal` — manual-trade journaling.
- `support_resistance_levels` — cached S/R levels from the `support_resistance` indicator.
- `liquidation_events` — raw liquidation event log (Phase 1 input).
- `performance_matrix_snapshots`, `strategy_analytics_history` — PAE snapshot history.

### 3.25 `instance_lifecycle` — per-instance lifecycle registry (added in v6.2, IL-13)

```sql
CREATE TABLE IF NOT EXISTS instance_lifecycle (
  instance_id         TEXT PRIMARY KEY,
  lifecycle_state     TEXT NOT NULL DEFAULT 'STOPPED'
                      CHECK (lifecycle_state IN ('RUNNING','PAUSED','STOPPING','STOPPED')),
  automation_json     TEXT CHECK (automation_json IS NULL OR json_valid(automation_json)),
  entered_state_at_ms INTEGER NOT NULL,
  deleted_at_ms       INTEGER,
  updated_at_ms       INTEGER NOT NULL
);
```

The column `lifecycle_state` carries the 4-value enum (RUNNING/lifecycle `PAUSED`/STOPPING/STOPPED). DELETED instances are represented by a non-NULL `deleted_at_ms` tombstone and are excluded from all query views. `entered_state_at_ms` drives `after_duration_secs` automation (IL-12). Full contract: [`03-03-06-tae-instance-lifecycle-spec.md §5`](../engines/trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md).

### 3.26 `instance_lifecycle_events` — full transition audit (added in v6.2, IL-13)

```sql
CREATE TABLE IF NOT EXISTS instance_lifecycle_events (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  instance_id  TEXT NOT NULL,
  from_state   TEXT CHECK (from_state IS NULL OR from_state IN ('RUNNING','PAUSED','STOPPING','STOPPED')),
  to_state     TEXT NOT NULL CHECK (to_state IN ('RUNNING','PAUSED','STOPPING','STOPPED','DELETED')),
  actor        TEXT NOT NULL CHECK (actor IN ('operator','automation','system')),
  reason_json  TEXT CHECK (reason_json IS NULL OR json_valid(reason_json)),
  timestamp_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_lifecycle_events_instance_time
  ON instance_lifecycle_events(instance_id, timestamp_ms DESC);
```

Every transition from §2 of the lifecycle spec writes one row. `actor` distinguishes operator commands, automation conditions, and system-internal transitions. `to_state` extends the lifecycle CHECK with `DELETED` (the DELETE endpoint produces tombstone transitions; the row is preserved for audit but excluded from active views).

---

## 4. Live Order Lifecycle (`open_orders` ↔ `order_fills`)

A trade's reconstruction:

1. `open_orders` transitions `PENDING → SUBMITTED → OPEN → PARTIALLY_FILLED → CLOSED` (or `REJECTED` / `CANCELLED`). Each persistent transition is timestamped.
2. For every fill at an exchange, a matching `order_fills` row is inserted with the same `order_id`. Multi-fill orders produce multiple `order_fills` rows, indexed by `fill_index`.
3. When the `open_orders.state` reaches `CLOSED`, the matching `paper_trades` row is reconciled from the `order_fills` rows (entry VWAP, exit VWAP, slippage, fees, funding). The `exit_reason` is the canonical vocabulary (§3.2).

`PRE_DISPATCH` orders (Gate 5 slippage ceiling) sit in **process memory only**. They are not persisted to `open_orders` (the in-memory order is lost on restart). The audit trail lives in `risk_control_events` with `decision = 'HELD_FOR_REVIEW'` and the operator action logged separately when the order is approved or discarded.

---

## 5. Reconstructed Candle Marker

A `market_snapshots` row with `reconstructed = 1` carries a `reconstruction_method`:

- `EXCHANGE_HISTORICAL` — restored from the venue's REST historical API.
- `EXPONENTIAL_MOVING_AVERAGE` — synthesised via the EMA fallback for sub-minute timeframes (≥ 50 history points).
- `LINEAR_EXTRAPOLATION` — synthesised via the linear extrapolation of the last two closes for sub-minute timeframes (2 ≤ N < 50). *(Renamed from `LinearExtrapolation` in v4.0 — the formula projects beyond the last known close, not between two endpoints. See [`08-04-candle-reconstruction.md`](../operations-and-compliance/08-04-candle-reconstruction.md) §Linear Extrapolation.)*
- `UNAVAILABLE` — the reconstructor cannot produce a value (insufficient history). The platform emits a `INSUFFICIENT_DATA` `state_label` for downstream consumers.

---

## 6. WAL & Concurrency

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA temp_store = MEMORY;
```

The engine holds a single writer connection per process. Read concurrency is achieved via `SQLITE_OPEN_FULLMUTEX`. The telemetry DB is **single-node, single-process**; horizontal scaling is not a current feature (see `docs/CHANGELOG.md`).

---

## 7. Encryption

- `exchange_keys.encrypted_*` columns use **AES-256-GCM** with the master key loaded from the `EXCHANGE_SECRET_KEY` environment variable (32 bytes; if absent, the engine panics on startup with a descriptive error).
- `nonce = random 96-bit per row`. AEAD tag is appended to the ciphertext column.
- Key rotation: rotate the master key — both the `last_rotated_at` column and a fresh `nonce` per row make re-encryption safe.

---

## 8. Backups & Replication

`telemetry.db` is a single SQLite file. Operators back it up with `sqlite3 telemetry.db ".backup telemetry-$(date +%Y%m%d-%H%M%S).db"` (the `.backup` command is online-safe). For multi-host setups, operators replicate via `rsync` or `rclone` snapshots of the file plus its `WAL` companion (`telemetry.db-wal`) — see [`01-02-global-architecture.md §4.4`](../conceptual-foundations/01-02-global-architecture.md).

---

## 9. Migration Strategy

Migrations live in `crates/database-storage/migrations/ (sqlx::migrate! consumes at build-time)`. The schema-version compatibility window is `user_version = N` where `N` is the most-recent migration applied. The engine refuses to start if `user_version` is **lower** than the minimum required version (no forward-only compatibility — re-run the migrations). Backward compatibility (newer code reading older `user_version`) is supported up to two minor versions.

The canonical v4.0 migration set adds eight changes:

1. **`open_orders` state vocabulary unification** — replaces any prior state literals with the canonical lifecycle from `03-03-03-tae-layer2-execution.md §4`.
2. **`risk_control_events` new table** — populated retroactively from any prior in-memory audit log; if absent, history before v4.0 is uncovered (operator-visible notice on first launch).
3. **`order_fills` activation** — added (live); the PAE contract is upgraded to per-fill attribution.
4. **`market_snapshots` partial-persistence scope** — schema unchanged; column-level persistence scope documented in §3.1 (the wire contract remains authoritative).
5. **`connection_quality_samples` instance + timeframe scope** — adds `timeframe_secs` and reindexes `idx_cq_pair_window_time` → `idx_cq_pair_timeframe_window_time`. Existing rows from earlier versions may lack the new `timeframe_secs` column and require the migration to default to `60` (micro).
6. **`liquidity_signals_json` always-present policy** — `DEFAULT '[]' CHECK (json_valid(...))`; migration backfills `'[]'` for any prior `NULL` or absent row.
7. **`funding_rate_8h` nullable semantics** — column type changes from `TEXT NOT NULL '0'` to `TEXT` (nullable) with `CHECK (value IS NULL OR value GLOB ...)`. Existing `0` literals remain `0` (= explicit-disable); missing values become `NULL` (= inherit global). This column is a named exception to the retired per-column GLOB CHECKs (§1.1) and retains its GLOB CHECK until its next migration. See [`06-01 §2.5 / §2.6`](06-01-api-gateway-contract.md).
8. **`open_orders.is_emergency_liquidation`** — adds `open_orders.is_emergency_liquidation` (active tables unchanged at 26).

---

## 10. Cross-References

- [`02-07-metrics-matrix.md §2.1`](../matrices/02-07-metrics-matrix.md) — canonical `MarketSnapshot` wire contract; top-level liquidity fields.
- [`02-08-opportunity-matrix.md §2.1`](../matrices/02-08-opportunity-matrix.md) — `invalidation_level` canonical name; migration from `invalidation_level` and `final_invalidation`.
- [`03-03-03-tae-layer2-execution.md §4`](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md) — order-state lifecycle; `PRE_DISPATCH` semantics.
- [`03-04-05-pme-layer4-portfolio.md §3`](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — safety-state machine and reconstruction from persisted columns.
- [`03-05-02-pae-layer1-trade-analytics.md §3`](../engines/performance-analytics-engine/03-05-02-pae-layer1-trade-analytics.md) — per-fill reconstruction contract.
- [`06-01-api-gateway-contract.md §2.10`](06-01-api-gateway-contract.md) — `POST /api/keys` encrypted-credential contract.
- [`08-02-pre-trade-risk-controls.md`](../operations-and-compliance/08-02-pre-trade-risk-controls.md) — gate ordering and `risk_control_events` provenance.
- [`08-04-candle-reconstruction.md`](../operations-and-compliance/08-04-candle-reconstruction.md) — reconstruction methods.
- [`08-05-connection-quality.md`](../operations-and-compliance/08-05-connection-quality.md) — `connection_quality_samples` data model.
