# API Gateway Contract

**Version:** 5.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Approved
**Purpose:** This document specifies the complete REST and WebSocket API surface of the Trading Platform — routes, request/response payloads, JSON-RPC 2.0 conventions, HTTP status codes, error envelope, and serialization rules.

---

## 1. Transport & Addressing

| Property | Value |
|----------|-------|
| Framework | Axum (Rust) on a Tokio runtime |
| Base URL | `http://127.0.0.1:3000` (localhost only) |
| Authentication | **Local-operator identity model.** Single-user deployments identify every override/audit event as `operator_id = "local_operator"` (fixed identity). Caller-supplied identity via `X-Operator-Id` header is on the v5.0 roadmap. There is no per-route authentication in v4.0. The `local_operator` identity is recorded in the `risk_control_events.operator_id` column (see [`06-02-database-schema-spec.md §3.10`](06-02-database-schema-spec.md)), the WebSocket control frame `operator_id` field, and the UI audit display. |
| Static assets | `ui/dist/` served via `tower_http::services::ServeDir` |

### 1.1 HTTP status codes and error envelope

Every response uses one of the following status codes:

| Code | Meaning |
|---|---|
| `200 OK` | Successful read or idempotent mutation |
| `201 Created` | New resource persisted (rare in v4.0) |
| `204 No Content` | Successful mutation, no body |
| `400 Bad Request` | JSON parse error, missing required field, malformed value |
| `404 Not Found` | Unknown resource (unknown `instance_id`, unknown pair_key, etc.) |
| `409 Conflict` | State conflict (e.g. duplicate symbol, paused instance cannot be reconfigured) |
| `422 Unprocessable Entity` | Semantic validation failure (e.g. operator override while veto condition still active) |
| `500 Internal Server Error` | Unhandled server-side failure |
| `503 Service Unavailable` | Engine not yet initialized, exchange connectivity down |

Every error response carries the stable JSON envelope:

```json
{
  "error": {
    "code": "INSTANCE_NOT_FOUND",
    "message": "No instance exists for id 42",
    "details": {},
    "request_id": "9c1f-…-…",
    "documentation_url": "/api/docs/errors/INSTANCE_NOT_FOUND"
  }
}
```

`code` is a stable SCREAMING_SNAKE_CASE identifier — clients should branch on `code`, not on `message`. `details` is a per-error object (e.g. `{ "field": "limit", "value": 5000, "max": 1000 }`). `request_id` is the same UUID logged server-side; quote it in support requests.

### 1.2 WebSocket error frames

WebSocket close codes follow the engine protocol; the engine never sends an error JSON-RPC frame on `/ws` (only notifications — see §3). When the underlying WebSocket cannot complete its handshake (`/ws` upgraded returns an error), the HTTP body uses the same JSON error envelope as §1.1.

---

## 2. REST API Reference

> **Per-instance matrices via WebSocket only.** The Decision Matrix, Analysis Matrix, Opportunity Matrix, Risk Matrix, and other per-Market-Instance MME outputs are delivered exclusively via the WebSocket envelope (`/ws`) — there is no per-matrix REST endpoint, because these matrices update on every completed candle and a polling REST surface would stale. Use `/ws?symbol=…&timeframe_secs=…` for live access; use `/api/history?symbol=…&timeframe_secs=…` for replay. Global aggregations (Overview Matrix) are also WebSocket-only.

### 2.1 Session Management

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `GET` | `/api/session/status` | — | `{ session_active: bool, currency: string, exchange: string, instance_count: u32 }` |
| `POST` | `/api/session/init` | `{ exchange: string, currency: string }` | Session status |
| `POST` | `/api/session/quit` | — | `204 No Content` → cleans all instances |

### 2.2 Configuration

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `GET` | `/api/config` | — | Full `AppConfig` (symbols, candles, indicators, instances, indicator_registry). |
| `POST` | `/api/config` | `AppConfig` JSON | `200 OK` with the persisted payload. Returns `400 Bad Request` on parse failure, `409 Conflict` on schema-version mismatch. |
| `GET` | `/api/rules` | — | `{ content: string }` (reads `docs/engines/market-monitoring-engine/03-02-09-mme-indicators-guide.md`). |
| `POST` | `/api/rules` | `{ content: string }` | Writes indicator guide. |

### 2.3 History & Monitor

