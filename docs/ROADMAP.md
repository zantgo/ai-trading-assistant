# Implementation Roadmap

**Version:** 6.8 (2026-08-03) — see [docs/CHANGELOG.md](./CHANGELOG.md) for the canonical version history.
**Status:** In progress — partial implementation; multiple engines still in WIP.
**Purpose:** This document is the **single source of truth for what is and is not built in the Trading Platform today**, and the **phased delivery plan** for the engines, layers, and dashboards that remain on the workbench. Every spec in `docs/` describes the **target system**; this roadmap tracks **actual delivery status**, names the work that is still in flight, and gives a checklist the operator (and the next maintainer) can run to verify the platform's behaviour against the documentation.

> **Reading order.** Read §1 for the high-level status picture, §2 for the engine-by-engine reality, §3 for the phased delivery plan, §4 for the visible WIP markers (UI banners, docs banners, code annotations), §5 for the canonical list of known WIP items with the audit IDs that already track them, and §6 for the **verification checklist** that must pass before any of the "WIP" labels can be removed.

---

## 1. High-level status picture

| Engine | Backend code | Frontend | Production-ready? | Status |
|---|---|---|---|---|
| **DIE — Data Infrastructure** | `crates/network-adapters`, `crates/database-storage`, L2–L4 in `crates/market-analyzer` | `DataInfraDashboard` (live fetches) | **Yes — implemented** | ✅ Implemented |
| **MME — Market Monitoring** | `crates/market-analyzer` (50 indicators, 4-TF pipeline, signals, multi-TF synthesis, MarketContext, Decision Matrix) | `LiveTerminal`, `TerminalMonitor`, `AlignmentPanel`, `OpportunitiesPanel`, `RiskPanel`, `AnalysisPanel`, `RecommendationPanel`, `LiquidityPanel`, `StructuralAnchorsStrip`, `MarketContextStrip` (all WS-fed) | **Yes — implemented** | ✅ Implemented |
| **TAE — Trade Automation** | `crates/portfolio-supervisor` (Policy engine, Execution engine, paper trading, lifecycle manager, trigger engine, veto loop) | `TradeAutomationDashboard` (hardcoded placeholder data — does not fetch) | **No — backend present, frontend is a placeholder; not production-ready** | ⚠️ WIP |
| **PME — Portfolio Management** | `crates/portfolio-supervisor` (Safety manager, position/exposure/capital/portfolio layers, registry) | `PortfolioDashboard` (hardcoded placeholder data — does not fetch) | **No — backend present, frontend is a placeholder; not production-ready** | ⚠️ WIP |
| **PAE — Performance Analytics** | `crates/performance-analytics` (stats compiler, performance evaluator, strategy analytics, risk analytics, strategy optimizer) | `PerformanceDashboard` (fetches real data; **Backtesting** tab is a UI-only mock) | **Partial — analytics APIs work; backtesting UI is a placeholder; not production-ready** | ⚠️ WIP |
| **Cross-cutting** (DTOs, config, API gateway, execution-daemon) | All 4 crates | App shell, store, websocket, settings, watchlist scanner | **Yes — implemented** | ✅ Implemented |

### 1.1 Three honest categories

The platform is **not** in two categories ("done" vs. "not started"). It is in three:

1. **Implemented** — the feature is wired end-to-end, exercised by integration tests, and observable in the running system. **DIE and MME are in this bucket.**
2. **WIP — backend code present, but not production-ready** — there is real Rust code that compiles, runs, and produces state, but the dashboard that surfaces the state is a hardcoded mock, or a portion of the API surface is intentionally not wired, or the operational mode (e.g. paper vs. live) is restricted. **TAE, PME, and PAE are in this bucket.**
3. **Not yet started** — only the spec exists; no Rust code, no UI, no API surface. **There is currently no engine or major feature in this bucket** (the only items still here are sub-features of the WIP engines; see §5).

> **Why "WIP" instead of "implemented" or "not yet started".** Calling TAE/PME/PAE "implemented" would imply the operator can drive a live trading session from the dashboards today; that is **false**. Calling them "not yet started" would imply no code exists; that is also **false**. The backend modules compile and run, the in-process TAE event loop spawns and consumes the veto channel, the paper matching engine fills orders, and the PAE scheduled tasks tick — but the dashboards that an operator clicks on to see any of this render hardcoded arrays of fake data. Until the dashboards fetch from the live API, none of the three engines can be considered production-complete.

