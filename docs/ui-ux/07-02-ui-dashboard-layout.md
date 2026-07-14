# UI Dashboard Layout Specification

**Version:** 2.0
**Status:** Approved
**Purpose:** This document specifies the dashboard layout — viewport grid, tab structure, wireframe descriptions of each panel (charts, metrics, alignment, opportunities, risk, analysis, decision, overview, settings), and the equity curve / calendar / workspace panels. Companion to the [UI Overview](07-01-ui-overview-spec.md).

---

## 1. High-Level Shell

```
┌──────────────────────────────────────────────────────────────┐
│ [Sidebar (collapsible)] │  [Tab Header (top-nav)]            │
│   Engine selector       ├───────────────────────────────────┤
│   Instance list         │                                   │
│   Add/remove/pause      │          Main Viewport            │
│   Connection status     │     (charts / panels / forms)     │
│                         │                                   │
├─────────────────────────┼───────────────────────────────────┤
│  [Bottom Console]       │  [Bottom Table / Telemetry]       │
└──────────────────────────────────────────────────────────────┘
```

Brutalist-grid lightweight layout optimized for the 127.0.0.1:3000 local-only context. No responsive design — fixed viewport.

---

## 2. Sidebar (Left Panel)

`Sidebar.svelte` — collapsible, state-driven.

| Section | Contents |
|---------|----------|
| Engine selector | Home / Portfolio / Market / Trading / Analysis (most are placeholder). |
| Instance manager | Per-pair workspace list with live price, 24 h change, pause/delete controls. |
| Connection | Per-WS live status indicator. |

---

## 3. Tab Header (Top Navigation)

`TabHeader.svelte` — renders contextual tabs based on the active engine:

| Engine context | Tabs |
|----------------|------|
| Market — root | Workspace · Overview · Settings |
| Market — workspace + instance | Charts · Metrics · Alignment · Opportunities · Risks · Analysis · Decision |

Active tab drives which panel component renders in the main viewport.

---

## 4. Main Viewport Panels

### 4.1 Charts Panel (`LiveTerminal.svelte`)

The primary chart workspace. Contains:

- **`PriceChart`** — Main OHLCV price chart with overlays. Configurable indicator overlays toggled via `ChartToggles`:
  - EMA ribbon (4 lines)
  - Bollinger Bands (3 lines)
  - VWAP / Anchored VWAP
  - Supertrend line
  - Keltner channel
  - Donchian channel
  - Fibonacci retracement/extension levels

- **Indicator Pane Charts** — Horizontally arranged or stacked panes:
  - RSI, MACD, ADX, ATR, Squeeze, BBWP, Volume, RVOL,
  - Stochastic, ChandeMO, OBV, CMF, MFI, HV,
  - Aroon, Choppiness, LinReg Slope, Z-Score

`ChartToggles` provides enable/disable checkboxes for all 19 indicator panes plus the overlay lines.

### 4.2 Metrics Panel (`TerminalMonitor.svelte`)

Multi-timeframe telemetry grid. Shows per-TF:

- Market context (trend/momentum/volatility/volume/liquidity dimensions + regime + overall).
- MTF indicator agreement matrix (cross-timeframe consensus for each key indicator).
- Signal summary with freshness.

### 4.3 Alignment Panel (`AlignmentPanel.svelte`)

Cross-timeframe alignment matrix display. Renders the 10 Alignment Dimensions as score bars + directional state, per-TF breakdown, trend agreement %. Visualizes timeframe conflict when agreement is low.

### 4.4 Opportunities Panel (`OpportunitiesPanel.svelte`)

Per-setup-type opportunity profiling display. Shows scored candidate setups (Breakout, Trend Continuation, Pullback, etc.) with setup-quality band, precondition checkmarks, and contributing signal labels.

### 4.5 Risk Panel (`RiskPanel.svelte`)

9-dimension unipolar risk display — per-dimension gauges/traffic lights with evidence lists. Overall risk score with trend indicator. Risk distribution across volatility / liquidity / structure / momentum / signal / execution / reward vectors.

### 4.6 Analysis Panel (`AnalysisPanel.svelte`)

Renders the Analysis Matrix: categorical bias + continuous `market_bias_score`, regime, all 7 assessments as qualitative labels, market interpretation text, supporting vs contradicting evidence lists.

### 4.7 Decision Panel (`AdvisoryPanel.svelte`)

Renders the Decision Matrix: directional guidance, market stance, entry/exit/protection/target strategy recommendations, confidence %, and the `final_recommendation` text.

### 4.8 Overview Panel (`OverviewPanel.svelte`)

Renders the [Overview Matrix](../matrices/02-09-overview-matrix.md) — the cross-symbol market synthesis produced by the MME Overview Layer (L7). Consumed by the PME veto loop for systemic risk gating.

