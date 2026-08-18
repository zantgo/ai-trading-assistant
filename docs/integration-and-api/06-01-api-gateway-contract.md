# API Gateway Contract

**Version:** 7.0 (2026-08-18) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Purpose:** This document specifies the complete REST and WebSocket API surface of the Trading Platform — routes, request/response payloads, JSON-RPC 2.0 conventions, HTTP status codes, error envelope, and serialization rules.

---

## 1. Transport & Addressing

| Property | Value |
|----------|-------|
| Framework | Axum (Rust) on a Tokio runtime |
| Base URL | `http://127.0.0.1:3000` (localhost only) |
| Authentication | **Single-operator local deployment.** The Trading Platform is built for a single operator and their team: one workspace, one operator identity (`operator_id = "local"`), no per-route authentication, no caller-supplied identity, and no multi-client model. Every audit event (`risk_control_events.operator_id`, see [`06-02-database-schema-spec.md §3.10`](06-02-database-schema-spec.md)) and WebSocket control frame carries `operator_id = "local"`. |
| Static assets | `ui/dist/` served via `tower_http::services::ServeDir` |

### 1.0 Canonical glossary (Market Instance identifier)

Throughout this API and the corpus, three names refer to the same identifier:

| Surface | Identifier name | Example |
|---------|----------------|---------|
| API query parameter / response field | `instance_id` | `?instance_id=BTC-USDT@Hyperliquid` |
| SQLite schema column | `pair_key` | `connection_quality_samples.pair_key` |
| Dashboard UI label | "Active pair" / "Market Instance" | "BTC-USDT (Hyperliquid)" |

All three denote the same runtime container — the **Market Instance**: a `(symbol, exchange)` pair (`pair_key` form: `BTC-USDT@Hyperliquid` or just `BTC-USDT` when exchange is unambiguous) on a single venue, owning up to four **TimeframePipelines** (one per configured timeframe tier), each with its own analyzer pipeline, telemetry stream, connection-quality tracker, and risk profile. The canonical identifier format on the wire is the **unified internal symbol** (e.g. `BTC-USDT`), with the exchange implied by the runtime configuration; `pair_key` extends it to `<symbol>@<exchange>` for unambiguous DB joins.

> **TimeframePipeline.** The per-`(symbol, timeframe)` analytical unit owned by a Market Instance — one pipeline per configured timeframe tier (micro/fast/slow/macro; 60/180/300/900 s by default), each running its own ingestion → indicator → matrix cascade. Per-timeframe telemetry and matrices are always scoped `instance_id × timeframe_secs` (see §2.3).

> **Cross-references.** This glossary is the single source of truth for the three names. Docs that previously used the three names interchangeably (e.g. `/api/connection-quality` formerly mixing `instance_id` and `pair_key`) now point here for resolution.

### 1.1 HTTP status codes and error envelope

Every response uses one of the following status codes:

| Code | Meaning |
|---|---|
| `200 OK` | Successful read or idempotent mutation |
| `201 Created` | New resource persisted (infrequent in current release) |
| `204 No Content` | Successful mutation, no body |
| `400 Bad Request` | JSON parse error, missing required field, malformed value |
| `404 Not Found` | Unknown resource (unknown `instance_id`, unknown pair_key, etc.) |
| `409 Conflict` | State conflict (e.g. duplicate symbol). Instances in lifecycle `PAUSED` state accept reconfiguration — the pipeline stays alive; Gate 0 blocks only new entries |
| `422 Unprocessable Entity` | Semantic validation failure (e.g. operator override while veto condition still active) |
| `500 Internal Server Error` | Unhandled server-side failure |
| `503 Service Unavailable` | Engine not yet initialized, exchange connectivity down |

> **Error envelope (audit 2026-08-18).** The JSON envelope shown below is the **intended** contract; the current release returns **plain-text bodies with HTTP status codes** from most handlers (e.g. `404 "Instance not found"`), and unknown `/api/*` paths fall through to the static-asset handler's plain-text 404. Clients must branch on the HTTP status code; the envelope ships in a later release:

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

