# MASTER SPECIFICATION — Unified Indicator System (Final Desired State)

Audit reference for the 30-indicator upgrade (Ichimoku + Pivot Points are the final
Advanced tier and are NOT part of the current build; the other 28 are). Describes
every indicator end-to-end (backend → transport → frontend), its functional group,
class (Leading/Hybrid/Lagging), scoring role, render location, signals, and where its
documentation/functionality lives. Use the Part VIII checklist to verify each phase
with zero regressions.

## Part I — Architecture layers (data flow, begin → end)

```
L0 Ingestion      Hyperliquid WS trades ──► CandleGenerator (micro60 / fast180 / slow300 / macro900)
L1 Calculators    crates/shared/src/indicators/<name>.rs  (pure math, Decimal, stateful update())
L2 Normalize+Sig  crates/shared/src/indicators/normalized/*  → NormalizedIndicatorValue { raw, normalized[-1,1], state_label, values{}, signals[] }
L3 Registry       crates/shared/src/indicators/registry.rs   (INDICATORS: &[IndicatorMeta] — SINGLE SOURCE OF TRUTH)
L4 Assembly       crates/engine/src/analyzer/{normalize,warm,mod}.rs → MarketSnapshot.indicators: HashMap<String, NormalizedIndicatorValue>
L5 Persist        crates/engine/src/db/queries/snapshots.rs  (dedicated cols + auxiliary_normalized_data JSON blob carrying full map incl. signals)
L5 Transport      WS /ws · REST /api/history (time-aligned arrays) · /api/config (registry manifest)
L6 FE State       crates/frontend/src/state.svelte.ts + stores/settings + lib/api.svelte.ts
L7 Render         Price overlays (PriceChart) · Levels (price lines) · Panes (<Name>Chart) · Markers · TelemetryTable (rows+badges)
L8 Scoring        crates/engine/src/profile_evaluation/scoring.rs (registry-driven: directional signed Σ + non-directional gates)
L9 AI             crates/engine/src/llm/{pipeline,agents,prompts}.rs + docs/indicators-guide.md
```

Golden rule: adding/altering an indicator = one registry entry + its L1 calculator +
L2 mapper + (its chart component). All L6/L7 lists (TelemetryTable rows, toggle
buttons, scoring UI) are derived from the registry manifest, not hand-maintained.

## Part II — The Registry (single source of truth)

`IndicatorMeta` fields (Rust, serde-serialized, exposed in `/api/config.indicators`):

| Field | Meaning |
|---|---|
| `key` | map key, e.g. `supertrend` |
| `display_name` | UI label |
| `group` | functional: Trend·Momentum·Volume·Volatility·Structure·Regime·Advanced |
| `class` | Leading·Hybrid·Lagging |
| `render` | Pane·PriceOverlay·PriceLevels·Marker |
| `directional` | true = signed [-1,1] scoring contributor; false = gate/multiplier |
| `supports_divergence` | eligible for divergence detection/lines |
| `signal_types` | capability list (SignalKind[]) |
| `default_weight` | scoring weight (1.0) |
| `default_enabled` | scoring on/off |
| `config_params` | period keys |
| `value_format` / `value_source` | frontend raw-cell render hint |
| `color` | primary chart color |
| `guide_section` | docs section id for the AI rulebook |

## Part III — Normalized value + Signals model

`NormalizedIndicatorValue { raw_value, normalized ∈ [-1,1], state_label, values?, signals[] }`

`IndicatorSignal { kind, direction (Bullish|Bearish|Neutral), status (Potential|Confirmed|Active), label, strength, points? }`

`SignalKind = Divergence | Crossover | Threshold | Breakout | BandTouch | ZeroLineCross | CompressionRelease | LevelTest | TrendFlip | VolumeClimax | StackChange | PatternForming`

Every signal an indicator produces this snapshot is recorded in `signals[]`, rides the
WS broadcast + DB JSON blob (no migration), and is shown as TelemetryTable badges +
Lightweight-Charts markers. `points` is populated for `Divergence` (future line drawing).

