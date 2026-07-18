# Trading Platform Documentation

This directory contains the full specification for the Trading Platform — a complete quantitative trading system built on a Two-Dimensional Architecture of 5 domain engines across sequenced analytical layers.

> **Numbering scheme.** Every file is named `NN-MM[-KK]-kebab-case.md` so cross-links survive restructuring:
> - `01-` Concept Foundations · `02-` Matrices · `03-` Engines (with engine sub-counter: `03-01` DIE, `03-02` MME, `03-03` TAE, `03-04` PME, `03-05` PAE)
> - `04-02-` MME Indicators (50 registry entries, kebab-case filenames; `04-02-00-` is the master index)
> - `05-02-` MME Signals (12 SignalKinds + master index)
> - `06-` Integration & API · `07-` UI/UX · `08-` Operations & Compliance

## Directory Map

```
docs/
├── README.md                                         ← you are here
├── conceptual-foundations/                           (01 — 8 files)
│   ├── 01-00-introduction-to-quantitative-trading.md ← textbook foundations: EV, returns, Sharpe, sizing curves, non-goals
│   ├── 01-01-ontology.md                             ← formal vocabulary, core concepts, lifecycle
│   ├── 01-02-global-architecture.md                  ← 5-engine blueprint, 2D framework, hybrid memory/math model
│   ├── 01-03-systemic-data-flow.md                   ← chronological event sequences across engines
│   ├── 01-04-timeframe-model.md                      ← 4-tier timeframe model, weighting, UTC alignment
│   ├── 01-05-liquidity-domain.md                     ← Phase 0-4 Liquidity Intelligence architecture
│   ├── 01-06-crate-layout-and-cycles.md              ← 9-crate workspace layout, dependency graph, cycle-breaking design decisions
│   └── 01-07-target-architecture-roadmap.md          ← SoA candle history, Phase-3 book depth, NTP, PD memory
├── matrices/                                         (02 — 15 files)
│   ├── 02-00-matrix-field-ownership.md                ← canonical per-field producer-layer mapping
│   ├── 02-00b-confidence-hierarchy.md                 ← confidence-field rename & flow
│   ├── 02-01-alignment-matrix.md                     ← 10-dimension cross-TF agreement
│   ├── 02-02-analysis-matrix.md                      ← bias, regime, qualitative assessments (pure state)
│   ├── 02-03-data-quality-matrix.md
│   ├── 02-04-decision-matrix.md                      ← TAE input contract (f64 boundary handoff; only synthesis point)
│   ├── 02-05-distribution-matrix.md
│   ├── 02-06-market-data-matrix.md
│   ├── 02-07-metrics-matrix.md                       ← MarketSnapshot schema (dual hot/cold rep)
│   ├── 02-08-opportunity-matrix.md                   ← canonical OpportunityType producer (incl. LiquiditySqueeze)
│   ├── 02-09-overview-matrix.md                      ← global breadth, systemic risk (graded sync_penalty)
│   ├── 02-10-raw-data-matrix.md
│   ├── 02-11-risk-matrix.md                          ← 8 unipolar risk dimensions (pure danger)
│   ├── 02-12-liquidity-matrix.md                     ← Phase 1 LiquidityFlow matrix
│   ├── 02-13-liquidation-cluster-matrix.md            ← Phase 2 LiquidationClusterMatrix
│   ├── 02-14-policy-matrix.md                         ← TAE L1: validated execution directives
│   └── 02-15-execution-matrix.md                      ← TAE L2: persistent order state log (materialized as `open_orders` table)
├── engines/
│   ├── data-infrastructure-engine/                     (03-01 — 6 files)
│   │   ├── 03-01-00-die-end-to-end-flow.md           ← single integrated end-to-end DIE flow narrative
│   │   ├── 03-01-01-die-overview-spec.md             ← DIE boundaries, adapters, fault tolerance
│   │   ├── 03-01-02-die-layer1-raw-data.md
│   │   ├── 03-01-03-die-layer2-market-data.md
│   │   ├── 03-01-04-die-layer3-data-quality.md
│   │   └── 03-01-05-die-layer4-data-distribution.md
│   ├── market-monitoring-engine/                       (03-02 — 12 files)
│   │   ├── 03-02-01-mme-overview-spec.md             ← MME boundaries, pipeline, bifurcation model
│   │   ├── 03-02-02-mme-layer1-metrics.md
│   │   ├── 03-02-03-mme-layer2-alignment.md
│   │   ├── 03-02-04-mme-layer3-analysis.md           ← bifurcation point
│   │   ├── 03-02-05-mme-layer4-opportunity.md        ← ∥ branch (L3 input)
│   │   ├── 03-02-06-mme-layer5-risk.md               ← ∥ branch (L3 input, no L4 dependency)
│   │   ├── 03-02-07-mme-layer6-decision-support.md   ← convergence boundary
│   │   ├── 03-02-08-mme-layer7-overview.md
│   │   ├── 03-02-09-mme-indicators-guide.md          ← indicator interpretation rulebook
│   │   ├── 03-02-10-mme-signals-guide.md             ← 12 SignalKind detection rulebook
│   │   ├── 03-02-11-mme-liquidity-extension.md        ← Phase 0-4 Liquidity Intelligence (L1.5 + L2.5)
│   │   ├── 03-02-12-mme-configurable-activation.md     ← Configurable activation (denylists, AUTO_PAUSED)
│   │   ├── indicators/                               (04-02 — 50 + 1 master index)
│   │   │   ├── 04-02-00-indicator-index.md
│   │   │   ├── 04-02-01-ema-stack.md
│   │   │   ├── … (10 Trend + 07 Momentum + 07 Volume + 06 Volatility + 05 Structure + 04 Regime + 04 Institutional + 07 Derivatives = 50)
│   │   │   └── 04-02-50-depth-bias.md
│   │   └── signals/                                  (05-02 — 12 + 1 master index)
│   │       ├── 05-02-00-signals-index.md
│   │       ├── 05-02-01-divergence.md
│   │       ├── … (12 SignalKinds)
│   │       └── 05-02-12-pattern-forming.md
│   ├── trade-automation-engine/                      (03-03 — 6 files)
│   │   ├── 03-03-01-tae-overview-spec.md             ← TAE boundaries, order lifecycle
│   │   ├── 03-03-02-tae-layer1-policy.md
│   │   ├── 03-03-03-tae-layer2-execution.md          ← f64→Decimal type-boundary cast + §3.3 stance→flag
│   │   ├── 03-03-04-tae-execution-policy-spec.md     ← policy syntax and semantics
│   │   ├── 03-03-05-tae-paper-trading-spec.md        ← simulated matching engine
│   │   └── 03-03-06-tae-instance-lifecycle-spec.md   ← LifecycleState, Gate 0, automation schema (v6.2)
│   ├── portfolio-management-engine/                  (03-04 — 5 files)
│   │   ├── 03-04-01-pme-overview-spec.md             ← PME boundaries, safety veto
│   │   ├── 03-04-02-pme-layer1-position.md
│   │   ├── 03-04-03-pme-layer2-exposure.md
│   │   ├── 03-04-04-pme-layer3-capital.md            ← Decimal ledger (available_margin)
│   │   └── 03-04-05-pme-layer4-portfolio.md
│   └── performance-analytics-engine/                 (03-05 — 5 files)
│       ├── 03-05-01-pae-overview-spec.md             ← PAE boundaries, scheduled tasks
│       ├── 03-05-02-pae-layer1-trade-analytics.md
│       ├── 03-05-03-pae-layer2-strategy-analytics.md ← Monte Carlo sign-randomization
│       ├── 03-05-04-pae-layer3-risk-analytics.md
│       └── 03-05-05-pae-layer4-performance.md
├── integration-and-api/                              (06 — 3 files)
│   ├── 06-00-consumer-onboarding.md                  ← single-page integrator orientation
│   ├── 06-01-api-gateway-contract.md                 ← REST + WebSocket API surface
│   └── 06-02-database-schema-spec.md                 ← 26-table SQLite schema (target)
├── ui-ux/                                            (07 — 4 files)
│   ├── 07-01-ui-overview-spec.md                     ← Svelte 5 architecture, stores
│   ├── 07-02-ui-dashboard-layout.md                  ← viewport grid, panels, components
│   ├── 07-03-ui-chart-component-map.md                ← per-indicator rendering map (50 → 19 dedicated components)
│   └── 07-04-ui-liquidity-panel-spec.md              ← LiquidityPanel (Phase 4)
└── operations-and-compliance/                        (08 — 7 files)
    ├── 08-01-user-manual.md                          ← operator guide (install, launch, monitor, troubleshooting)
    ├── 08-02-pre-trade-risk-controls.md              ← mandatory pre-trade gates, evaluation order, overrides
    ├── 08-03-connection-resilience.md                ← WebSocket reconnect policy + backoff state machine
    ├── 08-04-candle-reconstruction.md                ← gap detection + exchange historical fetch + sub-1m synthesis
    ├── 08-05-connection-quality.md                   ← rolling 1h/6h/24h quality score + dashboard panel
    ├── 08-06-clock-monitor.md                        ← NTP drift enforcement (≤50µs UTC budget)
    └── 08-07-exchange-key-rotation.md                ← exchange-key rotation procedure (pre-rotation, rotation, emergency)
```