> **Per-instance matrices via WebSocket only.** The Decision Matrix, Analysis Matrix, Opportunity Matrix, Risk Matrix, and other per-Market-Instance MME outputs are delivered exclusively via the WebSocket envelope (`/ws`) — there is no per-matrix REST endpoint, because these matrices update on every completed candle and a polling REST surface would stale. Use `/ws?symbol=…&timeframe_secs=…` for live access; use `/api/history?symbol=…&timeframe_secs=…` for replay. **Global aggregations (Overview Matrix) are served over REST** at `GET /api/overview` (polled by the dashboard every 3 s) — they are *not* WS-only. The aggregate carries `low_coverage` as a **top-level** field (`bool`, default `false` — `true` when fewer than 3 symbols are active (`active_symbols.len() < 3`); schema in [`02-09-overview-matrix.md §2.1`](../matrices/02-09-overview-matrix.md)). As of v6.10.3 the Overview Matrix additionally carries `alignment_distribution` (`map<string, u32>`), `alignment_consensus_index` (`f64`, [-100, 100]), and `multi_tf_agreement_pct` (`f64`, [0, 100]) — see [`02-09-overview-matrix.md §3.5`](../matrices/02-09-overview-matrix.md) and the per-asset `AssetRank.mtf_score` / `mtf_label` columns in [`02-09-overview-matrix.md §2.2`](../matrices/02-09-overview-matrix.md).

### 2.1 Session Management

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `GET` | `/api/session/status` | — | `{ active: bool, currency: string, exchange: string, instance_count: u32 }` (the response field is `active`, not `session_active` — the frontend reads `data.active`; corrected 2026-08-17) |
| `POST` | `/api/session/init` | `{ exchange: string, currency: string }` | Session status |
| `POST` | `/api/session/quit` | — | `200 OK` + JSON (cleanup result) → cleans all instances (corrected 2026-08-17 — the handler returns `200` with a body, not `204`) |

### 2.2 Configuration

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `GET` | `/api/config` | — | `{ api_key_configured: bool, symbols: string[], candles, indicators, instances, indicator_registry, api_failover }` — the full `AppConfig` surface including the `[api_failover]` block (see §2.2.1 below). |
| `POST` | `/api/config` | The `GET /api/config` response body (partial update) | `200 OK` on success. The operator-editable groups (`candles`, `indicators`, `instances`, `api_failover`) are merged into the currently loaded workspace config and persisted to `config.toml` (platform sections preserved via `save_workspace`); `config_version` increments on every accepted save. The body does **not** require `id`/`name` — the read-only fields echoed by GET (`api_key_configured`, `symbols`, `indicator_registry`) are accepted and ignored. `500 Internal Server Error` if `config.toml` cannot be written. (No per-field validation is performed; unknown keys are ignored by serde.) |

#### 2.2.1 `[api_failover]` configuration

The `[workspace.api_failover]` TOML block (surfaced as `api_failover` in both the `GET` response and the `POST` merge) tunes the derivatives-data pollers' tolerance for REST failures:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_retries_per_call` | `u32` | `5` | Per-call retry budget for the derivatives-data pollers. Carried for operator visibility; reserved for per-call retry wiring. |
| `retry_delay_seconds` | `u32` | `30` | Backoff between retry attempts. Carried for operator visibility; reserved for per-call retry wiring. |
| `max_consecutive_failures` | `u32` | `30` | **Consumed by the Hyperliquid derivatives poller**: after this many consecutive REST failures the poller is permanently disabled for the process lifetime. |

Defaults are applied via `#[serde(default)]` in `crates/config-models/src/models.rs::ApiFailoverConfig`; an absent block behaves identically to the defaults.
| `GET` | `/api/rules` | — | `{ content: string }` (reads `docs/engines/market-monitoring-engine/03-02-09-mme-indicators-guide.md`). |
| `POST` | `/api/rules` | `{ content: string }` | Writes indicator guide. |

### 2.3 History & Monitor