---

## 2. Engine-by-engine reality (what is and is not working today)

### 2.1 DIE — Data Infrastructure Engine ✅

- **WebSocket ingestion** of Hyperliquid and Bitget ticks + order-book deltas — live, both venues.
- **NTP clock monitor** with ≤50 µs UTC drift budget; configurable `BreachAction` (`Warn` / `Panic`).
- **Candle reconstruction** on reconnect gaps (≥ 1 min: REST backfill; < 1 min: EMA / last-N synthesis).
- **Connection-quality tracker** (rolling 1h / 6h / 24h windows, composite score, 60-second persistence loop).
- **Distribution channel** (`NormalizedCandle` broadcast) and `MarketSnapshot` analytical channel.
- **DB schema** with 30 migrations covering snapshot persistence, connection-quality, safety state, lifecycle, and PAE.

The Data Infrastructure dashboard (`ui/src/components/DataInfraDashboard.svelte`) reads `app.connectionQuality`, polls `GET /api/system/clock`, `GET /api/exchange-status`, and `GET /api/data-quality`, and surfaces the live numbers.

### 2.2 MME — Market Monitoring Engine ✅

- **50 indicators** across 8 functional groups (Trend, Momentum, Volume, Volatility, Structure, Regime, Institutional, Derivatives Data).
- **4 configurable timeframes** (micro / fast / slow / macro), each with its own `TimeframePipeline`.
- **12 `SignalKind` types** with 100 `(indicator, SignalKind)` declarations.
- **10-dimension Alignment Matrix**, **Analysis Matrix**, **Opportunity Matrix**, **Risk Matrix**, **Decision Matrix**, **Overview Matrix**.
- **Liquidity Intelligence Phases 0–2** (derivatives telemetry, liquidation flow, cluster matrix) feeding the L1.5 / L2.5 fractional layers; Phase 3 (cascade-risk aggregation) and Phase 4 (price-chart cluster overlay) are progressively shipping.
- **Multi-timeframe `MarketContext` synthesis** consumed by the Decision Layer.

The MME dashboards (`LiveTerminal`, `AlignmentPanel`, `OpportunitiesPanel`, `RiskPanel`, `AnalysisPanel`, `RecommendationPanel`, `LiquidityPanel`) all consume the WebSocket-fed `app.instancesMap[*].microTerm / fastTerm / slowTerm / macroTerm` state — no hardcoded data.

### 2.3 TAE — Trade Automation Engine ⚠️

**Backend (real, partial):**

- `crates/portfolio-supervisor/src/policy/{engine,evaluator,veto}.rs` — Policy engine with stance gating, conflict resolution (opposite-direction block, same-direction keep-strongest), cooldown handling, interval / candle-close / event-driven trigger modes.
- `crates/portfolio-supervisor/src/execution/{engine,gates,state_machine}.rs` — Position sizing `S = E·R / (Dₛₗ / 100)`, pre-trade gate chain (`crates/portfolio-supervisor/src/execution/gates.rs`, 490 lines, 32 tests), order lifecycle state machine.
- `crates/portfolio-supervisor/src/paper_trading.rs` — simulated matching engine in `Decimal`, with `submit_order` and `evaluate_order_fills`. **This is the default and currently the only execution path**; there is no live exchange order-dispatch code.
- `crates/portfolio-supervisor/src/{lifecycle,trigger_engine,veto_loop}.rs` — Lifecycle manager, trigger matrix, 5-second veto loop spawning `VetoEvent`s that feed back into the TAE.
- Wired in `crates/execution-daemon/src/main.rs` lines 321–718 — spawns the TAE event loop, drains the veto channel, and submits to the paper engine.

**Frontend (placeholder):**

- `ui/src/components/TradeAutomationDashboard.svelte` — line 15 carries the comment `// ── Placeholder data ───`. The component declares hardcoded arrays (`operationalMode`, `overviewStats`, `policies`, `observability`, `lifecycleInstances`) and never calls `fetch`. The five sidebar panels (Overview, Policies, Observability, Paper, Lifecycle) all render the static arrays.
- **Backend integration points that exist but are not wired in this dashboard:** `/api/instances/:id/policies`, `/api/instances/:id/triggers`, `/api/instances/:id/lifecycle`, `/api/instances/:id/paper/positions`, `/api/instances/:id/paper/orders`, `/api/instances/:id/paper/history`.

