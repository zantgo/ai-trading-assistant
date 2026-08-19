# Implementation Roadmap

**Version:** 7.1 (2026-08-18) — see [docs/CHANGELOG.md](./CHANGELOG.md) for the canonical version history.
**Status:** In progress — partial implementation; multiple engines still in WIP.
**Purpose:** This document is the **single source of truth for what is and is not built in the Trading Platform today**, and the **phased delivery plan** for the engines, layers, and dashboards that remain on the workbench. Every spec in `docs/` describes the **target system**; this roadmap tracks **actual delivery status**, names the work that is still in flight, and gives a checklist the operator (and the next maintainer) can run to verify the platform's behaviour against the documentation.

> **Reading order.** Read §1 for the high-level status picture, §2 for the engine-by-engine reality, §3 for the phased delivery plan, §4 for the visible WIP markers (UI banners, docs banners, code annotations), §5 for the canonical list of known WIP items with the audit IDs that already track them, and §6 for the **verification checklist** that must pass before any of the "WIP" labels can be removed.

---

## 1. High-level status picture

| Engine | Backend code | Frontend | Production-ready? | Status |
|---|---|---|---|---|
| **DIE — Data Infrastructure** | `crates/network-adapters`, `crates/database-storage`, L2–L4 in `crates/market-analyzer` | `DataInfraDashboard` (live fetches) | **Yes — implemented** | Implemented |
| **MME — Market Monitoring** | `crates/market-analyzer` (52 indicators, 4-TF pipeline, signals, multi-TF synthesis, MarketContext, Decision Matrix) | `LiveTerminal`, `TerminalMonitor`, `AlignmentPanel`, `OpportunitiesPanel`, `RiskPanel`, `AnalysisPanel`, `RecommendationPanel`, `LiquidityPanel`, `StructuralAnchorsStrip` (all WS-fed) | **Yes — implemented** | Implemented |
| **TAE — Trade Automation** | `crates/portfolio-supervisor` (v7: setup executor, unified execution engine + `ExecutionBackend` trait, `PaperSimulation`, `LiveBroker` + `BitgetLiveBroker`, lifecycle manager) | `TradeAutomationDashboard` (live fetches: automation state, orders, position, activity log, trade history) | **Yes — implemented** (paper default; live dispatch for Hyperliquid + Bitget) | Implemented |
| **PME — Portfolio Management** | `crates/portfolio-supervisor` (safety-state reporting, position/exposure/capital/portfolio layers, mark-to-market) | `PortfolioDashboard` (live fetches: overview, positions, exposure, capital, safety) | **Yes — informational** (read-only; the TAE executor applies the single safety soft gate) | Implemented (informational) |
| **PAE — Performance Analytics** | `crates/performance-analytics` (stats compiler, NHST strategy analytics, risk analytics, performance layer, strategy optimizer, **backtest runner**) | `PerformanceDashboard` (fetches real data incl. the **backtest tab**: form, edge verdict, equity curve) | **Yes — implemented** | Implemented |
| **Cross-cutting** (DTOs, config, API gateway, execution-daemon) | All 4 crates | App shell, store, websocket, settings, watchlist scanner | **Yes — implemented** | Implemented |

### 1.1 Three honest categories

The platform is **not** in two categories ("done" vs. "not started"). It is in three:

1. **Implemented** — the feature is wired end-to-end, exercised by integration tests, and observable in the running system. **DIE and MME are in this bucket.**
2. **WIP — backend code present, but not production-ready** — there is real Rust code that compiles, runs, and produces state, but the dashboard that surfaces the state is a hardcoded mock, or a portion of the API surface is intentionally not wired, or the operational mode (e.g. paper vs. live) is restricted. **TAE, PME, and PAE are in this bucket.**
3. **Not yet started** — only the spec exists; no Rust code, no UI, no API surface. **There is currently no engine or major feature in this bucket** (the only items still here are sub-features of the WIP engines; see §5).

> **Why "WIP" instead of "implemented" or "not yet started".** Calling TAE/PME/PAE "implemented" would imply the operator can drive a live trading session from the dashboards today; that is **false**. Calling them "not yet started" would imply no code exists; that is also **false**. The backend modules compile and run, the in-process TAE event loop spawns and consumes the veto channel, the paper matching engine fills orders, and the PAE scheduled tasks tick — but the dashboards that an operator clicks on to see any of this render hardcoded arrays of fake data. Until the dashboards fetch from the live API, none of the three engines can be considered production-complete.

