# UI Overview Specification

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Approved
**Purpose:** This document specifies the Svelte 5 frontend architecture — state management, rune patterns, WebSocket consumption, store layer, inline shell architecture, and performance targets. Companion to the [UI Dashboard Layout](07-02-ui-dashboard-layout.md).

---

## 1. Technology Stack

| Layer | Technology |
|-------|-----------|
| Framework | Svelte 5 (runes: `$state`, `$derived`, `$effect`) |
| Build | Vite |
| Charts | Lightweight Charts (TradingView) |
| Styling | Scoped CSS Modules (`.module.css`), kebab-case → camelCase via `localsConvention` |
| Visual style | "Premium Dark Cockpit" — Apple-inspired monochrome grid (see [07-02 §10](07-02-ui-dashboard-layout.md)) |
| Static serving | Engine binary serves `crates/frontend/dist/` |

---

## 2. State Management

### 2.1 Singleton Store Pattern

The application uses a module-level singleton `AppStore` (`state.svelte.ts`) accessed via `useAppStore()`. The store **must not** be named `state` — this conflicts with the `$state` rune. The convention is `app` or `store`.

```ts
// state.svelte.ts
import { AppStore } from './state.svelte';
const app = useAppStore();
```

### 2.2 Delegate Architecture

`AppStore` owns four sub-stores as instance fields. Each sub-store is a class that exposes its own `$state` fields and methods.

| Sub-Store | File | Responsibility |
|-----------|------|----------------|
| `SessionStore` | `stores/session.svelte.ts` | Session active state, exchange, currency, init/quit lifecycle, fetch status. |
| `SettingsStore` | `stores/settings.svelte.ts` | Config, indicator registry, ~50 indicator parameters per timeframe, rules content. |
| `AnalyticsStore` | `stores/analytics.svelte.ts` | Dashboard stats, trade ledger, journal, observability, system heartbeat. |
| `ProfileStore` | `stores/profiles.svelte.ts` | Decision/risk profile CRUD, risk calculation, commission projection, fee table. |

```ts
// state.svelte.ts (excerpt)
export class AppStore {
    settings  = new SettingsStore();
    analytics = new AnalyticsStore();
    session   = new SessionStore();
    profiles  = new ProfileStore();

    instancesMap     = $state<Record<string, InstanceState>>({});
    activeTab        = $state<string>('BTC-USDT');
    currentEngine    = $state<EngineKey>('profile');
    middleTab        = $state<string>('overview');
    activeEngineTab  = $state<'overview' | 'instance'>('overview');
    selectedInstance = $state<string | null>(null);
    // ...
}
```

### 2.3 Instance & Telemetry Model

- `instancesMap` — `Record<pairKey, InstanceState>` keyed by pair key (e.g. `"BTC-USDT"`).
- Each `InstanceState` contains **four** `TimeframeTelemetry` sub-objects named `microTerm`, `fastTerm`, `slowTerm`, `macroTerm`. There is **no** `timeframes.micro` namespace — the canonical names are the `*Term` fields directly on `InstanceState`.
- Per-TF telemetry fields:
  - `barDurationSec` — the timeframe duration in seconds (e.g. `60` for micro).
  - `indicators` — the full `NormalizedIndicatorValue` map for that TF.
  - `priceText`, `volText`, `avgVolText` — formatted display strings.
  - `historyPrices`, `latestSnapshot` — chart seed arrays and the most recent raw snapshot.
  - Per-TF indicator parameter scalars (`emaFastVal`, `rsiPeriodVal`, `macdFastVal`, … — ~50 fields).
  - **Liquidity Intelligence (Phase 0-4)** fields: `liquidity: LiquidityFlow | null`, `cluster: LiquidationClusterMatrix | null`, `liquiditySignals: LiquiditySignal[]`. These are surfaced by the WS demux in `crates/frontend/src/api/ws_client.rs` and live directly on each `TimeframeTelemetry` (e.g. `instance.microTerm.liquidity`).

```ts
// state.svelte.ts (excerpt)
function createInstanceState(symbol: string): InstanceState {
    return {
        symbol, exchange: 'Hyperliquid', isConnected: false,
        microTerm: createTimeframeTelemetry(symbol,  60),
        fastTerm:  createTimeframeTelemetry(symbol, 180),
        slowTerm:  createTimeframeTelemetry(symbol, 300),
        macroTerm: createTimeframeTelemetry(symbol, 900),
        historyLatestClose: '0',
        currentView: 'terminal',
        alignment: null, analysis: null, risk: null, advisory: null,
    };
}
```

### 2.4 WebSocket Update Loop (infinite-loop avoidance)

The store uses `$state` for mutable fields and `$derived` for computed views. To prevent `$effect` infinite-evaluation loops during WS frame ingestion, the demux follows these rules:

1. **Snapshot write goes through plain field assignment** (e.g. `tf.latestSnapshot = snap`) — never through a `$derived` that re-derives from itself.
2. **`$effect` blocks read store fields via local copies** at the top, perform side effects, and never mutate fields that the same effect reads.
3. **Heavy aggregation runs in `$derived` chains** that do not mutate source `$state` (e.g. `change24h = $derived.by(...)` reading `microTerm.priceText`).
4. The reconnect `$effect` only triggers when `activeTab` actually changes — see [`crates/frontend/src/App.svelte`](../../crates/frontend/src/App.svelte) for the canonical pattern.

---

## 3. WebSocket Client

**File:** `lib/websocket.svelte.ts`

| Property | Value |
|----------|-------|
| Connections per instance | 4 parallel (micro/fast/slow/macro). |
| URL pattern | `ws://host/ws?symbol=BTC-USDT&timeframe_secs=60`. |
| Protocol | Incoming JSON-RPC 2.0 `broadcast.market_snapshot` notifications. |
| `applySnapshotToTimeframe()` | Parses nested `snapshot`, writes to `*Term.latestSnapshot` and per-TF `indicators` map. |
| Reconnect | Exponential backoff (1 s → 30 s, ±20 % jitter, max 30 retries). |
| Lifecycle | Connect on mount, disconnect on destroy, reconnect when `activeTab` changes. |

---

## 4. API Client

**File:** `lib/api.svelte.ts`

- `fetchConfigFromServer()` — GET `/api/config` with cache busting.
- `applyConfigToStore()` — Parses config, initializes `instancesMap`, applies ~50 indicator parameters per `*Term`.
- `fetchHistory(symbol, timeframe_secs, limit=100)` — GET `/api/history` with cache busting. Called on chart mount before subscribing to WebSocket to seed the historical series; the response shape is `{ symbol, prices[], candles[], indicator_histories }` (see [06-01-api-gateway-contract.md §2.3](../../integration-and-api/06-01-api-gateway-contract.md)).
- `fetchMonitor(symbol)` — GET `/api/monitor` for per-TF regime, MTF agreement, MarketContext.
- `fetchConnectionQuality(window='one_hour')` — GET `/api/connection-quality?window=…` for the Connection Quality panel.
- `createInstance(base, quote)` — POST to create a new pair workspace.
- `saveRulesCall()` / `fetchRulesCall()` — indicator guide CRUD.

---

## 5. Inline Shell Architecture

The application shell renders the three navbars, the two slide-out drawers, and the content area **inline in `App.svelte`**. There is no separate `Sidebar` or `TabHeader` component mounted into the tree at runtime — the entire viewport chrome is composed in `App.svelte` from `class={styles.*}` bindings on the shared `brutalist-grid.module.css`.

This keeps the navigation hierarchy strictly data-driven: each navbar is conditionally mounted based on the trio of `currentEngine`, `middleTab`, and `selectedInstance`.

### 5.1 Navbar Mounting Rules

| Navbar | Mounts when | Tabs | Source field |
|--------|-------------|------|--------------|
| **Top** (Global, always on) | Session is active | Brand trigger · Exchange chip · Workspaces trigger | `app.currentEngine`, `app.sessionExchange`, `app.sessionCurrency`, `app.selectedInstance` |
| **Middle** (Workspace-level) | `!isHome` (any non-Profile engine) | For Market: `Workspace` (forced first) · `Overview` · `Settings`; for other engines: `Overview` · `Settings` | `app.middleTab` |
| **Bottom** (Instance-level) | `currentEngine === 'market_monitor' && middleTab === 'workspace' && selectedInstance` | `Charts` · `Metrics` · `Alignment` · `Opportunities` · `Risks` · `Connection` · `Analysis` · `Decision` · `Liquidity` | `app.activeEngineTab === 'instance'` and `pair.currentView` |

Full wireframes and component placement live in [07-02-ui-dashboard-layout.md](07-02-ui-dashboard-layout.md).

### 5.2 Drawer Overlay Model

Both drawers are conditionally mounted as siblings of `.gridContainer`:

- **Engines Sidebar (Left)** — toggled by `isSidebarOpen`; renders a backdrop overlay + left-anchored panel containing the engine list and `Quit Session` button.
- **Workspaces Sidebar (Right)** — toggled by `isWorkspacePanelOpen`; renders a backdrop overlay + right-anchored panel containing the symbol input, `+` action button, and per-instance rows with pause/delete controls.

The overlays sit above the content area but do **not** affect layout flow — clicking the backdrop dismisses the drawer.

---

## 6. Chart Architecture