| Method | Path | Params | Response |
|--------|------|--------|----------|
| `GET` | `/api/history` | `symbol`, `timeframe_secs`, `limit` (default `100`, max `1000`) | `{ symbol, prices[], candles[], indicator_histories }` |
| `GET` | `/api/monitor` | `symbol` | Multi-TF meta-intelligence (per-TF regime, MTF agreement matrix, MarketContext). |
| `GET` | `/api/connection-quality` | `instance_id` (required), `timeframe_secs` (required), `window=one_hour\|six_hour\|twenty_four_hour` (default `one_hour`) | `ConnectionQualityReport` JSON (`window`, `window_start_ms`, `window_end_ms`, `uptime_pct`, `disconnect_count`, `avg_reconnect_ms`, `total_data_loss_secs`, `reconstructed_candles`, `score`). The `instance_id` and `timeframe_secs` query parameters are **required** (the scope is `instance_id × timeframe_secs`, one WebSocket connection per `TimeframePipeline`; no process-wide aggregate). Returns `400 Bad Request` if either query parameter is missing. See [`08-05-connection-quality.md §REST API`](../operations-and-compliance/08-05-connection-quality.md) for the full schema and behaviour. |

### 2.4 Instances (Workspaces)

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/instances?pair_key=` | List running instance summaries. |
| `POST` | `/api/instances` | Create instance (`{ symbol: string }` — unified internal symbol, e.g. `"BTC-USDT"`). |
| `GET` | `/api/instances/:id` | Instance detail (equity, caution). |
| `DELETE` | `/api/instances/:id` | Delete instance. |
| `DELETE` | `/api/instances/by-pair/:pair_key` | Delete by pair key. |
| `POST` | `/api/instances/:id/config` | Reconfigure (`InstanceConfigPayload`) → recharge pipeline. |
| `POST` | `/api/instances/:id/pause` | Pause event loop. |
| `POST` | `/api/instances/:id/stop` | Stop instance. |
| `POST` | `/api/instances/:id/safety/reset` | Reset the per-symbol `consecutive_losses` counter (clears `consecutive_losses[sym]`; does **not** release a drawdown or systemic veto — see `/safety/release-veto` below). |
| `POST` | `/api/instances/:id/safety/release-veto` | **Release a hard drawdown / systemic veto.** The endpoint checks that the underlying veto condition (drawdown below threshold *and* `systemic_risk_score < systemic_risk_threshold`) has cleared, then restores the operator-configured default stances and clears the operator one-time-acknowledge flag. Returns `422 Unprocessable Entity` if the veto condition is still active, `200 OK` on success. Distinct from `/safety/reset` (which only clears the consecutive-loss counter). The `operator_id` from the local-operator model is recorded in the resulting `risk_control_events` row. |
| `POST` | `/api/instances/:id/manual/open` | Log manual position open. Request: `InstanceManualRequest { action: string (required), direction: Option<string> ("LONG"\|"SHORT"), price: Option<f64>, pre_dispatch_order_id: Option<string> }`. The optional `pre_dispatch_order_id` references a held order in `PRE_DISPATCH` from Gate 5; if present, the manual action approves the held order rather than opening a separate position. |
| `POST` | `/api/instances/:id/manual/close` | Log manual position close. Same `InstanceManualRequest` shape. |
| `POST` | `/api/instances/:id/intervals` | Set trigger loop intervals (`{ slow_seconds, normal_seconds, fast_seconds }`). |
| `POST` | `/api/orders/:id/override-readiness` | **Gate 2 override** — clears a `STAND_ASIDE` decision for a specific held order. The override is logged with `operator_id = "local_operator"`. Returns `422 Unprocessable Entity` if the order is not currently `HELD_FOR_REVIEW`. |

### 2.5 Decision & Risk Profiles

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/decision-profiles` | List profiles with indicators. |
| `POST` | `/api/decision-profiles` | Create (`{ profile_name, long_threshold, short_threshold }`). |
| `DELETE` | `/api/decision-profiles/:id` | Delete (CASCADE indicators). |
| `POST` | `/api/decision-profiles/:id` | Update thresholds. |
| `POST` | `/api/decision-profiles/:id/evaluate` | Evaluate snapshot → `DecisionScore`. |
| `POST` | `/api/decision-profiles/:id/indicators` | Add indicator rule (`{ indicator_name, weight, override_status }`). |
| `POST` | `/api/decision-profiles/:id/indicators/:iid` | Update indicator rule. |
| `DELETE` | `/api/decision-profiles/:id/indicators/:iid` | Delete indicator rule. |
| `GET` | `/api/risk-profiles` | List risk profiles. |
| `POST` | `/api/risk-profiles` | Create (`{ profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread }`). `funding_rate_8h` is nullable: `null` inherits the global config value; the string `"0"` disables funding accrual; a non-zero string sets an explicit per-profile override. |
| `DELETE` | `/api/risk-profiles/:id` | Delete. |
| `POST` | `/api/risk-profiles/:id` | Update fields with the same nullable `funding_rate_8h` semantics. |