> **What works.** The backend TAE event loop runs, paper-trades execute, veto events fire, and the data is observable through the API surface. **What does not work.** The dashboard that an operator would use to inspect a policy or a paper-trade is a hand-written mock. **This is not a production-complete trading experience.**

### 2.4 PME — Portfolio Management Engine ⚠️

**Backend (real, partial):**

- `crates/portfolio-supervisor/src/safety.rs` — `SafetyManager` with `consecutive_losses` tracker, dropout timer, drawdown evaluation, daily-PnL tracking, manual stance override, SQLite persistence.
- `crates/portfolio-supervisor/src/{capital_layer,exposure_layer,position_layer,portfolio_risk,portfolio_equity}.rs` — Capital, exposure, position, portfolio-risk, and equity layers; `position_layer.rs` `check_invalidation_breach`.
- `crates/portfolio-supervisor/src/{risk_calculator,commission}.rs` — Risk and commission math.
- `crates/portfolio-supervisor/src/registry/` — Full `add_instance` lifecycle with symbol-availability check, historical bootstrap, multi-TF pipeline build, buffer population, persistence to `config.toml`.
- `crates/portfolio-supervisor/src/workspace_state.rs` — Workspace aggregate.

**Frontend (placeholder):**

- `ui/src/components/PortfolioDashboard.svelte` — line 14 carries the comment `// ── Placeholder data ───`. The component declares hardcoded arrays (`safetyState`, `portfolioSummary`, `positions`, `concentration`, `stances`, `vetoTriggers`, `correlationMatrix`) and never calls `fetch`. The five sidebar panels (Overview, Positions, Exposure, Capital, Safety) all render the static arrays.
- **Backend integration points that exist but are not wired in this dashboard:** `/api/instances/:id/portfolio`, `/api/instances/:id/safety`, `/api/instances/:id/exposure`, `/api/instances/:id/capital`, `/api/instances/:id/veto`.

> **What works.** The SafetyManager tick produces authoritative state; the veto loop cascades into the TAE; the capital ledger persists across restarts. **What does not work.** The dashboard that an operator would use to read the safety state, exposure, or capital is a hand-written mock.

### 2.5 PAE — Performance Analytics Engine ⚠️

**Backend (real, partial):**

- `crates/performance-analytics/src/{stats_compiler,strategy_analytics,risk_analytics,performance_layer,strategy_optimizer,performance_evaluator}.rs` — All four layer modules are real and exercised by integration tests.
- `run_performance_evaluator` — 300-second cadence; spawn in `crates/execution-daemon/src/main.rs` lines 770–810.
- `run_strategy_optimizer` — 1-hour cadence regime analyzer with persisted `OptimizationReport`.

**Frontend (partial):**

- `ui/src/components/PerformanceDashboard.svelte` — lines 41–63 wire six real `fetch()` calls to `/api/dashboard/stats`, `/api/analytics/strategy`, `/api/analytics/risk`, `/api/analytics/performance`, `/api/analytics/optimization`, `/api/analytics/trades` and render real data into Overview / Strategy / Risk / Regimes / Trades panels.
- **However**, the **Backtesting** panel (lines 25–37) uses a `setTimeout(() => { btRunning = false; btResultsReady = true; }, 1200)` UI mock. There is no `/api/backtest` route; backtest runs are **not implemented**.
- Equity curve "visualization coming soon" placeholder card is rendered for every backtest.

> **What works.** Analytics: dashboard stats, strategy NHST, risk metrics, regime map, trade ledger — all read live from the PAE pipeline. **What does not work.** Backtesting is a UI-level mock.