Total: **140 markdown files** at v6.4.1 — 137 numbered docs + 3 governance docs (README, CHANGELOG, MANIFEST). Breakdown: 8 conceptual + 17 matrix + **34 engine** (6 DIE + 12 MME + 6 TAE + 5 PME + 5 PAE) + 51 indicator + 13 signal + 3 integration + 4 UI + 7 ops. MME's 7 core layers plus 2 fractional extension layers (L1.5, L2.5) are implemented across **12 specification files** (overview + 7 layer specs + 2 guides + 1 liquidity extension + 1 activation spec).

## The Five Engines

| Engine | Role | Layers | Key Output |
|--------|------|--------|------------|
| **Data Infrastructure Engine (DIE)** `03-01` | Data ingest, normalization, quality, broadcast | 4 | Market Data Matrix |
| **Market Monitoring Engine (MME)** `03-02` | 50 indicators, signals, multi-TF alignment, decision support | 7 (+2 fractional: L1.5, L2.5; L4 ∥ L5, converge at L6) | Decision Matrix + Overview Matrix |
| **Trade Automation Engine (TAE)** `03-03` | Policy evaluation, position sizing, order routing | 2 | Policy Matrix + Execution Matrix |
| **Portfolio Management Engine (PME)** `03-04` | Position tracking, exposure, capital, safety veto | 4 | Portfolio Matrix |
| **Performance Analytics Engine (PAE)** `03-05` | Trade reconstruction, NHST (sign-randomized Monte Carlo), drawdown/Sharpe, regime maps | 4 | Performance Matrix |