| Method | Path | Params | Response |
|--------|------|--------|----------|
| `GET` | `/api/history` | `symbol`, `timeframe_secs`, `limit` (default `100`, max `1000`) | `{ symbol, prices[], candles[], indicator_history, clusters?, volume_profiles?, liquidity_flows? }` — see below for the exact `indicator_history` shape. |

The response shape is:

```json
{
  "symbol": "BTC-USDT",
  "prices": ["64000.00"],
  "candles": [
    { "time": 1755090600000, "open": "64000.00", "high": "64100.00",
      "low": "63950.00", "close": "64050.00", "volume": "12.5",
      "reconstructed": "SYNTHETIC" }
  ],
  "indicator_history": {
    "symbol": "BTC-USDT",
    "timeframe_secs": 60,
    "times": [1755090600],
    "indicators": {
      "<key>": {
        "raw": [64000.0],
        "normalized": [0.62],
        "state_label": ["LIVE"],
        "values": { "<sub>": [64050.0] }
      }
    }
  },
  "clusters": { ... },
  "volume_profiles": { ... },
  "liquidity_flows": { ... }
}
```

- Candle `time` is epoch **milliseconds**; all OHLCV fields are strings. `reconstructed` is omitted when the candle is live-sourced; when present it is the `SCREAMING_SNAKE_CASE` `ReconstructionMethod` wire value (`EXCHANGE_HISTORICAL`, `EXPONENTIAL_MOVING_AVERAGE`, `LINEAR_INTERPOLATION`, `UNAVAILABLE`, `SYNTHETIC`).
- `indicator_history.times` is epoch **seconds** and is axis-aligned with every inner array: `times[i]` pairs with `raw[i]` / `normalized[i]` / `state_label[i]` / each `values["<sub>"][i]` (AUDIT-V8-006).
- **WARMING placeholders are filtered server-side.** Registry-gate placeholder rows carry `state_label == "WARMING"` with a `raw` value of `0.0`; the history handler never surfaces those — every array slot at such a timestamp is emitted as `null` (and gap-fill Doji bars without indicator data also produce `null` slots), so charts never paint a phantom `0.0` plateau.
- `clusters`, `volume_profiles`, and `liquidity_flows` are per-timeframe-slot maps (`micro`/`fast`/`slow`/`macro`); each is omitted when empty.
| `GET` | `/api/monitor` | `symbol` | Multi-TF meta-intelligence (per-TF regime, MTF agreement matrix, MarketContext). |
| `GET` | `/api/connection-quality` | `instance_id` (optional), `timeframe_secs` (optional), `window=one_hour\|six_hour\|twenty_four_hour` (default `one_hour`) | `ConnectionQualityReport` JSON (`window`, `window_start_ms`, `window_end_ms`, `uptime_pct`, `disconnect_count`, `avg_reconnect_ms`, `total_data_loss_secs`, `reconstructed_candles`, `score`). When both `instance_id` and `timeframe_secs` are supplied, returns the per-scope report for that `(instance_id, timeframe_secs)` pair. When either is absent, falls through to a cross-scope process-wide aggregate. See [`08-05-connection-quality.md §REST API`](../operations-and-compliance/08-05-connection-quality.md) for the full schema and behaviour. |

