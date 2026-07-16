# Database Schema Specification

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.

**Status:** Approved
**Purpose:** This document specifies the SQLite database schema — all persistent tables, indexes, WAL configuration, and migration strategy for the Trading Platform's shared telemetry store.

**Active tables (24):** `market_snapshots`, `open_orders`, `user_trades`, `paper_balances`, `active_positions`, `position_slots`, `position_equity_snapshots`, `paper_trades`, `exchange_keys`, `decision_profiles`, `profile_indicators`, `risk_profiles`, `portfolio_equity_history`, `trade_telemetry_history`, `trade_learning_journal`, `saved_edges`, `edge_analytics_cache`, `support_resistance_levels`, `connection_quality_samples`, `liquidation_events`, `performance_matrix_snapshots`, `strategy_analytics_history`, **`order_fills`** (B-6 — activated in v4.0), **`risk_control_events`** (B-5 — added in v4.0).

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
| `Decimal` (price, size, capital) | `TEXT NOT NULL CHECK (value GLOB '[+-]?[0-9]*([.][0-9]*)?')` — serializes via `rust_decimal::Decimal::to_string()` |
| Nullable value (optional Decimal) | `TEXT CHECK (value IS NULL OR value GLOB '[+-]?[0-9]*([.][0-9]*)?')` — distinguishes `NULL` (inherit global) from the canonical pattern (e.g. `"0"` = disable) |
| Boolean | `INTEGER NOT NULL CHECK (value IN (0, 1))` |
| JSON value | `TEXT NOT NULL CHECK (json_valid(value))` |
| JSON array | `TEXT NOT NULL CHECK (json_valid(value))` (always JSON, even for empty — the platform distinguishes JSON-empty from key-omitted via `json_valid` + presence) |
| JSON value (nullable) | `TEXT` (no CHECK) |
| String (enum union) | `TEXT NOT NULL CHECK (value IN (...))` |
| Foreign key | `INTEGER REFERENCES other_table(id) ON DELETE …` (only on the few FKs that exist — most references are denormalized config identifiers) |

All `Decimal` fields are serialized via `rust_decimal::Decimal::to_string()` and deserialized via `Decimal::from_str()`. The wire-format docs in [`02-07-metrics-matrix.md`](../matrices/02-07-metrics-matrix.md), [`02-09-overview-matrix.md`](../matrices/02-09-overview-matrix.md), and [`06-01-api-gateway-contract.md §2.10`](06-01-api-gateway-contract.md) mirror the same convention.

### 1.2 Schema-version invariant

Every table includes a `_schema_version` row in its body documentation. Migrations bump this column when the schema for that table changes. The engine refuses to start if the database `user_version` does not match the schema-version compatibility window — see §11.

---

## 2. Indexes & Query Performance

Indexes are created on each table for the query patterns the engine actually uses:

| Index | Columns | Use |
|---|---|---|
| `idx_market_snapshots_pair_time` | `(pair_key, timeframe_secs, timestamp DESC)` | Replay history fetch |
| `idx_market_snapshots_completed` | `(pair_key, timeframe_secs, timestamp DESC) WHERE is_completed = 1` | MME pipeline (only completed snapshots) |
| `idx_open_orders_state` | `(state, instance_id, timestamp)` | Live order lifecycle queries |
| `idx_position_slots_position_slot` | `(position_id, slot_index)` | Scaled Entry reconstruction |
| `idx_exchange_keys_exchange` | `(exchange)` | Key lookup by venue |
| `idx_risk_control_events_pair_time` | `(instance_id, gate_id, timestamp_ms DESC)` | Gate-rejection audit dashboards |
| `idx_risk_control_events_operator` | `(operator_id, timestamp_ms DESC)` | Override-history audit (`operator_id = "local_operator"`) |
| `idx_order_fills_trade` | `(trade_id)` | Per-fill PAE reconstruction |
| `idx_order_fills_order` | `(order_id)` | Per-order fill chain |
| `idx_connection_quality_samples_pair_window_time` | `(pair_key, window, timestamp_ms DESC)` | Connection-quality queries |