### 2.6 Risk Calculation & Commission

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `POST` | `/api/risk/calculate` | `RiskCalculationInput` (see schema below) | `RiskCalculation` (see schema below) |
| `GET` | `/api/risk/fee-table` | `order_type`, `capitals[]`, `leverages[]` | Fee table. |
| `POST` | `/api/risk/commission-projection` | `CommissionProjectionPayload` | Full dual-entry fee/sizing projection. |

#### 2.6.1 `RiskCalculationInput` schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `capital` | `Decimal` (string) | yes | Total account capital available for sizing. |
| `max_risk_pct` | `Decimal` (string) | yes | Per-trade risk as a raw percentage float (e.g. `1` = 1%). |
| `leverage` | `i32` | yes | Maximum cross leverage. |
| `direction` | `string` | yes | `"LONG"` or `"SHORT"`. Determines stop/target relation to entry. |
| `entry_price` | `Decimal` (string) | yes | Order entry reference price. |
| `stop_loss_price` | `Decimal` (string) | yes | Planned stop-loss price. Must be `< entry_price` for LONG, `> entry_price` for SHORT. |
| `take_profit_price` | `Decimal` (string) | yes | Planned take-profit price. Must be `> entry_price` for LONG, `< entry_price` for SHORT. |
| `commission_pct` | `Decimal` (string) | yes | Commission as a raw percentage float (e.g. `0.06` = 0.06%). |
| `funding_rate_8h` | `Decimal` (string) or `null` | yes | 8-hour funding rate as a raw percentage float. `null` inherits the global config value; the string `"0"` disables funding. |
| `spread` | `Decimal` (string) | yes | Round-trip spread cost (quote currency, per unit). |
| `atr_value` | `Decimal` (string) | no | Optional ATR for dynamic stop / target sizing. |
| `atr_multiplier` | `Decimal` (string) | no | ATR multiplier when `atr_value` is provided. |
| `atr_target_rr` | `Decimal` (string) | no | Target reward/risk ratio when `atr_value` is provided. |
| `use_dynamic_atr` | `bool` | no | `true` to compute the stop and target from ATR instead of the explicit prices. |
| `min_tick_size` | `Decimal` (string) | no | Minimum order size increment (base asset units). Position size is quantized to this tick. |

> Schema authoritative: `crates/api-gateway/src/types.rs::RiskCalculationPayload` and `crates/portfolio-supervisor/src/risk_calculator.rs::RiskCalculationInput`. The runtime casts the `Decimal` fields from strings at the wire boundary.

#### 2.6.2 `RiskCalculation` schema

| Field | Type | Description |
|-------|------|-------------|
| `risk_capital` | `Decimal` (string) | `capital × max_risk_pct / 100`. |
| `price_distance` | `Decimal` (string) | `|entry_price − stop_loss_price|`. |
| `position_size_units` | `Decimal` (string) | `risk_capital / price_distance`, optionally quantized to `min_tick_size`. |
| `position_notional` | `Decimal` (string) | `position_size_units × entry_price`. |
| `leverage_required` | `Decimal` (string) | `position_notional / capital`. |
| `leverage_selected` | `i32` | Echoed from input. |
| `margin_required` | `Decimal` (string) | `position_notional / leverage_selected` (when leverage > 0). |
| `liquidation_price` | `Decimal` (string) | Direction-adjusted (LONG: `entry − entry / leverage`; SHORT: `entry + entry / leverage`). |
| `risk_reward_ratio` | `Decimal?` (string) | `profit_distance × size / risk_capital`. Null when `risk_capital = 0`. |
| `estimated_profit` | `Decimal` (string) | Direction-adjusted (`LONG: (TP − entry) × size`; `SHORT: (entry − TP) × size`). |
| `total_fees` | `Decimal` (string) | `(commission_pct/100) × notional × 2 + (funding_rate_8h/100) × notional + spread`. |
| `net_pnl` | `Decimal` (string) | `estimated_profit − total_fees`. |

#### 2.6.3 `CommissionProjectionPayload` schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `capital` | `Decimal` (string) | yes | Total account capital. |
| `max_risk_pct` | `Decimal` (string) | yes | Per-trade risk percentage. |
| `leverage` | `i32` | yes | Maximum leverage. |
| `direction` | `string` | yes | `"LONG"` or `"SHORT"`. |
| `entry_price` | `Decimal` (string) | yes | Entry price. |
| `stop_loss_price` | `Decimal` (string) | yes | Stop-loss price. |
| `take_profit_price` | `Decimal` (string) | yes | Take-profit price. |
| `commission_pct` | `Decimal` (string) | yes | Commission rate. |
| `funding_rate_8h` | `Decimal` (string) | yes | Funding rate per 8 hours. |
| `spread` | `Decimal` (string) | yes | Spread cost. |
| `hold_hours` | `Decimal` (string) | no | Anticipated position hold time (default `8`). Used for funding accrual projection. |