### 2.4 Instances (Workspaces)

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/instances?pair_key=` | List running instance summaries. |
| `POST` | `/api/instances` | Create instance — request body `{ base: string, quote: string, ... }` (exchange + base/quote symbol parts; the UI sends `{ base, quote }`; a bare `{ symbol }` body is rejected `400/422`). |
| `GET` | `/api/instances/:instance_id` | Instance detail (equity, caution). |
| `DELETE` | `/api/instances/:instance_id` | Delete instance. |
| `DELETE` | `/api/instances/by-pair/:pair_key` | Delete by pair key. |
| `POST` | `/api/instances/:instance_id/config` | Reconfigure (`InstanceConfigPayload`) → recharge pipeline. |
| `POST` | `/api/instances/:instance_id/reload` | **Specified but not yet registered** (returns 404 per §5): tear down + rebuild a single TF pipeline (`slot=micro|fast|slow|macro`) or all four (`slot=all`). See [08-08 CB-11](../operations-and-compliance/08-08-candle-buffer-spec.md) and [03-01-06 DCP-09](../engines/data-infrastructure-engine/03-01-06-die-candle-pipeline-states.md). |
| `GET` | `/api/instances/:instance_id/activation` | **Specified but not yet registered** (returns 404 per §5). Planned response: the effective activation set (global `[activation]` ∪ instance `[instances."<id>".activation]`) as applied at the current `config_version` — `{ disabled_indicators: [], disabled_signals: [], disabled_signal_kinds: [], liquidity: {...}, config_version: u64 }`. The activation is applied config-side today (see [`03-02-12-mme-configurable-activation.md §2`](../engines/market-monitoring-engine/03-02-12-mme-configurable-activation.md)); the REST surface is AUDIT-V6-212. |
| `POST` | `/api/instances/:instance_id/start` | Transition STOPPED / instance PAUSED → RUNNING (executor admits entries); full lifecycle semantics per [03-03-06](../engines/trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md). |
| `POST` | `/api/instances/:instance_id/pause` | Pending entries cancelled; open positions still managed (TP/SL/invalidation remain armed); no new setups. See [03-03-06 §3](../engines/trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md). |
| `POST` | `/api/instances/:instance_id/stop` | Transition RUNNING/lifecycle PAUSED → STOPPING → STOPPED (flatten: cancel orders + market-close positions with `is_emergency_liquidation = true`, `reduce_only = true`). DELETE on a non-STOPPED instance returns `409` (see [03-03-06 IL-08](../engines/trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md)). |
| `POST` | `/api/instances/:instance_id/safety/reset` | Reset the per-symbol `consecutive_losses` counter (clears `consecutive_losses[sym]`; does **not** release a drawdown or systemic veto — see `/safety/release-veto` below). |
| `POST` | `/api/instances/:instance_id/safety/release-veto` | **Informational safety reset (v7).** Returns the safety state to `NORMAL` (only when the underlying drawdown condition has cleared) + optional peak-equity reset. Returns `422` if the drawdown condition is still active. Distinct from `/safety/reset` (consecutive-loss counters). `operator_id = "local"` is recorded in the resulting `risk_control_events` row. |
| `GET` | `/api/instances/:instance_id/automation` | **v7 TAE surface (served).** Full setup-executor state: mode (paper/live), phase, tracked setup + projected risk/return, entry + bracket orders, position, invalidation state, activity log, safety gate, lifecycle, equity. See [03-03-01 §8.1](../engines/trade-automation-engine/03-03-01-tae-overview-spec.md). |
| `POST` | `/api/instances/:instance_id/automation/close` | **v7 manual override (served).** Cancels pending/bracket orders and closes the open position at market. `exit_reason = "manual"`. |
| `POST` | `/api/instances/:instance_id/intervals` | Set trigger loop intervals (`{ slow_seconds, normal_seconds, fast_seconds }`). |
| `GET` | `/api/instances/:instance_id/portfolio` | **PME v7 (served): rich informational portfolio state** — equity, realized/unrealized/daily PnL, peak + max drawdown %, safety state + context, systemic risk, exposure block (gross/net/long/short/concentration), capital block (available/committed margin, usage, leverage, alert), positions with mark-to-market, lifecycle. Read-only. See [03-04-01 §5](../engines/portfolio-management-engine/03-04-01-pme-overview-spec.md). |
| `GET` | `/api/instances/:instance_id/exposure` | **PME v7 (served):** Exposure Matrix (gross/net/long/short, per-symbol concentration, max single-pair). Read-only. |
| `GET` | `/api/instances/:instance_id/capital` | **PME v7 (served):** Capital Matrix (available/committed margin, margin-usage + leverage ratios, alerts). Read-only. |
| `GET` | `/api/instances/:instance_id/safety` | PME Safety state (v7 extended: adds `max_drawdown_pct`, `daily_pnl`, `margin_usage_ratio`). |
| `POST` | `/api/instances/:instance_id/safety/session-reset` | **PME v7 (served, informational):** rebaseline peak equity + daily PnL to current equity (`session_reset`). |

### 2.5 Risk Profiles

| Method | Path | Purpose |
|--------|------|---------|
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
| `GET` | `/api/trade-journal/export/csv` | CSV export (1000 records). All per-trade metrics use `roi_pct` (the canonical field; the legacy export alias is deprecated — removal tracked as AUDIT-V4-044, target v7.0; retired name recorded in `docs/CHANGELOG.md`). |
| `GET` | `/api/trade-journal/export/json` | JSON export (1000 records). Same canonical `roi_pct` field. |
| `POST` | `/api/trades/telemetry` | Create telemetry history entry. |

### 2.8 Dashboard & System

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/api/dashboard/stats?initial_capital=` | `DashboardStats` (20+ stat categories). |
| `GET` | `/api/system/status` | `{ observation_loop_latency_ms, ingest_skew_ms, system_heartbeat_latency_ms, journal_mode, active_pairs_count }`. The three `*_latency_ms` / `*_skew_ms` fields are distinct: `observation_loop_latency_ms` is the end-to-end raw-frame-to-broadcast latency (DIE performance target, see [03-01-01 §3](../engines/data-infrastructure-engine/03-01-01-die-overview-spec.md) and [03-01-03 §5](../engines/data-infrastructure-engine/03-01-03-die-layer2-market-data.md)); `ingest_skew_ms` is the difference between local receipt time and `timestamp_ms` (per-trade skew); `system_heartbeat_latency_ms` is the round-trip of the most recent WS control frame. |
| `GET` | `/api/system/observability?symbol=` | `{ recent_decisions[], completed_trades[] }`. |
| `GET` | `/api/overview` | `OverviewMatrix` — the L7 global synthesis (`global_market_bias`, `market_breadth`, `breadth_pct`, `low_coverage`, `regime_distribution`, `opportunity_distribution`, `risk_distribution` + `risk_environment`, `cascade_risk_index`, `systemic_risk_score`, `asset_ranking`, `market_synchronization`, `market_health`, `alignment_distribution` / `alignment_consensus_index` / `multi_tf_agreement_pct`, `global_summary`, `instance_count`, `active_symbols`). Polled by the dashboard every 3 s; schema: [02-09-overview-matrix.md §2](../matrices/02-09-overview-matrix.md). |
| `GET` | `/api/liquidity/cluster-status?symbol=&slot=` | Per-TF `LiquidationClusterMatrix` refresh status (`ClusterStatusSnapshot`: `{ symbol, timeframe_slot, status, reason?, generated_at_ms?, valid_until_ms?, cluster? }` with status `Pending` while the refresh task has not produced a matrix, `Skipped` + reason when the per-TF kill switch or `[activation] cluster_estimation` disables it, `Ready` otherwise). Response shape adapts to the query: `?symbol=&slot=` → flat single snapshot; `?symbol=` → `{ symbol, slots: { micro|fast|slow|macro: snapshot } }`; no filter → `[{ symbol, slots }]`. Consumed by the LIQ HEATMAP overlay / `LiquidationHeatmapTierPicker`. |

