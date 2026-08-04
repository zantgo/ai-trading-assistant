# Documentation Changelog

> **Purpose.** Single canonical home for version history, every deferred-work item, every audit-issue identifier, and every cross-version migration note. Per `docs/README.md` §Key Conventions, this is the only file in `docs/` that is allowed to carry `MAT-##`, `SIG-##`, `EXE-##`, `UI-##`, `DB-##`, `OPS-##`, `API-##`, `AUDIT-##`, and `Issue NN` references. All normatively cited by other documents.

------

## v6.8 (2026-08-03) — Implementation-status register + WIP banner pass

Realigns the documentation corpus with the **actual delivery state** of the platform at v6.8. The previous corpus called DIE, MME, TAE, PME, and PAE "Implemented" in the Feature Status table and stamped every spec as "Specified — Approved / target of record"; neither is true today. The reality is:

- **DIE and MME are end-to-end implemented** — every layer, every dashboard, every primary endpoint.
- **TAE, PME, and PAE are WIP / partial** — real Rust backends compile, run, and produce state, but their dedicated dashboards (`TradeAutomationDashboard`, `PortfolioDashboard`, the `PerformanceDashboard` backtest panel) render hardcoded placeholder data.

Until the WIP dashboards are wired to the live API, the platform is **not production-complete for automated trading**. The dashboard wiring is the focus of Phases A–D of the new [`docs/ROADMAP.md`](ROADMAP.md).

### What changed

- **New doc — [`docs/ROADMAP.md`](ROADMAP.md)** — the canonical implementation-status register and the phased delivery plan (Phase A: wire TAE/PME dashboards; Phase B: TAE end-to-end paper trading with lifecycle / Gate 0; Phase C: PME end-to-end safety + configurable activation; Phase D: PAE backtesting; Phase E: production hardening). Includes a final verification checklist (§6) that must pass before any "WIP" label can be removed.
- **`docs/README.md`** — `§The Five Engines` now distinguishes Implemented / WIP / Not started; `§Feature Status` is rewritten with the unambiguous status legend and per-engine ⛔/⚠️/✅/🟡 markers; the directory map adds ROADMAP.md at the top; the stats line + corpus version bump to v6.8; the reading order begins with ROADMAP.md.
- **`README.md`** and **`AGENTS.md`** — top-of-page "Implementation status (v6.8)" callouts and the engine responsibility table now carries a Status column.
- **All 5 TAE spec files / 5 PME spec files / 5 PAE spec files** — `**Status:**` headers updated from `Approved` to `Specified — WIP; …` with a ROADMAP.md pointer, and stamps bumped to v6.8.
- **`docs/conceptual-foundations/01-02-global-architecture.md`** — `§2.3 Layer 2` line describing the Execution Layer no longer claims "currently only live execution is supported"; the target/actual split is documented explicitly. `§2.3 / §2.4 / §2.5` open with WIP callouts.
- **`docs/conceptual-foundations/01-06-crate-layout-and-cycles.md`** — `performance-analytics` no longer described as "stub"; the `portfolio-supervisor` and `performance-analytics` rows carry WIP status; the `invalidate_position` rationale no longer says "when the real paper-trading engine is implemented" (it is implemented).
- **`docs/conceptual-foundations/01-03-systemic-data-flow.md` Sequence B** — the "Type boundary" note is corrected: the canonical `f64 → Decimal` cast lives in `crates/portfolio-supervisor/src/execution/order.rs::construct_order` (cross-referenced from 03-03-03 §2).
- **`docs/ui-ux/07-02-ui-dashboard-layout.md` §5.3** — the engine mapping table is updated to flag TAE / PME / PAE dashboard rows as WIP; the Backtesting panel within PAE is flagged as a UI mock.
- **`docs/conceptual-foundations/01-07-target-architecture-roadmap.md`** — the stale `v6.5` forward-target entries for `cascade_risk_index` and the `pre_dispatch_orders` table are bumped to `v6.8` (or `Unscheduled` where the work is not yet committed to a release).
- **CHANGELOG §Open Items forward targets** — entries still pointing at `v6.5` or `v6.6` are bumped to `v6.8` (the new current corpus version) per release-gate G16.
- **Version stamps** — all 144 numbered docs re-stamped to `**Version:** 6.8 (2026-08-03)`.
- **`docs/DOCS-CONSISTENCY-MANIFEST.md`** — title bumped to v6.8; the new `ROADMAP.md` is added to the file inventory; the §12.0 release-gate table is updated to flag G1 / G2 / G8 / G16 as `FAIL` today and to list the remediation steps; the §12.6 verification checklist is extended to include ROADMAP.md placement and per-engine WIP-banner verification.

### Status reconciliation

The previous v6.5 corpus asserted "Approved" / "Implemented" for TAE, PME, and PAE. The v6.8 corpus takes the opposite view: the Rust backends are real, but the surfaces an operator clicks (dashboards, panels, lifecycle UI, backtest runner) are placeholders. The new `ROADMAP.md` is the single source of truth for which phase finishes which surface; once each phase's acceptance criteria pass, the corresponding row transitions from ⚠️ to ✅.

### Audit IDs newly opened

- **`AUDIT-V6-401`** — Wire `TradeAutomationDashboard` to live API (Phase A1).
- **`AUDIT-V6-402`** — Wire `PortfolioDashboard` to live API (Phase A2).
- **`AUDIT-V6-403`** — `POST /api/backtest/run` + `GET /api/backtest/:id` (Phase D1).
- **`AUDIT-V6-404`** — Replace `setTimeout` mock in `PerformanceDashboard.runBacktest` (Phase D1).
- **`AUDIT-V6-405`** — Equity-curve chart replaces "Equity curve visualization coming soon" (Phase D3).
- **`AUDIT-V6-406`** — Live Hyperliquid + Bitget order-dispatch adapter (Phase E1).
- **`AUDIT-V6-407`** — `f64` indicator signature migration (Phase E4, supersedes scoped AUDIT-V8-400 … V8-407).

------

## v6.7 (2026-07-31) — Per-Tab 1:1 Export Payload Architecture

The Market Monitor's `Export Data` button now produces a JSON payload that mirrors
exactly the data the active panel renders. The previous "kitchen-sink" design —
where every panel exported the entire `MarketSnapshot` (every matrix in one
JSON) — is replaced with per-tab scoped payloads produced by dedicated builders.

- **Frontend — `ui/src/lib/exportBuilders/`** — 8 new builder files
  (`shared.ts`, `chartsTab.ts`, `riskTab.ts`, `opportunityTab.ts`,
  `alignmentTab.ts`, `analysisTab.ts`, `recommendationTab.ts`, `metricsTab.ts`,
  `mtfTab.ts`). Each emits a typed payload whose field set is exactly the union
  of the rendered DOM fields on the corresponding panel.
- **Frontend — `ui/src/components/{RiskPanel,OpportunitiesPanel,AlignmentPanel,AnalysisPanel,RecommendationPanel,TerminalMonitor}.svelte`** — `buildExport()` now calls the matching per-tab builder. No call to the legacy
  `buildPanelExportJson` / `buildMetricsExportJson` remains.
- **Frontend — `ui/src/components/BottomConsole.svelte` + `BottomTable.svelte`** — both `handleCopyJson` handlers now route through the same four chart-sub-tab builders. The previous `slots` inconsistency between the two files (BottomTable included slots, BottomConsole did not) is fixed. The Plan tab now exports its own payload (previously it silently copied the history table).
- **Frontend — `ui/src/lib/metricsExport.ts`** — legacy `buildMetricsExportJson` and `buildPanelExportJson` preserved unchanged for backward compatibility with the existing test suite and any external consumers. Added a header comment documenting the new builder architecture.
- **Documentation — `docs/ui-ux/07-05-export-data-payload-schema.md`** — new document; lists every per-tab payload schema with worked examples and the migration notes for downstream consumers.
- **Tests** — 111 new unit tests (one per builder) plus 5 new component tests for `BottomConsole.test.ts`. The full test suite (518 tests) passes.

------

## v6.6 (2026-07-29) — Bitget V2 derivatives extraction + UI feed-state

Fixes the bug where the four derivatives indicators (Open Interest, OI Delta,
Funding Rate, OI-Price Divergence) stayed in `SILENT ⚡` whenever the active
exchange was Bitget. Root cause: Bitget V2 dropped the dedicated
`open-interest` and `funding-rate` WebSocket channels and now pushes the data
on the `ticker` channel under field names `holdingAmount` (OI, base-asset
units), `fundingRate`, and `nextFundingTime`. The previous adapter subscribed
to dead channels and parsed the wrong field name; the parse-failure arm was
silent (`Err(_) => continue`) so no diagnostic ever surfaced. Hyperliquid was
unaffected because it uses a separate REST poller.

- **Backend — `crates/network-adapters/src/adapters/bitget_derivatives.rs`** —
  extended `BitgetTickerData` with `holding_amount` / `funding_rate` /
  `next_funding_time`; new `ticker_to_derivatives_events` helper produces
  `MarkPrice` + USD-converted `OpenInterest` + `FundingRate` events from a
  single ticker payload, mirroring Hyperliquid's `derivatives_ctx_to_events`
  shape.
- **Backend — `crates/network-adapters/src/adapters/bitget.rs`** — dropped
  the dead `open-interest` and `funding-rate` WS subscriptions; ticker arm
  now calls the new helper and feeds `mark_px_override` from the cached
  mark. Per-channel silent diagnostic (Layer 5) now also logs channels that
  never received any frame (previously masked behind `if v == 0`).
- **Backend — `crates/portfolio-supervisor/src/registry/pipelines.rs`** —
  `ClusterRefreshError::NoOpenInterest` now carries the active exchange; the
  skip-reason message templates on Hyperliquid (REST poller) vs Bitget
  (ticker channel).
- **Backend — `crates/core-domain/src/indicator_dtos.rs`** — new `FeedState`
  enum (`Live`, `WaitingFeed`, `Silent`, `Stale`) on
  `IndicatorLifecycleStatus`. Defaults to `Live` so older snapshots
  deserialize unchanged.
- **Backend — `crates/market-analyzer/src/analyzer/mod.rs`** —
  `build_indicator_lifecycle_map` stamps `FeedState::WaitingFeed` for
  `DataOnly` / `Conditional` / candle-based indicators whose lifecycle is
  `Live` but no value-map entry exists yet.
- **Frontend — `ui/src/components/facets/IndicatorsView.svelte`** — new
  `WAITING FEED ⏳` branch in `stateDisplay` renders amber with a faster
  pulse, distinct from the existing `SILENT ⚡` grey pulse. The
  `lifecycleStatus` helper now passes `feed_state` and `silent` through.
- **Frontend — `ui/src/components/facets/IndicatorsView.module.css`** —
  `.stateWaitingFeed` class with amber color (`#f59e0b`) and
  `waitingFeedPulse` keyframes.
- **Frontend — `ui/src/types.ts`** — `FeedState` enum + `feed_state` /
  `silent` fields on `IndicatorLifecycleStatus`.
- **New doc — `docs/engines/data-infrastructure-engine/03-01-08-die-bitget-v2-derivatives.md`** —
  full V2 wire-format reference + anti-patterns list.
- **Tests added (9 total):** 7 in `bitget_derivatives.rs`, 3 in
  `bitget_liquidation_schema.rs` (1 replacement + 2 new), 1 in
  `cluster_refresh_per_tf.rs`, 1 in `cluster_status_api.rs`, 1 in
  `IndicatorsView.test.ts`.

------

## v6.5.1 (2026-07-28) — Watchlist Scanner

A new **Watchlist Scanner** modal lets the user paste a tag-style list of base symbols (e.g. `BTC ETH SOL #AVAX`) and have the Market Monitor pipeline run on each pair sequentially. After the first `decision_context.trade_readiness` value lands for each pair, the modal keeps only pairs with `trade_readiness === 'READY'` AND a directional bias (`StrongLong | Long | Short | StrongShort`), and DELETE-removes the rest. The CTA is a dashed-divider button at the bottom of the Market Monitor Overview (`GeneralDashboard.svelte`). The flow has three phases — input, running, done — sharing a single dialog.

- **New:** `ui/src/components/WatchlistScannerModal.svelte` + companion module-css — three-phase modal.
- **New:** `ui/src/components/WatchlistRunnerButton.svelte` + companion module-css — compact CTA at the bottom of `GeneralDashboard`.
- **New:** `ui/src/lib/watchlistScanner.ts` — pure helpers `parseSymbols`, `decide`, `reasonFor`, `summarize`, `reasonLabel`, `detectBackendErrorKind`.
- **Extended:** `ui/src/lib/api.svelte.ts` — `waitForAdvisory(app, pairKey, timeoutMs)` polls `pair.decisionContext.trade_readiness`; `deleteInstanceById(instanceId)` is the DELETE-path wrapper.
- **Extended:** `ui/src/lib/websocket.svelte.ts` — `applySnapshotToTimeframe` now mirrors `decision_context` from the WS frame to `pair.decisionContext` so the scanner can read the L6 gate without trawling every TF's `latestSnapshot`.
- **Extended:** `ui/src/types.ts` + `ui/src/state.svelte.ts` — `InstanceState.decisionContext` field.
- **Extended:** `ui/src/components/layout/AppPageRouter.svelte` — forwards `wssMap` to `GeneralDashboard`.