### 2.7 Trades & Journal

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/trades` | Last 100 user trades. |
| `POST` | `/api/trades` | Create trade (`{ symbol, direction, outcome, risk_multiplier, reward_multiplier }`). |
| `GET` | `/api/trade-ledger?limit=` | Telemetry history. |
| `GET` | `/api/trade-journal?limit=` | Journal entries (JOINed). |
| `POST` | `/api/trade-journal/:id/notes` | Update journal (`{ human_notes, execution_score }`). |
| `GET` | `/api/trade-journal/export/csv` | CSV export (1000 records). All per-trade metrics use `roi_pct` (the canonical name; `roi_percentage` is a deprecated alias scheduled for removal at v5.0). |
| `GET` | `/api/trade-journal/export/json` | JSON export (1000 records). Same canonical `roi_pct` field. |
| `POST` | `/api/trades/telemetry` | Create telemetry history entry. |

### 2.8 Dashboard & System

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/api/dashboard/stats?initial_capital=` | `DashboardStats` (20+ stat categories). |
| `GET` | `/api/system/status` | `{ connected, latency_ms, journal_mode, active_pairs_count }`. |
| `GET` | `/api/system/observability?symbol=` | `{ recent_decisions[], completed_trades[] }`. |

### 2.9 Pre-dispatch Approval (Gate 5)

> `PRE_DISPATCH` orders are held in process memory only by the TAE Execution Layer. The HTTP resource below provides the durable audit-trail surface that is missing in earlier versions. The `risk_control_events` table (see [`06-02-database-schema-spec.md §3.10`](06-02-database-schema-spec.md)) is the persistent record of every gate-rejection and every pre-dispatch event.

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `GET` | `/api/pre-dispatch?instance_id=` | — | `{ items: PreDispatchOrder[] }` — every `PRE_DISPATCH` order matching the instance scope (or all instances if `instance_id` is omitted). `PreDispatchOrder { order_id, instance_id, pair_key, side, requested_size, estimated_slippage_pct, gate_reasons, held_since_ms }`. |
| `POST` | `/api/pre-dispatch/:id/approve` | `{ operator_id?: string, accept_slippage_pct: f64 }` | `200 OK` on success (the held order resumes the dispatch flow past Gate 5 with the operator-acknowledged slippage). `422 Unprocessable Entity` if the order is no longer in `PRE_DISPATCH` (e.g. timed out). The default `operator_id` is `"local_operator"`; caller-supplied identity is on the v5.0 roadmap. |
| `DELETE` | `/api/pre-dispatch/:id` | — | `204 No Content` (the held order is discarded without dispatch; the associated `risk_control_events` row is preserved). |

`Pre-dispatch` orders are not persisted to the `open_orders` table (per [`03-03-03-tae-layer2-execution.md §4`](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md)); they live only in process memory. An engine restart, crash, or process termination during the slippage-review window **loses the held order with no audit trail**. Operators relying on Gate 5 for slippage review in a 24/7 deployment should design workflows around the manual-review API rather than expecting engine-replayable recovery. The future `pre_dispatch_orders` table for crash-recoverable persistence is on the v4.1 roadmap (see `docs/CHANGELOG.md`).

### 2.10 Exchange Keys (encrypted credentials)

> Live credentials must be entered through the encrypted `exchange_keys` SQLite table, **not** through `config.toml`. `config.toml` holds no secret material. The encryption contract is in [`06-02-database-schema-spec.md §3.5`](06-02-database-schema-spec.md).

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `POST` | `/api/keys` | `{ exchange: string, api_key: string, api_secret: string, passphrase?: string }` | `201 Created` on insert. Body is **never** echoed back; `api_secret` and `passphrase` are stored encrypted with `EXCHANGE_SECRET_KEY` (AES-256-GCM). |
| `GET` | `/api/keys?exchange=` | — | `{ items: [{ exchange, key_id, created_at, last_rotated_at }] }` (the encrypted credentials are **not** in the response). |
| `DELETE` | `/api/keys/:key_id` | — | `204 No Content` |

---

## 3. WebSocket Protocol

### 3.1 Endpoint

```
GET /ws?symbol=<PAIR-KEY>&timeframe_secs=<SECONDS>
```