### 2.8.1 Snapshot Export (v6.10.4+)

The periodic JSON dump scheduler — see
[`../operations-and-compliance/08-09-snapshot-export.md`](../operations-and-compliance/08-09-snapshot-export.md)
for the operator manual and
[`06-03-snapshot-export-schema.md`](06-03-snapshot-export-schema.md) for the on-disk schema.

| Method | Path | Body | Response |
|--------|------|------|----------|
| `GET` | `/api/snapshot-export/status` | — | `SnapshotExportRuntime { enabled, output_path, interval_secs, max_snapshots_retained, tabs[], last_snapshot_at, total_snapshots_written, last_error, last_instance_count }`. |
| `PUT` | `/api/snapshot-export/config` | `SnapshotExportConfigPatch` (every field optional): `{ enabled?: bool, output_path?: string, interval_secs?: u64, max_snapshots_retained?: u32, tabs?: string[] }` | Updated `SnapshotExportRuntime`. Validation: `output_path` non-empty (otherwise 400); `interval_secs` clamped to `[5, 3600]`; `max_snapshots_retained` clamped to `[10, 100000]`; unknown `tabs` IDs silently dropped; empty `tabs` list falls back to all 9 canonical ids. |
| `POST` | `/api/snapshot-export/run-now` | — | `{ triggered: true, path: string, note: "Tick scheduled; …" }`. Fires an immediate tick (the next scheduled tick proceeds as usual). |