## Part IV — Scoring system (registry-driven)

- Directional entries contribute `weight × normalized` to a signed sum ÷ total active weight → [-1,1].
- Non-directional gates (adx, atr, bbwp, hv, volume, rvol, choppiness) scale conviction, never enter the signed sum.
- Per-indicator `enabled` + `weight` (default 1.0) in `ScoringConfig` map, seeded from registry; existing 8-factor defaults preserved.

## Part V — Documentation & AI model

- `docs/indicators-guide.md` = one section per indicator (Description → AI Input Schema → Signal Threshold Matrix → rules), served at `/api/rules`.
- `agents.rs::get_guide_section` maps each indicator → section; `pipeline.rs` runs a phase-1 agent per indicator; `prompts.rs` holds rules.

## Part VI — Rendering model

- Price overlays (PriceChart): EMA Ribbon, Bollinger, VWAP, Supertrend, Keltner, Donchian (+ Ichimoku, Advanced). Each: ChartToggles button, color, price-format, history, live update, visibility.
- Price levels: Fibonacci, Support/Resistance (+ Pivots, Advanced).
- Markers: Candlestick Patterns + all signal markers.
- Panes (<Name>Chart in all 4 timeframe columns): RSI, Stochastic, ChandeMO, MACD, ADX, ATR, BBWP, Squeeze, RVOL, Volume, OBV, CMF, MFI, HV, Aroon, Choppiness, LinReg Slope, Z-Score.

## Part VII — Per-indicator specifications (28 active; Ichimoku/Pivots deferred)

Class: L=Leading, H=Hybrid, G=Lagging. Score: DIR=directional / GATE=non-directional. Phase: E=exists.

### TREND (6)
| Key | Class | Score | Render | Config | Normalization | Signals | Doc | Phase |
|---|---|---|---|---|---|---|---|---|
|`ema_stack`|G|DIR|Overlay 4 lines|ema 10/50/100/200|stack ±1 (retest ±0.8)|StackChange,Crossover|§6|E|
|`supertrend`|G|DIR|Overlay flip line|period10,mult3.0|+1/-1 dir; values{line,direction}|TrendFlip,Crossover|new|1A|
|`donchian`|G|DIR|Overlay u/m/l|period20|breakout ±1; values{upper,middle,lower}|Breakout,BandTouch|new|1A|
|`keltner`|G|DIR|Overlay u/m/l|ema20,atr10,mult2|price-vs-channel; values{upper,middle,lower}|Breakout,BandTouch|new|1A|
|`adx`|G|GATE|Pane|period14|strength gate|TrendFlip(DI),Threshold|§4|E|
|`vwap`|G|DIR|Overlay line|session|premium/discount ±|LevelTest|§7|E|

### MOMENTUM (5 + divergence)
| Key | Class | Score | Render | Config | Normalization | Signals | Doc | Phase |
|---|---|---|---|---|---|---|---|---|
|`rsi`|L|DIR·Div|Pane|period14|OB/OS piecewise|Divergence,Threshold,ZeroLineCross|§1|E|
|`rsi_divergence`|L|DIR|annotation|-|±1/±0.5|Divergence|§1|E|
|`stochastic`|L|DIR·Div|Pane|18/5/9|(k-50)/50|Crossover,Threshold,Divergence|§12|E→doc|
|`chandemo`|L|DIR·Div|Pane|period12|cmo/100|ZeroLineCross,Threshold,Divergence|§13|E→doc|
|`macd`|G|DIR·Div|Pane|12/26/9|crossover/zero/hist|Crossover,ZeroLineCross,Divergence|§2|E|
|`macd_divergence`|G|DIR|annotation|-|±1/±0.5|Divergence|§2|E|

