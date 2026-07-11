# Market Monitor — Indicator System Master Specification

This is the authoritative reference specification for the Market Monitor's indicator
system. It describes the full pipeline from data ingestion to dashboard rendering,
the registry as single source of truth, the signal model, the scoring system, and
the per-indicator specifications.

For the formal ontology (entities, metrics, signals, states, decisions, and the 12
classification axes), see [ontology.md](ontology.md).
For the complete 58-indicator reference table, see [metrics-matrix.md](metrics-matrix.md).
For the three-matrix architecture, see [monitor-matrices.md](monitor-matrices.md).

---

## Part I — Architecture Layers

```
L0 Ingestion      Hyperliquid + Bitget WS → CandleGenerator (micro60 / fast180 / slow300 / macro900)
L1 Calculators    crates/shared/src/indicators/<name>.rs  (pure math, Decimal, stateful update())
L2 Normalize+Sig  crates/shared/src/indicators/normalized/*  → NormalizedIndicatorValue { raw, normalized[-1,1], state_label, values{}, signals[] }
L2.5 Alignment    crates/shared/src/alignment.rs  → AlignmentMatrix { MTF alignment per symbol }
L3 Registry       crates/shared/src/indicators/registry.rs   (INDICATORS: &[IndicatorMeta] — SINGLE SOURCE OF TRUTH)
L4 Assembly       crates/engine/src/analyzer/{normalize,warm,mod}.rs → MarketSnapshot.indicators: HashMap<String, NormalizedIndicatorValue>
L4.25 Risk        crates/shared/src/risk.rs  → RiskMatrix { market risk assessment per symbol }
L4.5 Analysis     crates/shared/src/analysis.rs  → AnalysisMatrix { market assessment per symbol }
L5 Persist+Transport  SQLite telemetry.db · WS /ws · REST /api/history · /api/config (registry manifest)
L5.5 State        crates/shared/src/state_matrix.rs  → StateMatrix { system-wide aggregation }
L6 FE State       crates/frontend/src/state.svelte.ts + stores/settings + lib/api.svelte.ts
L7 Render         GeneralDashboard · Metrics Panel · Alignment Panel · Risk Panel · Analysis Panel
L8 Scoring        Equal-weighted: all directional indicators contribute equally to signed mean
L9 Agent (future) crates/engine/src/llm/ + docs/indicators-guide.md
```

Golden rule: adding/altering an indicator = one registry entry + its L1 calculator +
L2 mapper + (its chart component). All L6/L7 lists are derived from the registry
manifest, not hand-maintained. See [ontology.md](ontology.md) for the complete
classification axis definitions that apply across all layers.

---

## Part II — The Registry (Single Source of Truth)

`IndicatorMeta` fields (Rust, serde-serialized, exposed in `/api/config.indicators`):

| Field | Meaning |
|---|---|
| `key` | map key, e.g. `supertrend` |
| `display_name` | UI label |
| `group` | functional: Trend / Momentum / Volume / Volatility / Structure / Regime / Institutional / DerivativesData |
| `class` | Leading / Hybrid / Lagging |
| `render` | Pane / PriceOverlay / PriceLevels / Marker |
| `directional` | true = signed [-1,1] scoring contributor; false = gate/multiplier |
| `supports_divergence` | eligible for divergence detection |
| `signal_types` | capability list (SignalKind[]) |
| `default_enabled` | scoring on/off |
| `config_params` | period keys |
| `value_format` / `value_source` | frontend raw-cell render hint |
| `color` | primary chart color |
| `guide_section` | docs section id for the interpretation guide |

All indicators contribute equally — there is no per-indicator weighting.
Every directional indicator receives equal contribution to the signed mean.

---

## Part III — Normalized Value + Signals Model