No new backend endpoints; the scanner reuses the existing `POST /api/instances` and `DELETE /api/instances/:id` routes. Tests: `ui/src/lib/watchlistScanner.test.ts` (33 cases, all `decide`/`reasonFor` truth-table branches + parser cases), `ui/src/components/WatchlistScannerModal.test.ts` (13 cases covering input/running/done phases).

---

## v6.5 (2026-07-24) — Standardized candle formation + unified indicator lifecycle

Platform-wide refactor replacing the ad-hoc per-exchange bootstrap with a single exchange-independent contract, and replacing the implicit "is this indicator ready?" opacity with explicit per-indicator lifecycle states.

**Single source of truth for candle count.** `[candle_buffer] size` (default **500**) replaces the previous `analysis_limit` field. Every per-timeframe in-memory buffer — `NormalizedCandle` history, `MarketSnapshot` history, indicator warm-up buffers — is rolled at exactly `size` entries (CB-03). The previous `analysis_limit` field on `TimeframeConfig` is **removed**; legacy keys in `config.toml` are logged as warnings and ignored.

**Exchange-independent bootstrap.** The new `HistoricalFetchPolicy` trait (Hyperliquid and Bitget implementations) replaces the previous divergent per-adapter code. Sub-minute timeframes (`timeframe_secs < 60`) bypass historical fetch entirely — no SQLite, no exchange REST (HFP-03); the pipeline starts at 0 candles and fills from live trades. ≥ 1 minute timeframes paginate the REST endpoint until exactly `size` candles are returned, then merge with the SQLite cache (HFP-04 … HFP-10). The Bitget `limit=200` per-page constraint is now paginated, not terminal. The Hyperliquid "no limit parameter" defect is fixed via backward `startTime` cursor pagination.

**Two-level indicator lifecycle.** Every one of the 50 indicators per timeframe now carries an explicit `IndicatorLifecycleState` (`LOADING | LIVE | STALE | FAILED`) plus metadata (`bars_seen`, `bars_required`, `last_updated_at`, `last_error`, `stale_threshold_secs`). Every per-timeframe pipeline carries an explicit `CandlePipelineState` (`INITIALIZING | LOADING | LIVE | STALE | FAILED`). The pipeline state is the **most-severe** aggregate of its 50 indicator states, gated by the parent `ConnectionStatus`. Both fields are published on every emitted `MarketSnapshot` — the dashboard renders a TF header badge and a per-row badge.

**Sub-minute / ≥ 1 minute behavior split is binary and uniform.** Sub-minute: empty buffer → accumulate live candles → indicators `Loading` until each reaches its `bars_required`. ≥ 1 minute: paginated historical fetch to exactly `size` → pipeline `LIVE` on first paint. No third branch. The two paths are documented in [08-08](operations-and-compliance/08-08-candle-buffer-spec.md) §4 (CB-04 … CB-10) and implemented by the trait caller in [03-01-07](engines/data-infrastructure-engine/03-01-07-die-historical-fetch-policy.md).

**TF-change reload.** A new `reload_timeframe(instance_id, slot, new_config)` API tears down and rebuilds only the affected TF pipeline. Other three TFs continue uninterrupted. Cold-start and boot-time `add_instance` continue to build all four.

**Reconstruction ↔ lifecycle interaction.** Reconstructed candles count toward `bars_seen` but do not by themselves promote `Loading → Live` for indicators whose `bars_required` is otherwise met — at least `bars_required` of true live candles must also be present (CB-06, ILS-13). The reconstructed candle's `reconstructed: Some(…)` flag is preserved in `quality_envelope` so the UI can render a synthesized badge.

**Web-mode boot fix.** The `--web` boot path no longer deactivates the session before auto-spawning configured instances (the v6.4.1 `main.rs:261` defect that suppressed cold-start bootstrap). AUDIT-V7-306.

**Frontend neutrality cleanup.** The `IndicatorsView.svelte` `ratio2` `1.00 / OFF` neutralization workaround is removed; missing values render as `--` with a Loading badge instead of faking a neutral reading. AUDIT-V7-307, AUDIT-V7-334.

### Documentation updates
- **New:** `docs/operations-and-compliance/08-08-candle-buffer-spec.md` — master contract (CB-01 … CB-12).
- **New:** `docs/engines/data-infrastructure-engine/03-01-06-die-candle-pipeline-states.md` — TF pipeline state machine (DCP-01 … DCP-15).
- **New:** `docs/engines/data-infrastructure-engine/03-01-07-die-historical-fetch-policy.md` — exchange-independent fetch contract (HFP-01 … HFP-10).
- **New:** `docs/engines/market-monitoring-engine/03-02-15-mme-indicator-lifecycle-states.md` — per-indicator lifecycle (ILS-01 … ILS-15).
- **New:** `docs/conceptual-foundations/01-08-candle-buffer-and-indicator-lifecycle.md` — conceptual overview tying the four new docs together.
- Updated: `01-04-timeframe-model.md` §5 — sub-minute / ≥ 1 minute behavior split explicit.
- Updated: `01-07-target-architecture-roadmap.md` — unified candle formation removed from target list (now implemented).
- Updated: `08-04-candle-reconstruction.md` — ILS-13 interaction added.
- Updated: `03-01-04-die-layer3-data-quality.md` §2 — bootstrap algorithm rewritten to use `HistoricalFetchPolicy`; §7 gains `Buffer invariance` + `Exchange independence` + `Lifecycle visibility` guarantees.
- Updated: `03-02-02-mme-layer1-metrics.md` §3 — `MarketSnapshot` extended with `pipeline_state` + `indicator_lifecycle`.
- Updated: `03-02-09-mme-indicators-guide.md` §1.1 — operational lifecycle table.

### Tests (planned, see Open Items §AUDIT-V7-NN)
- 5 `HistoricalFetchPolicy` tests in `crates/network-adapters/tests/historical_fetch.rs` (HFP-03 sub-minute short-circuit, HFP-05 Hyperliquid pagination, HFP-06 Bitget pagination, HFP-09 DB-precedence, HFP-10 timeout).
- 6 `IndicatorLifecycle` tests in `crates/market-analyzer/tests/indicator_lifecycle.rs` (each of the 8 transitions, reconstructed-candle `bars_seen++` without `Loading → Live`, double-stale escalation, self-recovery).
- 4 `CandlePipelineState` tests in `crates/market-analyzer/tests/candle_pipeline_state.rs` (severity aggregation, `ConnectionStatus` conjunctive gate, `reload_timeframe` cascade, sub-minute cold start → `LOADING → LIVE`).
- 2 UI tests in `ui/src/components/IndicatorStatusBadge.test.ts` (loading badge + live badge render with correct colors).

### Migration
- `analysis_limit` field on `TimeframeConfig` is **removed**. Legacy `config.toml` keys are logged as warnings and ignored; the canonical number is `[candle_buffer] size`.
- Sub-minute historical bootstrap no longer requests 1m candles from exchange REST. Existing pre-v6.5 telemetry rows are unchanged; the change affects only the in-memory pipeline state on cold start.
- `MarketSnapshot.indicators` map is unchanged; `MarketSnapshot.indicator_lifecycle` is a new sibling map. Frontends that do not read `indicator_lifecycle` continue to function (the field is additive, not breaking).

------

## v6.4.2 (2026-07-18) — Liquidation clusters per-timeframe + cluster refresh rewiring

Major change to the Liquidity Intelligence subsystem (MME Phase 2). Liquidation clusters are now computed **per-timeframe** (one matrix per `micro`/`fast`/`slow`/`macro`) instead of per-instance. Each chart in the dashboard now shows clusters at its own horizon — micro=fast-magnet, macro=slow-magnet.

- **Cluster refresh per TF.** `TimeframePipeline` gains its own `cluster_matrix: Arc<RwLock<Option<LiquidationClusterMatrix>>>`. `ActivePair` no longer carries a shared cluster handle. Each TF reads its own price history (200 candles of *that* TF, not just micro) for the swing-low/high seeds, while OI/funding are still pair-level (shared at `ActivePair`).
- **Per-TF refresh tasks.** `portfolio-supervisor/registry/pipelines.rs::spawn_tasks` now spawns **four** cluster refresh tasks (one per slot). Each runs at the TF's own candle cadence — sub-second TFs refresh at sub-second intervals, matching every other MME indicator/signal.
- **First fire is immediate.** The 5-min delayed first tick is gone: each task computes and writes once before entering its loop, so the cluster is populated on the first completed candle of each TF.
- **Default `cluster_refresh_secs` changed** from 300 → **0** (means "synchronize with TF candle cadence"). Operators may override with any value ≥ 1 (clamped to 1).
- **Diagnostic logs.** Every cluster refresh tick logs its outcome: `✅ Cluster Refresh: BTC-USDT mid=50000.00 OI=$1.2M → 3 short + 5 long clusters (12ms)` (success) or `⚠️  Cluster Refresh: BTC-USDT skipped this tick: no open_interest yet (...)` (failure with reason). Errors are typed (`ClusterRefreshError::{NoSnapshotYet, InvalidMidPrice, NoOpenInterest, InsufficientHistory}`) so a missing cluster on the chart is debuggable from the logs alone.
- **`/api/history` enrichment.** The endpoint now also returns `clusters` and `volume_profiles` maps keyed by TF slot. This gives the frontend both overlays on first-mount, before the WS broadcast has delivered a snapshot.
- **Cross-TF L4/L5 unchanged.** `LiquiditySqueeze` preconditions (L4) and `cascade_risk` (L5) continue to consume the **micro** TF's cluster as the authoritative "fastest-magnet" signal — same semantics as v6.4.x.
- **Sub-minute TFs supported.** New `dynamic_bin_count_handles_sub_minute_tfs` and `dynamic_bin_count_sub_minute_clamped_to_30` tests verify the volume-profile bin formula is sane for 1s/5s/15s/30s TFs. CPU impact: ~10 ms/sec for a 4-TF pair at 1s/15s/60s/900s cadences — well below any concern on a normal PC. See new `03-02-14-mme-sub-min-tf-feasibility.md`.

### Documentation updates
- `01-05-liquidity-domain.md` Phase 2 rewritten (cluster per-TF).
- `03-02-11-mme-liquidity-extension.md` L2.5 outputs section updated.
- `02-13-liquidation-cluster-matrix.md` adds multi-TF section.
- New `03-02-14-mme-sub-min-tf-feasibility.md` documents Rust efficiency for sub-minute TFs.
- `07-03-ui-chart-component-map.md` toggle table updated.
- `AGENTS.md` runtime details updated.

### Tests
- New `cluster_refresh_per_tf.rs` (3 tests): per-TF handle isolation, per-TF history isolation, failure-modes (`NoSnapshotYet`, `NoOpenInterest`).
- New `volume_profile::dynamic_bin_count_handles_sub_minute_tfs` and `dynamic_bin_count_sub_minute_clamped_to_30`.
- Updated `phase0_derivatives::liquidity_config_default_is_safe` to expect `cluster_refresh_secs == 0`.
- All 481+ pre-existing tests still pass; net delta: +5 unit tests.

------

## v6.4.1 (2026-07-18) — DIE documentation-reality alignment

Documentation-only correction pass syncing the DIE corpus with the shipped implementation, following the DIE feature-completeness audit (2026-07-18). Only divergences resolved *toward the code* are listed here; all other audit findings remain resolved *toward the docs* (the spec is unchanged) and are tracked as pending code work.

- **Decimal wire format:** the corpus-wide "Decimal-as-string" convention was never shipped — `core-domain` serializes `Decimal` via `rust_decimal`'s `serde-float` feature (plain JSON numbers). Corrected 06-01 §4, 06-00 §3.2, 03-01-05 §4.2; unquoted the numeric literals in the JSON examples of 01-01, 02-03, 02-05, 02-06, 02-07, 02-08, 02-10 (`trade_id` stays a string).
- **`NormalizedCandle` duration field:** 03-01-03 §2 showed a `timeframe_secs: u64` struct field; the actual struct field is `duration_ms: u64` (milliseconds). The wire name `timeframe_secs` (seconds) is unchanged; the 02-06 §2 field-name registry was already correct and is now cross-referenced.
- **Phase-3 REST handlers are served:** `/api/system/clock`, `/api/exchange-status`, `/api/data-quality` moved out of 06-01 "Planned endpoints" into the new §2.11 "System diagnostics endpoints" (planned list renumbered to §2.12, now key-rotation only). 03-01-00 §5 and 03-01-04 §5 no longer describe `/api/data-quality` as unserved; 07-02 §5 no longer marks the Exchange Status / NTP Clock Monitor backends as pending. `GET /api/system/clock.breach_count` reports a placeholder `0` until the persistent counter lands (code work).
- **Stale source paths:** `run_event_router` lives in `crates/market-analyzer/src/analyzer/mod.rs` (spawned from `crates/portfolio-supervisor/src/registry/pipelines.rs`), and `collect_candles()` / `fetch_and_warm_bootstrap()` live in `crates/portfolio-supervisor/src/registry/bootstrap.rs` — not `crates/network-adapters/src/registry/…`. Corrected 03-01-00 §1 and 03-01-04 §2/§6.
- **`WarmedPipelineState`:** 03-01-04 §6.1 previously showed a per-timeframe-map struct (`per_tf_indicator_buffer`, `per_tf_last_bar_ms`, `warmup_complete`, `source_history_len`) that does not exist; rewritten to the authoritative `warm.rs` shape (one warm state per `(symbol, timeframe)` holding ~40 warmed indicator instances plus a capped candle history).
- **Clock-monitor config keys:** `query_timeout_secs` and `jitter_window_size` **are** exposed via `[clock_monitor]` in `config.toml` (08-06 previously claimed both were runtime-only). 08-06 §Public API, §Configuration example, and the key-mapping table corrected.
- Re-stamped only the corrected files to v6.4.1: 01-01, 02-03, 02-05, 02-06, 02-07, 02-08, 02-10, 03-01-00, 03-01-03, 03-01-04, 03-01-05, 06-00, 06-01, 07-02, 08-06 (+ README status row). The remainder of the corpus stays at v6.4.