### VOLUME (5)
| Key | Class | Score | Render | Config | Normalization | Signals | Doc | Phase |
|---|---|---|---|---|---|---|---|---|
|`volume`|H|GATE|Pane histogram|avg_period20|raw magnitude gate|VolumeClimax|§6|E→formalize|
|`rvol`|H|GATE|Pane|thresholds 1.5/3.0|activity gate|VolumeClimax|§6|E|
|`obv`|G|DIR·Div|Pane|smoothing|slope-vs-SMA tanh|Divergence,TrendFlip|new|1A|
|`cmf`|H|DIR·Div|Pane 0,±0.2|period20|native [-1,1]|ZeroLineCross,Divergence|new|1A|
|`mfi`|H|DIR·Div|Pane 80/20|period14|RSI-style|Threshold,Divergence|new|1A|

### VOLATILITY (5)
| Key | Class | Score | Render | Config | Normalization | Signals | Doc | Phase |
|---|---|---|---|---|---|---|---|---|
|`atr`|G|GATE|Pane|period14|raw-only (0) gate|-|§5|E|
|`bollinger`|H|DIR|Overlay u/m/l|20/2|%B/position ±|Breakout,BandTouch|§5|E(+%B)|
|`bbwp`|L|GATE|Pane|20/252|compression/expansion gate|CompressionRelease|§9|E|
|`squeeze`|H|DIR·Div|Pane|period20|compression0/release±1|CompressionRelease,Divergence|§3|E(+Div)|
|`hv`|G|GATE|Pane|period20|raw-only (0) gate|-|new|1A|

### MARKET STRUCTURE (3)
| Key | Class | Score | Render | Config | Normalization | Signals | Doc | Phase |
|---|---|---|---|---|---|---|---|---|
|`fibonacci`|L|DIR|Levels|swing|GP rebound/rejection|LevelTest|§8|E|
|`support_resistance`|L|DIR|Levels|pivots|proximity ±/flip|LevelTest,Breakout|§8|E|
|`patterns`|L|DIR|Marker|slope tol|bullish/bearish ± gated RVOL|PatternForming|§10|E|

### MARKET REGIME (4 — Phase 1B)
| Key | Class | Score | Render | Config | Normalization | Signals | Doc | Phase |
|---|---|---|---|---|---|---|---|---|
|`aroon`|H|DIR|Pane Up/Down+osc|period25|(Up-Down)/100|Crossover,Threshold,TrendFlip|new|1B|
|`choppiness`|H|GATE|Pane 38.2/61.8|period14|regime gate (high=choppy)|Threshold,CompressionRelease|new|1B|
|`linreg_slope`|G|DIR|Pane osc0|period20|tanh(norm slope)|ZeroLineCross,Threshold|new|1B|
|`zscore`|L|DIR|Pane ±2/±3|period20|mean-rev clamp(-z/3)|Threshold,ZeroLineCross|new|1B|

### ADVANCED (2 — deferred, NOT in this build)
| Key | Class | Score | Render | Notes |
|---|---|---|---|---|
|`ichimoku`|G|DIR|Overlay + cloud|client-side ±displacement shift|
|`pivots`|L|DIR|Levels|prior-day OHLC tracker|

### Divergence scored keys (8, Phase 2)
`rsi_divergence`, `macd_divergence` (exist) + new: `stochastic_divergence`,
`chandemo_divergence`, `mfi_divergence`, `cmf_divergence`, `obv_divergence`,
`squeeze_divergence`. Each: class/group = parent's, DIR, own weight, produced by
generalized DivergenceDetector; also emits a `Divergence` signal (with points) on the parent.

## Part VIII — Phase-by-phase audit checklist

### Phase 0 — Registry + Signals foundation
- [ ] `registry.rs`: `IndicatorMeta` (group + class + render + directional + divergence + signal_types + weight/enabled + value_format) + entries for all active indicators.
- [ ] `signals: Vec<IndicatorSignal>` + `SignalKind` on `NormalizedIndicatorValue` (serde default empty).
- [ ] Registry serialized in `/api/config`.
- [ ] Frontend: IndicatorMeta/signal TS types; settings store holds manifest; TelemetryTable + ChartToggles derive from manifest (group ↔ class switch).
- [ ] `volume` formalized as registry/normalized entry.
- [ ] Regression: existing indicators still render/persist/broadcast; all suites green.