---

## 3. Active Table Catalog

Tables are grouped by ownership. Each entry shows the canonical schema (DDL-style), invariants, and migration notes. The `id` column is `INTEGER PRIMARY KEY AUTOINCREMENT` unless explicitly noted.

### 3.1 `market_snapshots` — MME telemetry persistence (DIE ownership)

The primary time-series table — one row per completed candle, paired with the rolled-up MME matrix outputs that ride the WS `MarketSnapshot`.

```sql
CREATE TABLE IF NOT EXISTS market_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key TEXT NOT NULL,
    timeframe_secs INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    is_completed INTEGER NOT NULL DEFAULT 1,
    exchange TEXT NOT NULL,
    mid_price TEXT NOT NULL CHECK (mid_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    bid_price TEXT NOT NULL CHECK (bid_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    ask_price TEXT NOT NULL CHECK (ask_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    bid_size TEXT NOT NULL CHECK (bid_size GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    ask_size TEXT NOT NULL CHECK (ask_size GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    funding_rate TEXT CHECK (funding_rate GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    open TEXT NOT NULL CHECK (open GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    high TEXT NOT NULL CHECK (high GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    low TEXT NOT NULL CHECK (low GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    close TEXT NOT NULL CHECK (close GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    volume TEXT NOT NULL CHECK (volume GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    average_volume TEXT CHECK (average_volume GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    open_interest TEXT CHECK (open_interest GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    oi_delta_1h TEXT CHECK (oi_delta_1h GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    prev_day_px TEXT CHECK (prev_day_px GLOB '[+-]?[0-9]*([.][0-9]*)?'),
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
    decision_context_json TEXT CHECK (decision_context_json IS NULL OR json_valid(decision_context_json)),
    context_json TEXT CHECK (context_json IS NULL OR json_valid(context_json)),
    statistical_context_json TEXT CHECK (statistical_context_json IS NULL OR json_valid(statistical_context_json)),
    risk_profile_json TEXT CHECK (risk_profile_json IS NULL OR json_valid(risk_profile_json))
);
CREATE INDEX IF NOT EXISTS idx_market_snapshots_pair_time ON market_snapshots(pair_key, timeframe_secs, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_market_snapshots_completed ON market_snapshots(pair_key, timeframe_secs, timestamp DESC) WHERE is_completed = 1;
CREATE INDEX IF NOT EXISTS idx_market_snapshots_reconstructed ON market_snapshots(pair_key, timeframe_secs, reconstruction_method) WHERE reconstructed = 1;
```