### 2.6 Cross-cutting layers ✅

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
| A1. `TradeAutomationDashboard` fetches `/api/instances/:id/policies`, `/api/instances/:id/triggers`, `/api/instances/:id/paper/positions`, `/api/instances/:id/paper/orders`, `/api/instances/:id/paper/history`, `/api/instances/:id/lifecycle` | UI | All five sidebar panels render live data; no `Placeholder data` comment in source |
| A2. `PortfolioDashboard` fetches `/api/instances/:id/portfolio`, `/api/instances/:id/safety`, `/api/instances/:id/exposure`, `/api/instances/:id/capital`, `/api/instances/:id/veto` | UI | All five sidebar panels render live data; safety state banner reflects `safety_state` from the API |
| A3. Replace `// ── Placeholder data ───` with `// ── Live data ───` and call the relevant `fetch` | UI | grep `'// ── Placeholder data ───'` returns 0 matches in `ui/src/components/TradeAutomationDashboard.svelte` and `ui/src/components/PortfolioDashboard.svelte` |
| A4. Render Backend integration point sanity tests | UI + Rust | Vitest suite asserts each panel renders API data |
| A5. Remove "Dashboard → Engine Map" placeholder wording in `docs/ui-ux/07-02-ui-dashboard-layout.md §5.3` | Docs | New wording reflects "live data" for TAE/PME panels |

### Phase B — "TAE end-to-end paper trading"

| Item | Owner | Acceptance criterion |
|---|---|---|
| B1. `LifecycleState` enum + `instance.automation` struct + `[candle_buffer]` config block | `config-models`, `database-storage` | AUDIT-V6-202, AUDIT-V6-203, AUDIT-V7-300 … V7-307 closed in CHANGELOG §Open Items |
| B2. `POST /api/instances/:id/start`, `/api/instances/:id/pause`, `/api/instances/:id/stop` actually drive the engine | `api-gateway`, `portfolio-supervisor` | AUDIT-V6-204 closed; integration test boots an instance, starts it, pauses it, stops it, asserts lifecycle transitions |
| B3. Gate 0 (lifecycle) is enforced | `portfolio-supervisor` | AUDIT-V6-205 closed; integration test confirms `STOPPED` blocks entries |
| B4. STOP flatten orchestration: cancel-all + market-close with `is_emergency_liquidation = true`, `reduce_only = true` | `execution-daemon` | AUDIT-V6-206 closed; integration test confirms flatten behavior |
| B5. `TradeAutomationDashboard` lifecycle panel drives start/pause/stop via the new endpoints | UI | Phase A1 + AUDIT-V6-207 closed |

### Phase C — "PME end-to-end safety + stance control"

| Item | Owner | Acceptance criterion |
|---|---|---|
| C1. `ConfigurableActivation`: denylists, `config_version`, `AUTO_PAUSED` | `config-models`, `market-analyzer`, `core-domain`, `database-storage`, `api-gateway`, `portfolio-supervisor`, UI | AUDIT-V6-208 … V6-214 closed in CHANGELOG §Open Items |
| C2. `PortfolioDashboard` activation panel renders live state | UI | Phase A2 + AUDIT-V6-214 closed |
| C3. `safety_state` deterministic reconstruction algorithm unit-tested | `portfolio-supervisor`, `database-storage` | AUDIT-V4-046 closed |
| C4. Pre-dispatch crash-recoverable persistence | `database-storage`, `api-gateway`, `portfolio-supervisor` | AUDIT-V4-079 closed; new `pre_dispatch_orders` table added |
| C5. PME / TAE communication contracts under load | `portfolio-supervisor` | Stress test demonstrates veto-loop responsiveness under 10 symbols × 4 TFs |

### Phase D — "PAE backtesting"

| Item | Owner | Acceptance criterion |
|---|---|---|
| D1. `POST /api/backtest/run` + `GET /api/backtest/:id` endpoints | `api-gateway`, `performance-analytics`, `portfolio-supervisor` | New audit IDs registered; integration test posts a backtest request and asserts a result row |
| D2. Backtest engine: replay historical `market_snapshots` through the existing TAE event loop with paper-trading only | `performance-analytics` | Integration test asserts win-rate / drawdown within ±5% of a known reference scenario |
| D3. Equity curve chart: replace "Equity curve visualization coming soon" with a real render | UI | AUDIT-V6-304 closed; new audit ID for backtest equity curve |
| D4. `liquidation_events` → PAE backtest ingestion | `core-domain`, `performance-analytics` | AUDIT-V4-080 closed |
| D5. PAE → DB feedback (persist analytical feedback to configuration databases for offline policy optimization) | `performance-analytics`, `database-storage` | AUDIT-V6-304 closed |

### Phase E — "Production hardening"