### 2.10 Exchange Keys (encrypted credentials)

> **Not yet mounted (returns 404 per §5).** The key-management handler exists in `crates/api-gateway/src/handlers/keys.rs` (list/add/delete) but is **not registered on the router** — do not build clients against it. Since audit 2026-08-18 the handler encrypts `api_secret`/`passphrase` with `EXCHANGE_SECRET_KEY` (AES-256-GCM, `database_storage::crypto`) and refuses to store plaintext when no master key is provisioned (`503`).

> Live credentials must be entered through the encrypted `exchange_keys` SQLite table, **not** through `config.toml`. `config.toml` holds no secret material. The encryption contract is in [`06-02-database-schema-spec.md §3.5`](06-02-database-schema-spec.md).

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `POST` | `/api/keys` | `{ exchange: string, api_key: string, api_secret: string, passphrase?: string }` | `201 Created` on insert. Body is **never** echoed back; `api_secret` and `passphrase` are stored encrypted with `EXCHANGE_SECRET_KEY` (AES-256-GCM). |
| `GET` | `/api/keys?exchange=` | — | `{ items: [{ exchange, key_id, created_at, last_rotated_at }] }` (the encrypted credentials are **not** in the response). |
| `DELETE` | `/api/keys/:key_id` | — | `204 No Content` |

### 2.11 System diagnostics endpoints