**Persistence rule.** `market_snapshots` stores the **core candle + MME matrix fields** that are needed for replay and historical backtesting. Fields that are local to the live WS broadcast envelope (e.g. live shadow values that flicker until the next completed candle, the §A.7 `cascade_risk_index` placeholder) are **not persisted** to keep the table tight. The canonical wire contract is in [`02-07-metrics-matrix.md §2.1`](../matrices/02-07-metrics-matrix.md); live-only fields are computed on the wire and recomputed by the MME during replay. The wire and persistent contracts are deliberately scoped — see `docs/CHANGELOG.md` for the `AUDIT-V4-042` resolution.

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
    requested_size TEXT NOT NULL CHECK (requested_size GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    filled_size TEXT NOT NULL DEFAULT '0' CHECK (filled_size GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    entry_price TEXT CHECK (entry_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    stop_loss_price TEXT CHECK (stop_loss_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    take_profit_price TEXT CHECK (take_profit_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    invalidation_level TEXT CHECK (invalidation_level GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    is_reduce_only INTEGER NOT NULL DEFAULT 0,
    slippage_bps INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    acknowledged_at INTEGER,
    filled_at INTEGER,
    close_reason TEXT CHECK (close_reason IS NULL OR close_reason IN ('STOP_LOSS','TAKE_PROFIT','SIGNAL_EXIT','MANUAL','VETO','TIMEOUT','EMERGENCY_LIQUIDATION'))
);
CREATE INDEX IF NOT EXISTS idx_open_orders_state ON open_orders(state, instance_id, timestamp);
```

**Per `03-03-03-tae-layer2-execution.md §4`:** `PRE_DISPATCH` orders are held in process memory only and are **never** persisted to `open_orders`. The `risk_control_events` table (§3.10) is the persistent audit trail for every held order; the `/api/pre-dispatch/*` resource ([`06-01 §2.9`](06-01-api-gateway-contract.md)) is the operator surface.

The `close_reason` vocabulary is canonical: `STOP_LOSS`, `TAKE_PROFIT`, `SIGNAL_EXIT`, `MANUAL`, `VETO`, `TIMEOUT`, `EMERGENCY_LIQUIDATION`. The PAE contract consumes this exact enum without aliasing.

### 3.3 `user_trades` — operator-created manual trades

```sql
CREATE TABLE IF NOT EXISTS user_trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('LONG', 'SHORT')),
    outcome TEXT NOT NULL CHECK (outcome IN ('WIN', 'LOSS', 'BREAKEVEN', 'OPEN')),
    risk_multiplier TEXT NOT NULL CHECK (risk_multiplier GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    reward_multiplier TEXT NOT NULL CHECK (reward_multiplier GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    opened_at INTEGER NOT NULL,
    closed_at INTEGER,
    notes TEXT
);
```

### 3.4 `paper_balances` — persistent per-instance paper balance

```sql
CREATE TABLE IF NOT EXISTS paper_balances (
    instance_id TEXT PRIMARY KEY,
    balance TEXT NOT NULL CHECK (balance GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    initial_balance TEXT NOT NULL CHECK (initial_balance GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    starting_session_equity TEXT NOT NULL CHECK (starting_session_equity GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    peak_equity TEXT NOT NULL CHECK (peak_equity GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    cooldown_start_ms INTEGER,
    active_stance TEXT NOT NULL DEFAULT 'ACTIVE' CHECK (active_stance IN ('ACTIVE','CLOSE_ONLY','AVOID','SUSPENDED')),
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
    entry_price TEXT NOT NULL CHECK (entry_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    average_entry_price TEXT NOT NULL CHECK (average_entry_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    position_size TEXT NOT NULL CHECK (position_size GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    invalidation_level TEXT CHECK (invalidation_level GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    stop_loss_price TEXT CHECK (stop_loss_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    take_profit_price TEXT CHECK (take_profit_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    current_price TEXT CHECK (current_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    unrealized_pnl TEXT CHECK (unrealized_pnl GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    roi_pct TEXT NOT NULL CHECK (roi_pct GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    opened_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

The `invalidation_level` field is canonical across L4 Opportunity Matrix, L6 Decision Matrix, and this Position Matrix. `roi_pct` is the canonical field; the legacy `roi_percentage` is deprecated and removed at v5.0 (see [`06-01-api-gateway-contract.md §2.7`](06-01-api-gateway-contract.md)).

### 3.6 `position_slots` — scaled-entry reconciliation

```sql
CREATE TABLE IF NOT EXISTS position_slots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL REFERENCES active_positions(id) ON DELETE CASCADE,
    slot_index INTEGER NOT NULL,
    size TEXT NOT NULL CHECK (size GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    price TEXT NOT NULL CHECK (price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
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
    gross_pnl TEXT NOT NULL CHECK (gross_pnl GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    net_pnl TEXT NOT NULL CHECK (net_pnl GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    roi_pct TEXT NOT NULL CHECK (roi_pct GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    open_time INTEGER NOT NULL,
    close_time INTEGER NOT NULL,
    MFE TEXT NOT NULL CHECK (MFE GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    MAE TEXT NOT NULL CHECK (MAE GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    entry_vwap TEXT NOT NULL CHECK (entry_vwap GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    exit_vwap TEXT NOT NULL CHECK (exit_vwap GLOB '[+-]?[0-9]*([.][0-9]*)?'),
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
    filled_size TEXT NOT NULL CHECK (filled_size GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    fill_price TEXT NOT NULL CHECK (fill_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    target_price TEXT CHECK (target_price GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    fill_slippage_bps INTEGER NOT NULL DEFAULT 0,
    fee_currency TEXT NOT NULL CHECK (fee_currency IN ('MAKER', 'TAKER')),
    fee_paid TEXT NOT NULL CHECK (fee_paid GLOB '[+-]?[0-9]*([.][0-9]*)?'),
    funding_accrued TEXT NOT NULL DEFAULT '0' CHECK (funding_accrued GLOB '[+-]?[0-9]*([.][0-9]*)?'),
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

Instance-scoped in v4.0. See the §3.10 audit-v4 migration for the column-pair-key:

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

### 3.10 `risk_control_events` — gate-rejection and override audit (new in v4.0)

Every pre-trade gate failure (Gates 1–7) and every operator override is logged with the local-operator identity, gate id, decision, prior state, resulting state, and a retention timestamp:

```sql
CREATE TABLE IF NOT EXISTS risk_control_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    policy_id TEXT,
    instance_id TEXT NOT NULL,
    gate_id INTEGER NOT NULL,
    decision TEXT NOT NULL CHECK (decision IN ('BLOCK', 'HELD_FOR_REVIEW', 'MODIFIED_AND_CONTINUED', 'CLIP_AND_CONTINUE', 'OVERRIDE')),
    reason TEXT NOT NULL,
    requested_disposition TEXT NOT NULL,
    operator_id TEXT NOT NULL DEFAULT 'local_operator' CHECK (operator_id IN ('local_operator', 'anonymous')),
    prior_state TEXT,
    resulting_state TEXT,
    pre_dispatch_order_id TEXT,
    timestamp_ms INTEGER NOT NULL,
    retention_until_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_rce_instance_gate_time ON risk_control_events(instance_id, gate_id, timestamp_ms DESC);
CREATE INDEX IF NOT EXISTS idx_rce_operator_time ON risk_control_events(operator_id, timestamp_ms DESC);
```

`operator_id = 'local_operator'` is the fixed identity in v4.0 (per the local-only authentication model in [`06-01 §1`](06-01-api-gateway-contract.md)); `'anonymous'` is reserved for cases where the API layer forwards without an explicit identity (not currently surfaced). Caller-supplied identity via `X-Operator-Id` is on the v5.0 roadmap.

### 3.11 — 3.24 Remaining tables

The remaining 14 tables retain their pre-v4.0 schemas with two v4.0 updates that apply consistently across the corpus:

- `decimal_value` columns use the same `CHECK (value GLOB ...)` constraint pattern as the canonical schema.
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
- `LINEAR_EXTRAPOLATION` — synthesised via the linear extrapolation of the last two closes for sub-minute timeframes (2 ≤ N < 50). *(Renamed from `LinearInterpolation` in v4.0 — the formula projects beyond the last known close, not between two endpoints. See [`08-04-candle-reconstruction.md`](../operations-and-compliance/08-04-candle-reconstruction.md) §Linear Extrapolation.)*
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

The engine holds a single writer connection per process. Read concurrency is achieved via `SQLITE_OPEN_FULLMUTEX`. The telemetry DB is **single-node, single-process**; horizontal scaling is not a v4.0 feature (see `docs/CHANGELOG.md`).

---

## 7. Encryption

- `exchange_keys.encrypted_*` columns use **AES-256-GCM** with the master key loaded from the `EXCHANGE_SECRET_KEY` environment variable (32 bytes; if absent, the engine panics on startup with a descriptive error).
- `nonce = random 96-bit per row`. AEAD tag is appended to the ciphertext column.
- Key rotation: rotate the master key by deploying `crypto_kms_rotate` (or equivalent) — both the `last_rotated_at` column and a fresh `nonce` per row make re-encryption safe.

---

## 8. Backups & Replication

`telemetry.db` is a single SQLite file. Operators back it up with `sqlite3 telemetry.db ".backup telemetry-$(date +%Y%m%d-%H%M%S).db"` (the `.backup` command is online-safe). For multi-host setups, operators replicate via `rsync` or `rclone` snapshots of the file plus its `WAL` companion (`telemetry.db-wal`) — see [`01-02-global-architecture.md §4.4`](../conceptual-foundations/01-02-global-architecture.md).

---

## 9. Migration Strategy

Migrations live in `crates/database-storage/migrations/ (sqlx::migrate! consumes at build-time)`. The schema-version compatibility window is `user_version = N` where `N` is the most-recent migration applied. The engine refuses to start if `user_version` is **lower** than the minimum required version (no forward-only compatibility — re-run the migrations). Backward compatibility (newer code reading older `user_version`) is supported up to two minor versions.

The canonical v4.0 migration set adds three changes:

1. **`open_orders` state vocabulary unification** — replaces any prior state literals with the canonical lifecycle from `03-03-03-tae-layer2-execution.md §4`.
2. **`risk_control_events` new table** — populated retroactively from any prior in-memory audit log; if absent, history before v4.0 is uncovered (operator-visible notice on first launch).
3. **`order_fills` activation** — added (live); the PAE contract is upgraded to per-fill attribution.
4. **`market_snapshots` partial-persistence scope** — schema unchanged; column-level persistence scope documented in §3.1 (the wire contract remains authoritative).
5. **`connection_quality_samples` instance + timeframe scope** — adds `timeframe_secs` and reindexes `idx_cq_pair_window_time` → `idx_cq_pair_timeframe_window_time`. Existing rows from earlier versions may lack the new `timeframe_secs` column and require the migration to default to `60` (micro).
6. **`liquidity_signals_json` always-present policy** — `DEFAULT '[]' CHECK (json_valid(...))`; migration backfills `'[]'` for any prior `NULL` or absent row.
7. **`funding_rate_8h` nullable semantics** — column type changes from `TEXT NOT NULL '0'` to `TEXT` (nullable) with `CHECK (value IS NULL OR value GLOB ...)`. Existing `0` literals remain `0` (= explicit-disable); missing values become `NULL` (= inherit global). See [`06-01 §2.5 / §2.6`](06-01-api-gateway-contract.md).

---

## 10. Cross-References

- [`02-07-metrics-matrix.md §2.1`](../matrices/02-07-metrics-matrix.md) — canonical `MarketSnapshot` wire contract; top-level liquidity fields.
- [`02-08-opportunity-matrix.md §2.1`](../matrices/02-08-opportunity-matrix.md) — `invalidation_level` canonical name; migration from `invalid_level` and `final_invalidation_level`.
- [`03-03-03-tae-layer2-execution.md §4`](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md) — order-state lifecycle; `PRE_DISPATCH` semantics.
- [`03-04-05-pme-layer4-portfolio.md §3`](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — safety-state machine and reconstruction from persisted columns.
- [`03-05-02-pae-layer1-trade-analytics.md §3`](../engines/performance-analytics-engine/03-05-02-pae-layer1-trade-analytics.md) — per-fill reconstruction contract.
- [`06-01-api-gateway-contract.md §2.10`](06-01-api-gateway-contract.md) — `POST /api/keys` encrypted-credential contract.
- [`08-02-pre-trade-risk-controls.md`](../operations-and-compliance/08-02-pre-trade-risk-controls.md) — gate ordering and `risk_control_events` provenance.
- [`08-04-candle-reconstruction.md`](../operations-and-compliance/08-04-candle-reconstruction.md) — reconstruction methods.
- [`08-05-connection-quality.md`](../operations-and-compliance/08-05-connection-quality.md) — `connection_quality_samples` data model.