---

## v6.4 (2026-07-17) — Documentation consistency release

Documentation-only release applying the corpus-wide architectural consistency audit (8 HIGH / 40 MEDIUM / ~25 LOW findings). No platform behavior changes beyond the four documented contract adjustments below.

### Contract adjustments

- **C-1:** `open_orders.is_emergency_liquidation INTEGER NOT NULL DEFAULT 0 CHECK (… IN (0,1))` added (06-02 §3.2) — closes the emergency-liquidation audit gap for in-flight orders (H6).
- **C-2:** `market_breadth.low_coverage` added to the Overview Matrix schema (02-09 §3.2) (M19).
- **C-3:** `risk_control_events.decision` vocabulary aligned to {BLOCK, HELD_FOR_REVIEW, CLIP_AND_CONTINUE, OVERRIDE}; unused MODIFIED_AND_CONTINUED removed; `operator_id` CHECK relaxed to plain TEXT, forward-compatible with AUDIT-V4-076 (M9/M26).
- **C-4:** `NormalizedEvent` gains a `Liquidation` variant (02-10 §2) (M12).

### HIGH-severity fixes

- **H1:** version coherence ratchet (MANIFEST §12.12 + gate G1): v6.3 content ratified; corpus re-stamped v6.4.
- **H2:** canonical scenario chain rebuilt from the 02-01 §6 seed (three seed dimension scores corrected: structure 33.3→65, volume 55→72, volatility 60→75); primary opportunity corrected to TREND_CONTINUATION per the §4 tree; ontology Appendix A regenerated; MANIFEST §12.3 CQ row corrected to 60.5 (inputs 300/600, 50/100).
- **H3:** systemic-risk enforcement consolidated to Gate 7 + PME veto; the CAUTIOUS safety-state mapping is removed.
- **H4:** 08-07 §3.2 master-key rotation runbook rewritten (record out-of-band → stop → start on new key → re-insert → verify → scrub).
- **H5:** ontology Appendix A demoted to illustrative worked example; matrices/02-* are the sole normative wire schemas (gate G13).
- **H7:** Market Instance definition unified to the (symbol, exchange) container; canonical glossary 06-01 §1.0.
- **H8:** errata — the v6.3 entry (below) claimed the "Gate-1 deadlock" rationale was removed from 01-03 Sequence D; it was not. Removed now (gate G12 guards regression).

### Bands, enums, semantics