Served since v6.4.1 (previously tracked as the Phase-3 handlers under AUDIT-V6-301; see `docs/CHANGELOG.md`):

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/api/system/clock` | `ClockStatusResponse` — `{ within_threshold, drift_us?, jitter_rms_us?, last_poll_ms?, breach_count, breach_action, ntp_servers, sample_count, threshold_micros }` (`crates/api-gateway/src/handlers/clock.rs`). Returns `503 Service Unavailable` when the clock monitor is disabled. `breach_count` is a running `AtomicU32` counter incremented on each observed breach since process start. |
| `GET` | `/api/exchange-status` | Per-exchange connectivity status (`crates/api-gateway/src/handlers/exchange_status.rs`). |
| `GET` | `/api/data-quality` | `PipelineReliabilityMetrics` — `{ coverage, gap_count, outliers_rejected, outliers_bypassed, out_of_order_dropped, total_candles_processed, reconstructed_candles, source_mix }` where `source_mix` has `{ db_warm, rest_gap, live }` (`crates/api-gateway/src/handlers/data_quality.rs`; contract in [03-01-04 §5](../engines/data-infrastructure-engine/03-01-04-die-layer3-data-quality.md)). Aggregates process-wide across all instances. |

### 2.12 Planned endpoints (not yet served)

The following are **specified but not yet served** (return `404` per §5). Listed for forward planning only.

| Method | Path | Planned purpose | Audit ID |
|--------|------|-----------------|----------|
| `POST` | `/api/keys/rotate` | In-process re-encryption of stored exchange credentials under a new master key (no daemon restart). | AUDIT-V6-077 |
| `GET` | `/api/keys/backup` | Encrypted-backup export of stored credentials, keyed by a passphrase. | AUDIT-V6-077 |
| `GET` | `/api/instances/:id/activation` | Effective activation set (global `[activation]` ∪ per-instance) at the current `config_version`. | AUDIT-V6-212 |

### 2.13 Performance Analytics endpoints (live)

The Performance Analytics Engine exposes serving endpoints under `/api/analytics/*` and `/api/backtest/*`. These are **live** and consumed by the `PerformanceDashboard` Overview / Strategy / Risk Metrics / Regime Map / Trade Analytics / Backtesting panels.

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/api/analytics/summary` | `{ total_trades, total_pnl, win_rate, ... }` aggregate analytics summary. |
| `GET` | `/api/analytics/strategy` | `StrategyAnalyticsRow[]` — per-setup-type NHST (T-Stat, P-Value, P_MC, α = 0.05, edge classification). |
| `GET` | `/api/analytics/risk` | `RiskAnalyticsRow` — Sharpe, Sortino, Calmar, Ulcer, VaR, ES, max drawdown. |
| `GET` | `/api/analytics/performance` | `PerformanceMatrixRow[]` — regime-strategy performance map. |
| `GET` | `/api/analytics/optimization` | `OptimizationReport` — per-regime performance + recommendations. |
| `GET` | `/api/analytics/strategy/history?limit=` | `StrategyAnalyticsRow[]` — historical strategy analytics. |
| `GET` | `/api/analytics/trades?limit=200` | `TradeAnalyticsRecord[]` — reconstructed closed trades (default limit 200). |
| `GET` | `/api/analytics` | Catch-all for `/api/analytics/*` references — **not registered** (returns 404; only the concrete `/api/analytics/*` rows above are served). |
| `POST` | `/api/backtest/run` | Run a backtest `{ symbol, timeframe_secs, from_ms, to_ms, initial_capital }` — replays recorded MME decisions through the setup executor (paper only); returns `{ backtest_id, summary, stats (NHST + α), trades, equity_curve }`. See [03-05-06](../engines/performance-analytics-engine/03-05-06-pae-layer5-backtest.md). |
| `GET` | `/api/backtest/:id` | Fetch a persisted backtest run (or 404). |
| `GET` | `/api/backtest` | Catch-all for `/api/backtest/*` references — **not registered** (returns 404; only the concrete rows above are served). |

The endpoints above are documented in segment form (not in the canonical `(METHOD, /path)` row layout) for readability. Each path is canonical to the per-tab `PerformanceDashboard` `fetch()` call and the `api-gateway` HTTP handler in `crates/api-gateway/src/handlers/analytics.rs`.

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
    "timeframe_slot": "micro",
    "timeframe_secs": 60,
    "snapshot": { /* MarketSnapshot — byte-for-byte per 02-07-metrics-matrix.md §2.1 */ }
  }
}
```

`params.timeframe_slot` is the authoritative wire-side slot identifier (`micro` / `fast` / `slow` / `macro`) — every notification carries it so clients never have to re-derive the slot from `timeframe_secs`. New connections may also select their slot explicitly via `?slot=micro|fast|slow|macro`; legacy clients omit it and the server derives a best-effort slot from the requested duration.

The `snapshot` field is the serialized `MarketSnapshot` schema defined in [`02-07-metrics-matrix.md §2.1`](../matrices/02-07-metrics-matrix.md) — fields per 02-07 §2.1 (MANIFEST gate G13). Both `is_completed = true` completed snapshots and `is_completed = false` shadow snapshots ride the same channel; shadow snapshots are display-only and never enter the L4/L5/L6 synthesis cascade.

Key properties:
- No `id` field (notification — no response expected).
- Client send-to-server is not used — **write-only push** from engine.
- The frontend maintains 4 parallel connections (one per timeframe: micro/fast/slow/macro).
- Exponential backoff on disconnect: initial 1 s → max 30 s; jitter is applied **before** the cap (effective range `[0.8 × delay_n, min(1.2 × delay_n, 30 s)]`). See [`08-03-connection-resilience.md §Backoff Formula`](../operations-and-compliance/08-03-connection-resilience.md).
- `applySnapshotToTimeframe()` parses the nested `snapshot` object and writes to the Svelte 5 rune store.

> **Producer attribution.** The `MarketSnapshot` frames streamed over this WebSocket are produced by **MME Layer 1** (the Metrics Layer), not by DIE L4. MME L1 is the sole producer of `MarketSnapshot` content; DIE L4 owns only the upstream `NormalizedCandle` transport channel. See [03-02-02-mme-layer1-metrics.md §8](../engines/market-monitoring-engine/03-02-02-mme-layer1-metrics.md) for the channel specification and [03-01-05-die-layer4-data-distribution.md](../engines/data-infrastructure-engine/03-01-05-die-layer4-data-distribution.md) for the DIE L4 `NormalizedCandle` channel.

### 3.3 JSON-RPC 2.0 method names

The shared crate (`crates/core-domain/src/jsonrpc_methods.rs`) defines JSON-RPC method constants for inter-engine RPC. The single canonical method used by the engine today is the **`broadcast.market_snapshot`** notification (server→client, §3.2 — the frame every `/ws` connection receives, one per `(symbol, timeframe_slot)` subscription). Internal request/response methods (`execution.open_position`, `safety.check`, `config.update`, `config.query`) round-trip via the same RPC envelope but are only used by paired-server flows; clients should only consume `broadcast.market_snapshot` and the documented REST surface.

The `operator_id` field on internal `execution.*` and `safety.*` control frames carries the single-operator identity (see §1) — always `"local"`. There is no caller-supplied identity.

---

## 4. Serialization Conventions

| Rule | Effect |
|------|--------|
| Decimal-as-number | All price/size `Decimal` fields serialize as plain JSON numbers via `rust_decimal`'s `serde-float` feature (`crates/core-domain/Cargo.toml`). Standard JSON parsers materialize them as IEEE-754 doubles; consumers requiring exact decimal semantics must re-parse the raw number literal with a decimal type (see [06-00 §3.2](06-00-consumer-onboarding.md)). |
| Nullable omission | Many `Option` fields serialize as JSON `null` — only fields carrying `#[serde(skip_serializing_if = "Option::is_none")]` (or `HashMap::is_empty` for map fields) are omitted from the wire. Annotated examples: `clusters` / `volume_profiles` / `liquidity_flows` on `/api/history`, `MarketSnapshot` fields behind the `skip_serializing_if` gate. Unannotated `Option`s appear explicitly as `null`. |
| Liquidity `Vec<LiquiditySignal>` | Omitted via `skip_serializing_if = "Vec::is_empty"` when no signals fired (never serialized as `[]`). |
| Empty collection omission | Other empty arrays/maps omitted (non-liquidity). |
| Enum casing | `SCREAMING_SNAKE_CASE` (`BULLISH`, `OVERBOUGHT`, `STRONG_BULL_MTF`). |
| Timestamps | `u64` Unix epoch (seconds or ms, field-dependent). |