**Rendered fields:**

| UI Element | Source Field | Type |
|------------|-------------|------|
| Global Bias badge | `global_market_bias` | Colored chip: green (`STRONG_BULLISH` / `BULLISH`), red (`STRONG_BEARISH` / `BEARISH`), grey (`NEUTRAL` / `MIXED`) |
| Breadth gauge bar | `market_breadth` | Horizontal bar from −100% to +100%, labelled with `STRONG_POSITIVE` … `STRONG_NEGATIVE` |
| Regime distribution | `regime_distribution` | Stacked bar or pie chart; one segment per `MarketRegime` fraction |
| Opportunity distribution | `opportunity_distribution` | Count table per `OpportunityType` |
| Risk traffic lights | `risk_distribution` | Three colored indicators: `low_pct` (green), `moderate_pct` (amber), `high_pct` (red); `risk_environment` label beneath |
| Asset ranking table | `asset_ranking` | Sortable leaderboard: `symbol`, `score`, `bias`, `confidence`, `regime`, `risk_level` |
| Systemic Risk Score | derived | Prominent numeric gauge; computed from `risk_distribution` + synchronization |
| Synchronization indicator | `market_synchronization` | Text label + bar: `HIGHLY_SYNCHRONIZED` … `HIGHLY_FRAGMENTED` |
| Market health | `market_health` | Badge: `POOR` … `STRONG` |
| Global summary | `global_summary` | Natural-language paragraph |
| Instance footer | `instance_count`, `active_symbols` | Summary text: "N active instances across M symbols" |

---

## 5. Bottom Console & Data Table

| Component | Content |
|-----------|---------|
| `BottomConsole` | 4-column grid for notes / trading journal quick-edit. |
| `BottomTable` / `TelemetryTable` | Tabular indicator telemetry — raw values, normalized scores, state labels, signal counts per timeframe. |

---

## 6. Settings & Workspace Panels

### 6.1 Home / Settings (`GeneralSettings.svelte`)

Fee calculator, commission projection, exchange settings (API key configuration via `ExchangeSettings.svelte`), and ATR/leverage multiplier config.

### 6.2 Workspace Settings (`WorkspaceSettings.svelte`)

Instance-specific configuration:

- **`TimeframeSettings`** — Indicator periods per timeframe (40+ params from config).
- **`TriggerConfigPanel`** — Trigger mode (Interval / CandleClose / EventDriven) and frequency.
- **`PositionScalingPanel`** — 4-slot dynamic margin allocation, allocation-curve model selection (Stepped / Linear / Exponential).

---

## 7. Performance & Equity Views

| Component | Data |
|-----------|------|
| `PositionPerformanceChart` | Per-position equity curve, drawdown overlay. |
| `PositionSelector` | Active/historical position selector. |
| `MomentumMeter` | Visual momentum gauge. |

### 7.1 Equity Curve

The `DashboardStats.equity_curve` field provides timestamped cumulative PnL for the equity chart (Lightweight Charts area series). `compounded_curve` provides the compounded-returns variant. Both rendered in the dashboard stats view.

### 7.2 PnL Calendar

`DashboardStats.pnl_calendar` provides a `CalendarDay[]` (date, pnl, month, day) for rendering a GitHub-style PnL heatmap. Currently the data model exists but no dedicated calendar UI component is mounted — the data is accessible but unrendered in the current dashboard shell.

---

## 8. Special Panels

| Panel | Purpose |
|-------|---------|
| `RiskCalculator.svelte` | Interactive risk sizing form: capital, risk %, entry/stop/target, dynamic ATR toggle → live `RiskCalculation` output. |
| `CommissionCalculator.svelte` | Fee projection: dual-entry breakdown, viability check, break-even profit %. |
| `WelcomeGate.svelte` | Session init screen — exchange + currency selection, disabled before session is active. |
| `QuitDialog.svelte` | Session termination confirmation. |

---

## 9. Symbol-Specific Panels

The `instancesMap` pattern means every panel — charts, alignment, risk, analysis, decision, opportunities — is symbol-scoped. Selecting a different instance re-routes the entire viewport to that symbol's data. The sidebar's instance list is the global workspace navigator.

---

## 10. Cross-References

- [UI Overview](07-01-ui-overview-spec.md) — State + technology.
- [API Gateway Contract](../integration-and-api/06-01-api-gateway-contract.md) — Data sources.
- [MME Layer 7 — Overview](../engines/market-monitoring-engine/03-02-08-mme-layer7-overview.md) — OverviewPanel.svelte data source.
- [Decision Matrix](../matrices/02-04-decision-matrix.md) — Decision Matrix panel data.
