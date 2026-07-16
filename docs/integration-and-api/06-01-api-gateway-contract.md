# API Gateway Contract

**Version:** 2.0
**Status:** Approved
**Purpose:** This document specifies the complete REST and WebSocket API surface of the Trading Platform — routes, request/response payloads, JSON-RPC 2.0 conventions, and serialization rules.

---

## 1. Transport & Addressing

| Property | Value |
|----------|-------|
| Framework | Axum (Rust) |
| Base URL | `http://127.0.0.1:3000` (localhost only) |
| Authentication | None (local-only daemon) |
| Static assets | `crates/frontend/dist/` served via `tower_http::services::ServeDir` |

---

## 2. REST API Reference

> **Per-instance matrices via WebSocket only.** The Decision Matrix, Analysis Matrix, Opportunity Matrix, Risk Matrix, and other per-Market-Instance MME outputs are delivered exclusively via the WebSocket envelope (`/ws`) — there is no per-matrix REST endpoint, because these matrices update on every completed candle and a polling REST surface would stale. Use `/ws?symbol=…&timeframe_secs=…` for live access; use `/api/history?symbol=…&timeframe_secs=…` for replay. Global aggregations (Overview Matrix) are also WebSocket-only.

### 2.1 Session Management

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `GET` | `/api/session/status` | — | `{ session_active: bool, currency: string, exchange: string, instance_count: u32 }` |
| `POST` | `/api/session/init` | `{ exchange: string, currency: string }` | Session status |
| `POST` | `/api/session/quit` | — | 200 OK → cleans all instances |

### 2.2 Configuration

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `GET` | `/api/config` | — | Full `AppConfig` (symbols, candles, indicators, instances, indicator_registry). |
| `POST` | `/api/config` | `AppConfig` JSON | Writes to `config.json`. |
| `GET` | `/api/rules` | — | `{ content: string }` (reads `docs/engines/market-monitoring-engine/03-02-09-mme-indicators-guide.md`). |
| `POST` | `/api/rules` | `{ content: string }` | Writes indicator guide. |

### 2.3 History & Monitor

| Method | Path | Params | Response |
|--------|------|--------|----------|
| `GET` | `/api/history` | `symbol`, `timeframe_secs` | `{ symbol, prices[], candles[], indicator_histories }` |
| `GET` | `/api/monitor` | `symbol` | Multi-TF meta-intelligence (per-TF regime, MTF agreement matrix, MarketContext). |
| `GET` | `/api/connection-quality` | `window=one_hour\|six_hour\|twenty_four_hour` (default `one_hour`) | `ConnectionQualityReport` JSON (`window`, `window_start_ms`, `window_end_ms`, `uptime_pct`, `disconnect_count`, `avg_reconnect_ms`, `total_data_loss_secs`, `reconstructed_candles`, `score`). See [08-05-connection-quality.md](../operations-and-compliance/08-05-connection-quality.md) for the full schema and source-of-truth behaviour. |

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
| `POST` | `/api/instances/:id/safety/release-veto` | **Release a hard drawdown / systemic veto** (Issue 4.O). The endpoint checks that the underlying veto condition (drawdown below threshold *and* `systemic_risk_score < systemic_risk_threshold`) has cleared, then restores the operator-configured default stances and clears the operator one-time-acknowledge flag. Returns `400` if the veto condition is still active, `200` on success. Distinct from `/safety/reset` (which only clears the consecutive-loss counter). |
| `POST` | `/api/instances/:id/manual/open` | Log manual position open. Request: `InstanceManualRequest { action: string (required), direction: Option<string> ("LONG"\|"SHORT"), price: Option<f64> }`. |
| `POST` | `/api/instances/:id/manual/close` | Log manual position close. Request: `InstanceManualRequest { action: string (required), direction: Option<string>, price: Option<f64> }`. |
| `POST` | `/api/instances/:id/intervals` | Set trigger loop intervals (`{ slow_seconds, normal_seconds, fast_seconds }`). |

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
| `POST` | `/api/risk-profiles` | Create (`{ profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread }`). |
| `DELETE` | `/api/risk-profiles/:id` | Delete. |
| `POST` | `/api/risk-profiles/:id` | Update fields. |

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
| `funding_rate_8h` | `Decimal` (string) | yes | 8-hour funding rate as a raw percentage float. |
| `spread` | `Decimal` (string) | yes | Round-trip spread cost (quote currency, per unit). |
| `atr_value` | `Decimal` (string) | no | Optional ATR for dynamic stop / target sizing. |
| `atr_multiplier` | `Decimal` (string) | no | ATR multiplier when `atr_value` is provided. |
| `atr_target_rr` | `Decimal` (string) | no | Target reward/risk ratio when `atr_value` is provided. |
| `use_dynamic_atr` | `bool` | no | `true` to compute the stop and target from ATR instead of the explicit prices. |
| `min_tick_size` | `Decimal` (string) | no | Minimum order size increment (base asset units). Position size is quantized to this tick. |