---

## 2. Engine-by-engine reality (what is and is not working today)

### 2.1 DIE — Data Infrastructure Engine

- **WebSocket ingestion** of Hyperliquid and Bitget ticks + order-book deltas — live, both venues.
- **NTP clock monitor** with a 10 ms default UTC drift budget (`[clock_monitor] threshold_micros`); configurable `BreachAction` (`Warn` / `Panic`).
- **Candle reconstruction** on reconnect gaps (≥ 1 min: REST backfill; < 1 min: EMA / last-N synthesis).
- **Connection-quality tracker** (rolling 1h / 6h / 24h windows, composite score, 60-second persistence loop).
- **Distribution channel** (`NormalizedCandle` broadcast) and `MarketSnapshot` analytical channel.
- **DB schema** with 30 migrations covering snapshot persistence, connection-quality, safety state, lifecycle, and PAE.

The Data Infrastructure dashboard (`ui/src/components/DataInfraDashboard.svelte`) reads `app.connectionQuality`, polls `GET /api/system/clock`, `GET /api/exchange-status`, and `GET /api/data-quality`, and surfaces the live numbers.

### 2.2 MME — Market Monitoring Engine

- **52 indicators** across 8 functional groups (Trend, Momentum, Volume, Volatility, Structure, Regime, Institutional, Derivatives Data; the v6.6 `mark_index_spread` registry entry moved Derivatives to 8 rows and the total to 51, and the v6.11 `price_trend_sharpe` entry moved Regime to 5 rows and the total to 52 — see [01-01-ontology.md Appendix B §B.2](conceptual-foundations/01-01-ontology.md)).
- **4 configurable timeframes** (micro / fast / slow / macro), each with its own `TimeframePipeline`.
- **12 `SignalKind` types** with 100 `(indicator, SignalKind)` declarations.
- **10-dimension Alignment Matrix**, **Analysis Matrix**, **Opportunity Matrix**, **Risk Matrix**, **Decision Matrix**, **Overview Matrix**.
- **Liquidity Intelligence Phases 0–2** (derivatives telemetry, liquidation flow, cluster matrix) feeding the L1.5 / L2.5 fractional layers; Phase 3 (cascade-risk aggregation) and Phase 4 (price-chart cluster overlay) are progressively shipping.
- **Multi-timeframe `MarketContext` synthesis** consumed by the Decision Layer.

The MME dashboards (`LiveTerminal`, `AlignmentPanel`, `OpportunitiesPanel`, `RiskPanel`, `AnalysisPanel`, `RecommendationPanel`, `LiquidityPanel`) all consume the WebSocket-fed `app.instancesMap[*].microTerm / fastTerm / slowTerm / macroTerm` state — no hardcoded data.

### 2.3 TAE — Trade Automation Engine

**v7 redesign (2026-08-18).** The policy engine was **erased**. The TAE is now a **setup executor** that consumes the MME's top setup directly (best Actionable/READY profile across the 4 TF snapshots) and manages the trade lifecycle (entry limit at zone midpoint → TP/SL bracket → LEVEL/SIGNAL invalidation) through a **single unified execution engine** whose only mode-dependent part is the `ExecutionBackend` (`PaperSimulation` today; a `LiveBroker` later — same fees/slippage/funding/PnL accounting in both modes). See [03-03-01-tae-overview-spec.md](engines/trade-automation-engine/03-03-01-tae-overview-spec.md).

**Backend (real, v7):**