## Recommended Reading Order

1. **Conceptual Foundations (`01-`)** — overall architecture and terminology
   - `01-01-ontology.md` — core concepts, design philosophy, lifecycle
   - `01-02-global-architecture.md` — Two-Dimensional Framework + hybrid memory/math architecture (incl. §6 DOD/OOP target)
   - `01-03-systemic-data-flow.md` — how data flows through the system (incl. Sequence A bifurcation)
   - `01-04-timeframe-model.md` — 4-tier timeframe configuration + §3.1 UTC alignment rules
   - `01-06-crate-layout-and-cycles.md` — 9-crate physical workspace, dependency graph, cycle-breaking design rationale (read this when mapping a feature to its crate)

2. **Engine Overviews (`03-01-01`, `03-02-01`, `03-03-01`, `03-04-01`, `03-05-01`)** — each engine's boundaries

3. **Matrices (`02-`)** — data contracts between engine layers
   - Start with `02-07-metrics-matrix.md` (foundational MarketSnapshot schema) → `02-01-alignment-matrix.md` → `02-02-analysis-matrix.md` → `02-08-opportunity-matrix.md` → `02-11-risk-matrix.md` → `02-04-decision-matrix.md` → `02-09-overview-matrix.md`

4. **Engine Layer Specs** — deep-dive into each layer's processing logic (follow `03-XX-NN-…` per engine)