- SetupQuality bands converted to lower-inclusive [a, b): 85.0 → PRIME (supersedes the v6.3 note that made 85.0 STRONG).
- UNKNOWN empty-state sentinel standardized across assessment enums (StructureAssessment UNCLEAR → UNKNOWN); MarketPhase = 4 phases + UNKNOWN.
- SUSPENDED scoped to the safety axis (scoped-enum rule corrected).
- TradeReadiness rules made total and non-overlapping (ordered rules, 02-04 §4).
- Stance is per-symbol state; the policy-schema `stance` field is removed (policies read stance at dispatch).
- Exposure-limit disposition unified: reject at Gate 6 pre-trade; post-fill breach vetoes to CLOSE_ONLY (no Hard Exit).
- Liquidity data-flow invariant pinned: L1.5 → {L4, L5}; L2.5 → {L4, L5}; L4 + L5 → L6.
- ADX classified directional only (removed from the guide's gate list; 41 + 9 = 50 holds).
- Ingestion topology pinned: one adapter task + one WS connection per TimeframePipeline.

### Process

- MANIFEST: Canonical Source Registry (§13), terminology register, executable gates G1–G16 (§12.0).
- Open Items re-baselined (below): every item carries a target ≥ v6.5 or "Unscheduled".
- ~60 documents corrected in place; zero files added or removed (inventory stays 138).

---

## v6.3 (2026-07-17) — Consistency remediation release

### Fixed (logic)

- **Decision Matrix §3.2:** reordered MarketStance rules 4/5 — AGGRESSIVE (EXCELLENT + risk < 20) now evaluates before CONSTRUCTIVE (GOOD|EXCELLENT + risk < 30). AGGRESSIVE is reachable (was shadowed).
- **Hard-Exit invariant:** removed the false "Gate 1 would block the emergency order" rationale (03-03-02 §7, 03-04-05 §4.2, 01-03 Sequence D). `is_emergency_liquidation` bypass is unconditional per 08-02; the 2a→2c ordering is re-justified on sizing-snapshot and audit grounds.
- **07-04 §5.2:** cascade_asymmetry sign mapping corrected to match 02-13 (`> +0.3` → SHORT_SQUEEZE_RISK; `< −0.3` → LONG_SQUEEZE_RISK).
- **UI navigation:** instance tab set is 7 tabs everywhere (Charts / Metrics / Alignment / Opportunities / Risks / Analysis / Decision). Liquidity inline on Charts; Connection Quality under Data Infrastructure. Updated 08-01 §4, 07-01 §5.1, 07-02 §1 wireframe.
- **Connection-quality persistence:** single owner (`network-adapters::connection_quality_tracker::run_persistence_loop` → `connection_quality_samples`). 01-06 §3.3 corrected; `connection_quality_events` / `connection_quality_persistence/mod.rs` references are retired-concept only.

### Fixed (worked examples)

- **Canonical example chain unified** (02-01 §6 → 02-02 §5 → 02-08 §7 → 02-04 §6 → 01-01 §A.1–A.7). All values recompute (see MANIFEST §12.13 item 1).
- **01-01 §A.4:** `setup_quality` STRONG at 85.0 (was PRIME — 85.0 ≤ 85).
- **01-01 §A.6:** `trade_readiness` FORMING at 46.61 (was READY — confidence ∈ [40, 60)). `directional_guidance` LONG (was STRONG_LONG). `decision_context.bias` BULLISH (was STRONG_BULLISH). `decision_context.score` 88.8. Recommendation confidence 46.6% (was 58%).

### Fixed (contracts & schema)

- **B-12:** Alignment dimension 7 (Confidence) reclassified to unsigned rule (ALIGNED/PARTIAL/DIVERGENT). Signed dimensions now document the score/state independence (`score = a × 100`; `state = f(m)`). N=1 default corrected (§3 rule, not §5 "default to 50").
- **B-11:** BBWP sourced from L1 Metrics raw percentile ([0,100]), not from signed `MarketContext.volatility.score` ([−1,1]). ADX sourced from macro timeframe L1 Metrics value. Fixes unreachable Scalp BBWP precondition.
- **B-1:** 06-02: removed invalid GLOB regex CHECKs on Decimal columns (validation at serde layer); `idx_open_orders_state` now indexes `created_at`; `active_stance` CHECK no longer admits SUSPENDED; §9 count corrected (3 → 7); §3.11 preamble updated (14 retained + 2 added).
- **B-2:** Added nullable `mark_price`/`index_price`/`mark_index_spread_pct` to 02-07 §2.1 and 06-02 §3.1 (Phase-3 writer pending). 03-04-04 §2.1 persistence mapping rewritten against real `paper_balances` columns.
- **B-3:** `DELETE /api/instances/by-pair` single semantic (instance deletion). Manual liquidation → `POST /api/instances/:id/manual/close`. Held-order cancel → `DELETE /api/pre-dispatch/:id`.
- **B-4:** 08-03 backoff formula unified (jitter before cap; capped range [24 s, 30 s] at attempt 6+).
- **B-5:** Gate 2 `WATCH` passes with warning; `STAND_ASIDE` = hold-for-review. Gate 4 = clip-and-continue (no hold). Ontology §7.2 updated.
- **B-6:** Drawdown trigger → strict `<` everywhere. Margin thresholds unified at `≥ 0.80`/`≥ 0.95`. Daily drawdown removed from veto enumeration.
- **B-7:** 03-04-04 §7 sizing query made truly read-only (margin committed at dispatch, not query time).
- **B-8:** Five self-referential rename notes corrected (`invalidation_level` → `invalid_level`/`final_invalidation_level`; `roi_pct` → legacy `roi_percentage`).
- **B-9:** Manifest §12.3/§12.8 re-verified at v6.3 (46.61; 62.5; 26 tables). All rows date-stamped.
- **B-10:** CHANGELOG reordered descending. AUDIT-V4-071 reversal documented (9/26/11/8). Ontology §B.1 Aroon note corrected (Crossover 9, TrendFlip 8).

### Fixed (editorial — 27 items across ~20 files)

See commit list for full details. Summary: annualization examples corrected (C-1), williams-r range claims removed (C-2), SMC terminology swap + convention note (C-3), BBWP boost note deduplication (C-4), anchored VWAP daily anchor (C-4b), PAE classification gap closed (C-5), 01-06 test-doc bucket added (C-6), backward channel wording fixed (C-7), SLA row label (C-8), reduce_only attribute note (C-9), stale version targets swept (C-11), execution candle → micro-tier (C-12), JSON → TOML examples + JSONB → TEXT (C-13), 08-05 fraction-scale sentence removed (C-14), activation spec denominator + liquidation example (C-15), SR/OFI placeholders (C-16), ws_client path corrected (C-17), sidebar duplicates removed (C-18), PascalCase → SCREAMING_SNAKE in ontology examples (C-19), fractional-layer references standardized (C-20), event_type discriminator note (C-21), NTP threshold justification (C-22), systemic-risk CAUTIOUS transition + bogus cross-ref removed (C-23), sector table corrected (C-24), crypto_kms_rotate + audit trail wording (C-25), missing breadth_pct + heading renumber + dangling cross-ref (C-26), config surfaces + config.json sunset (C-27).

### Process

- **New README §Feature Status register** (D-1) — single source of implementation truth. Specs describe target system; status asserted only here and in CHANGELOG.
- **MANIFEST §12.13** (D-2) — 10 new release-gate verification rows (example recompute, file inventory, sign conventions, endpoint semantics, boundary operators, stale versions, status fields, placeholders, enum casing, reachability).
- **01-06 §5** (D-3) — `test-doc` bucket documented (inventory regeneration, worked-example recomputation, grep sweeps).
- **D-4** — this CHANGELOG entry.

### File inventory (re-verified)

138 files = 135 numbered + 3 governance (README, CHANGELOG, MANIFEST). Engine specs: 34 = 6 DIE + 12 MME + 6 TAE + 5 PME + 5 PAE. Growth: v5.0 = 132 → v6.1 = 136 (+01-07, +03-01-00, +06-00, +08-07) → v6.2 = 138 (+03-02-12, +03-03-06) → v6.3 = 138 (zero files added/removed; all edits in-place). Active tables (target): 26.

---

## v6.2 (2026-07-17) — Remediation, Activation, Lifecycle

### Remediation bundle (Phases 1–7 + 8)

- **Canonical numbers corrected.** `state_confidence = 0.65` (formula-driven), `confidence_assessment = 46.61` (from `0.65 × 0.717 × 100`), AssetRank `87.5`, candle-quality example `100.0`, opportunity example `STRONG` (score 85.0), connection-quality score `60.5` under point-scale formula `score = 50·(uptime/100) + 30·(1 − dc_rate) + 20·(1 − rc_rate) − 5·min(loss/600, 1) − 5·min(reconstructed/100, 1)`.
- **Opportunity Matrix on the wire.** `MarketSnapshot.opportunity` field added; `market_snapshots.opportunity_json` column added; documented in `02-07 §2.1`, `06-01 §3.2`, `06-00 §3.1`, `06-02 §3.1`.
- **Signal registry counts corrected.** 9/9/26/9/4/11/4/14/8/2/1/3 = 100, in `04-02-00 §Summary`, `05-02-00 §Summary`, `01-01 §B.3`, `MANIFEST §12.2`. Mechanism: per-indicator tally verified by `scripts/check_docs.py`.
- **AUDIT-V4-071 reversal noted.** The v6.2 corrected values (Crossover = 9, TrendFlip = 8) differ from the v4.0 corrections (Crossover 9→10, TrendFlip 8→10); the final counts 9/9/26/9/4/11/4/14/8/2/1/3 = 100 are registry-verified.
- **AlignState enum extended.** 4 → 7 values; unsigned dimensions use ALIGNED/PARTIAL/DIVERGENT; MIXED redefined for signed dims with low sign-agreement. Wire-compatible (additive).
- **Architecture boundary rewrites.** DIE does NOT build the MarketSnapshot (MME does); DIE transports candles. Single canonical input-edge table in `02-00 §5`. LifecycleState enum redefined (Market Instance = (symbol, exchange) container). Order lifecycle standardized. Gate 2 moved from hard-stop to hold-for-review.
- **Terminology unification.** `emergency_liquidation` → `is_emergency_liquidation`; `reward_risk` → `expected_rr`; `invalid_level` → `invalidation_level`; "Direction Matrix" → "Decision Matrix"; "Regime Compatibility Matrix" → "Performance Matrix's regime_compatibility section"; "Portfolio Matrix (PME)" → "Capital Matrix (PME)"; `QualityMatrix` envelope → `CandleQualityEnvelope`; `retry_cooldown` → `backoff`; phase namespacing enforced; enum disambiguation between MarketRegime vs MarketPhase ACCUMULATION/DISTRIBUTION.
- **Mechanics.** 27 broken relative links fixed; all 138 docs re-stamped `Version: 6.2 (2026-07-17)`; `scripts/check_docs.py` with 10 mechanical checks; `./manage.sh test-doc` is the acceptance gate; 28 docs edited under Phases 1–7.

### Configurable Data Activation (Phase 9, ADDITIVE)

- Users may disable indicators, per-(indicator, SignalKind), per-SignalKind globally, and liquidity subsystems via `[activation]` and `[liquidity]` config tables. **DEFAULT = everything enabled** (current behavior).
- Gating occurs at MME L1 computation; disabled content is absent from the Metrics Matrix and from all downstream inference (treated as NO_DATA). The 50/12/100 registry is a **capability manifest and is UNCHANGED** by config (CA-14).
- `MarketSnapshot.metrics_config` block added (omitted at defaults ⇒ wire-compatible). `market_snapshots.metrics_config_json` column added. `GET /api/instances/:id/activation` endpoint added. Policies referencing disabled inputs are rejected at save time (409) or auto-paused on config change.
- `config_version` is a **new AppConfig field** (`config-models`), NOT the SQLite `user_version` PRAGMA (which remains the migration counter). Incremented exactly once per successful POST /api/config.
- Policy guardrail state named `AUTO_PAUSED` (NOT `PAUSED`) to avoid collision with instance lifecycle `PAUSED`.
- Canonical spec: [`03-02-12-mme-configurable-activation.md`](engines/market-monitoring-engine/03-02-12-mme-configurable-activation.md). 26 docs edited.

### Instance Lifecycle & Programmable State Control (Phase 10, ADDITIVE)

- New `LifecycleState` enum: `RUNNING` / `PAUSED` / `STOPPING` / `STOPPED`. A third operational axis alongside `OperationalMode` × `TriggerMode`, orthogonal to `active_stance` and `safety_state`.
- **STOP = immediate flatten**: cancel all open orders, market-close all positions via transitional `STOPPING`; analytics remain fully readable; restart and manual-only deletion from STOPPED.
- **PAUSE closes the entry gate only**; the event loop and policy-driven exits continue. (Redefines previous "Pause event loop" description.)
- **New Gate 0 (lifecycle)** in pre-trade chain; exits always bypass; existing Gates 1–7 keep their numbers.
- **Programmable per-instance start/pause/stop conditions** (price / time / duration), editable while running; **creation and deletion remain manual-only**.
- New endpoint `POST /api/instances/:instance_id/start`; `/pause` and `/stop` semantics redefined; DELETE requires STOPPED and tombstones.
- New tables `instance_lifecycle` + `instance_lifecycle_events` (active tables 24 → 26).
- **Scoped-enum rule** added to conventions: `instance PAUSED` (lifecycle), `AUTO_PAUSED` (policy), `SUSPENDED` (stance and safety) never co-refer.
- Canonical spec: [`03-03-06-tae-instance-lifecycle-spec.md`](engines/trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md). 13 docs edited. Inventory: 137 → 138 markdown files.

---

## v6.1 (2026-07-16) — DIE second-tier closure

### What changed

This revision closes the second-tier documentation issues identified after the v6.0 DIE closure. It adds 4 new docs (canonical glossary, end-to-end flow, target-architecture roadmap, consumer onboarding, exchange-key rotation procedure), extends the connection-quality composite score formula to use all 5 quantitative report fields, disambiguates overlapping terminology, and adds operational acceptance criteria to every DIE layer.

### Doc-internal / logical issues resolved

| ID | Doc | Issue | Resolution |
|---|---|---|---|
| `AUDIT-V6-021` | `02-10-raw-data-matrix.md` | JSON example used a fictional `event_type` discriminator field that the actual wire format does not emit. | Replaced with per-variant flat-object examples matching the real `NormalizedEvent` JSON shape. |
| `AUDIT-V6-022` | `02-10-raw-data-matrix.md`, `03-01-02-die-layer1-raw-data.md` | `bids`/`asks` notation inconsistent (map vs array). | Standardized on `Vec<[Decimal; 2]>` with `[price, size]` tuples; both docs updated. |
| `AUDIT-V6-023` | `06-01-api-gateway-contract.md` | Three names (`instance_id` / `pair_key` / "Active pair") for the same identifier; no canonical glossary. | Added §1.0 "Canonical glossary (Market Instance identifier)" as the single source of truth. |
| `AUDIT-V6-024` | `08-04-candle-reconstruction.md` | "Reconstruction" / "Synthesis" / "Fill" used interchangeably without distinction. | Added glossary at the top of the doc: synthesis ⊂ fill; reconstruction is the whole process. |
| `AUDIT-V6-025` | `02-03-data-quality-matrix.md`, `03-01-04-die-layer3-data-quality.md` | "Data Quality Matrix" (per-candle) and "Reliability Metrics" (per-instance) share the word "Quality" with divergent schemas. | Renamed: per-candle envelope is `CandleQualityEnvelope`; per-instance rollup is `PipelineReliabilityMetrics`. Both terms now disambiguated in each doc. |
| `AUDIT-V6-026` | `08-05-connection-quality.md` | Composite score formula used 3 of 5 report fields; `total_data_loss_secs` and `reconstructed_candles` were "informational only" with no defined role. | Extended formula to use all 5 fields with documented saturation points (5 s reconnect, 10 disconnects, 600 s data loss, 100 reconstructed candles). New worked example recomputes to `65.45` (clamped to `65.5`). |
| `AUDIT-V6-027` | `03-01-01-die-overview-spec.md` | `ConnectionStatus` lifecycle diagram (`Reconnecting → Connecting` arrow) implied a cyclic state machine not present in the enum. | Replaced with `Connecting → Connected ↔ Disconnected ↔ Reconnecting → Connected (on resume) | Failed` matching the enum and 08-03 §State Transitions. |
| `AUDIT-V6-028` | `03-01-03-die-layer2-market-data.md` | `average_volume` field is consumed by the MME but its provenance was undocumented. | Added note in §2: `average_volume` is derived from `volume / trades_count` on the MME side; the L2 layer never emits it. |
| `AUDIT-V6-029` | `03-01-01-die-overview-spec.md` | Retry budget described only the supervisor layer; the adapter-layer `ReconnectPolicy` was implicit. | §3 Performance Targets now points to 08-03 §Retry Budgets for the three-layer model. §4.1 distinguishes "supervisor cycles" from "adapter max_attempts". |
| `AUDIT-V6-030` | `08-06-clock-monitor.md` | Drift-breach consequence on candle alignment was undocumented — `warn` mode silently violates the alignment invariant. | Added §Drift-breach consequence paragraph explaining the silent violation and the recommended operational pattern (`panic` for fail-fast, or `warn` + active monitoring via `/api/system/clock`). |
| `AUDIT-V6-031` | `03-01-01-die-overview-spec.md` | "Micro" (tier name), "sub-minute" (duration class), "<1m" (shorthand) — three terms for the same concept without a glossary. | Added §1.0 glossary: micro is the tier; sub-minute/<1m describe the duration class. |
| `AUDIT-V6-032` | `03-01-04-die-layer3-data-quality.md` | `out_of_order_dropped` counter had no persistence path. | Added to §5 `PipelineReliabilityMetrics`; documented as in-memory only, surfaced through `/api/data-quality`. |
| `AUDIT-V6-033` | `03-01-04-die-layer3-data-quality.md` | `outlier_tolerance` parameter default and config key were undefined. | Added canonical defaults table: `median_window_size = 20`, `outlier_tolerance = 0.05`, `bypass_on_zero_median = true`. |
| `AUDIT-V6-034` | `03-01-01..05` | Operational acceptance criteria were missing (only unit-test names listed). | Added §1.3 (DIE), §6.1 (L2), §7.1 (L3), §5.1 (L4) with concrete AC-DIE-NN / AC-LN-NN criteria and verification pointers. |
| `AUDIT-V6-035` | `03-01-01..05`, `02-03`, `02-06`, `02-07`, `02-10`, `06-01`, `06-02`, `08-03..06` | No integrated end-to-end DIE flow document. | Created `03-01-00-die-end-to-end-flow.md` as the single integrated narrative. |
| `AUDIT-V6-036` | `06-02 §3.8`, `06-01 §2.10` | Exchange-key rotation procedure was undocumented. | Created `08-07-exchange-key-rotation.md` with pre-rotation checklist, rotation procedure, emergency rotation, and future-work list. |
| `AUDIT-V6-037` | `03-01-01`, `03-01-02`, `03-01-03`, `03-01-04`, `03-01-05` | "Target Architecture (Not Yet Implemented)" callouts scattered across 4 layer docs without a roadmap. | Created `01-07-target-architecture-roadmap.md` as the single source of truth; updated each callout to point to the roadmap. |
| `AUDIT-V6-038` | `06-01`, `06-02` | No consumer onboarding summary; new integrators had to assemble the contract from 3+ docs. | Created `06-00-consumer-onboarding.md` as the single-page orientation. |
| `AUDIT-V6-039` | `03-01-01 §4.1` | `retry_cooldown` term used inconsistently with 08-03's `backoff`. | Renamed `retry_cooldown` → `backoff` throughout §4.1; matches `ReconnectPolicy` field names. |
| `AUDIT-V6-040` | `02-07-metrics-matrix.md` | "Aggregate envelope" claim implied portfolio-wide aggregates ride a single WS frame. | Reworded: composite envelope is per-instance only; portfolio-wide Overview Matrix L7 lives on a separate path. |
| `AUDIT-V6-041` | `03-01-03`, `03-01-04` | L2 vs L3 boundary on sequence auditing was unclear (both docs mentioned chronological order / dedup). | L2 owns single-stream candle generation; L3 owns cross-stream integrity. Boundary table added. |
| `AUDIT-V6-042` | `03-01-05 §2.1` | "Zero Shared State" claim was aspirational; `Arc<…>` state is shared via `RegistryContext`. | Reworded to "Decoupled Producer/Consumer"; shared-state caveat paragraph added. |
| `AUDIT-V6-043` | `06-01 §2.8`, `03-01-03 §5` | `latency_ms` field ambiguous (observation loop vs ingest skew vs heartbeat). | Renamed to `observation_loop_latency_ms`, `ingest_skew_ms`, `system_heartbeat_latency_ms`; `/api/system/status` updated. |
| `AUDIT-V6-044` | `08-01-user-manual.md §7` | 7-day retention hard-coded without config surface. | Documented `config.toml [retention]` block (Phase 1 surfaces it); manual updated. |

### New files added

- `docs/engines/data-infrastructure-engine/03-01-00-die-end-to-end-flow.md` — single integrated DIE flow doc.
- `docs/operations-and-compliance/08-07-exchange-key-rotation.md` — operator procedure.
- `docs/conceptual-foundations/01-07-target-architecture-roadmap.md` — single home for target-architecture callouts.
- `docs/integration-and-api/06-00-consumer-onboarding.md` — single-page integrator orientation.

### Deferred to subsequent phases

- Phase 1 (`AUDIT-V6-101`): wire `MarketDataOrchestrator` and `run_with_reconnect` into the production adapters.
- Phase 2 (`AUDIT-V6-201`): drop `connection_quality_persistence/mod.rs` (already absent from the active schema per `06-02` §3.9 — no code change required), add `pair_key` + `timeframe_secs` columns migration, per-instance tracker.
- Phase 3 (`AUDIT-V6-301`): new REST handlers `/api/system/clock`, `/api/exchange-status`, `/api/data-quality`; surface `mark_index_spread_pct` writers.
- Phase 4 (`AUDIT-V6-401`): DataInfraDashboard full UI (5 sub-panels, CSS Modules cleanup).
- Phase 5 (`AUDIT-V6-501`): test closure (AC-DIE/LN tests).
- Phase 6 (`AUDIT-V6-601`): full-suite verification + DOCS-CONSISTENCY-MANIFEST v6.0 re-run.

---
## v6.0 (2026-07-16) — DIE closure (Phase 0)

### What changed

Phase 0 of the v6.0 DIE closure plan resolves **20 DIE-surface documentation issues** identified in the pre-implementation audit (10 doc-internal contradictions, 10 logical/spec gaps). This release is **docs-only** — no source code changes. Subsequent phases (`Phase 1` through `Phase 6`) will align the code with the now-consistent docs.

### Doc-internal contradictions resolved

| ID | Doc | Issue | Resolution |
|---|---|---|---|
| `AUDIT-V6-001` | `08-05-connection-quality.md` | Two conflicting `CREATE TABLE connection_quality_samples` blocks in the same file (8-column process-wide + 10-column per-instance). | Merged into a single 11-column per-instance DDL (`id, pair_key, timeframe_secs, timestamp_ms, window, uptime_pct, disconnect_count, avg_reconnect_ms, total_data_loss_secs, reconstructed_candles, score`) with the unified index `idx_cq_pair_timeframe_window_time`. |
| `AUDIT-V6-002` | `02-06-market-data-matrix.md` | `LinearInterpolation` variant still listed in the field table despite `AUDIT-V4-024` rename. | Renamed to `LinearExtrapolation` to match `08-04` and `06-02`. |
| `AUDIT-V6-003` | `08-06-clock-monitor.md` | "*no* `config.toml` exists" wording inside a `config.toml` spec. | Replaced with positive "single source of configuration truth" wording; JSON example rewritten in TOML. |
| `AUDIT-V6-004` | `02-10-raw-data-matrix.md` | `Status` payload field `state` (mismatched `03-01-02` `status`). | Field renamed `state → status` to match the Rust enum and the layer doc. |
| `AUDIT-V6-005` | `02-05-distribution-matrix.md` | "Channel per symbol" granularity (4× coarser than `03-01-05` claim). | Reworded to "Channel per `(symbol, timeframe)` pipeline" matching `03-01-05 §2`. |
| `AUDIT-V6-006` | `03-01-03-die-layer2-market-data.md` | `NormalizedCandle` struct missing `exchange` field (present in `02-06`). | Added `exchange: String` to the struct block; aligned with the matrix spec. |
| `AUDIT-V6-007` | `08-05-connection-quality.md` | Frontend placement described as "between Risks and Analysis workspace tabs" (stale post-`bbfd184`). | Replaced with "Data Infrastructure → Overview → Connectivity" sub-tab reference. |
| `AUDIT-V6-008` | `08-06-clock-monitor.md` | JSON-key ↔ Rust-struct unit mapping (secs, micros, Duration) undocumented. | Added mapping table covering `poll_interval_secs`, `threshold_micros`, `breach_action`, `jitter_window_size`, `query_timeout`. |
| `AUDIT-V6-009` | `08-06-clock-monitor.md` | "The TODO comment … has been replaced" claim with no verification. | Reworded to a verifiable cross-reference: "see `crates/market-analyzer/src/candle_aggregator.rs` (verify post-Phase 1)". |
| `AUDIT-V6-010` | `02-06-market-data-matrix.md` | Field-name registry explained only `reconstructed`/`reconstruction_method`, silent on `timestamp` ↔ `start_time_ms` and `timeframe_secs` ↔ `duration_ms`. | Expanded the registry to all three field-name surfaces (provenance, timestamp, duration). |

### Logical / spec gaps resolved

| ID | Doc | Issue | Resolution |
|---|---|---|---|
| `AUDIT-V6-011` | `03-01-04-die-layer3-data-quality.md` | "Out-of-order arrival → reorder into interval bucket" vs L4 immutability invariant — undocumented conflict. | Late ticks are **dropped** at L3 and counted in the new `out_of_order_dropped` reliability metric. L4 immutability wins. |
| `AUDIT-V6-012` | `03-01-04-die-layer3-data-quality.md` | §6 said "feeds through all indicator calculators" (contradicts DIE "no market interpretation" boundary). | Reworded: DIE feeds sanitized candle histories to the MME warm-up pipeline; indicator computation is MME's responsibility. |
| `AUDIT-V6-013` | `08-04-candle-reconstruction.md` | EMA N=200 with only 50 closes is conceptually misleading. | Added "EMA seeded at first close" note explaining the warm-up behaviour. |
| `AUDIT-V6-014` | `03-01-05-die-layer4-data-distribution.md` | "Shadow throttling" wording implied undocumented rate-limiting. | Removed "throttling"; replaced with "Shadow frames stream at tick cadence; any local rate-limiting is the consumer's responsibility". |
| `AUDIT-V6-015` | `03-01-04-die-layer3-data-quality.md` | §2.1 conflated startup bootstrap with live gap-fill. | Split into §2.1.1 (startup bootstrap: DB → REST 200-cap → live) and §2.1.2 (live gap-fill: `GapDetector` → EMA/linear). |
| `AUDIT-V6-016` | `08-05-connection-quality.md` | `total_data_loss_secs` and `reconstructed_candles` not in the composite score formula; role undefined. | Added "Informational-only fields" note. **Superseded by AUDIT-V6-026** (v6.5): the two fields were later added as subtractive penalty terms to the composite score formula, making this informational-only resolution stale. |
| `AUDIT-V6-017` | `08-05-connection-quality.md` | "All three windows computed and persisted in parallel" ambiguous about API shape. | Reworded: three independent rows per tick; REST API returns one report per request; tab switch re-fetches. |
| `AUDIT-V6-018` | `03-01-05-die-layer4-data-distribution.md` | Two broadcast topologies (`NormalizedCandle` vs `MarketSnapshot`) implicit but undocumented. | Added paragraph explicitly distinguishing the two channels and their consumers. |
| `AUDIT-V6-019` | `03-01-04-die-layer3-data-quality.md` | `WarmedPipelineState` referenced but undefined anywhere. | Defined inline in §6.1 with a 4-field shape (`per_tf_indicator_buffer`, `per_tf_last_bar_ms`, `warmup_complete`, `source_history_len`). |
| `AUDIT-V6-020` | `06-02-database-schema-spec.md` + `08-05-connection-quality.md` | `connection_quality_events` table referenced in code (`connection_quality_persistence/mod.rs`) is not in the active schema catalog. | Confirmed single canonical home: `connection_quality_samples` with the 11-column shape. The `connection_quality_events` path is removed in a future phase. |

### Bumped to v6.0

- `02-05-distribution-matrix.md`, `02-06-market-data-matrix.md`, `02-10-raw-data-matrix.md`
- `03-01-03-die-layer2-market-data.md`, `03-01-04-die-layer3-data-quality.md`, `03-01-05-die-layer4-data-distribution.md`
- `06-02-database-schema-spec.md`
- `08-04-candle-reconstruction.md`, `08-05-connection-quality.md`, `08-06-clock-monitor.md`

---

## v5.0 (2026-07-16) — Workspace restructure

### What changed

**The two monolithic crates (`crates/engine` + `crates/shared`) are split into 9 specialized crates.** This is the physical-workspace refactor that the user's plan described. The five logical engines (DIE, MME, TAE, PME, PAE) and the three cross-cutting concerns (domain types, config models, HTTP gateway, headless daemon) are now mapped one-to-one to the crates below. The full crate table, the dependency graph, and the four cycle-breaking design decisions are in [`01-06-crate-layout-and-cycles.md`](conceptual-foundations/01-06-crate-layout-and-cycles.md).

| Old (v4.0) | New (v5.0) | Purpose |
|---|---|---|
| `crates/shared` (monolithic DTOs + 50 indicators) | `crates/core-domain` + `crates/market-analyzer` | Stateless DTOs split from raw indicator math |
| `crates/engine` (everything else) | `crates/network-adapters` + `crates/database-storage` + `crates/portfolio-supervisor` + `crates/performance-analytics` + `crates/api-gateway` + `crates/execution-daemon` | One engine → one crate |
| (none) | `crates/config-models` | New leaf crate for `*Config` structs + `load_config()` |
| `crates/engine/src/main.rs` | `crates/execution-daemon/src/main.rs` | Renamed binary; `cargo run --bin execution-daemon` |
| `crates/backend/` (orphan) | (deleted) | Was a stale duplicate, never in workspace |

### Why four cycle-breaking decisions were needed

Rust Cargo forbids cyclic crate deps. Four of the natural edges between the new crates would have been cycles; each was resolved by moving either the **type definition** or the **function body** to the right crate:

1. **`MarketContext`** struct in `core-domain`; `synthesize_market_context()` function in `market-analyzer`. (Avoids `core-domain → market-analyzer`.)
2. **`AppState`** in `api-gateway`; `RegistryContext` in `portfolio-supervisor` with `AppState::registry_context()` as a one-method bridge. (Avoids `api-gateway ↔ portfolio-supervisor`.)
3. **`ConnectionQualityTracker`** event-emitter in `network-adapters` (no `sqlx`); persistence loop in `database-storage::connection_quality_persistence`. (Avoids `network-adapters → database-storage`.) *(Corrected in v6.4.1 — the persistence loop now lives in `network-adapters::connection_quality_tracker::run_persistence_loop`; `database-storage` exposes only the query layer `list_connection_quality`.)*
4. **`paper_trading::invalidate_position`** stub removed from the analyzer pipeline. (Avoids `market-analyzer → portfolio-supervisor`.)

Full rationale in [`01-06 §3`](conceptual-foundations/01-06-crate-layout-and-cycles.md).

### Configuration format change

`config.toml` is now the canonical configuration file (replacing the legacy `config.json`). The `config-models::load_config()` reader recognizes **both** — `config.toml` is preferred for new deploys, `config.json` is accepted as a legacy fallback. The `manage.sh` `destroy` command scaffolds `config.toml` from `config.default.toml`. The 08-01-user-manual.md `§2` and `§5` are updated to reflect the new canonical format.

### Documentation path rewrites

89 stale `crates/engine` / `crates/shared` / `crates/backend` references in `docs/`, `AGENTS.md`, `README.md`, and `manage.sh` were updated in commit `docs: rewire crates/{engine,shared} -> 9-crate paths + config.json -> config.toml`. The grep audit at the end of that commit showed zero remaining stale paths.

### Test suite changes

| Before (v4.0) | After (v5.0) |
|---|---|
| `./manage.sh test-core` ran `cargo test -p shared` | runs `-p core-domain -p market-analyzer -p config-models` |
| `./manage.sh test-engine` ran `cargo test -p engine` | runs `-p database-storage -p api-gateway -p portfolio-supervisor -p performance-analytics -p network-adapters -p execution-daemon` |
| `./manage.sh run` ran `cargo run --` | runs `cargo run --bin execution-daemon --` |

481 tests pass after the restructure (up from 284 in v4.0; the additional tests include previously dormant property tests + new fault-tolerance and e2e tests that the engine crate had suppressed).

### New audit register

This version introduces the `AUDIT-V5-NN` series for tracking gaps between the new doc `01-06-crate-layout-and-cycles.md` and the actual workspace layout. See **Resolved Issues in v5.0** below.

### Migration notes for downstream consumers

- **Code paths** that referenced `shared::` or `engine::` were updated in commit `d0e3ac2` (the source restructure) and again in this commit (the docs restructure). There is no in-tree code or doc that still references the old crate names.
- **Config files**: existing `config.json` files continue to work — `load_config()` reads them as a fallback. Operators may rename to `config.toml` at their leisure; the two file formats are structurally identical (TOML keys = JSON keys, with TOML's `[table]` syntax for nested objects).
- **Database schema**: unchanged. The 24 migrations moved from `crates/engine/migrations/` to `crates/database-storage/migrations/`; `sqlx::migrate!()` reads them relative to `CARGO_MANIFEST_DIR` so the move is transparent at runtime.
- **Frontend**: the frontend reads config via `GET /api/config` and never imports from any Rust crate, so the split is invisible. The two Rust-comment references in `ui/src/types.ts` to "Rust shared::indicators" were updated to "Rust market-analyzer::indicators".

### Resolved Issues in v5.0

| ID | Issue | Resolution |
|---|---|---|
| `AUDIT-V5-001` | `docs/CHANGELOG.md` v4.0 reconciliation paragraph cited `crates/shared/src/indicators/registry.rs` | Path updated to `crates/market-analyzer/src/indicators/registry.rs` |
| `AUDIT-V5-002` | `docs/conceptual-foundations/01-01-ontology.md` Appendix B §B.3 cited `crates/shared/src/indicators/registry.rs` | Path updated to `crates/market-analyzer/src/indicators/registry.rs` |
| `AUDIT-V5-003` | `docs/conceptual-foundations/01-05-liquidity-domain.md` claimed "no `config.toml` exists" | Sentence replaced; the legacy `config.json` form is documented as the historical predecessor and `config.toml` is now canonical |
| `AUDIT-V5-004` | `docs/engines/trade-automation-engine/03-03-03-tae-layer2-execution.md` cited `crates/engine/src/profile_evaluation/*` | Path updated to `crates/portfolio-supervisor/src/profile_evaluation/*` |
| `AUDIT-V5-005` | `docs/engines/portfolio-management-engine/03-04-01-pme-overview-spec.md` cited `crates/engine/src/safety.rs` | Path updated to `crates/portfolio-supervisor/src/safety.rs` |
| `AUDIT-V5-006` | `docs/operations-and-compliance/08-06-clock-monitor.md` §Module path cited `crates/engine/src/main.rs` | Path updated to `crates/execution-daemon/src/main.rs` |
| `AUDIT-V5-007` | `docs/operations-and-compliance/08-04-candle-reconstruction.md` cited `crates/engine/src/adapters/reconnection_handler.rs` (file never existed) | Path updated to `crates/network-adapters/src/adapters/reconstruction.rs` |
| `AUDIT-V5-008` | `docs/operations-and-compliance/08-03-connection-resilience.md` cited `crates/engine/src/api_client` (file never existed) | Stale reference deleted; replaced with `crates/network-adapters/src/adapters/*_rest.rs` |
| `AUDIT-V5-009` | `AGENTS.md` test-coverage table cited `crates/engine/tests/phase0_derivatives.rs` and `crates/engine/tests/phase1_liquidation_e2e.rs` | Paths updated to `crates/portfolio-supervisor/tests/...` |
| `AUDIT-V5-010` | `README.md` Workspace Structure described a 2-crate workspace | Replaced with the 9-crate tree |
| `AUDIT-V5-011` | `AGENTS.md` duplicated the dependency-graph ASCII diagram | Removed; `01-06-crate-layout-and-cycles.md` is the single source of truth |

(11 AUDITs; commit `docs: rewire crates/{engine,shared} -> 9-crate paths + config.json -> config.toml` resolved AUDIT-V5-001..010; this changelog resolution is AUDIT-V5-011.)

---

## v4.0 (2026-07-16) — Corpus closure

### What changed

**Reconciliation against the source registry.** The entire corpus is reconciled against `crates/market-analyzer/src/indicators/registry.rs` (read-only verification at 2026-07-16). Outcomes:

1. The `VolatilityCycle` SignalKind rename (introduced in a v2.1 docs-only patch) never propagated to the registry. The registry still emits the variant `CompressionRelease`. v4.0 reverts the docs to **`CompressionRelease`** as the canonical name, with this entry recorded here as the only place the v2.1 rename is acknowledged.
2. Per-SignalKind counts in `01-01 §B.3` and `04-02-00 §Summary` were stale against the registry. v4.0 publishes the registry-verified counts (see **Per-SignalKind counts (registry-truth)** below).
3. The `×N` notation in per-indicator manifest rows (e.g. `PatternForming×2`) was undocumented, leading to misreading as declaration-count. v4.0 publishes a normative paragraph clarifying that **×N is internal event multiplicity**, not declaration count.

**Closure of v2.1 deferred-work items.** All audit-issue identifiers (MAT-, SIG-, EXE-, OPS-, UI-, DB-, API-, AUDIT-) are absorbed into the **Resolved Issues in v4.0** section below. They no longer appear in normative sections.

**Architecture corrections.**
- `cascade_risk` is the **8th** of the eight unipolar danger sub-dimensions in the Risk Matrix (not the 9th).
- Clock-monitor candle boundaries use exact UTC epoch multiples (`:MM:00.000`), never `:MM:59.999`.
- LiquidityPanel `cascade_asymmetry` sign convention: `> 0` ⇒ short squeeze risk, `< 0` ⇒ long squeeze risk.
- Risk Matrix `overall_risk.score` worked example recomputes to **28.3** (was 28.0).
- Decision Matrix `entry_danger.level` for `score = 20.0` is **`LOW`**, not `VERY_LOW` (half-open band boundary).
- Decision Matrix drops `DecisionGuard` confusion: Gates 1 and 7 are sequential, not duplicated. Gate 7 reads the stance set by Gate 1's PME upstream.

**API contract.**
- WebSocket `/ws` payload gets a normative reference to `02-07-metrics-matrix.md §2.1`. The `/* MarketSnapshot */` placeholder is removed.
- HTTP status & error envelope documented (`{ error: { code, message, details, request_id } }`).
- `/api/history?limit=` parameter documented.
- `/api/connection-quality` is instance-scoped (`instance_id`, `timeframe_secs`).
- Pre-dispatch approval resource added: `GET /api/pre-dispatch`, `POST /api/pre-dispatch/:id/approve`, `DELETE /api/pre-dispatch/:id`.
- `local_operator` identity model documented; carries through UI audit, DB, WebSocket control frames.

**Database schema.**
- New `risk_control_events` table for gate-rejection and override audit; `operator_id TEXT NOT NULL DEFAULT 'local'` (originally `'local_operator'` in the v4.0 draft, renamed to `'local'` in the final v4.0 release — see `06-01` §1).
- `order_fills` table activated as live; PAE contract upgraded to "complete per-fill attribution".
- All `id` PKs use `INTEGER PRIMARY KEY AUTOINCREMENT` (canonical SQLite).
- Vocabularies canonicalized: `exit_reason` (5-value enum), order states (Execution Matrix lifecycle), `roi_pct` (deprecate `roi_percentage`).

**UI.**
- LiquidityPanel normative color/sign mapping documented.
- Dashboard's "19 indicator panes" corrected to "18 dedicated indicator panes + PriceChart overlay + shared generic panes".
- Decision Panel lists all canonical Decision Matrix fields including the four that were previously omitted.
- Connection Quality tab is instance-scoped.

### File count

`docs/` at v4.0: **130** markdown files (1 README + 128 numbered + this CHANGELOG). Net delta from v2.x: +1 file (this CHANGELOG).

### Per-SignalKind counts (registry-truth)

| # | SignalKind | Count | Notes |
|---|---|---|---|
| 1 | `Divergence` | **9** | 8 nested on parent (`supports_divergence: true`: `rsi`, `stochastic`, `chandemo`, `macd`, `obv`, `cmf`, `mfi`, `squeeze`) + 1 standalone (`oi_price_divergence`, own registry entry) |
| 2 | `Crossover` | **10** | |
| 3 | `Threshold` | **21** | |
| 4 | `Breakout` | **9** | |
| 5 | `BandTouch` | **4** | |
| 6 | `ZeroLineCross` | **13** | |
| 7 | `CompressionRelease` | **4** | Renamed from `VolatilityCycle` in docs v2.1; never propagated to registry; v4.0 docs revert to this canonical name |
| 8 | `LevelTest` | **14** | |
| 9 | `TrendFlip` | **10** | |
| 10 | `VolumeClimax` | **2** | |
| 11 | `StackChange` | **1** | |
| 12 | `PatternForming` | **3** | `patterns`, `candlestick`, `smc_liquidity` (each contributing one declaration) |
| | **TOTAL** | **100** | Sum-check: 9+10+21+9+4+13+4+14+10+2+1+3 = 100 |

**×N notation norm.** The `×N` suffix on per-indicator manifest rows (e.g. `PatternForming×2`) counts **internal event multiplicity within a single declaration**, not declaration count. For example, `patterns` has one `(patterns, PatternForming)` declaration but emits multiple PatternForming event subtypes — the `×2` reflects that internal multiplicity. **It does not** mean "2 declarations of `(patterns, PatternForming)`". The 100-declaration total is the sum of distinct `(indicator, SignalKind)` pairs across all 50 indicators.

---

## Resolved Issues in v4.0

Every audit issue from v2.x is closed below. New identifiers (`AUDIT-V4-NN`) are the canonical IDs going forward; legacy identifiers (`MAT-NN`, `SIG-NN`, etc.) are preserved for grep-back-compat but the normative reference is the new ID.

### MAT / SIG (SignalKind contract)

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| `Issue 2.A` | `AUDIT-V4-001` | `state_confidence` rename (L3) | Stable in v2.1; doc-confirmed in v4.0 |
| `Issue 2.B` | `AUDIT-V4-002` | `opportunity_classification` removed from L6 (canonical is L4 `primary_opportunity`) | Confirmed in v4.0 |
| `Issue 2.C` | `AUDIT-V4-003` | L4 institutional redesign fields (`entry_zone`, `target_zone`, `invalidation_level`, `expected_rr_internal`, `time_horizon`) | Stable in v2.1; v4.0 confirms `invalidation_level` (not `invalid_level`) is the canonical name across L4 and Position Matrix |
| `Issue 2.D` | `AUDIT-V4-004` | `entry_danger` rename (L6, from `environment_favorability`) | Stable in v2.1; doc-confirmed |
| `Issue 2.E` | `AUDIT-V4-005` | `cascade_risk_index` placeholder in Overview Matrix | Stable (still placeholder, deferred to v4.x follow-up) |
| `SIG-02` | `AUDIT-V4-006` | Aroon `Crossover` → `TrendFlip` reclassification | Stable in v2.1 |
| `SIG-03` | `AUDIT-V4-007` | Supertrend `BandTouch` → `LevelTest` reclassification | Stable in v2.1 |
| `MAT-04` | `AUDIT-V4-008` | Risk Matrix banding (half-open intervals) | Confirmed in v4.0 §Decision Matrix §3.8 and Risk Matrix §2.3 cross-reference |
| `MAT-06` | `AUDIT-V4-009` | Opportunity Matrix `SCALP` cadence | Cell updated to "Every completed sub-minute candle" |
| `MAT-10` | `AUDIT-V4-010` | Decision Matrix `NO_RECOMMENDATION` reachability | Added explicit rule path for `NO_RECOMMENDATION` |
| `MAT-16` | `AUDIT-V4-011` | Opportunity Matrix setup-quality bands (half-open) | Stable in v2.1; v4.0 keeps |
| `MAT-17` | `AUDIT-V4-012` | Liquidity fields top-level (not nested in `indicators`) | Confirmed in v4.0 |
| `MAT-18` | `AUDIT-V4-013` | Liquidity-extension test count | 55 unit + 1 integration = 56 |
| `Issue 4.O` | `AUDIT-V4-014` | Veto-release endpoint `/api/instances/:id/safety/release-veto` | Confirmed in v4.0 |
| `Issue 4.N` | `AUDIT-V4-015` | Timeframe-weight divisor = slowest enabled tier | Confirmed in v4.0 §Timeframe Model §4 |
| `Issue 5.A` | `AUDIT-V4-016` | Config file is `config.json`, not `config.toml` | Confirmed in v4.0 |

### EXE (Execution / Order lifecycle)

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| `EXE-08` | `AUDIT-V4-017` | Pre-dispatch state has no persistent table | Resolved by `risk_control_events` table (DB) and pre-dispatch approval resource (API) |
| — | `AUDIT-V4-018` | Order states vocab diverged between DB and Execution Matrix | Unified in v4.0 to Execution Matrix lifecycle vocabulary |
| — | `AUDIT-V4-019` | `exit_reason` DB vocabulary diverged from PAE | Unified to 5-value canonical enum (`STOP_LOSS`, `TAKE_PROFIT`, `SIGNAL_EXIT`, `MANUAL`, `VETO`) |
| — | `AUDIT-V4-020` | `order_fills` table deferred | Activated in v4.0; PAE upgraded to per-fill attribution |

### OPS (Operations & Compliance)

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-021` | Clock-monitor `:14:59.999` candle-close convention | Corrected to `:15:00.000` |
| — | `AUDIT-V4-022` | Connection-Quality worked example arithmetic (67.5 → 65.5) | Corrected |
| — | `AUDIT-V4-023` | Backoff jitter post-cap (could exceed `max_backoff`) | Jitter applied before cap in v4.0 |
| — | `AUDIT-V4-024` | Linear extrapolation misnamed "LinearInterpolation" | Renamed to `LinearExtrapolation` |
| — | `AUDIT-V4-025` | NTP server order inconsistent between API and config | Standardized on `["pool.ntp.org", "time.aws.com"]` |
| — | `AUDIT-V4-026` | User manual mixed recovery paths for missing config | Unified canonical scaffold flow |
| — | `AUDIT-V4-027` | User manual tab list stale | Updated to include `Connection Quality` and `Liquidity` |
| — | `AUDIT-V4-028` | User manual instructed placing credentials in `config.json` | Replaced with reference to encrypted `exchange_keys` SQLite table |

### UI

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-029` | LiquidityPanel reversed `cascade_asymmetry` sign | Fixed; normative mapping block added |
| — | `AUDIT-V4-030` | LiquidityPanel data path `microTerm` | Kept as `instance.microTerm.*` (canonical; the `timeframes.micro` alias was removed per v6.3) |
| — | `AUDIT-V4-031` | LiquidityPanel used shortened signal names | Replaced with canonical `LIQUIDITY_*` prefixed names |
| — | `AUDIT-V4-032` | UI dashboard "19 indicator panes" | Corrected to "18 dedicated indicator panes + PriceChart overlay + shared generics" |
| — | `AUDIT-V4-033` | UI chart map: `volume_profile` / `oi_price_divergence` placement | Moved into PriceChart overlay bucket |
| — | `AUDIT-V4-034` | UI analysis panel "6 assessments" claim | Reconciled with 5 assessment fields + market-quality + market-phase |
| — | `AUDIT-V4-035` | UI overview panel missed `breadth_pct` numeric | Added `breadth_pct: f64 ∈ [-100, 100]` to Overview Matrix |
| — | `AUDIT-V4-036` | UI Decision panel omitted 4 canonical fields | Added `trade_readiness`, `entry_danger`, `expected_reward_risk_ratio`, `stop_loss_distance_pct` |
| — | `AUDIT-V4-037` | Connection Quality tab mounted per-instance but API unscoped | API now requires `instance_id` + `timeframe_secs` |
| — | `AUDIT-V4-038` | LiquidityPanel color semantics conflicting (state vs intensity) | Two-channel color mapping documented |
| — | `AUDIT-V4-039` | CSS Modules pattern underexplained | Normative block added in `07-01` |

### DB

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-040` | Header table inventory listed `individual_indicator_logs` (not in §3) | Removed; canonical telemetry is `market_snapshots.indicators_json` |
| — | `AUDIT-V4-041` | `id | SERIAL PK` PostgreSQL-style types against SQLite contract | Replaced with `INTEGER PRIMARY KEY AUTOINCREMENT` |
| — | `AUDIT-V4-042` | `market_snapshots` partial persistence of canonical MarketSnapshot | Documented as deliberate (not-persisted fields enumerated) |
| — | `AUDIT-V4-043` | `policy_id` labelled as FK without `execution_policies` table | Re-labelled as configuration string key |
| — | `AUDIT-V4-044` | `roi_percentage` legacy alias | Canonical `roi_pct`; `roi_percentage` deprecated at v5.0 |
| — | `AUDIT-V4-045` | `funding_rate_8h` cannot distinguish 0 from unset | Nullable column semantics (NULL = inherit; '0' = disable) |
| — | `AUDIT-V4-046` | Safety state persistence incomplete | Documented reconstruction rule from persisted metrics |

### API

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-047` | `/ws` payload `/* MarketSnapshot */` placeholder | Normative reference to `02-07-metrics-matrix.md §2.1` |
| — | `AUDIT-V4-048` | `liquidity_signals` empty-array omission policy ambiguous | Settled: always serialize `[]` when empty |
| — | `AUDIT-V4-049` | `/api/history` limit parameter undocumented | Documented (default 100, max 1000) |
| — | `AUDIT-V4-050` | Connection-quality API unscoped | Now requires `instance_id`, `timeframe_secs` |
| — | `AUDIT-V4-051` | Pre-dispatch approval flow has no API | New resource: `GET / POST / DELETE /api/pre-dispatch` |
| — | `AUDIT-V4-052` | HTTP status codes and error envelope undocumented | Documented JSON envelope with stable codes |
| — | `AUDIT-V4-053` | SPA fallback can mask `/api/*` typos | Scoped to non-`/api/*` paths |
| — | `AUDIT-V4-054` | Authentication = None conflicts with operator-ID requirement | `local_operator` identity model |
| — | `AUDIT-V4-055` | `roi_percentage` legacy field in journal API | Canonical `roi_pct` |

### Architecture narrative

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-056` | MME overview "seven sequential layers" — but L4 ∥ L5 parallel | Re-phrased: "L1–L3 sequential; L4 ∥ L5 parallel; L6–L7 sequential after convergence" |
| — | `AUDIT-V4-057` | MME L5 §1 input description omitted L1.5/L2.5 | Added explicit input list (Analysis Matrix + indicator map + LiquidityFlow + LiquidationClusterMatrix for `cascade_risk`) |
| — | `AUDIT-V4-058` | MME L6 confidence-attenuation formula LHS was `confidence` | Renamed to `confidence_assessment` |
| — | `AUDIT-V4-059` | `02-04-decision-matrix.md §6` cross-reference `§3.7 weights` invalid | Added `confluence_score` formula in `02-04 §2.3`; corrected §6 reference |
| — | `AUDIT-V4-060` | `02-00-matrix-field-ownership.md §2.6` missing `stop_loss_distance_pct` | Added row |
| — | `AUDIT-V4-061` | `02-00-matrix-field-ownership.md §1` ASCII diagram abbreviation `expected_rr_ratio` | Replaced with full name |

### Cascade / Liquidity

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-062` | `cascade_risk` called "9th dimension" in `03-02-06 §7` | Corrected to "8th sub-dimension" |
| — | `AUDIT-V4-063` | `03-02-11` cascade-invariant diagram omitted L4 edge | Updated to `L1.5 → L2.5 → {L4, L5} → L6` |
| — | `AUDIT-V4-064` | `03-02-11` test-coverage table arithmetic | Aligned with canonical 55 + 1 |
| — | `AUDIT-V4-065` | `cascade_asymmetry` threshold split (0.5 event vs 0.3 continuous) | Documented split |
| — | `AUDIT-V4-066` | `cascade_risk_index` placeholder shown in `01-01 §A.7` with non-canonical `trend` field | `trend` removed |

### Authoring hygiene

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-067` | Inline correction notes (`(MAT-XX — correction)` etc.) in normative sections | Stripped from normative text; preserved here |
| — | `AUDIT-V4-068` | Source-line citations (`crates/...rs::func`) in normative sections | Replaced with cross-doc references |
| — | `AUDIT-V4-069` | Subjective adjectives ("most defensible", "best estimate") in algorithmic specs | Replaced with measurable criteria |
| — | `AUDIT-V4-070` | Revision history tables in every doc | Consolidated into this CHANGELOG; tables deleted from individual docs |

### Counts / Renames (closed)

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-071` | Stale per-SignalKind counts (Threshold 26→21, Crossover 9→10, TrendFlip 8→10, ZeroLineCross 11→13) | Corrected |
| — | `AUDIT-V4-072` | `VolatilityCycle` rename to `CompressionRelease` (v4.0) | Reverted docs to registry-truth name |
| — | `AUDIT-V4-073` | `entry_danger.level = VERY_LOW` at score 20.0 violates banding | Corrected to `LOW` |
| — | `AUDIT-V4-074` | `01-01 §A.5` `overall_risk.score = 28.0` (canonical 28.3) | Corrected to 28.3; downstream formulas re-derived |

---

## Open Items (forwarded to future versions)

These are the items deferred from v4.0. They are tracked here only; downstream docs **must link here**, never restate status.

| ID | Item | Status | Target |
|---|---|---|---|
| `AUDIT-V4-005` | `cascade_risk_index` aggregation into `systemic_risk_score` | Open (placeholder field in canonical schema; aggregation formula deferred) | v6.8 |
| `AUDIT-V4-044` | `roi_percentage` legacy field removal | Deprecated in v4.0; remove entirely | v6.8 |
| `AUDIT-V4-046` | `safety_state` deterministic reconstruction algorithm | Open (reconstruction rule documented but not yet unit-tested) | Unscheduled |
| `AUDIT-V4-076` | `X-Operator-Id` optional header for caller-supplied operator identity | Open (single-user `local_operator` fixed identity; caller-supplied identity deferred) | Unscheduled |
| `AUDIT-V4-077` | Authentication beyond `local_operator` (multi-user / OAuth / mTLS) | Open | Unscheduled |
| `AUDIT-V4-078` | Per-WASM lightweight connection-quality scoring | Open | Unscheduled |
| `AUDIT-V4-079` | PriceChart marker overlay for cluster positions (Phase 4 extension) | Deferred | Unscheduled |
| `AUDIT-V4-080` | `liquidation_events` → PAE backtest ingestion | Deferred | Unscheduled |
| `AUDIT-V6-077` | In-process exchange-key rotation tool (`POST /api/keys/rotate` re-encryption under a new master key, SIGHUP hot rotation, encrypted-backup export) — manual procedure documented in `08-07` | Open | Unscheduled |
| `AUDIT-V6-202` | `config-models`: add `LifecycleState` enum; add `instance.automation` struct (start/pause/stop conditions) | Open (specified in `03-03-06` §7) | v6.8 |
| `AUDIT-V6-203` | `database-storage`: add `instance_lifecycle` + `instance_lifecycle_events` migrations; bump `user_version` | Open (specified in `03-03-06` §7) | v6.8 |
| `AUDIT-V6-204` | `api-gateway`: implement `POST /api/instances/:instance_id/start`; rewrite `/pause` (entry-gate semantics) and `/stop` (STOPPING → flatten → STOPPED); DELETE requires STOPPED + tombstone | Open (specified in `03-03-06` §7) | v6.8 |
| `AUDIT-V6-205` | `portfolio-supervisor`: implement Gate 0 check in pre-trade chain | Open (specified in `03-03-06` §7) | v6.8 |
| `AUDIT-V6-206` | `execution-daemon`: orchestrate STOP flatten via cancel-all + market-close with `is_emergency_liquidation = true` and `reduce_only = true` | Open (specified in `03-03-06` §7) | v6.8 |
| `AUDIT-V7-300` | `config-models`: introduce `CandleBufferConfig` struct + `[candle_buffer]` block; remove `analysis_limit` from `TimeframeConfig`; add migration log line for legacy `analysis_limit` keys | Open (specified in `08-08` §7) | v6.8 |
| `AUDIT-V7-301` | `core-domain`: introduce `CandlePipelineState`, `IndicatorLifecycleState`, `IndicatorLifecycleStatus` (see `03-01-06` §2 and `03-02-15` §2) | Open (specified in `03-01-06` §7) | v6.8 |
| `AUDIT-V7-302` | `network-adapters`: introduce `HistoricalFetchPolicy` trait; implement `HyperliquidHistoricalFetch` (paginated backward cursor); implement `BitgetHistoricalFetch` (paginated forward cursor with `limit=200` per page) | Open (specified in `03-01-07` §7) | v6.8 |
| `AUDIT-V7-303` | `market-analyzer`: replace `HIST_BUFFER_MAX = 1000` with `candle_buffer.size`; ensure deque never exceeds `size`; populate `IndicatorLifecycleStatus` for all 50 registry entries; publish `tf.pipeline_state` | Open (specified in `03-01-06` §7) | v6.8 |
| `AUDIT-V7-304` | `portfolio-supervisor`: rewrite `collect_candles` to use `HistoricalFetchPolicy`; sub-minute returns empty Vec; ≥ 1 minute paginates until `size` then merges DB; expose `reload_timeframe(instance_id, slot, new_config)` API | Open (specified in `08-08` §7) | v6.8 |
| `AUDIT-V7-305` | `api-gateway`: add `POST /api/instances/:instance_id/reload?slot=`; extend `/api/history` clamp to `candle_buffer.size`; add `pipeline_state` + `indicator_lifecycle` to the `/api/history` response | Open (specified in `03-01-06` §7) | v6.8 |
| `AUDIT-V7-306` | `execution-daemon`: fix `--web` boot so `init_session` does not deactivate before auto-spawning configured instances | Open (specified in `08-08` §7) | v6.8 |
| `AUDIT-V7-307` | `ui`: introduce `IndicatorStatusBadge.svelte`; honor `tf.pipeline_state` in chart headers; stop merging old values when a snapshot arrives with `pipeline_state = LOADING`; remove the `analysisLimit` selector (replace with read-only display of `candle_buffer.size`) | Open (specified in `08-08` §7) | v6.8 |
| `AUDIT-V7-310` | `core-domain`: add `CandlePipelineState` enum + `IndicatorLifecycleStatus` map type; extend `MarketSnapshot` with `pipeline_state` + `indicator_lifecycle` fields | Open (specified in `03-01-06` §7) | v6.8 |
| `AUDIT-V7-311` | `database-storage`: migration `00XX_add_candle_pipeline_state_events.sql` + `00XX_alter_market_snapshots.sql`; bump `user_version` | Open (specified in `03-01-06` §7) | v6.8 |
| `AUDIT-V7-312` | `market-analyzer`: in `TimeframePipeline`, track `pipeline_state`; transition on every bootstrap return, on every completed candle (DCP-04/DCP-13), on stale-timer tick (DCP-05), on connection-status callback (DCP-09) | Open (specified in `03-01-06` §7) | v6.8 |
| `AUDIT-V7-313` | `portfolio-supervisor`: implement `reload_timeframe` API + cascade transitions per CB-11 | Open (specified in `03-01-06` §7) | v6.8 |
| `AUDIT-V7-314` | `api-gateway`: add `POST /api/instances/:instance_id/reload?slot=`; extend `/api/history` to include per-row `pipeline_state` and `indicator_lifecycle` | Open (specified in `03-01-06` §7) | v6.8 |
| `AUDIT-V7-320` | `network-adapters`: introduce `HistoricalFetchPolicy` trait + request/error types in `adapters/historical_fetch.rs` | Open (specified in `03-01-07` §7) | v6.8 |
| `AUDIT-V7-321` | `network-adapters`: implement `HyperliquidHistoricalFetch` with backward `startTime` cursor pagination (HFP-05) | Open (specified in `03-01-07` §7) | v6.8 |
| `AUDIT-V7-322` | `network-adapters`: implement `BitgetHistoricalFetch` with forward `startTime` cursor pagination + `limit=200` per page (HFP-06) | Open (specified in `03-01-07` §7) | v6.8 |
| `AUDIT-V7-323` | `portfolio-supervisor`: replace `collect_candles` with `HistoricalFetchPolicy` caller; HFP-03 sub-minute short-circuit; HFP-09 merge; HFP-10 timeout handling | Open (specified in `03-01-07` §7) | v6.8 |
| `AUDIT-V7-324` | `tests`: add 5 tests — (a) sub-minute returns empty, (b) Hyperliquid paginates to `size`, (c) Bitget paginates `limit=200` to `size`, (d) DB-precedence on overlap, (e) timeout returns partial + warning | Open (specified in `03-01-07` §7) | v6.8 |
| `AUDIT-V7-330` | `core-domain`: add `IndicatorLifecycleState` enum + `IndicatorLifecycleStatus` struct; extend `MarketSnapshot` with `indicator_lifecycle` + `pipeline_state` fields | Open (specified in `03-02-15` §8) | v6.8 |
| `AUDIT-V7-331` | `market-analyzer/registry`: add `bars_required: u32` to each of the 50 indicator metadata entries in `crates/market-analyzer/src/indicators/registry.rs` | Open (specified in `03-02-15` §8) | v6.8 |
| `AUDIT-V7-332` | `market-analyzer`: in `run_single`, populate `IndicatorLifecycleStatus` for every active-set indicator on every completed candle; apply ILS-05–ILS-10 transitions; apply ILS-14 confidence override | Open (specified in `03-02-15` §8) | v6.8 |
| `AUDIT-V7-333` | `market-analyzer`: in `warm_indicators_for_timeframe`, initialize every indicator's lifecycle to `Loading` with `bars_seen = 0`; rely on the first completed candle to begin ILS-02 counting | Open (specified in `03-02-15` §8) | v6.8 |
| `AUDIT-V7-334` | `ui`: introduce `IndicatorStatusBadge.svelte`; update `IndicatorsView.svelte` to render the badge and stop merging old values when `pipeline_state = LOADING` (replaces the existing `applySnapshotToTimeframe` per-key merge for indicators that arrive `Loading`); update `TimeframeSettings.svelte` to remove `analysisLimit` selector | Open (specified in `03-02-15` §8) | v6.8 |
| `AUDIT-V8-400` | `market-analyzer/indicators/traits.rs`: DOD hot-path contract applied — `BarInput` fields are `f64`, `Indicator::Output = f64`. Migration code-converter at the trait boundary for all ~30 `Indicator` impls. | Staged (v6.5) | v6.8 |
| `AUDIT-V8-401` | `market-analyzer/indicators/ema.rs`: migrate EMA `update(price: Decimal) → update(price: f64)`. Expected: ~50 line change (10 lines signature + 40 lines test). | Staged | v6.8 |
| `AUDIT-V8-402` | `market-analyzer/indicators/rsi.rs`: migrate RSI `update(close: Decimal) → update(close: f64)`. Expected: ~60 line change. | Staged | v6.8 |
| `AUDIT-V8-403` | `market-analyzer/indicators/macd.rs`: migrate MACD `update(close: Decimal) → update(close: f64)`. Expected: ~80 line change. | Staged | v6.8 |
| `AUDIT-V8-404` | `market-analyzer/indicators/{atr,adx,bbwp,stochastic,chandemo,supertrend,keltner,donchian,obv,cmf,mfi,hv,aroon,choppiness,linreg,zscore,bollinger,squeeze,cci,psar,williams_r,hull_ma,awesome_oscillator,force_index,stddev_channel,ichimoku,anchored_vwap,pivot_points,candlestick,patterns,fibonacci,smart_money,volume_profile,open_interest,funding}.rs`: migrate remaining 35 indicator `update()` signatures from `Decimal` to `f64`. Per-indicator commits, ~50-70 line changes each (signature + arithmetic + tests). Total: ~1750-2450 line change across 35 files. | Staged | v6.8 |
| `AUDIT-V8-405` | `market-analyzer/src/analyzer/mod.rs` (`run_single`): add single `Decimal→f64` batch conversion at the top of the per-candle hot loop (OHLCV → `open_f/high_f/low_f/close_f/volume_f`); feed `_f` values to every indicator `update()` call. Remove 150+ inline `completed.close.to_f64()` per-candle conversions. | Staged | v6.8 |
| `AUDIT-V8-406` | `market-analyzer/src/analyzer/warm.rs` (`warm_indicators_for_timeframe`): same pattern — single `Decimal→f64` batch conversion per historical candle; feed `_f` values to indicators. | Staged | v6.8 |
| `AUDIT-V8-407` | `market-analyzer/src/analyzer/normalize.rs`: update `NormalizeParams` to accept `f64`; remove `d2f()`/`od2f()` conversion helpers; simplify `build_indicator_map` to consume `f64` directly. | Staged (dependent on AUDIT-V8-401…V8-404) | v6.8 |
| `AUDIT-V6-207` | `ui`: Svelte 5 lifecycle badges; start/pause/stop inline-confirm buttons; automation summary line | Open (specified in `03-03-06` §7) | v6.8 |
| `AUDIT-V6-208` | `config-models`: add `AppConfig.config_version: u64` (initial 1, +1 per POST success); add `[activation]` and `[liquidity]` tables | Open (specified in `03-02-12` §9) | v6.8 |
| `AUDIT-V6-209` | `market-analyzer`: build Active Set from `Arc<RwLock<AppConfig>>` at pipeline construction; gate evaluations to active set | Open (specified in `03-02-12` §9) | v6.8 |
| `AUDIT-V6-210` | `core-domain`: add `metrics_config` field (`skip_serializing_if`) to `MarketSnapshot`; auto-pause serialization for `decision_profiles.status` | Open (specified in `03-02-12` §9) | v6.8 |
| `AUDIT-V6-211` | `database-storage`: add migration for `market_snapshots.metrics_config_json` column; bump `user_version` | Open (specified in `03-02-12` §9) | v6.8 |
| `AUDIT-V6-212` | `api-gateway`: implement `GET /api/instances/:id/activation`; POST `/api/config` validation responses; increment `config_version` on 200 | Open (specified in `03-02-12` §9) | v6.8 |
| `AUDIT-V6-213` | `portfolio-supervisor`: implement `AUTO_PAUSED` policy state and transition | Open (specified in `03-02-12` §9) | v6.8 |
| `AUDIT-V6-214` | `ui`: Svelte 5 IndicatorActivation panel; three-state pane styling | Open (specified in `03-02-12` §9) | v6.8 |
| `AUDIT-V6-301` | Phase-3 REST handlers `/api/system/clock`, `/api/exchange-status`, `/api/data-quality`; surface `mark_index_spread_pct` writers | Partially resolved (v6.4.1): the three handlers are served (06-01 §2.11). Remaining open: `mark_index_spread_pct` writers; persistent `/api/system/clock.breach_count` counter (placeholder `0` today) | v6.8 |
| `AUDIT-V6-302` | WS per-timeframe subscriptions (subscribe/unsubscribe individual timeframes on the `/ws` feed) | Open | v6.8 |
| `AUDIT-V6-303` | Timeframe editor (operator-editable timeframe set beyond the default 4 tiers) | Open | v6.8 |
| `AUDIT-V6-304` | PAE→DB feedback (persist PAE analytical feedback to configuration databases for off-line policy optimization) | Open | Unscheduled |
| `AUDIT-V6-305` | Remote config backends (load platform configuration from remote backends, not only local `config.toml`) | Open | Unscheduled |
| `AUDIT-V6-401` | Wire `TradeAutomationDashboard` to live API (`/api/instances/:id/{policies,triggers,paper/{positions,orders,history},lifecycle}`) — Phase A of [`docs/ROADMAP.md`](ROADMAP.md) | Open | v6.8 |
| `AUDIT-V6-402` | Wire `PortfolioDashboard` to live API (`/api/instances/:id/{portfolio,safety,exposure,capital,veto}`) — Phase A + C of [`docs/ROADMAP.md`](ROADMAP.md) | Open | v6.8 |
| `AUDIT-V6-403` | `POST /api/backtest/run` + `GET /api/backtest/:id` — Phase D of [`docs/ROADMAP.md`](ROADMAP.md) | Open | v6.8 |
| `AUDIT-V6-404` | Replace `setTimeout` UI mock in `PerformanceDashboard.runBacktest` with a real `fetch` — Phase D of [`docs/ROADMAP.md`](ROADMAP.md) | Open | v6.8 |
| `AUDIT-V6-405` | Equity-curve chart replaces "Equity curve visualization coming soon" — Phase D of [`docs/ROADMAP.md`](ROADMAP.md) | Open | v6.8 |
| `AUDIT-V6-406` | Live Hyperliquid + Bitget order-dispatch adapter (live exchange path) — Phase E of [`docs/ROADMAP.md`](ROADMAP.md) | Open | v6.8 |
| `AUDIT-V6-407` | Live Hyperliquid + Bitget order-dispatch adapter (live exchange path) — Phase E of [`docs/ROADMAP.md`](ROADMAP.md) | Open | v6.8 |

---

## Conventions enforced in v4.0

> Stated here once; referenced from individual docs so they don't drift.

1. **All enum values serialize as `SCREAMING_SNAKE_CASE`** (e.g. `STRONG_BULLISH`, `TRENDING_BULL`, `AVOID`, `CLOSE_ONLY`).
2. **File and directory names are lowercase-kebab-case**, prefixed `NN-MM[-KK]-…` per the section scheme in `docs/README.md`.
3. **Per-matrix field renames** are applied uniformly across every cite. The current canonical names:
   - L3 Analysis: `state_confidence`, `market_quality`, `market_regime`, `market_phase`, `bias`, `*_assessment` (×5)
   - L4 Opportunity: `forecast_confidence`, `primary_opportunity`, `opportunity_score`, `setup_quality`, `entry_zone`, `target_zone`, `invalidation_level`, `expected_rr_internal`, `time_horizon`
   - L5 Risk: 8 sub-dims (`market_risk`, `volatility_risk`, `execution_liquidity_risk`, `structure_risk`, `momentum_risk`, `signal_risk`, `execution_risk`, `cascade_risk`) + `overall_risk`
   - L6 Decision: `confidence_assessment`, `trade_readiness`, `entry_danger`, `expected_reward_risk_ratio`, `stop_loss_distance_pct`, `protection_strategy`, `target_strategy`, `directional_guidance`, `market_stance`, `strategy_environment`, `entry_guidance`, `exit_guidance`, `final_recommendation`
   - L7 Overview: `systemic_risk_score`, `market_breadth`, `breadth_pct`, `regime_distribution`, `opportunity_distribution`, `risk_distribution`, `cascade_risk_index`, `asset_ranking`, `market_synchronization`, `market_health`, `instance_count`, `active_symbols`
4. **The data plane is unidirectional**: no downstream engine mutates upstream state. The only backward channels are: (1) TAE→PME read-only sizing query; (2) PME→TAE VetoMessage; (3) PME→TAE LiquidateCommand; (4) PAE→config offline analytical feedback. Information flows `Data Infrastructure → Market Monitoring → Trade Automation → Portfolio Management → Performance Analytics`.
5. **Every engine layer produces exactly one immutable matrix** as its output contract.
6. **Engine bifurcation** (MME L4 ∥ L5, converging at L6) is preserved everywhere it is referenced.
7. **Sizing formula** `S = (E × R) / (D_sl / 100)` with `E = available_margin` (Decimal from PME Capital Matrix), `R = risk_per_trade_pct / 100`, `D_sl = stop_loss_distance_pct` (raw percent float from Decision Matrix) — cast to Decimal at the type-boundary handoff (`03-03-03-tae-layer2-execution.md §2`).
8. **Two distinct drawdown metrics**: `max_daily_drawdown_pct` (5% early-warning) and `drawdown_limit_pct` (30% hard veto). See `03-04-05-pme-layer4-portfolio.md §3–§4`.
9. **Candle aggregation** uses exact UTC epoch-multiple boundaries: `interval_start = ⌊timestamp_ms / duration_ms⌋ × duration_ms`. Candles close at `interval_start + duration_ms`. The clock-monitor drift budget is `≤ 100µs` of UTC.
10. **Timeframe weighting**: `w_tf = clamp(duration_seconds / divisor, 0.2, 1.0)`, with `divisor = max(duration_seconds for tier in enabled_tiers)`.
11. **Systemic risk score**: `SystemicRisk = 0.6 × high_pct + 0.4 × sync_penalty`.
12. **Operator identity** is `local_operator` (fixed identity for single-user deployments); multi-user identity is on the v5.0 roadmap.
13. **All cross-doc audit-issue identifiers** (`MAT-##`, `SIG-##`, `EXE-##`, `OPS-##`, `UI-##`, `DB-##`, `API-##`, `AUDIT-##`, `Issue NN`) live **only** in this CHANGELOG. They are not in normative text.