- `crates/portfolio-supervisor/src/setup_executor.rs` — `extract_top_setup` (4-TF aggregation, Actionable/READY/min-RR filter, zone-midpoint geometry), per-symbol state machine (Idle → PendingEntry → PositionOpen), LEVEL/SIGNAL/REPLACED invalidation, direction-flip market close, safety + lifecycle gates, global position cap, setup-fingerprint dedup, `compute_risk` sizing + projected risk/return.
- `crates/portfolio-supervisor/src/execution/{engine,backend,state_machine}.rs` — unified `ExecutionEngine` (orders, positions, equity ledger, fees/slippage/funding), `ExecutionBackend` trait + `PaperSimulation` (limit/stop/market fills, instant marketable-limit fills, SL-before-TP on gaps, bracket cleanup), canonical persistence to `paper_trades` / `trade_telemetry_history` / `portfolio_equity_history` / `automation_activity` + `tae_open_state` restart-recovery tables.
- `crates/execution-daemon/src/main.rs` — 1s setup-executor loop per instance (reads all 4 TF buffers, fills → executor tick → equity sync), 8h funding, STOPPING flatten, boot recovery.
- **Erased:** `policy/`, `trigger_engine.rs`, `veto_loop.rs`, `execution/gates.rs`, `execution/order.rs` (sizing folded into the executor), decision-profiles API, pre-dispatch/manual open-close routes, `execution_policies`/`Stance`/`TriggerMode` config.

**Frontend (live):**

- `ui/src/components/TradeAutomationDashboard.svelte` — live fetches: `/api/instances/:id/automation` (mode badge, tracked setup + projected risk/return, order board, position card with manual Close now, invalidation banner, activity log), `/api/trade-ledger` (trade history). PAPER/LIVE badge; no placeholder data.

**API (served):** `GET /api/instances/:id/automation`, `POST /api/instances/:id/automation/close`, `GET /api/instances/:id/portfolio` (real positions/equity), `GET /api/instances/:id/safety`.

### 2.4 PME — Portfolio Management Engine

**v7 redesign (2026-08-18).** PME is now **purely informational**: the veto/stance machinery was erased; PME maintains the safety-state ladder (WARN / CAUTIOUS / SUSPENDED / DRAWDOWN_STOP) as a read-only status that the TAE setup executor's soft gate consumes before opening new entries. See [03-04-01-pme-overview-spec.md](engines/portfolio-management-engine/03-04-01-pme-overview-spec.md).

**Backend (real, v7):**

- `crates/portfolio-supervisor/src/safety.rs` — `SafetyManager::update(equity)` per tick (peak equity, drawdown → DRAWDOWN_STOP, daily → WARN); `record_trade_outcome` on close (CAUTIOUS/SUSPENDED ladder); `paper_balances` persistence; informational resets (session, consecutive losses, release).
- `crates/portfolio-supervisor/src/{position_layer,exposure_layer,capital_layer,portfolio_layer}.rs` — pure `Decimal` math, unchanged; `PortfolioMatrix` stance fields removed; peak equity now real.
- `crates/portfolio-supervisor/src/execution/engine.rs` — mark-to-market per tick (live unrealized PnL) + last-close outcome tracking feeding `record_trade_outcome`.
- **Erased:** `portfolio_risk.rs` (uncalled), `SafetyManager::evaluate_all` (VetoTrigger emission), manual stance, `check_allow_trade`.

**Frontend (live):**

- `ui/src/components/PortfolioDashboard.svelte` — live fetches: `/api/instances/:id/portfolio` (Overview / Positions / Exposure / Capital panels) + `/api/instances/:id/safety` (Safety panel). Informational resets only; no placeholder data.

**API (served, read-only):** `GET /api/instances/:id/portfolio` (rich), `GET /api/instances/:id/exposure`, `GET /api/instances/:id/capital`, `GET /api/instances/:id/safety` (extended), `POST /api/instances/:id/safety/session-reset`.

### 2.5 PAE — Performance Analytics Engine

**Backend (real):**

- `crates/performance-analytics/src/{stats_compiler,strategy_analytics,risk_analytics,performance_layer,strategy_optimizer,performance_evaluator,backtest}.rs` — all layer modules real; the v7 backtest runner (`backtest.rs`) replays recorded decision matrices through the unchanged setup executor + unified paper engine and applies the full NHST treatment (t-test, 10k Monte Carlo, α = 0.05, edge classification) to the simulated trades.
- Strategy analytics grouped by **setup type** (`trigger_source`) — the v7 successor of per-policy grouping.
- `run_performance_evaluator` — 300-second cadence; `run_strategy_optimizer` — 1-hour cadence with persisted `OptimizationReport`.

**Frontend (real):**