5. **Indicator (`04-02-`) & Signal (`05-02-`) Deep-Dives** — per-component specifications
   - `04-02-00-indicator-index.md` is the master manifest; each indicator file documents math, normalization, signals, configuration
   - `05-02-00-signals-index.md` is the master manifest for the 12 SignalKinds

6. **Integration & API (`06-`)**
   - `06-01-api-gateway-contract.md` → `06-02-database-schema-spec.md`

7. **UI/UX (`07-`)** — frontend architecture and dashboard layout
   - `07-01-ui-overview-spec.md` → `07-02-ui-dashboard-layout.md`

8. **Operations & Compliance (`08-`)** — operator procedures, pre-trade risk gating, and audit
   - `08-01-user-manual.md` → `08-02-pre-trade-risk-controls.md` → `08-03-connection-resilience.md` (followed by `08-04-candle-reconstruction.md` → `08-05-connection-quality.md` → `08-06-clock-monitor.md`)

## Feature Status

This table is the **single source of implementation truth** — every spec in `docs/` describes the **target system**; this register tracks what is built.

| Feature | Status | Spec of record |
|---------|--------|---------------|
| Core cascade (DIE → MME → TAE → PME → PAE) | Specified / Implemented | `01-02`, `03-01`…`03-05` |
| Multi-timeframe indicators (50) | Implemented | `04-02-00` |
| Signal pipeline (12 SignalKinds, 100 declarations) | Implemented | `05-02-00` |
| WebSocket ingestion (Hyperliquid, Bitget) | Implemented | `03-01-01`, `03-01-02` |
| Candle reconstruction | Implemented | `03-01-03`, `08-04` |
| Connection resilience + backoff | Implemented | `08-03` |
| Connection quality tracking + persistence | Implemented | `08-05`, `03-01-00` |
| Clock monitor (NTP) | Implemented | `08-06` |
| Pre-trade risk gates (1–7) | Implemented | `08-02` |
| Position sizing + execution | Implemented | `03-03-02`, `03-03-03` |
| PME safety veto + stance control | Implemented | `03-04-05` |
| Performance analytics (PAE L1–L4) | Implemented | `03-05-01`…`03-05-05` |
| Overview UI panel | Implemented | `07-02`, `03-02-08` |
| Instance lifecycle (Gate 0, lifecycle tables, automation) | Specified; implementation v6.5 (AUDIT-V6-202…207) | `03-03-06` |
| Configurable activation (denylists, config_version, AUTO_PAUSED) | Specified; implementation v6.5 (AUDIT-V6-208…214) | `03-02-12` |
| Pre-dispatch persistence (pre_dispatch_orders table) | Not started | `06-01` §2.9 |
| Liquidity Intelligence (Phases 0-4) | Partial (Phase 0-2 implemented) | `01-05`, `03-02-11` |
| Exchange key rotation | Manual rotation procedure documented (08-07); in-process rotation tool unscheduled (AUDIT-V6-077) | `08-07` |
| Phase-3 REST handlers (`/api/system/clock`, `/api/exchange-status`, `/api/data-quality`) | Implemented — served surface documented in `06-01` §2.11 (v6.4.1); `clock.breach_count` placeholder pending (AUDIT-V6-301) | `06-01` |

## Key Conventions