```
NormalizedIndicatorValue {
    raw_value: f64,
    normalized: f64 ∈ [-1.0, +1.0],
    state_label: String,
    values?: HashMap<String, f64>,
    signals: Vec<IndicatorSignal>,
    confidence: f64 ∈ [0.0, 1.0],
}

IndicatorSignal {
    kind: SignalKind,
    direction: Bullish | Bearish | Neutral,
    status: Potential | Confirmed | Active,
    label: String,
    strength: f64,
    age_bars: u32,
    points?: Vec<SignalPoint>,
}

SignalKind = Divergence | Crossover | Threshold | Breakout | BandTouch
           | ZeroLineCross | CompressionRelease | LevelTest | TrendFlip
           | VolumeClimax | StackChange | PatternForming
```

Every signal an indicator produces this snapshot is recorded in `signals[]`,
broadcast via WebSocket, persisted in the database JSON blob, and displayed as
TelemetryTable badges + chart markers. See [monitor-matrices.md](monitor-matrices.md)
for how signals flow through the Metrics → State → Decision pipeline.

---

## Part IV — Scoring System

- **Directional** entries: all contribute equally to a simple signed mean (Σnormalized / n) → [-1,1].
- **Non-directional gates** (9 total): `adx`, `atr`, `bbwp`, `hv`, `volume`, `rvol`, `choppiness`, `funding_rate`, `spread` — scale conviction via regime-dependent dampening, never enter the signed sum.
- **Divergence mirrors** (8 total): emit a separate scored entry with Confirmed ±1.0 / Potential ±0.5.
- No per-indicator weights, no regime-aware indicator multipliers, no configurable scoring weights.
- Regime gates modulate overall score: Range ×0.5, Compression ×0.3, Expansion ×1.2, Trending ×1.0.

---

## Part V — Documentation & Interpretation Guide

- `docs/indicators-guide.md`: one section per indicator (Description → Schema → Signal Threshold Matrix → rules).
- `docs/indicators/`: 12 per-indicator deep-dive documents.
- `docs/ontology.md`: formal ontology with 12 classification axes.
- `docs/metrics-matrix.md`: complete 58-indicator × 12-signal-kind reference.
- `docs/monitor-matrices.md`: Metrics Matrix → State Matrix → Decision Matrix flow.

---

## Part VI — Rendering Model

- **Price Overlays** (PriceChart): EMA Ribbon, Bollinger, VWAP, Supertrend, Keltner, Donchian, Anchored VWAP, Ichimoku, PSAR, Hull MA, StdDev Channel. Each: toggle button, color, price-format, history, live update, visibility.
- **Price Levels**: Fibonacci, Support/Resistance, Pivot Points, Volume Profile.
- **Markers**: divergence annotations, Candlestick Patterns, Chart Patterns, SMC signals.
- **Panes** (<Name>Chart in 4 timeframe columns): RSI, Stochastic, ChandeMO, MACD, ADX, ATR, BBWP, Squeeze, RVOL, Volume, OBV, CMF, MFI, HV, Aroon, Choppiness, LinReg Slope, Z-Score, AO, CCI, Williams %R, Force Index, OI Delta, Open Interest, Funding Rate, Order Flow Imbalance, Depth Bias.

---

## Part VII — Per-Indicator Specifications (58 Total Entries)

### TREND (10)
| Key | Class | Score | Source | Render | Signals |
|---|---|---|---|---|---|
| `ema_stack` | Lagging | DIR | Price | PriceOverlay | StackChange, Crossover |
| `supertrend` | Lagging | DIR | Price | PriceOverlay | TrendFlip, Crossover |
| `donchian` | Lagging | DIR | Price | PriceOverlay | Breakout, BandTouch |
| `keltner` | Lagging | DIR | Price | PriceOverlay | Breakout, BandTouch |
| `adx` | Lagging | GATE | Price | Pane | TrendFlip, Threshold |
| `vwap` | Lagging | DIR | Price | PriceOverlay | LevelTest |
| `anchored_vwap` | Lagging | DIR | Price | PriceOverlay | Crossover, LevelTest |
| `ichimoku` | Hybrid | DIR | Price | PriceOverlay | Crossover, Breakout, TrendFlip, LevelTest |
| `hull_ma` | Lagging | DIR | Price | PriceOverlay | Crossover |
| `psar` | Lagging | DIR | Price | PriceOverlay | TrendFlip, Crossover |