- `ui/src/components/PerformanceDashboard.svelte` — six live fetches (`/api/dashboard/stats`, `/api/analytics/strategy|risk|performance|optimization`, `/api/trades`); Strategy panel shows per-setup-type NHST (t, p, Monte Carlo p, α = 0.05, edge verdict); **Backtesting tab is real** (form → `POST /api/backtest/run` → stat cards, edge verdict, trade log, equity-curve chart; results survive restart via `GET /api/backtest/:id`).

**API (served):** `POST /api/backtest/run`, `GET /api/backtest/:id`.

### 2.6 Cross-cutting layers

- **`core-domain`** — All DTOs (`MarketSnapshot`, matrices, indicator value types), JSON-RPC envelopes, liquidity module.
- **`config-models`** — `load_config()` / `load_instances()`, every `*Config` struct.
- **`api-gateway`** — Axum router, WebSocket broadcast, 60+ HTTP routes including `/api/instances/:id/start`, `/api/instances/:id/pause`, `/api/instances/:id/stop`.
- **`database-storage`** — 30 migrations, WAL telemetry logger, query layer, encryption helpers.
- **`execution-daemon`** — 813-line `main.rs` that wires everything together in `--web` mode.
- **App shell** (`ui/src/App.svelte`, `state.svelte.ts`, `lib/websocket.svelte.ts`, `lib/api.svelte.ts`, `lib/router.svelte.ts`) — Singleton `AppStore`, WS demux with infinite-loop avoidance, hash-fragment routing, per-tab export builders.

---

## 3. Phased delivery plan (TAE → PME → PAE → Production)

Each phase ships when its acceptance criteria pass and the verification checklist (§6) reports `OK` end-to-end. Status transitions are recorded in [docs/CHANGELOG.md](./CHANGELOG.md) under a new `## vX.Y` header with a `Released:` stamp.

### Phase A — "Wire the dashboards" (TAE + PME frontend, ~1 sprint)

| Item | Owner | Acceptance criterion |
|---|---|---|
| A1. `TradeAutomationDashboard` fetches live v7 automation state | UI | ✅ Delivered (2026-08-18): the dashboard polls `/api/instances/:id/automation` + `/api/trade-ledger`; no `Placeholder data` comment in source |
| A2. `PortfolioDashboard` fetches `/api/instances/:id/portfolio`, `/api/instances/:id/safety`, `/api/instances/:id/exposure`, `/api/instances/:id/capital`, `/api/instances/:id/veto` | UI | All five sidebar panels render live data; safety state banner reflects `safety_state` from the API |
| A3. Replace `// ── Placeholder data ───` with `// ── Live data ───` and call the relevant `fetch` | UI | grep `'// ── Placeholder data ───'` returns 0 matches in `ui/src/components/TradeAutomationDashboard.svelte` and `ui/src/components/PortfolioDashboard.svelte` |
| A4. Render Backend integration point sanity tests | UI + Rust | Vitest suite asserts each panel renders API data |
| A5. Remove "Dashboard → Engine Map" placeholder wording in `docs/ui-ux/07-02-ui-dashboard-layout.md §5.3` | Docs | New wording reflects "live data" for TAE/PME panels |

### Phase B — "TAE end-to-end paper trading" ✅ (delivered as the v7 redesign, 2026-08-18)

| Item | Owner | Acceptance criterion |
|---|---|---|
| B1. `LifecycleState` enum + `instance.automation` struct + `[candle_buffer]` config block | `config-models`, `database-storage` | AUDIT-V6-202, AUDIT-V6-203, AUDIT-V7-300 … V7-307 closed in CHANGELOG §Open Items |
| B2. `POST /api/instances/:id/start`, `/api/instances/:id/pause`, `/api/instances/:id/stop` actually drive the engine | `api-gateway`, `portfolio-supervisor` | AUDIT-V6-204 closed; integration test boots an instance, starts it, pauses it, stops it, asserts lifecycle transitions |
| B3. Gate 0 (lifecycle) is enforced | `portfolio-supervisor` | AUDIT-V6-205 closed; integration test confirms `STOPPED` blocks entries |
| B4. STOP flatten orchestration: cancel-all + market-close with `is_emergency_liquidation = true`, `reduce_only = true` | `execution-daemon` | AUDIT-V6-206 closed; integration test confirms flatten behavior |
| B5. `TradeAutomationDashboard` lifecycle panel drives start/pause/stop via the new endpoints | UI | Phase A1 + AUDIT-V6-207 closed |