> Source of truth: `crates/engine/src/server/types.rs::RiskCalculationPayload` and `crates/engine/src/risk_calculator.rs::RiskCalculationInput`. The runtime casts the `Decimal` fields from strings at the wire boundary.

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
| `GET` | `/api/trade-journal/export/csv` | CSV export (1000 records). |
| `GET` | `/api/trade-journal/export/json` | JSON export (1000 records). |
| `POST` | `/api/trades/telemetry` | Create telemetry history entry. |

### 2.8 Dashboard & System

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/api/dashboard/stats?initial_capital=` | `DashboardStats` (20+ stat categories). |
| `GET` | `/api/system/status` | `{ connected, latency_ms, journal_mode, active_pairs_count }`. |
| `GET` | `/api/system/observability?symbol=` | `{ recent_decisions[], completed_trades[] }`. |

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
    "snapshot": { /* MarketSnapshot */ }
  }
}
```

Key properties:
- No `id` field (notification — no response expected).
- Client send-to-server is not used — **write-only push** from engine.
- The frontend maintains 4 parallel connections (one per timeframe: micro/fast/slow/macro).
- Exponential backoff on disconnect: initial 1 s → max 30 s, up to 30 retries.
- `applySnapshotToTimeframe()` parses the nested `snapshot` object and writes to the Svelte 5 rune store.

### 3.3 JSON-RPC 2.0 Method Names

The shared crate (`crates/shared/src/jsonrpc_methods.rs`) defines JSON-RPC method constants for inter-engine RPC. The single canonical method used by the engine today is `broadcast.market_snapshot` (server→client notification). Internal request/response methods (`execution.open_position`, `safety.check`, `config.update`, `config.query`) round-trip via the same RPC envelope but are only used by paired-server flows; clients should only consume `broadcast.market_snapshot` and the documented REST surface.

---

## 4. Serialization Conventions

| Rule | Effect |
|------|--------|
| Decimal-as-string | All price/size `Decimal` fields serialize as strings (no float precision loss). |
| Nullable omission | `Option::None` fields omitted via `skip_serializing_if`. |
| Empty collection omission | Empty arrays/maps omitted. |
| Enum casing | `SCREAMING_SNAKE_CASE` (`BULLISH`, `OVERBOUGHT`, `STRONG_BULL_MTF`). |
| Timestamps | `u64` Unix epoch (seconds or ms, field-dependent). |

---

## 5. Fallback Route

All unrecognized paths serve static assets from `crates/frontend/dist/` (SPA client-side routing). `/favicon.ico` redirects 301 → `/favicon.svg`.

---

## 6. Cross-References

- [Database Schema](06-02-database-schema-spec.md) — Persistent state.
- [UI Overview](../ui-ux/07-01-ui-overview-spec.md) — Frontend consumption.
- [Systemic Data Flow](../conceptual-foundations/01-03-systemic-data-flow.md) — Sequence diagrams.