### MOMENTUM (11)
| Key | Class | Score | Source | Render | Signals |
|---|---|---|---|---|---|
| `rsi` | Leading | DIR·Div | Price | Pane | Divergence, Threshold, ZeroLineCross |
| `rsi_divergence` | Leading | DIR | Price | Marker | Divergence |
| `stochastic` | Leading | DIR·Div | Price | Pane | Crossover, Threshold, Divergence |
| `chandemo` | Leading | DIR·Div | Price | Pane | ZeroLineCross, Threshold, Divergence |
| `williams_r` | Leading | DIR | Price | Pane | Threshold, ZeroLineCross |
| `awesome_oscillator` | Leading | DIR | Price | Pane | ZeroLineCross, Threshold |
| `cci` | Leading | DIR | Price | Pane | Threshold, ZeroLineCross |
| `macd` | Lagging | DIR·Div | Price | Pane | Crossover, ZeroLineCross, Divergence |
| `macd_divergence` | Lagging | DIR | Price | Marker | Divergence |
| `stochastic_divergence` | Leading | DIR | Price | Marker | Divergence |
| `chandemo_divergence` | Leading | DIR | Price | Marker | Divergence |

### VOLUME (10)
| Key | Class | Score | Source | Render | Signals |
|---|---|---|---|---|---|
| `volume` | Hybrid | GATE | Volume | Pane | VolumeClimax |
| `rvol` | Hybrid | GATE | Volume | Pane | VolumeClimax |
| `volume_profile` | Hybrid | DIR | Volume | PriceLevels | Breakout, LevelTest, TrendFlip |
| `obv` | Lagging | DIR·Div | Volume | Pane | Divergence, TrendFlip |
| `cmf` | Hybrid | DIR·Div | Volume | Pane | ZeroLineCross, Divergence |
| `mfi` | Hybrid | DIR·Div | Volume | Pane | Threshold, Divergence |
| `force_index` | Hybrid | DIR | Composite | Pane | ZeroLineCross, Threshold |
| `mfi_divergence` | Hybrid | DIR | Volume | Marker | Divergence |
| `cmf_divergence` | Hybrid | DIR | Volume | Marker | Divergence |
| `obv_divergence` | Lagging | DIR | Volume | Marker | Divergence |

### VOLATILITY (7)
| Key | Class | Score | Source | Render | Signals |
|---|---|---|---|---|---|
| `atr` | Lagging | GATE | Composite | Pane | Threshold, CompressionRelease |
| `bollinger` | Hybrid | DIR | Composite | PriceOverlay | Breakout, BandTouch |
| `bbwp` | Leading | GATE | Composite | Pane | CompressionRelease |
| `squeeze` | Hybrid | DIR·Div | Composite | Pane | CompressionRelease, Divergence |
| `hv` | Lagging | GATE | Composite | Pane | Threshold |
| `stddev_channel` | Hybrid | DIR | Composite | PriceOverlay | Breakout, BandTouch |
| `squeeze_divergence` | Hybrid | DIR | Composite | Marker | Divergence |

### MARKET STRUCTURE (5)
| Key | Class | Score | Source | Render | Signals |
|---|---|---|---|---|---|
| `fibonacci` | Leading | DIR | Price | PriceLevels | LevelTest |
| `support_resistance` | Leading | DIR | Price | PriceLevels | LevelTest, Breakout |
| `pivot_points` | Leading | DIR | Price | PriceLevels | LevelTest, Breakout, Crossover |
| `patterns` | Leading | DIR | Price | Marker | PatternForming |
| `candlestick` | Leading | DIR | Price | Marker | PatternForming |