Defaults: symbol = first configured symbol; timeframe_secs = 60.

### 3.2 Frame Format — JSON-RPC 2.0 Notification

Every server→client frame:

```json
{
  "jsonrpc": "2.0",
  "method": "broadcast.market_snapshot",
  "params": {
    "symbol": "BTC-USDT",
    "timeframe_secs": 60,
    "snapshot": { /* MarketSnapshot — byte-for-byte per 02-07-metrics-matrix.md §2.1 */ }
  }
}
```

The `snapshot` field is the serialized `MarketSnapshot` schema defined in [`02-07-metrics-matrix.md §2.1`](../matrices/02-07-metrics-matrix.md). Every top-level field is present on the wire: `indicators`, `alignment`, `analysis`, `risk`, `advisory`, `decision_context`, `context`, `liquidity`, `cluster`, `liquidity_signals`, `statistical_context`, `risk_profile` (each is `Option::None` or empty-object as appropriate). Both `is_completed = true` completed snapshots and `is_completed = false` shadow snapshots ride the same channel; shadow snapshots are display-only and never enter the L4/L5/L6 synthesis cascade.

Key properties:
- No `id` field (notification — no response expected).
- Client send-to-server is not used — **write-only push** from engine.
- The frontend maintains 4 parallel connections (one per timeframe: micro/fast/slow/macro).
- Exponential backoff on disconnect: initial 1 s → max 30 s; jitter is applied **before** the cap (effective range `[0.8 × delay_n, min(1.2 × delay_n, 30 s)]`). See [`08-03-connection-resilience.md §Backoff Formula`](../operations-and-compliance/08-03-connection-resilience.md).
- `applySnapshotToTimeframe()` parses the nested `snapshot` object and writes to the Svelte 5 rune store.

### 3.3 JSON-RPC 2.0 method names

The shared crate (`crates/core-domain/src/jsonrpc_methods.rs`) defines JSON-RPC method constants for inter-engine RPC. The single canonical method used by the engine today is `broadcast.market_snapshot` (server→client notification). Internal request/response methods (`execution.open_position`, `safety.check`, `config.update`, `config.query`) round-trip via the same RPC envelope but are only used by paired-server flows; clients should only consume `broadcast.market_snapshot` and the documented REST surface.

The `operator_id` field on internal `execution.*` and `safety.*` control frames carries the local-operator identity (see §1) — `local_operator` in v4.0, caller-supplied via `X-Operator-Id` header in v5.0.

---

## 4. Serialization Conventions

| Rule | Effect |
|------|--------|
| Decimal-as-string | All price/size `Decimal` fields serialize as strings (no float precision loss). |
| Nullable omission | `Option::None` fields omitted via `skip_serializing_if`. |
| Liquidity `Vec<LiquiditySignal>` | **Always serialized** as `[]` when no signals fired (never omitted via `skip_serializing_if`). |
| Empty collection omission | Other empty arrays/maps omitted (non-liquidity). |
| Enum casing | `SCREAMING_SNAKE_CASE` (`BULLISH`, `OVERBOUGHT`, `STRONG_BULL_MTF`). |
| Timestamps | `u64` Unix epoch (seconds or ms, field-dependent). |

---

## 5. Fallback Route

`/api/*` routes that do not match any documented endpoint return `404 Not Found` with the JSON error envelope of §1.1. Only non-`/api/*` paths fall through to the static-asset SPA handler serving `ui/dist/`. `/favicon.ico` redirects `301` → `/favicon.svg`.

---

## 6. Cross-References

- [Database Schema](06-02-database-schema-spec.md) — Persistent state; persistent `risk_control_events` table (§3.10); encrypted `exchange_keys` table (§3.5); canonical `order_fills` table (§3.7); `open_orders` lifecycle (§3.2).
- [UI Overview](../ui-ux/07-01-ui-overview-spec.md) — Frontend consumption and `instance.timeframes.{micro|fast|slow|macro}` demux shape.
- [Systemic Data Flow](../conceptual-foundations/01-03-systemic-data-flow.md) — Sequence diagrams for the engine pipeline.
- [Pre-Trade Risk Controls](../operations-and-compliance/08-02-pre-trade-risk-controls.md) — Gate ordering and override semantics for `/api/pre-dispatch/*` and `/api/orders/:id/override-readiness`.
- [Connection Quality](../operations-and-compliance/08-05-connection-quality.md) — `/api/connection-quality` scope (instance × timeframe) and score formula.
- [Connection Resilience](../operations-and-compliance/08-03-connection-resilience.md) — Backoff and retry budgets referenced from §3.2.