- All file/directory names are **lowercase-kebab-case** and prefixed `NN-MM[-KK]-…` per section.
- All enum values serialize as **SCREAMING_SNAKE_CASE** (e.g. `STRONG_BULLISH`, `TRENDING_BULL`).
- The **corpus version** is defined by four-point coherence: the value appearing simultaneously in this README's stats line, the `CHANGELOG.md` top entry, the `DOCS-CONSISTENCY-MANIFEST.md` title, and every numbered-doc `**Version:**` stamp (currently 6.4.1).
- All **score→label bands** are lower-inclusive half-open `[a, b)` (e.g. `entry_danger` 20.0 → `LOW`; SetupQuality 85.0 → `PRIME`). The single documented exception is the `MarketBias` NEUTRAL band, closed `[-20, 20]`. Canonical band tables per the MANIFEST §13 Canonical Source Registry.
- All configuration is stored in **`config.toml`** at the workspace root (legacy `config.json` is still recognized as a fallback by `load_config()`).
- Engine communication on the data plane is **unidirectional**: no downstream engine mutates upstream state. The only backward channels are: (1) TAE→PME read-only sizing query; (2) PME→TAE VetoMessage; (3) PME→TAE LiquidateCommand; (4) PAE→config offline analytical feedback.
- Every engine **layer** produces exactly one immutable **Matrix** as its output contract.
- The platform is **strategy-agnostic** — engines interpret markets; execution policies are user-defined.
- MME Layers 4 (Opportunity) and 5 (Risk) execute **in parallel** from L3 (Analysis) and converge at L6 (Decision Support).
- All candle aggregation closes candles at the **exact epoch-duration multiple of UTC** (a 60 s candle for a trade at 123456 ms aligns to `[120000, 180000)`, closing at 180000 ms = `:00.000` of the next minute) — see `01-04-timeframe-model.md §3.1`. Local clock drift budget is ≤ 50 µs of UTC, enforced at runtime by `crates/network-adapters/src/clock_monitor.rs` (configurable via the `[clock_monitor]` section of `config.toml`).
- Position sizing uses **available margin** (`available_margin`), not raw equity, with formula `S = E·R / (D_sl / 100)` (see `03-03-03-tae-layer2-execution.md §2`).
- Divergences are nested `Divergence` signals on the parent indicator key — there are no separate `*_divergence` registry entries (see `04-02-00-indicator-index.md`).
- Monte Carlo significance uses **sign-randomization** (±1 on each PnL), not order-shuffling (see `03-05-03-pae-layer2-strategy-analytics.md §3.3`).
- The PME vetoes new entries by switching the affected symbol's **stance** to `AVOID` *or* `CLOSE_ONLY` per trigger severity (see `03-04-05-pme-layer4-portfolio.md §4.1` and `01-03-systemic-data-flow.md Sequence D`). A `CLOSE_ONLY` stance is a Policy-Layer scope restriction, *not* an order attribute. Every order packet generated from a `CLOSE_ONLY` stance is forced to carry the Execution-Layer **`reduce_only` flag** (a per-order boolean, exchange-native term) — see `03-03-03-tae-layer2-execution.md §3.3`. The DB column `is_reduce_only` and the wire field `reduce_only` mirror Hyperliquid/Bitget/Binance and are intentionally unchanged for exchange-protocol parity.
- Two drawdown metrics exist and are **distinct**:
  - `max_daily_drawdown_pct` — cumulative PnL decline within the trading session; default 5 %; used as an early-warning threshold.
  - `drawdown_limit_pct` — equity peak-to-trough ratio; default 30 %; this is the **hard veto** threshold.
  See `03-04-05-pme-layer4-portfolio.md §3–§4` and `03-04-01-pme-overview-spec.md §3`.
- The registry contains **50 indicators** in 8 functional groups (10 Trend + 7 Momentum + 7 Volume + 6 Volatility + 5 Structure + 4 Regime + 4 Institutional + 7 Derivatives) and **100 signal-kind declarations** across 12 SignalKind types (one declaration per `(indicator, SignalKind)` pair; the `×N` notation in the index counts multiplicity *within* a single declaration, e.g. 5 RSI threshold zones). The 101 → 100 transition is documented in [`01-01-ontology.md` Appendix B §B.3 editor's note](conceptual-foundations/01-01-ontology.md) and is the canonical source of truth for the count. See also Appendix B of `01-01-ontology.md` and `04-02-00-indicator-index.md`.
