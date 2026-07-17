# UI Dashboard Layout Specification

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Purpose:** This document specifies the dashboard layout — viewport grid, the three-tier navbar model, the two slide-out drawers, the wireframes of each panel (charts, metrics, alignment, opportunities, risk, analysis, decision, overview, settings), and the internal sub-sidebar pattern. Companion to the [UI Overview](07-01-ui-overview-spec.md).

---

## 1. High-Level Shell

The viewport is composed of **three independently-mounted navbars** stacked above a single content area, with **two slide-out drawers** that overlay the content. All shell chrome is rendered inline in `App.svelte` against the shared `brutalist-grid.module.css` (see [UI Overview §5](07-01-ui-overview-spec.md)).

```
┌──────────────────────────────────────────────────────────────────────────┐
│ NAVBAR 1 — Top (Global, always on)                                       │
│  [TRADING PLATFORM ▼] [Hyperliquid · USDC]  ...........  [Workspaces ▶]  │
├──────────────────────────────────────────────────────────────────────────┤
│ NAVBAR 2 — Middle (Workspace-level)  ····· mounts when !isHome ·····    │
│  [Workspace] [Overview] [Settings]                                       │
├──────────────────────────────────────────────────────────────────────────┤
│ NAVBAR 3 — Bottom (Instance-level) ··· mounts when Market+Workspace+Sel ·│
│  [Charts][Metrics][Alignment][Opps][Risks][Conn][Analysis][Dec][Liquid]  │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│                       MAIN CONTENT AREA                                  │
│           (charts / panels / forms / settings pages)                    │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘

LEFT DRAWER (Engines Sidebar)        RIGHT DRAWER (Workspaces Sidebar)
 · overlay, not in-flow               · overlay, not in-flow
 · slides from left                   · slides from right
 · engine list + Quit Session         · symbol input + instance list
```

**CSS contract:**

| Class | Height | Background | Border |
|-------|--------|-----------|--------|
| `.rowNavbar` | 52 px | `--bg` (`#000`) | bottom 1 px `--line` |
| `.rowTabs` | 40 px | `--bg-elev` (`#0a0a0a`) | (none — sits flush above rowSubTabs) |
| `.rowSubTabs` | 40 px | `--bg-elev-2` (`#0f0f0f`) | (none — flush against content area) |