### Phase 1A — Core 7
- [ ] 7 calculators + tests; mod.rs; IndicatorInputs + normalize_*; registry entries; models.rs accessors.
- [ ] config params + config.toml/default; migration cols + snapshots.rs binds.
- [ ] NormalizeParams/build_indicator_map; warm.rs; analyzer/mod.rs (cold+warm+broadcast); server/types.rs.
- [ ] 4 pane charts + 3 PriceChart overlays (+toggles/colors/price-format/history/live/visibility).
- [ ] historyAdapter/telemetry/state/api/TimeframeSettings/WorkspaceSettings.
- [ ] docs sections (7) + backfill Stochastic/ChandeMO.
- [ ] Regression + suites green.

### Phase 1B — Regime 4
- [ ] 4 calculators + tests; normalize (Choppiness gate); registry entries (group=Regime).
- [ ] config + migration + engine/DB/frontend wiring; 4 panes.
- [ ] docs sections (4).
- [ ] Regression + suites green.

### Phase 2 — Divergence + Signals emission
- [ ] Generalized DivergenceDetector across 8 oscillators → 6 new *_divergence scored keys + registry entries.
- [ ] signals[] emitted for all indicators.
- [ ] FE badges in TelemetryTable + setMarkers on panes/price; divergence points recorded (lines deferred).
- [ ] docs divergence rules.
- [ ] Regression + suites green.

### Phase 3 — Configurable scoring + AI agents
- [ ] ScoringConfig → per-indicator {enabled,weight} map (registry defaults; existing preserved); gates as multipliers; profile_evaluation key match extended.
- [ ] scoring-weight settings UI (manifest-driven).
- [ ] LLM phase-1 agents + get_guide_section + prompts for all new indicators.
- [ ] Score parity on unchanged config; suites green.

### Deferred (tracked): divergence connecting-lines; Ichimoku; Pivot Points.

## Part IX — Regression-safety guarantees

- Existing indicators keep keys, normalization, DB columns, panes/overlays, toggles.
- `auxiliary_normalized_data` JSON blob authoritative; legacy rows deserialize (serde defaults).
- `/api/history` generic/time-aligned; `/ws` additive only.
- Existing scoring weights/behavior unchanged unless re-weighted.
- Leading/Hybrid/Lagging preserved; functional `group` added alongside.
- No source file > 500 lines; Scoped CSS Modules; Svelte 5 runes only.

---

# ADDENDUM — Meta-Intelligence Layer (post-30 upgrade)

Backend-authoritative meta-intelligence built atop the indicator suite, surfaced in a
dedicated **Terminal Monitor** tab (moved out of the charts tab).

| Feature | Backend | Frontend |
|---|---|---|
| Indicator confidence | `NormalizedIndicatorValue.confidence` (base `\|normalized\|` + signal boost) | `%` in telemetry matrix |
| Signal age/freshness | `IndicatorSignal.age_bars` (analyzer stateful tracker) | badge `·N`, feed "now/Nb" |
| Market context | `shared::market_context::MarketContext` on `MarketSnapshot.context` | 6-dimension panel + overall gauge |
| MTF confirmation | `GET /api/monitor` (per-indicator agreement + trend %) | MTF matrix |
| Regime-aware weights | `ScoringConfig.regime_weight_multipliers` × `calculate_registry_confluence` | Scoring Weights panel |
| Cross-indicator confluence | `calculate_registry_confluence` (directional Σ + gates) | per-TF confluence bars |

**Terminal Monitor tab** (`TerminalMonitor.svelte`, view `'monitor'` in `MODE_TABS.user`):
Market Context strip · MTF confirmation matrix · per-timeframe confluence bars · live
signals feed (freshness-sorted) · detailed telemetry matrix (moved from `LiveTerminal`,
now enriched with confidence + signal-age).

**New endpoints:** `GET /api/monitor`, `POST /api/config/scoring-weights`.

**Deferred:** feeding `MarketContext`/MTF summary into the phase-2 orchestrator prompt (the
enriched indicator map — confidence + signals — already reaches the AI); divergence
connecting-lines; dedicated LLM phase-1 agents for the new indicators.