> **v7 supersession (2026-08-18).** Phase B was delivered through a **different design**: the policy/trigger/veto machinery (the subject of B1–B5) was **erased** and replaced by the setup executor + unified execution engine ([03-03-01-tae-overview-spec.md](engines/trade-automation-engine/03-03-01-tae-overview-spec.md)). Lifecycle start/pause/stop, STOP flatten, and the dashboard all exist in the v7 form. The v6 audit IDs B1–B5 tracked are therefore **superseded** by the v7 design rather than closed against it.

### Phase C — "PME informational surface" ✅ (delivered as the v7 redesign, 2026-08-18)

| Item | Owner | Acceptance criterion |
|---|---|---|
| C1. `ConfigurableActivation`: denylists, `config_version`, `AUTO_PAUSED` | `config-models`, `market-analyzer`, `core-domain`, `database-storage`, `api-gateway`, `portfolio-supervisor`, UI | **Runtime wiring shipped** (2026-08-17 MME audit sweep): AUDIT-V6-208…210 closed, V6-211 cancelled (live-wire attribution, not persisted), V6-212 (activation REST surface) + V6-213 (`AUTO_PAUSED` policy state) + V6-214 (activation panel) remain open |
| C1b. `instances[*].custom_pipelines` runtime wiring (custom TF slots) | `portfolio-supervisor`, `api-gateway` | PRI-07 code paths exist; until wired, config validation **rejects** custom-pipeline declarations at boot (fail-fast, see `config-models` `CustomTimeframesUnsupported`) |
| C2. `PortfolioDashboard` activation panel renders live state | UI | Phase A2 + AUDIT-V6-214 closed |
| C3. `safety_state` deterministic reconstruction algorithm unit-tested | `portfolio-supervisor`, `database-storage` | AUDIT-V4-046 closed |
| C4. Pre-dispatch crash-recoverable persistence | `database-storage`, `api-gateway`, `portfolio-supervisor` | AUDIT-V4-079 closed; new `pre_dispatch_orders` table added |
| C5. PME / TAE communication contracts under load | `portfolio-supervisor` | Stress test demonstrates veto-loop responsiveness under 10 symbols × 4 TFs |

> **v7 supersession (2026-08-18).** Phase C was delivered through a **different design**: the veto/stance/pre-dispatch/activation-panel items (C2–C5) were **erased or superseded** by the informational PME ([03-04-01-pme-overview-spec.md](engines/portfolio-management-engine/03-04-01-pme-overview-spec.md)). The PME surface that remains — rich `/portfolio`, `/exposure`, `/capital`, extended `/safety`, live dashboard — is delivered; the v6 audit IDs were superseded rather than closed against them.

### Phase D — "PAE backtesting" ✅ (delivered 2026-08-18)