| Item | Owner | Acceptance criterion |
|---|---|---|
| E1. Live exchange adapter (Hyperliquid + Bitget order dispatch) | `network-adapters`, `portfolio-supervisor`, `execution-daemon` | New audit IDs registered; integration test submits a `reduce_only` order to a mock adapter |
| E2. In-process exchange-key rotation tool (`POST /api/keys/rotate`, SIGHUP hot rotation, encrypted-backup export) | `api-gateway`, `config-models` | AUDIT-V6-077 closed |
| E3. Caller-supplied `X-Operator-Id` identity | `api-gateway` | AUDIT-V4-076 closed; auth contract documented |
| E4. DOD hot-path migration (f64 indicator signatures) | `market-analyzer` | AUDIT-V8-400 … V8-407 closed; `Indicator::BarInput` is `f64`; per-indicator `update()` is `f64` |
| E5. WS per-timeframe subscriptions | `api-gateway`, `network-adapters` | AUDIT-V6-302 closed |
| E6. Timeframe editor (operator-editable timeframe set) | `config-models`, `market-analyzer`, UI | AUDIT-V6-303 closed |

> **Rescinding the "WIP" label.** Each engine's row in §1 transitions from ⚠️ WIP to ✅ Implemented **only** when every phase whose first column names that engine has a passing acceptance criterion. Until then, the doc corpus — README, AGENTS.md, every `**Status:**` header, every UI banner, every docs banner — must continue to say **WIP**.

---

## 4. Visible WIP markers (where the platform tells the operator "this is not done")

This section is the canonical inventory of every place the platform surfaces its WIP status to the reader, viewer, or operator. If a marker is removed without the corresponding phase being completed, the verification checklist (§6) reports `FAIL`.

### 4.1 Documentation banners

| Banner location | Text |
|---|---|
| `docs/ROADMAP.md` (this file) | Top-of-page status banner (§1) |
| `docs/README.md §Feature Status` | New "Implementation reality" table differentiating Implemented / WIP / Not started |
| `docs/conceptual-foundations/01-02-global-architecture.md §2.3, §2.4, §2.5` | Per-engine "WIP / partial" callout |
| `docs/engines/trade-automation-engine/03-03-*.md` | `**Status:** Specified — WIP; backend present, frontend placeholder; see [docs/ROADMAP.md](ROADMAP.md) §3 (Phase A–B)` |
| `docs/engines/portfolio-management-engine/03-04-*.md` | `**Status:** Specified — WIP; backend present, frontend placeholder; see [docs/ROADMAP.md](ROADMAP.md) §3 (Phase A + C)` |
| `docs/engines/performance-analytics-engine/03-05-*.md` | `**Status:** Specified — WIP; analytics live, backtest UI mock; see [docs/ROADMAP.md](ROADMAP.md) §3 (Phase D)` |
| `docs/integration-and-api/06-01-api-gateway-contract.md` | Endpoint table marks `/api/backtest/*` as "Planned; Phase D" |
| `docs/ui-ux/07-02-ui-dashboard-layout.md §5.3` | Engine mapping table marks TAE/PME/PAE rows with `(WIP)` suffix |
| `README.md §Quick Start` + `AGENTS.md §Project overview` | New "Implementation status" callout |

### 4.2 UI banners

| Banner location | Text |
|---|---|
| `ui/src/components/TradeAutomationDashboard.svelte` (above the sidebar) | Amber banner: **"Work in progress — this dashboard shows placeholder data; live wiring lands in Phase A (see [docs/ROADMAP.md §3 Phase A](ROADMAP.md))."** |
| `ui/src/components/PortfolioDashboard.svelte` (above the sidebar) | Same banner text, scoped to Phase A + C |
| `ui/src/components/PerformanceDashboard.svelte` Backtesting panel | Amber banner: **"Work in progress — the backtest runner is a UI mock; backend lands in Phase D (see [docs/ROADMAP.md §3 Phase D](ROADMAP.md))."** |

### 4.3 Source-code comments

| File | Comment |
|---|---|
| `ui/src/components/TradeAutomationDashboard.svelte` line 15 | Existing `// ── Placeholder data ───` is preserved as a `grep`-able anchor |
| `ui/src/components/PortfolioDashboard.svelte` line 14 | Existing `// ── Placeholder data ───` is preserved |
| `ui/src/components/PerformanceDashboard.svelte` line 25–37 | `// ── Backtesting state (UI-only mock; see ROADMAP.md Phase D) ───` |

