# UI Overview Specification

**Version:** 2.0
**Status:** Approved
**Purpose:** This document specifies the Svelte 5 frontend architecture — state management, rune patterns, WebSocket consumption, store layer, tab routing, and performance targets.

---

## 1. Technology Stack

| Layer | Technology |
|-------|-----------|
| Framework | Svelte 5 (runes: `$state`, `$derived`, `$effect`) |
| Build | Vite |
| Charts | Lightweight Charts (TradingView) |
| Styling | Scoped CSS Modules (`.module.css`), kebab-case → camelCase Vite config |
| Static serving | Engine binary serves `crates/frontend/dist/` |

---

## 2. State Management

### 2.1 Singleton Store Pattern

The application uses a module-level singleton `AppStore` (`state.svelte.ts`) accessed via `useAppStore()`. The store **must not** be named `state` — this conflicts with the `$state` rune. The convention is `app` or `store`.

### 2.2 Delegate Architecture

`AppStore` delegates to four sub-stores via `Object.defineProperty`:

| Sub-Store | File | Responsibility |
|-----------|------|----------------|
| `SessionStore` | `stores/session.svelte.ts` | Session active state, currency, exchange, init/quit lifecycle. |
| `SettingsStore` | `stores/settings.svelte.ts` | Config, indicator registry, 40+ indicator parameters per timeframe, rules content. |
| `AnalyticsStore` | `stores/analytics.svelte.ts` | Dashboard stats, trade ledger, journal, observability, system heartbeat. |
| `ProfileStore` | `stores/profiles.svelte.ts` | Decision/risk profile CRUD, risk calculation, commission projection, fee table. |

### 2.3 Instance & Telemetry Model

- `instancesMap` — per-pair `InstanceState` keyed by pair key (e.g. `"BTC-USDT"`).
- Each `InstanceState` contains 4 `TimeframeTelemetry` sub-objects (micro/fast/slow/macro).
- Per-TF telemetry stores: `indicators` (the full `NormalizedIndicatorValue` map), `priceText`, `volText`, and derived chart data arrays.

---

## 3. WebSocket Client

**File:** `lib/websocket.svelte.ts`

| Property | Value |
|----------|-------|
| Connections per instance | 4 parallel (micro/fast/slow/macro). |
| URL pattern | `ws://host/ws?symbol=BTC-USDT&timeframe_secs=60`. |
| Protocol | Incoming JSON-RPC 2.0 `broadcast.market_snapshot` notifications. |
| `applySnapshotToTimeframe()` | Parses nested `snapshot`, writes to rune store. |
| Reconnect | Exponential backoff (1 s → 30 s, max 30 retries). |
| Lifecycle | Connect on mount, disconnect on destroy, reconnect when `activeTab` changes. |

---

## 4. API Client

**File:** `lib/api.svelte.ts`

- `fetchConfigFromServer()` — GET `/api/config` with cache busting.
- `applyConfigToStore()` — Parses config, initializes instances, applies 40+ indicator parameters per timeframe.
- `createInstance()`, `postInstanceConfig()`, `readDraftFromPair()` — workspace management.
- `saveRulesCall()` / `fetchRulesCall()` — indicator guide CRUD.

---

## 5. Tab Routing

Routing is **state-driven** (no URL-based routing). Navigation model:

| Level | Tabs | Component |
|-------|------|-----------|
| Engine (sidebar) | Home / Portfolio (placeholder) / Market / Trading (placeholder) / Analysis (placeholder) | `Sidebar` |
| Middle (Market) | Workspace / Overview / Settings | `TabHeader` |
| Inner (Workspace + instance) | Charts / Metrics / Alignment / Opportunities / Risks / Analysis / Decision | `TabHeader` |

The `currentEngine` field selects the active engine; `middleTab` and `activeEngineTab` control sub-navigation.

---

## 6. Chart Architecture

Native `Lightweight Charts` canvases, one per indicator. No wrapper framework — raw canvas initialization.

| Pane Component | Indicator |
|----------------|-----------|
| `PriceChart` | OHLCV + EMA overlays + Bollinger/VWAP/Supertrend/Keltner/Donchian + fib levels |
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

---

## 7. CSS Architecture

Every Svelte component with custom styles follows the **Scoped CSS Modules** pattern:

1. `<style>` block removed from `.svelte` file.
2. Companion `[ComponentName].module.css` created.
3. Imported as `import styles from './[ComponentName].module.css'`.
4. Classes bound via `class={styles.className}`.
5. Chart-only components (AtrChart, RsiChart, etc.) with minimal wrapper styles are exempt.

---

## 8. Performance Targets

| Metric | Target |
|--------|--------|
| WS frame rendering | < 8 ms per frame |
| Chart update cadence | Tick-driven (shadow) + candle-close (completed) |
| Page load (SPA) | < 2 s (static assets served from Rust binary) |

---

## 9. Cross-References

- [UI Dashboard Layout](07-02-ui-dashboard-layout.md) — Wireframes and component placement.
- [API Gateway Contract](../integration-and-api/06-01-api-gateway-contract.md) — WS and REST consumed by the frontend.
- [AGENTS.md](../../AGENTS.md) — Build instructions and Svelte 5 conventions.