| Item | Owner | Acceptance criterion |
|---|---|---|
| D1. `POST /api/backtest/run` + `GET /api/backtest/:id` endpoints | `api-gateway`, `performance-analytics`, `portfolio-supervisor` | **Delivered**: recorded decision matrices are replayed through the unchanged setup executor + paper engine; results (summary, NHST stats, trades, equity curve) persist to `backtest_runs`; integration tests assert a run round-trips. |
| D2. Backtest engine: replay historical `market_snapshots` through the existing TAE event loop with paper-trading only | `performance-analytics` | **Delivered** as the v7 `BacktestRunner` (recorded-decision replay through the setup executor — see [03-05-06](engines/performance-analytics-engine/03-05-06-pae-layer5-backtest.md)); deterministic, bounded, paper-only. |
| D3. Equity curve chart: replace "Equity curve visualization coming soon" with a real render | UI | **Delivered**: lightweight-charts equity curve in the backtest tab. |
| D4. `liquidation_events` → PAE backtest ingestion | `core-domain`, `performance-analytics` | AUDIT-V4-080 — superseded by the recorded-decision replay design (liquidation clusters are already embedded in each recorded snapshot's opportunity matrix). |
| D5. PAE → DB feedback (persist analytical feedback to configuration databases for offline policy optimization) | `performance-analytics`, `database-storage` | AUDIT-V6-304 — superseded with the policy engine erasure (v7); the optimization report persists to `optimization_reports` for offline review. |

### Phase E — "Production hardening"

| Item | Owner | Acceptance criterion |
|---|---|---|
| E1. Live exchange adapter (Hyperliquid + Bitget order dispatch) | `network-adapters`, `portfolio-supervisor`, `execution-daemon` | **Delivered (v7.1):** `LiveBroker` (Hyperliquid, EIP-712) + `BitgetLiveBroker` (Bitget V5 HMAC) — see the [venue matrix](engines/trade-automation-engine/03-03-03-tae-layer2-execution.md); signing + keys + mode-toggle tests green |
| E2. In-process exchange-key rotation tool (`POST /api/keys/rotate`, SIGHUP hot rotation, encrypted-backup export) | `api-gateway`, `config-models` | AUDIT-V6-077 closed |
| E3. **Single-operator identity** — every audit event carries `operator_id = "local"`; no caller-supplied identity, no multi-client model (AUDIT-V4-076 **cancelled** by design) | `api-gateway`, docs | Delivered: `06-01 §1` single-operator statement; all audit surfaces stamped `local` |
| E4. DOD hot-path migration (f64 indicator signatures) | `market-analyzer` | AUDIT-V8-400 … V8-407 closed; `Indicator::BarInput` is `f64`; per-indicator `update()` is `f64` |
| E5. WS per-timeframe subscriptions | `api-gateway`, `network-adapters` | AUDIT-V6-302 closed |
| E6. Timeframe editor (operator-editable timeframe set) | `config-models`, `market-analyzer`, UI | AUDIT-V6-303 closed |

> **Rescinding the "WIP" label.** Each engine's row in §1 transitions from WIP to Implemented **only** when every phase whose first column names that engine has a passing acceptance criterion. Until then, the doc corpus — README, AGENTS.md, every `**Status:**` header, every UI banner, every docs banner — must continue to say **WIP**.

---

## 4. Delivery markers (v7.0 — removed)

The v6-era "visible WIP" inventory (documentation banners, UI amber banners, `// ── Placeholder data ───` anchors) is **fully retired**: every TAE/PME/PAE dashboard fetches live data, no placeholder comments or amber banners remain, and every engine row in §1 reads Implemented. This section is intentionally empty.

---

## 5. Audit register (all terminal — see docs/CHANGELOG.md §Open Items)

This section mirrors — and is subordinate to — [`docs/CHANGELOG.md §Open Items`](./CHANGELOG.md). The CHANGELOG is the canonical audit-ID register; this section is the canonical **WIP summary** with one line per item. Items appear here only when they block one of the engines from being labeled "Implemented".

| Audit ID | Phased by | Item |
|---|---|---|
| `AUDIT-V4-005` | Phase D | `cascade_risk_index` aggregation into `systemic_risk_score` (PAE) |
| `AUDIT-V4-046` | Phase C | `safety_state` deterministic reconstruction algorithm unit-tested (PME) |
| `AUDIT-V4-079` | Phase C | PriceChart marker overlay for cluster positions (UI; not part of the WIP engines but listed for completeness) |
| `AUDIT-V4-080` | Phase D | `liquidation_events` → PAE backtest ingestion |
| `AUDIT-V6-077` | Phase E | In-process exchange-key rotation tool (cross-cutting) |
| `AUDIT-V6-202` | Phase B | `LifecycleState` enum + `instance.automation` struct (TAE) |
| `AUDIT-V6-203` | Phase B | `instance_lifecycle` migrations (TAE) |
| `AUDIT-V6-204` | Phase B | `POST /api/instances/:id/start`, `/api/instances/:id/pause`, `/api/instances/:id/stop` (TAE) |
| `AUDIT-V6-205` | Phase B | Gate 0 (lifecycle) in pre-trade chain (TAE) |
| `AUDIT-V6-206` | Phase B | STOP flatten orchestration (TAE) |
| `AUDIT-V6-207` | Phase B | UI lifecycle badges (TAE) |
| `AUDIT-V6-208` … `AUDIT-V6-214` | Phase C | Configurable activation (TAE / PME / MME) |
| `AUDIT-V6-302` | Phase E | WS per-timeframe subscriptions (cross-cutting) |
| `AUDIT-V6-303` | Phase E | Timeframe editor (cross-cutting) |
| `AUDIT-V6-304` | Phase D | PAE → DB feedback (PAE) |
| `AUDIT-V6-401` | Phase A | Wire `TradeAutomationDashboard` to live API |
| `AUDIT-V6-402` | Phase A | Wire `PortfolioDashboard` to live API |
| `AUDIT-V6-403` | Phase D | `POST /api/backtest/run` + `GET /api/backtest/:id` |
| `AUDIT-V6-404` | Phase D | Replace `setTimeout` mock in `PerformanceDashboard.runBacktest` |
| `AUDIT-V6-405` | Phase D | Equity-curve chart |
| `AUDIT-V6-406` | Phase E | Live Hyperliquid + Bitget order-dispatch adapter |
| `AUDIT-V7-300` … `AUDIT-V7-307` | Phase B | CandleBufferConfig + per-indicator lifecycle (cross-cutting) |
| `AUDIT-V7-310` … `AUDIT-V7-314` | Phase B | CandlePipelineState + reload (cross-cutting) |
| `AUDIT-V7-320` … `AUDIT-V7-324` | Phase B | HistoricalFetchPolicy trait (DIE) |
| `AUDIT-V7-330` … `AUDIT-V7-334` | Phase B | IndicatorLifecycleState per registry entry (MME) |
| `AUDIT-V8-400` … `AUDIT-V8-407` | Phase E | DOD hot-path f64 indicator migration (MME) |

---

## 6. Verification checklist

Every item below must report `OK` before any "WIP" label can be removed from the engine it refers to. The check runs as part of the `./manage.sh test` and `./manage.sh test-doc` pipelines.

### 6.1 Documentation consistency

- [x] **`docs/ROADMAP.md` exists and is linked from `docs/README.md` and `README.md`**
- [x] **`docs/README.md §Feature Status` distinguishes Implemented / WIP / Not started for every engine and major feature** (all five engines: Implemented)
- [x] **Every `**Status:**` header in the engine docs reflects the v7.0 implemented state**
- [x] **`docs/conceptual-foundations/01-02-global-architecture.md §2.3-2.5` no WIP callouts remain**
- [x] **`docs/conceptual-foundations/01-06-crate-layout-and-cycles.md` no longer calls `performance-analytics` / `portfolio-supervisor` WIP** (both crate rows read Implemented — closed 2026-08-18)
- [x] **`docs/conceptual-foundations/01-02-global-architecture.md` §2.3 Layer 2 status callout reflects the implemented execution path** (paper default, live Hyperliquid/Bitget dispatch available — closed 2026-08-18)
- [x] **`README.md §Quick Start` carries the v7 implementation-status callout**
- [x] **`AGENTS.md §Project overview` reflects the completed v7.0 state**
- [x] **All numbered docs carry `**Version:** 7.1 (2026-08-18)` and the CHANGELOG top entry is `## v7.0 (2026-08-18)`**
- [x] **`./manage.sh test-doc`** passes (release gates G1–G16) — **ALL CHECKS PASSED** (2026-08-18)

### 6.2 Source-code verification

- [x] **No `// ── Placeholder data ───` comment in `TradeAutomationDashboard.svelte`** (v7 live dashboard — delivered 2026-08-18)
- [x] **No `// ── Placeholder data ───` comment in `PortfolioDashboard.svelte`** (v7 live dashboard — delivered 2026-08-18)
- [x] **`runBacktest` in `PerformanceDashboard.svelte` is replaced by a `fetch`** (v7 backtest tab — delivered 2026-08-18)
- [x] **No amber banner mounted in the engine dashboard indicates WIP status** (WIP badges removed from `AppEngineSidebar` — closed 2026-08-18)

### 6.3 Backend integration points

**TAE (v7 — served):**
- [x] **`GET /api/instances/:id/automation` returns the live setup-executor state** (mode, phase, tracked setup + projected risk/return, entry/bracket orders, position, invalidation state, activity log, safety gate, lifecycle, equity)
- [x] **`POST /api/instances/:id/automation/close`** cancels pending/bracket orders and closes the open position at market
- [x] **`GET /api/instances/:id/portfolio` returns live equity + open position**
- [x] **`GET /api/instances/:id/safety` returns the live safety state**
- [x] **`GET /api/trade-ledger` returns closed trades with `trigger_source` = setup type**

**Erased with the policy engine:** `/policies`, `/triggers`, `/paper/positions`, `/paper/orders`, `/paper/history`, `/veto`, pre-dispatch, manual open/close, decision-profiles — the v6 checklist items below no longer apply (the TAE surface is the `/automation` contract).

**PME (v7 — served, read-only):**
- [x] **`GET /api/instances/:id/portfolio`** returns the rich informational portfolio state (equity, PnL, drawdown, exposure, capital, positions, safety, systemic risk)
- [x] **`GET /api/instances/:id/exposure`** returns the Exposure Matrix
- [x] **`GET /api/instances/:id/capital`** returns the Capital Matrix + margin alert
- [x] **`GET /api/instances/:id/safety`** returns the safety state + context + drawdown/daily metrics
- [x] **`POST /api/instances/:id/safety/session-reset`** rebaselines peak equity + daily PnL (informational)
- [x] **`POST /api/backtest/run` + `GET /api/backtest/:id` exist and round-trip a result** (v7 — delivered 2026-08-18)

**Live trading (v7.1 — served):**
- [x] **`POST /api/keys`, `GET /api/keys`, `DELETE /api/keys/:id`, `POST /api/keys/rotate`, `GET /api/keys/backup`** — encrypted credential management (both venues)
- [x] **`POST /api/instances/:id/mode`** — engine-wide paper/live switch (requires a key; persists to config)
- [x] **Hyperliquid + Bitget live dispatch** via `ExecutionBackend` (see [03-03-03 §5b](engines/trade-automation-engine/03-03-03-tae-layer2-execution.md))

### 6.4 Tests

- [x] **`./manage.sh test-core`** passes
- [x] **`./manage.sh test-indicators`** passes
- [x] **`./manage.sh test-engine`** passes
- [x] **`./manage.sh test-ui`** passes
- [x] **`./manage.sh test-doc`** passes (release gates G1–G16) — **ALL CHECKS PASSED** (2026-08-18)
- [x] **`cargo fmt --all -- --check`** passes
- [x] **clippy (workspace, deny-lints)** passes
- [x] **`bun run check`** (svelte-check + tsc) passes

### 6.5 Final sign-off

- [x] **All WIP labels in §1 of this roadmap removed** (DIE, MME, TAE, PME, PAE)
- [x] **`docs/README.md §Feature Status` row for each engine reads "Implemented"**
- [x] **Amber UI banners removed from `TradeAutomationDashboard`, `PortfolioDashboard`, `PerformanceDashboard`**
- [x] **`docs/CHANGELOG.md` top entry reads `## v7.0 (2026-08-18) — TAE / PME / PAE production-ready`** with sub-bullets referencing the closed audit IDs

---

## 7. Cross-references

- [docs/README.md](./README.md) — entry point for the documentation corpus
- [docs/CHANGELOG.md](./CHANGELOG.md) — canonical audit-ID register, version history
- [docs/DOCS-CONSISTENCY-MANIFEST.md](./DOCS-CONSISTENCY-MANIFEST.md) — release-gate documentation
- [docs/conceptual-foundations/01-02-global-architecture.md](./conceptual-foundations/01-02-global-architecture.md) — five-engine blueprint
- [docs/conceptual-foundations/01-06-crate-layout-and-cycles.md](./conceptual-foundations/01-06-crate-layout-and-cycles.md) — physical crate layout
- [docs/conceptual-foundations/01-07-target-architecture-roadmap.md](./conceptual-foundations/01-07-target-architecture-roadmap.md) — target-architecture tracker (orthogonal to this implementation roadmap; future design improvements vs. delivery of the as-spec'd system)
- [docs/operations-and-compliance/08-01-user-manual.md](./operations-and-compliance/08-01-user-manual.md) — operator guide
- [AGENTS.md](../AGENTS.md) — developer guide