Native `Lightweight Charts` canvases, one per indicator. No wrapper framework — raw canvas initialization.

| Pane Component | Indicator |
|----------------|-----------|
| `PriceChart` | OHLCV + EMA overlays + Bollinger/VWAP/Supertrend/Keltner/Donchian + fib levels + SMC markers |
| `RsiChart` | RSI + OB/OS lines |
| `MacdChart` | MACD line/signal/histogram |
| `AdxChart` | ADX + DI+/DI− |
| `AtrChart` | ATR |
| `SqueezeChart` | TTM Squeeze momentum |
| `BbwpChart` | BBWP |
| `VolumeChart` | Volume bars + average |
| `RvolChart` | Relative Volume |
| `StochasticChart` | Stoch %K/%D |
| `ChandeMoChart` | Chande Momentum Oscillator |
| `ObvChart` | On-Balance Volume |
| `CmfChart` | Chaikin Money Flow |
| `MfiChart` | Money Flow Index |
| `HvChart` | Historical Volatility |
| `AroonChart` | Aroon Up/Down |
| `ChoppinessChart` | Choppiness Index |
| `LinRegSlopeChart` | Linear Regression Slope |
| `ZScoreChart` | Z-Score |

The full per-indicator rendering map (overlay vs. dedicated pane vs. shared pane) lives in [07-03-ui-chart-component-map.md](07-03-ui-chart-component-map.md).

---

## 7. CSS Architecture

Every Svelte component with custom styles follows the **Scoped CSS Modules** pattern:

1. The `<style>` block is removed from the `.svelte` file and moved into a companion `[ComponentName].module.css` file in the same directory.
2. The companion module is imported via `import styles from './[ComponentName].module.css'`.
3. CSS class names use kebab-case (`.welcome-card`); Vite's `localsConvention: camelCaseOnly` maps them to camelCase for `<script>` bindings (`styles.welcomeCard`).
4. Conditional class bindings use a template literal: `class="{styles.tab} {isActive ? styles.tabActive : ''}"`.
5. Component-specific styling only — global tokens (palette, typography, spacing) live in `crates/frontend/src/styles/app.css` and `crates/frontend/src/styles/brutalist-grid.module.css`.
6. Chart-only components (AtrChart, RsiChart, MacdChart, SqueezeChart, VolumeChart, AdxChart) that wrap a single canvas via Lightweight Charts with a minimal wrapper style (`.chart-container { width: 100%; height: 100% }`) are exempt from the companion-module requirement.

**No single source file (`.svelte`, `.ts`, `.css`) may exceed 1000 lines of code.**

### 7.1 Vite Configuration (normative)

```ts
// vite.config.ts
export default defineConfig({
    plugins: [svelte(), svelteTesting()],
    css: {
        modules: {
            localsConvention: 'camelCaseOnly',
            generateScopedName: '[name]__[local]___[hash:base64:5]',
        },
    },
    // ...
});
```

The `camelCaseOnly` mode guarantees that `.liquidity-cluster-row` is accessible as `styles.liquidityClusterRow` (NOT also as `styles.liquidity-cluster-row`). This is the only mode supported by the project — `camelCase` (which exposes both forms) is explicitly forbidden because it would allow inconsistent bindings.

### 7.2 Normative Binding Example

```css
/* LiquidityPanel.module.css */
.cluster-row {
    padding: 0.5rem 0.75rem;
}
.cluster-row-active {
    background-color: var(--color-accent);
}
```

```svelte
<!-- LiquidityPanel.svelte -->
<script lang="ts">
    import styles from './LiquidityPanel.module.css';
    let { active = false }: { active?: boolean } = $props();
</script>

<div class="{styles.clusterRow} {active ? styles.clusterRowActive : ''}">
    ...
</div>
```

---

## 8. Performance Targets

| Metric | Target |
|--------|--------|
| WS frame rendering | < 8 ms per frame |
| Chart update cadence | Tick-driven (shadow) + candle-close (completed) |
| Page load (SPA) | < 2 s (static assets served from Rust binary) |
| Drawer open/close | < 50 ms (single CSS transition) |

---

## 9. Cross-References

- [UI Dashboard Layout](07-02-ui-dashboard-layout.md) — Wireframes and component placement for the 3 navbars and 2 drawers.
- [Chart Component Map](07-03-ui-chart-component-map.md) — Per-indicator rendering destinations.
- [Liquidity Panel Spec](07-04-ui-liquidity-panel-spec.md) — Phase 4 Liquidity Intelligence tabs.
- [API Gateway Contract](../integration-and-api/06-01-api-gateway-contract.md) — WS and REST consumed by the frontend.
- [AGENTS.md](../../AGENTS.md) — Build instructions and Svelte 5 conventions.
