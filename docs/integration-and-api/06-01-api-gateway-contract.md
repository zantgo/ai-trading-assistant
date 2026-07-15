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
| `POST` | `/api/instances/:id/safety/reset` | Reset consecutive loss counter. |
| `POST` | `/api/instances/:id/manual/open` | Log manual position open. |
| `POST` | `/api/instances/:id/manual/close` | Log manual position close. |
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
| `POST` | `/api/risk/calculate` | `RiskCalculationInput` (11 fields + optional ATR) | `RiskCalculation` (12 fields: S, notional, margin, liquidation, R:R, fees, net). |
| `GET` | `/api/risk/fee-table` | `order_type`, `capitals[]`, `leverages[]` | Fee table. |
| `POST` | `/api/risk/commission-projection` | `CommissionProjectionPayload` | Full dual-entry fee/sizing projection. |

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