---

## 5. Fallback Route

`/api/*` routes that do not match any documented endpoint return `404 Not Found` (plain text in the current release — see the §1.1 note; the JSON envelope ships later). Only non-`/api/*` paths fall through to the static-asset SPA handler serving `ui/dist/`. `/favicon.ico` redirects `301` → `/favicon.svg`.

---

## 6. Cross-References

- [Database Schema](06-02-database-schema-spec.md) — Persistent state; persistent `risk_control_events` table (§3.10); encrypted `exchange_keys` table (§3.5); canonical `order_fills` table (§3.7); `open_orders` lifecycle (§3.2).
- [UI Overview](../ui-ux/07-01-ui-overview-spec.md) — Frontend consumption and the `microTerm` / `fastTerm` / `slowTerm` / `macroTerm` demux shape (07-01 §2.3).
- [Systemic Data Flow](../conceptual-foundations/01-03-systemic-data-flow.md) — Sequence diagrams for the engine pipeline.
- [Pre-Trade Risk Controls](../operations-and-compliance/08-02-pre-trade-risk-controls.md) — the v7 executor's risk gates (informational).
- [Connection Quality](../operations-and-compliance/08-05-connection-quality.md) — `/api/connection-quality` scope (instance × timeframe) and score formula.
- [Connection Resilience](../operations-and-compliance/08-03-connection-resilience.md) — Backoff and retry budgets referenced from §3.2.