### MARKET REGIME (4)
| Key | Class | Score | Source | Render | Signals |
|---|---|---|---|---|---|
| `aroon` | Hybrid | DIR | Price | Pane | Crossover, Threshold, TrendFlip |
| `choppiness` | Hybrid | GATE | Price | Pane | Threshold, CompressionRelease |
| `linreg_slope` | Lagging | DIR | Price | Pane | ZeroLineCross, Threshold |
| `zscore` | Leading | DIR | Price | Pane | Threshold, ZeroLineCross |

### INSTITUTIONAL / SMC (4)
| Key | Class | Score | Source | Render | Signals |
|---|---|---|---|---|---|
| `smc_structure` | Leading | DIR | Composite | Marker | Breakout, TrendFlip |
| `smc_liquidity` | Leading | DIR | Composite | Marker | Threshold, PatternForming |
| `smc_fvg` | Leading | DIR | Composite | Marker | LevelTest |
| `smc_order_blocks` | Leading | DIR | Composite | Marker | LevelTest, TrendFlip |

### DERIVATIVES DATA (7)
| Key | Class | Score | Source | Render | Signals |
|---|---|---|---|---|---|
| `open_interest` | Hybrid | DIR | Derivatives | Pane | Threshold |
| `oi_delta` | Leading | DIR | Derivatives | Pane | Threshold, ZeroLineCross |
| `funding_rate` | Hybrid | GATE | Derivatives | Pane | Threshold |
| `oi_price_divergence` | Leading | DIR | Derivatives | Marker | Divergence |
| `order_flow_imbalance` | Leading | DIR | OrderBook | Pane | Threshold |
| `spread` | Hybrid | GATE | OrderBook | Pane | Threshold |
| `depth_bias` | Leading | DIR | OrderBook | Pane | Threshold |

---

## Part VIII — Status

The registry is fully populated with 58 entries across 8 functional groups.
All 43 unique indicator calculators are implemented, tested, and normalized.
The 12 signal kinds are emitted and tracked per snapshot. The MarketContext
synthesis and confluence scoring are live.

### Active
- [x] Registry: 58 `IndicatorMeta` entries across all 8 groups
- [x] Signal model: `NormalizedIndicatorValue.signals` with lifecycle tracking
- [x] Normalization: unified [-1, +1] scale with context-aware labels
- [x] Scoring: registry-driven directional Σ + 9 non-directional gates
- [x] Divergence: 8 oscillator divergence keys with S/R confirmation
- [x] MarketContext: 6-dimension per-instance synthesis
- [x] Warm pipeline: historical pre-warming for all stateful indicators
- [x] Live + completed snapshots: dual-path broadcast
- [x] DB persistence: snapshots persisted to SQLite
- [x] Documentation: ontology, metrics matrix, monitor matrices, interpretation guide

### In Progress (current build)
- [ ] Alignment Matrix: cross-timeframe MTF agreement per symbol
- [ ] Risk Matrix: market risk assessment per symbol
- [ ] Analysis Matrix: market assessment per symbol from Alignment + Risk
- [ ] State Matrix: system-wide instance + bias aggregation
- [ ] Frontend panels: Metrics Panel, Alignment Panel, Risk Panel, Analysis Panel, General Dashboard update

### Future
- [ ] Agent integration layer (L9)
- [ ] Divergence connecting lines on price chart

---

## Part IX — Regression Safety Guarantees

- Existing indicators keep keys, normalization, DB columns, panes/overlays, toggles.
- `auxiliary_normalized_data` JSON blob authoritative; legacy rows deserialize (serde defaults).
- `/api/history` generic/time-aligned; `/ws` additive only.
- Existing scoring behavior unchanged. All indicators contribute equally to the signed mean.
- Leading/Hybrid/Lagging preserved; functional `group` added alongside.
- No source file > 1000 lines; Scoped CSS Modules; Svelte 5 runes only.