Brutalist-grid lightweight layout optimized for the `127.0.0.1:3000` local-only context. No responsive design for the shell itself — fixed viewport. Inner panels (e.g. `TimeframeSettings`) may use responsive grids for their content (see [§9](#9-timeframesettings-grid)).

---

## 2. Top Navbar (Global State & Controls)

The Top Navbar is always rendered while the session is active. It uses a 4-column grid (`auto auto 1fr auto`) that distributes: Brand trigger · Exchange chip · spacer · Workspaces trigger.

| Cell | Content | Behavior |
|------|---------|----------|
| **Brand trigger** | `TRADING PLATFORM` (when on Home) or the active engine label (`MARKET`, `PORTFOLIO`, `TRADING`, `ANALYSIS`) | Click toggles the Engines Sidebar (left drawer). Shows a downward chevron on Home, the engine icon elsewhere. |
| **Exchange chip** | `{app.sessionExchange} · {app.sessionCurrency}` (e.g. `Hyperliquid · USDC`) | Read-only. Monospace font, dim text. |
| **Spacer** | empty | flex-grow column. |
| **Workspaces trigger** | When no instance is selected: `Workspaces` label + 2x2 grid icon. When an instance is selected: pair label + live price + 24 h change % | Click toggles the Workspaces Sidebar (right drawer). |

**Class bindings:** `class="{styles.cell} {styles.cellBrand} {styles.cellNavbar} {styles.cellClickable}"` on the brand cell; `class="{styles.cell} {styles.cellMono} {styles.cellNavbar}"` on the exchange chip; `class="{styles.cell} {styles.cellNavbar} {styles.cellClickable} {isWorkspacePanelOpen ? styles.cellActive : ''}"` on the workspaces cell.

**Live data:** When an instance is selected, the workspaces cell replaces the static label with three inline sub-cells — pair (e.g. `BTC/USDC`), `microTerm.priceText` formatted as a price, and the 24 h change percentage computed from `microTerm.latestSnapshot.mid_price` vs `prev_day_px`. The change cell color binds to `styles.changeUp` / `styles.changeDown` / `styles.changeFlat` based on sign.

---

## 3. Middle Navbar (Workspace-Level)

The Middle Navbar mounts when `!isHome` (any non-Profile engine). It is a single horizontal row of tab cells that switch `app.middleTab`.

### 3.1 Mounting Rules

| Engine | Tabs (left-to-right) |
|--------|---------------------|
| `profile` (Home) | *Navbar hidden entirely* (the `!isHome` guard collapses both Middle and Bottom). |
| `market_monitor` (Market) | `Workspace` (forced first) · `Overview` · `Settings` |
| `portfolio` (Portfolio) | `Overview` · `Settings` |
| `trade_automation` (Trading) | `Overview` · `Settings` |
| `performance` (Analysis) | `Overview` · `Settings` |

`Workspace` is hard-coded for Market because the Market engine is the only one with active workspace instances; the other engines render the generic two-tab pair. Selecting `Workspace` from a non-Market engine is impossible by construction (the tab is not rendered).

### 3.2 Active Tab Behavior

- Clicking any tab sets `app.middleTab = key`.
- The active tab receives `styles.cellActive` (subtle background lift via `rgba(255,255,255,0.08)`).
- For Market, when `middleTab === 'workspace'` and `selectedInstance` is set, the Bottom Navbar mounts (see [§4](#4-bottom-navbar-instance-level)).

### 3.3 Content Dispatch

```svelte
{#if app.currentEngine === 'profile'}
    <GeneralSettings />
{:else if app.currentEngine === 'market_monitor'}
    {#if app.middleTab === 'workspace'}
        {#if app.selectedInstance && activePair}
            <!-- instance view, see §4 -->
        {:else}
            <GeneralDashboard />
        {/if}
    {:else if app.middleTab === 'overview'}
        <GeneralDashboard />
    {:else}
        <WorkspaceSettings pair={activePair} tabKey={app.activeTab} />
    {/if}
{:else}
    <!-- placeholder for non-Market engines -->
{/if}
```

---

## 4. Bottom Navbar (Instance-Level)

The Bottom Navbar mounts **only** when all three conditions hold:

1. `app.currentEngine === 'market_monitor'`
2. `app.middleTab === 'workspace'`
3. `app.selectedInstance && activePair` (a workspace instance is selected)

It applies an additional CSS class `styles.rowSubTabs` on top of `styles.rowTabs`, lifting the background to `--bg-elev-2` to visually distinguish it from the Middle Navbar above.

### 4.1 Tabs

| Tab key (`CurrentView`) | Label | Component |
|-------------------------|-------|-----------|
| `terminal` | Charts | `LiveTerminal` |
| `monitor` | Metrics | `TerminalMonitor` |
| `alignment` | Alignment | `AlignmentPanel` |
| `opportunity` | Opportunities | `OpportunitiesPanel` |
| `risk` | Risks | `RiskPanel` |
| `analysis` | Analysis | `AnalysisPanel` |
| `advisory` | Decision | `AdvisoryPanel` |

### 4.2 Active Tab Behavior

- Clicking a tab sets `activePair.currentView = view` and `app.activeEngineTab = 'instance'`.
- The active cell receives both `styles.cellActive` and `styles.cellActiveUnderline` (a 2 px bottom border accent).
- When an instance is deselected (e.g. via the Workspaces Sidebar delete action), the Bottom Navbar unmounts; the content area falls back to `GeneralDashboard`.

### 4.3 Removed Tabs (v6.0)

- **`Liquidity`** — removed as a standalone tab. The liquidation cluster heatmap and cascade risk data belong to the Metrics Layer (L1) and render inline on the Charts tab alongside the price chart and indicator panes. See `03-02-02-mme-layer1-metrics.md §Liquidity fields` and the deprecated `07-04-ui-liquidity-panel-spec.md`.
- **`Connection`** — moved to the new **Data Infrastructure** engine. Connection quality (WebSocket uptime, disconnect count, reconnect latency, composite score) is now accessible under Data Infrastructure → Overview → Connectivity (see §7).

---

## 5. Engines Sidebar (Left Drawer)

The Engines Sidebar slides out from the **left edge** when `isSidebarOpen` is `true`. It is a sibling of `.gridContainer`, not a child of any cell — clicking the backdrop overlay closes it.

### 5.1 Wireframe

```
┌─────────────────────────────────┐
│                                 │
│  TRADING PLATFORM               │ ← sidebarBrand (top, uppercase)
│                                 │
│  [⊞] Data Infrastructure        │
│  [∿] Market Monitoring     ●    │ ← active (cellActive)
│  [$] Trade Automation           │
│  [⊡] Portfolio Management       │
│  [⊕] Performance Analytics      │
│                                 │
│                                 │
│                                 │
│  [⏻] Quit Session               │ ← sidebarFooter (bottom-anchored)
└─────────────────────────────────┘
```

### 5.2 Components

| Element | Class | Notes |
|---------|-------|-------|
| Backdrop overlay | `styles.sidebarOverlay` | Fixed-position; `role="presentation"`; click closes the drawer. |
| Panel container | `styles.sidebarPanel` | Slides from left via transform animation. |
| Brand label | `styles.sidebarBrand` | Plain `TRADING PLATFORM` text; non-interactive. |
| Nav container | `styles.sidebarNav` | Holds the engine buttons. |
| Engine button | `styles.sidebarItem` (+ `styles.sidebarItemActive` if current) | Real `<button>` element with inline SVG icon + label. |
| Footer | `styles.sidebarFooter` | Anchored to panel bottom. |
| Quit Session | `styles.sidebarQuitBtn` | Triggers `showQuitDialog = true` after closing the drawer. |

### 5.3 Engine Mapping

| Display label | Internal key | Active content when selected |
|---------------|--------------|-------------------------------|
| Data Infrastructure | `data_infra` | `DataInfraDashboard` — lateral panel with Connectivity (moved from Market Monitor), Exchange Status, NTP Clock Monitor. Overview + Settings tabs. |
| Market Monitoring | `market_monitor` | Full Market cockpit — Workspace / Overview / Settings middle tabs + per-instance sub-tabs (Charts, Metrics, Alignment, Opportunities, Risks, Analysis, Decision). |
| Trade Automation | `trade_automation` | `EngineOverview` card describing the execution policy engine, paper/live trading path, and sizing protocol. Settings tab for strategy config. |
| Portfolio Management | `portfolio` | `EngineOverview` card describing position tracking, margin utilization, exposure, and safety veto. Settings tab for safety/fees config. |
| Performance Analytics | `performance` | `EngineOverview` card describing dashboard stats, strategy optimizer, Monte Carlo significance testing. Settings tab for analytics cadences. |
| Analysis | `performance` | Placeholder ("Coming soon"). |

### 5.4 Quit Session Flow

1. User clicks **Quit Session** in the sidebar footer.
2. Drawer closes (`isSidebarOpen = false`).
3. `showQuitDialog = true` mounts the centered `<QuitDialog>` modal.
4. The modal is a `QuitDialog.svelte` overlay with an `onclose` callback that clears `showQuitDialog`.
5. Confirming the dialog tears down the session via the engine's `/api/session/quit` endpoint and returns the user to `WelcomeGate`.

---

## 6. Workspaces Sidebar (Right Drawer)

The Workspaces Sidebar slides out from the **right edge** when `isWorkspacePanelOpen` is `true`. It is the primary control surface for managing active workspace instances.

### 6.1 Wireframe

```
                              ┌──────────────────────────────────┐
                              │  [⊞] Workspaces            ✕    │ ← wsPanelHeader
                              ├──────────────────────────────────┤
                              │  [ BTC      ][USDC][ + ]        │ ← wsPanelCreateBar
                              │                                  │
                              │  ●  BTC/USDC   64912  +0.40%     │ ← wsPanelRow
                              │                       ⏸     🗑    │
                              │  ●  ETH/USDC   3245   −1.20%     │
                              │                       ⏸     🗑    │
                              │  ●  SOL/USDC   142.3  +2.10%     │
                              │                       ⏸     🗑    │
                              │                                  │
                              └──────────────────────────────────┘
```

### 6.2 Components

| Element | Class | Behavior |
|---------|-------|----------|
| Backdrop overlay | `styles.workspacePanelOverlay` | Click closes the drawer and clears any pending inline confirmation. |
| Panel container | `styles.workspacePanel` | Slides from right via transform animation. |
| Header | `styles.wsPanelHeader` | Title (`[⊞] Workspaces`) + close button (`✕`). |
| Title | `styles.wsPanelTitle` | Inline SVG icon + `Workspaces` text. |
| Close button | `styles.wsPanelClose` | Click closes the drawer. |
| Create bar | `styles.wsPanelCreateBar` | Holds the symbol input, quote chip, and `+` button. |
| Symbol input | `styles.wsPanelInput` | `<input type="text" maxlength="10" placeholder="Symbol (e.g. BTC)" />`. Submit on Enter. |
| Quote chip | `styles.wsPanelQuoteChip` | Read-only `{app.quote}` (e.g. `USDC`). |
| Create button | `styles.wsPanelCreateBtn` | Disabled while `createLoading` or empty input. Shows a 3-dot waving spinner while loading. |
| Error line | `styles.wsPanelError` | Inline error from `/api/instances` POST (e.g. symbol already exists). |
| List | `styles.wsPanelList` | Empty-state messages or one row per instance. |
| Empty state | `styles.wsPanelEmpty` | Shown when `wsLoading` or no instances exist. |
| Row | `styles.wsPanelRow` | Click selects the instance via `app.enterInstance(pairKey)` and closes the drawer. |
| Status dot | `styles.statusDot` + variant class | `running` (green) · `paused` (amber) · `stopped` (grey). |
| Pair | `styles.wsPanelPair` | `BTC/USDC` display. |
| Price | `styles.wsPanelPrice` | `microTerm.priceText`. |
| Change % | `styles.change` + variant | 24 h change % colored up/down/flat. |
| Action button | `styles.wsPanelActionBtn` | Holds pause (`⏸`) and delete (`🗑`) controls. Delete variant adds `styles.danger`. |
| Inline confirm | `styles.confirmRow` | Replaces the icon when the user clicks an action; shows `Cancel` + `Confirm` buttons. |
| Confirm button | `styles.confirmBtn` (+ `styles.confirmBtnDanger` for delete) | Real `<button>` element. |

### 6.3 Input States

| State | Visual | Trigger |
|-------|--------|---------|
| Empty input | `+` button disabled | `!newBase.trim()` |
| Typing | Live clear of `createError` | `oninput` handler |
| Submitting | `+` replaced by 3-dot waving spinner | During `createLoading` |
| Server error | Red error line below create bar | `result.ok === false` |
| Success | Input cleared, instance appears in list | `result.ok === true` |

### 6.4 Row Inline Confirmation

Clicking an action button (`⏸` or `🗑`) **does not** immediately mutate server state. Instead, the icon is replaced by an inline two-button confirm row:

```
Cancel  Pause      ← pause variant
Cancel  Delete     ← delete variant (danger)
```

This keeps the action reversible with one click and prevents accidental terminations. Confirming the action calls `POST /api/instances/{id}/{action}` (pause/resume) or `DELETE /api/instances/{id}` (delete). On delete, if the deleted pair was `selectedInstance`, the store calls `app.exitInstance()` to drop back to the empty Market view.

> **Lifecycle controls (v6.2).** The inline-confirm pattern extends to **Start** and **Stop** actions. Each instance row carries a lifecycle badge (RUNNING / instance PAUSED / STOPPING-flashing / STOPPED) and three lifecycle action icons: `▶ Start` (visible on PAUSED/STOPPED), `⏸ Pause` (visible on RUNNING), `■ Stop` (danger-styled like Delete, visible on RUNNING/PAUSED). An **automation summary line** lists active `start`/`pause`/`stop` conditions with an inline edit affordance that re-arms any edited condition per [03-03-06 IL-12](../engines/trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md). STOPPED instances remain fully navigable across every analytics page; deleted instances vanish from the list. See [03-03-06 §3/§6](../engines/trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md).

---

## 7. Internal Sub-Sidebar Pattern (distinct from global drawers)

Inside a selected engine page, a **static in-content sub-sidebar** may render a vertical menu of sub-views. This is a layout component of that page — it does **not** slide, does **not** overlay, and is fully contained within the content area.

### 7.1 Canonical Example — `GeneralSettings`

The Profile engine (`currentEngine === 'profile'`) mounts `GeneralSettings` full-viewport. Its layout is a 2-column grid: a vertical sub-sidebar on the left and the active form on the right.

| Sub-sidebar item | Section key | Form rendered |
|------------------|-------------|---------------|
| Fee Projection | `'fee'` | `FeeReferenceCalculator` (see [§8.1](#81-feereferencecalculator)). |
| Settings | `'settings'` | `ApiFailover` form (see [§8.2](#82-apifailover)). |

The sub-sidebar uses the same `sidebarItem` / `sidebarItemActive` styling pattern as the global Engines Sidebar but lives inside `<div class={styles.profileLayout}>` — there is no overlay, no backdrop, no transform animation.

### 7.2 Distinguishing Rules

| Property | Global slide-out drawer | Internal sub-sidebar |
|----------|-------------------------|----------------------|
| Triggered by | Top navbar click | Component-local state (`activeSection`) |
| Mounted as | Sibling of `.gridContainer` | Child of the page's content area |
| Animation | CSS transform slide | None |
| Backdrop overlay | Yes | No |
| Affects content layout | No | Yes (defines the page's own grid) |

This separation is critical: a global drawer can be opened over any page, while a sub-sidebar only exists inside its host page.

---

## 8. Interactive Form Views

### 8.1 FeeReferenceCalculator

Located in the **Fee Projection** sub-section of `GeneralSettings`. Three inputs and four derived outputs:

**Inputs:**

| Field | Type | Range | Step | Default |
|-------|------|-------|------|---------|
| Leverage | number | 1–150 | 1 | 10 |
| Capital ($) | number | ≥ 1 | 100 | 1000 |
| Exchange Fee (%) | number | 0–10 | 0.01 | 0.06 |

**Derived outputs:**

| Output | Formula | Warning threshold |
|--------|---------|-------------------|
| Notional Value | `capital × leverage` | — |
| Round-Trip Fees | `(fee_pct / 100) × notional × 2` | — |
| Min Profit to Cover | same as Round-Trip Fees | — |
| Min Profit % | `(fees / capital) × 100` | amber (`styles.feeWarn`) when > 3 % |

The "Min Profit %" cell is the only field with a state-based color shift — color alone is never the sole carrier of meaning (the numeric value is always shown).

### 8.2 ApiFailover

Located in the **Settings** sub-section of `GeneralSettings`. Three numeric inputs persisted to `config.api_failover` via `POST /api/config`:

| Field | Config key | Range | Default |
|-------|------------|-------|---------|
| Max Retries Per Call | `max_retries_per_call` | 1–20 | 5 |
| Retry Delay (seconds) | `retry_delay_seconds` | 1–300 | 30 |
| Max Consecutive Failures | `max_consecutive_failures` | 1–50 | 10 |

**Save button states:**

| Status | Class | Label |
|--------|-------|-------|
| `idle` | `styles.saveBtn` | `Save API Failover` |
| `saving` | `styles.saveBtn` + `disabled` | `Saving...` |
| `success` | `styles.saveBtn` | `Saved` (auto-reverts to `idle` after 2 s) |
| `error` | `styles.saveBtn` | `Save API Failover` (no visible change; consider an error toast — currently silent retry) |

The form is loaded once via `GET /api/config` on mount (`$effect`) and re-loaded if the parent `GeneralSettings` remounts.

---

## 9. TimeframeSettings Grid

The instance-level settings page (`WorkspaceSettings` → `TimeframeSettings`) renders a responsive 4-column grid of timeframe cards, one card per timeframe (micro / fast / slow / macro).

### 9.1 Layout

```css
.cards-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
}
@media (max-width: 1400px) { .cards-grid { grid-template-columns: repeat(2, 1fr); } }
@media (max-width:  800px) { .cards-grid { grid-template-columns: 1fr; } }
```

| Viewport width | Columns |
|----------------|---------|
| > 1400 px | 4 (one card per TF) |
| 800–1400 px | 2 (TF cards wrap to a 2-column grid) |
| < 800 px | 1 (single column stack) |

### 9.2 Card Anatomy

Each `.term-card` contains:

1. **Card title** (`.card-title`) — uppercase TF name (e.g. `MICRO`).
2. **TF select** (`.tf-select`) — single `<select>` bound to `TIMEFRAME_OPTIONS`; sets `barDurationSec`.
3. **Indicator inputs scroll** (`.indicator-inputs-scroll`) — `max-height: 520 px`, `overflow-y: auto`, contains a vertical list of `.input-row` entries.

### 9.3 Indicator Inputs (~50 rows per card)

Each input row is a label + numeric input pair (`flex; justify-content: space-between`). The full list per timeframe (mirrors `TermDraft` in `TimeframeSettings.svelte`):

| Group | Fields | Count |
|-------|--------|-------|
| EMA stack | `emaFast`, `emaMedium`, `emaSlow`, `emaLong` | 4 |
| RSI / MACD | `rsiPeriod`, `macdFast`, `macdSlow`, `macdSignal` | 4 |
| MACD thresholds | `macdExtremeHigh`, `macdExtremeLow`, `macdContraction` | 3 |
| ADX | `adxPeriod`, `atrPeriod`, `squeezePeriod`, `adxTrendThreshold`, `adxExhaustionThreshold`, `adxSlopeLookback` | 6 |
| Squeeze | `squeezeMinDuration`, `squeezeBbPeriod`, `squeezeBbStdDev`, `squeezeKcPeriod`, `squeezeKcAtrMult` | 5 |
| Bollinger / BBWP | `bbwpPeriod`, `bbwpLookback` | 2 |
| Stochastic | `stochKPeriod`, `stochDPeriod`, `stochSPeriod` | 3 |
| Chande MO | `chandemoPeriod` | 1 |
| Supertrend | `supertrendPeriod`, `supertrendMultiplier` | 2 |
| Keltner | `keltnerEmaPeriod`, `keltnerAtrPeriod`, `keltnerMultiplier` | 3 |
| Donchian / OBV / CMF / MFI / HV | `donchianPeriod`, `obvSmoothing`, `cmfPeriod`, `mfiPeriod`, `hvPeriod` | 5 |
| Aroon / Choppiness / LinReg / Z-Score | `aroonPeriod`, `chopPeriod`, `linregPeriod`, `zscorePeriod` | 4 |
| ATR / R:R | `atrMultiplier`, `atrTargetRR` | 2 |
| Volume / RVOL | `volumeAvgPeriod`, `rvolInstitutional`, `rvolClimax` | 3 |
| Duration / limit | `durationSeconds`, `analysisLimit` | 2 |
| **Total per TF** | | **~50** |

### 9.4 Save Behavior

- **Save button:** bottom of the page (outside the grid), single instance-level apply.
- **Status feedback:** `saveStatus` flows through `idle` → `saving` → `success` / `error`.
- **On success:** the four `*Term` objects are mutated in-place via `applyTermToTelemetry(...)`; `latestSnapshot` is cleared so the next WS frame re-seeds the chart.

---

## 10. Visual Design — Premium Dark Cockpit

The shell uses the **Premium Dark Cockpit** aesthetic (see `brutalist-grid.module.css`). It is intentionally NOT "brutalist" in the rough-architectural sense — it is a refined monochrome dark theme inspired by Apple's pro tool palette.

| Token | Value | Used for |
|-------|-------|----------|
| `--bg` | `#000000` | Top navbar, page background |
| `--bg-elev` | `#0a0a0a` | Middle navbar, elevated surfaces |
| `--bg-elev-2` | `#0f0f0f` | Bottom navbar, deeper surfaces |
| `--line` | `rgba(255, 255, 255, 0.06)` | Hairline dividers (1 px) |
| `--line-strong` | `rgba(255, 255, 255, 0.12)` | Active cell borders, focus rings |
| `--text` | `#f5f5f7` | Primary text |
| `--text-dim` | `rgba(245, 245, 247, 0.55)` | Secondary text, inactive tabs |
| `--hover` | `rgba(255, 255, 255, 0.04)` | Hover background |
| `--active` | `rgba(255, 255, 255, 0.08)` | Active cell background lift |
| `--sans` | `-apple-system, "SF Pro Display", ...` | Body type |
| `--mono` | `"SF Mono", "JetBrains Mono", ...` | Numerics, prices, code |

**Typography:** 11 px uppercase tracked-out labels (`letter-spacing: 0.06em`, `font-weight: 600`) for navbar cells and section headers; 13 px for the brand trigger; 12 px monospace for prices and exchange chips.

**Buttons:** Solid white (filled `#fff`, black text) reserved for primary CTAs only (e.g. Save buttons); outlined (`1 px solid --line-strong`, transparent fill) for secondary actions (Cancel, inline confirm); subtle background-only (`--hover` / `--active`) for navbar tab cells.

**Containers:** 1 px hairline borders with `border-radius: 8–10 px`. No drop shadows. No gradients except on the welcome/loading screen.

---

## 11. Special Panels

| Panel | Purpose |
|-------|---------|
| `RiskCalculator.svelte` | Interactive risk sizing form: capital, risk %, entry/stop/target, dynamic ATR toggle → live `RiskCalculation` output. |
| `CommissionCalculator.svelte` | Fee projection: dual-entry breakdown, viability check, break-even profit %. |
| `WelcomeGate.svelte` | Session init screen — exchange + currency selection, disabled before session is active. |
| `QuitDialog.svelte` | Session termination confirmation modal (triggered from Engines Sidebar footer). |

---

## 12. Symbol-Specific Panels

The `instancesMap` pattern means every panel mounted by the Bottom Navbar is **symbol-scoped**. Selecting a different instance re-routes the entire content area (and re-binds `pairKey={app.activeTab}` on every mounted component) to that symbol's `*Term` data. The Workspaces Sidebar is the global instance navigator; the Bottom Navbar selects which telemetry tab renders for the active instance.

---

## 13. Performance & Equity Views

| Component | Data |
|-----------|------|
| `PositionPerformanceChart` | Per-position equity curve, drawdown overlay. |
| `PositionSelector` | Active/historical position selector. |
| `MomentumMeter` | Visual momentum gauge. |

### 13.1 Equity Curve

`DashboardStats.equity_curve` provides timestamped cumulative PnL for the equity chart (Lightweight Charts area series). `compounded_curve` provides the compounded-returns variant.

### 13.2 PnL Calendar

`DashboardStats.pnl_calendar` provides a `CalendarDay[]` (date, pnl, month, day) for rendering a GitHub-style PnL heatmap. The data model exists; no dedicated calendar UI component is mounted in the current shell.

---

## 14. Cross-References

- [UI Overview](07-01-ui-overview-spec.md) — State, runes, CSS contract, inline-shell rationale.
- [Chart Component Map](07-03-ui-chart-component-map.md) — Per-indicator rendering destinations for the Charts tab.
- [Liquidity Panel Spec](07-04-ui-liquidity-panel-spec.md) — Phase 4 Liquidity Intelligence tabs.
- [API Gateway Contract](../integration-and-api/06-01-api-gateway-contract.md) — Data sources for the WS demux.
- [MME Layer 7 — Overview](../engines/market-monitoring-engine/03-02-08-mme-layer7-overview.md) — OverviewPanel data source.
- [Decision Matrix](../matrices/02-04-decision-matrix.md) — Decision Matrix panel data.