---

## 5. Known WIP items (linked to existing audit IDs)

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

- [ ] **`docs/ROADMAP.md` exists and is linked from `docs/README.md` and `README.md`**
- [ ] **`docs/README.md §Feature Status` distinguishes Implemented / WIP / Not started for every engine and major feature**
- [ ] **Every `**Status:**` header in `docs/engines/{trade-automation-engine,portfolio-management-engine,performance-analytics-engine}/**` reads `WIP` with a ROADMAP.md pointer**
- [ ] **`docs/conceptual-foundations/01-02-global-architecture.md §2.3, §2.4, §2.5` carry a WIP callout**
- [ ] **`docs/conceptual-foundations/01-06-crate-layout-and-cycles.md` no longer calls `performance-analytics` "stub" (it's a real evaluator, even if PAE is WIP)**
- [ ] **`docs/conceptual-foundations/01-02-global-architecture.md §2.3 Layer 2 line 132** no longer claims "currently only live execution is supported" — paper trading is the default and only path**
- [ ] **`README.md §Quick Start` carries an "Implementation status" callout linking to this roadmap**
- [ ] **`AGENTS.md §Project overview` distinguishes Implemented / WIP / Not started**
- [ ] **All numbered docs carry `**Version:** 6.8` and `CHANGELOG.md` top entry is `v6.8`**
- [ ] **`./manage.sh test-doc`** passes (release gates G1–G16)

### 6.2 Source-code verification

- [ ] **No `// ── Placeholder data ───` comment in `TradeAutomationDashboard.svelte` after Phase A1**
- [ ] **No `// ── Placeholder data ───` comment in `PortfolioDashboard.svelte` after Phase A2**
- [ ] **`runBacktest` in `PerformanceDashboard.svelte` is replaced by a `fetch` after Phase D1**
- [ ] **No amber banner mounted in the engine dashboard indicates WIP status**

### 6.3 Backend integration points

- [ ] **`GET /api/instances/:id/policies` returns the live `PolicyMatrix` set**
- [ ] **`GET /api/instances/:id/triggers` returns recent `ObservableTrigger` events**
- [ ] **`GET /api/instances/:id/paper/positions` returns paper-trade state**
- [ ] **`GET /api/instances/:id/paper/orders` returns paper-trade state**
- [ ] **`GET /api/instances/:id/paper/history` returns paper-trade state**
- [ ] **`GET /api/instances/:id/lifecycle` returns the live `LifecycleState` per instance**
- [ ] **`GET /api/instances/:id/portfolio` returns the live PME state**
- [ ] **`GET /api/instances/:id/safety` returns the live PME state**
- [ ] **`GET /api/instances/:id/exposure` returns the live PME state**
- [ ] **`GET /api/instances/:id/capital` returns the live PME state**
- [ ] **`GET /api/instances/:id/veto` returns the live PME state**
- [ ] **`POST /api/backtest/run` + `GET /api/backtest/:id` exist and round-trip a result**

### 6.4 Tests

- [ ] **`./manage.sh test-core`** passes (~280 tests)
- [ ] **`./manage.sh test-indicators`** passes (37 indicator-pipeline tests)
- [ ] **`./manage.sh test-engine`** passes (~177 tests)
- [ ] **`./manage.sh test-ui`** passes (24+ tests)
- [ ] **`./manage.sh test-doc`** passes (release gates G1–G16)
- [ ] **`cargo fmt --all -- --check`** passes
- [ ] **`cargo clippy --workspace --all-targets --no-deps -- -D clippy::await_holding_lock -D static_mut_refs -D clippy::items_after_test_module`** passes
- [ ] **`bun run check`** (svelte-check + tsc) passes

### 6.5 Final sign-off

- [ ] **All WIP labels in §1 of this roadmap removed** (DIE ✅, MME ✅, TAE ✅, PME ✅, PAE ✅)
- [ ] **`docs/README.md §Feature Status` row for each engine reads "Implemented"**
- [ ] **Amber UI banners removed from `TradeAutomationDashboard`, `PortfolioDashboard`, `PerformanceDashboard`**
- [ ] **`docs/CHANGELOG.md` top entry reads `## v7.0 (date) — TAE / PME / PAE production-ready`** with sub-bullets referencing each closed audit ID

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
