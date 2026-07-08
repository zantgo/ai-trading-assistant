# ITIL — Institutional Technical Indicator Layer

> **Layer 1 of 10 in the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: COMPLETE** — All 51 indicators implemented and tested.

---

## Purpose

The Institutional Technical Indicator Layer (ITIL) answers the foundational analytical question:

> **What mathematical patterns exist in the price data?**

ITIL transforms raw OHLCV candle data into structured, normalized, machine-readable signals. It is the **feature engineering layer** — turning price action into 51 normalized indicator values and 115 discrete signal events that feed every layer above it. Without ITIL, there is no quantitative basis for regime classification, structure mapping, confluence scoring, or any downstream decision-making.

**ITIL is purely mathematical. It never makes trading decisions.**

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| OHLCV candles | L0 — Hyperliquid WebSocket → Candle Aggregator | `NormalizedCandle` per timeframe (5 TFs) |
| Registry manifest | `crates/shared/src/indicators/registry.rs` | 51 `IndicatorMeta` entries (compile-time constant) |
| Config parameters | `config.toml` `[indicators]` section | Per-indicator period/smoothing/threshold values |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| Normalized indicator map | `HashMap<String, NormalizedIndicatorValue>` | IRCL (regime indicators), ISML (structure indicators), ICSL (all 44 directional contributors) |
| Signal event stream | `Vec<IndicatorSignal>` per indicator per snapshot | IRCL, ICSL, IASL (Analyst Agent document sections) |
| Per-group aggregation | Dominant direction, signal count, mean confidence | IASL (Analyst Agent `market_summary` + group sections) |
| Indicator confidence scores | `confidence ∈ [0,1]` per indicator | ICSL (weighted scoring), IDCL (probability computation) |

---

## Sub-Components

---

### A. Indicator Computation Engine

The engine processes 51 indicators across 7 functional groups. Every indicator follows the same lifecycle, enforced by the registry manifest:

```
calculator (pure math, stateful update on each candle)
    → normalizer (maps raw value → NormalizedIndicatorValue)
        → signal derivation (applies threshold/crossover/pattern rules)
            → scoring (directional contribution or non-directional gate)
```

**Golden rule:** Adding an indicator requires exactly four artifacts — one registry entry (`registry.rs`), one calculator (`indicators/<name>.rs`), one normalizer mapper (`normalized/all.rs` or `signals.rs`), and one chart component (if rendered). No scattered hardcoded lists.

**Code location:** `crates/shared/src/indicators/` (51 calculators + normalizers), `crates/shared/src/indicators/registry.rs` (single source of truth), `crates/shared/src/indicators/normalized/` (normalization engine).

---

### B. 7 Functional Groups

Every indicator belongs to exactly one group. Groups determine the indicator's role in the decision pipeline — directional contributors feed the confluence score, while non-directional gates scale conviction.

**Group summary:**

| Group | Count | Directional | Non-Directional (Gate) | Role |
|-------|-------|-------------|----------------------|------|
| **Trend** | 10 | 9 | 1 (ADX) | Establish directional bias, identify support/resistance flow |
| **Momentum** | 11 | 11 | 0 | Measure trend acceleration/deceleration; identify overbought/oversold |
| **Volume** | 10 | 8 | 2 (Volume, RVOL) | Confirm or reject price moves via participation levels |
| **Volatility** | 7 | 4 | 3 (ATR, BBWP, HV) | Measure risk environment; detect compression/expansion cycles |
| **Structure** | 5 | 5 | 0 | Map key levels, detect patterns, classify market geometry |
| **Regime** | 4 | 3 | 1 (Choppiness) | Classify market type independently of direction |
| **Institutional** | 4 | 4 | 0 | Smart Money Concepts — order flow, liquidity, imbalance zones |
| **Total** | **51** | **44** | **7** | |

---

### C. Complete 51-Entry Inventory

Every entry is sourced from the authoritative registry (`crates/shared/src/indicators/registry.rs`). Class: L=Leading, H=Hybrid, G=Lagging. Render: P=Pane, PO=PriceOverlay, PL=PriceLevels, M=Marker.

#### TREND (10 indicators)

| # | Key | Display Name | Cls | Dir | Div | Signal Count | SignalKinds | Doc |
|---|-----|-------------|-----|-----|-----|-------------|-------------|-----|
| 1 | `ema_stack` | EMA Ribbon | G | Y | — | 5 | StackChange, Crossover×4 | [ema.md](../indicators/ema.md) |
| 2 | `supertrend` | Supertrend | G | Y | — | 4 | TrendFlip, Crossover×2, BandTouch | [supertrend.md](../indicators/supertrend.md) |
| 3 | `donchian` | Donchian | G | Y | — | 6 | Breakout×2, BandTouch×2, LevelTest×2 | [donchian.md](../indicators/donchian.md) |
| 4 | `keltner` | Keltner | G | Y | — | 6 | Breakout×2, BandTouch×2, LevelTest×2 | [keltner.md](../indicators/keltner.md) |
| 5 | `adx` | ADX | G | N | — | 2 | TrendFlip, Threshold | [adx.md](../indicators/adx.md) |
| 6 | `vwap` | VWAP | G | Y | — | 1 | LevelTest | [vwap.md](../indicators/vwap.md) |
| 7 | `anchored_vwap` | Anchored VWAP | G | Y | — | 4 | LevelTest×2, Crossover×2 | [anchored_vwap.md](../indicators/anchored_vwap.md) |
| 8 | `ichimoku` | Ichimoku Cloud | H | Y | — | 9 | Crossover×2, Breakout×2, LevelTest×3, TrendFlip×2 | [ichimoku.md](../indicators/ichimoku.md) |
| 9 | `psar` | Parabolic SAR | G | Y | — | 4 | TrendFlip, Crossover×3 | [psar.md](../indicators/psar.md) |
| 10 | `hull_ma` | Hull MA | G | Y | — | 2 | Crossover×2 | [hull_ma.md](../indicators/hull_ma.md) |

**Subtotal: 10 indicators, 42 signal emissions. 1 non-directional gate (ADX).**

#### MOMENTUM (11 indicators)

| # | Key | Display Name | Cls | Dir | Div | Signal Count | SignalKinds | Doc |
|---|-----|-------------|-----|-----|-----|-------------|-------------|-----|
| 11 | `rsi` | RSI | L | Y | Y | 7 | ZeroLineCross, Divergence, Threshold×5 | [rsi.md](../indicators/rsi.md) |
| 12 | `rsi_divergence` | RSI Divergence | L | Y | Y | 1 | Divergence | (covered in rsi.md) |
| 13 | `stochastic` | Stochastic | L | Y | Y | 9 | Crossover×2, ZeroLineCross×2, Divergence, Threshold×4 | [stochastic.md](../indicators/stochastic.md) |
| 14 | `stochastic_divergence` | Stoch Divergence | L | Y | Y | 1 | Divergence | (covered in stochastic.md) |
| 15 | `chandemo` | Chande MO | L | Y | Y | 6 | ZeroLineCross, Divergence, Threshold×4 | [chandemo.md](../indicators/chandemo.md) |
| 16 | `chandemo_divergence` | CMO Divergence | L | Y | Y | 1 | Divergence | (covered in chandemo.md) |
| 17 | `williams_r` | Williams %R | L | Y | — | 2 | ZeroLineCross, Threshold | [williams_r.md](../indicators/williams_r.md) |
| 18 | `awesome_oscillator` | AO | L | Y | — | 3 | Threshold×2, ZeroLineCross | [awesome_oscillator.md](../indicators/awesome_oscillator.md) |
| 19 | `cci` | CCI | L | Y | — | 5 | Threshold×4, ZeroLineCross | [cci.md](../indicators/cci.md) |
| 20 | `macd` | MACD | G | Y | Y | 6 | Crossover×2, TrendFlip×2, Divergence, Threshold | [macd.md](../indicators/macd.md) |
| 21 | `macd_divergence` | MACD Divergence | G | Y | Y | 1 | Divergence | (covered in macd.md) |

**Subtotal: 11 indicators (7 calculators + 4 divergence keys), 42 signal emissions. All directional. 8 divergence-bearing.**

#### VOLUME (10 indicators)

| # | Key | Display Name | Cls | Dir | Div | Signal Count | SignalKinds | Doc |
|---|-----|-------------|-----|-----|-----|-------------|-------------|-----|
| 22 | `volume` | Volume | H | N | — | 0 (gate only) | VolumeClimax | [volume.md](../indicators/volume.md) |
| 23 | `rvol` | RVOL | H | N | — | 1 | VolumeClimax | [rvol.md](../indicators/rvol.md) |
| 24 | `volume_profile` | Volume Profile | H | Y | — | 4 | Breakout×2, LevelTest×2 | [volume_profile.md](../indicators/volume_profile.md) |
| 25 | `obv` | OBV | G | Y | Y | 5 | TrendFlip, Divergence, Threshold×3 | [obv.md](../indicators/obv.md) |
| 26 | `obv_divergence` | OBV Divergence | G | Y | Y | 1 | Divergence | (covered in obv.md) |
| 27 | `cmf` | Chaikin MF | H | Y | Y | 5 | ZeroLineCross, Divergence, Threshold×3 | [cmf.md](../indicators/cmf.md) |
| 28 | `cmf_divergence` | CMF Divergence | H | Y | Y | 1 | Divergence | (covered in cmf.md) |
| 29 | `mfi` | Money Flow Idx | H | Y | Y | 4 | ZeroLineCross, Divergence, Threshold×2 | [mfi.md](../indicators/mfi.md) |
| 30 | `mfi_divergence` | MFI Divergence | H | Y | Y | 1 | Divergence | (covered in mfi.md) |
| 31 | `force_index` | Force Idx | H | Y | — | 2 | Threshold, ZeroLineCross | [force_index.md](../indicators/force_index.md) |

**Subtotal: 10 indicators (7 calculators + 3 divergence keys), 24 signal emissions. 2 non-directional gates (Volume, RVOL).**

#### VOLATILITY (7 indicators)

| # | Key | Display Name | Cls | Dir | Div | Signal Count | SignalKinds | Doc |
|---|-----|-------------|-----|-----|-----|-------------|-------------|-----|
| 32 | `atr` | ATR | G | N | — | 2 | Threshold, CompressionRelease | [atr.md](../indicators/atr.md) |
| 33 | `bollinger` | Bollinger | H | Y | — | 7 | Breakout×2, BandTouch×2, LevelTest×3 | [bollinger.md](../indicators/bollinger.md) |
| 34 | `bbwp` | BBWP | L | N | — | 1 | Threshold | [bbwp.md](../indicators/bbwp.md) |
| 35 | `squeeze` | TTM Squeeze | H | Y | Y | 5 | CompressionRelease×3, Divergence, Threshold | [squeeze_momentum.md](../indicators/squeeze_momentum.md) |
| 36 | `squeeze_divergence` | Squeeze Divergence | H | Y | Y | 1 | Divergence | (covered in squeeze_momentum.md) |
| 37 | `hv` | Hist. Volatility | G | N | — | 1 | Threshold | [hv.md](../indicators/hv.md) |
| 38 | `stddev_channel` | StdDev Chnl | H | Y | — | 5 | Breakout×2, BandTouch×2, LevelTest | [stddev_channel.md](../indicators/stddev_channel.md) |

**Subtotal: 7 indicators (6 calculators + 1 divergence key), 22 signal emissions. 3 non-directional gates (ATR, BBWP, HV).**

#### STRUCTURE (5 indicators)

| # | Key | Display Name | Cls | Dir | Div | Signal Count | SignalKinds | Doc |
|---|-----|-------------|-----|-----|-----|-------------|-------------|-----|
| 39 | `fibonacci` | Fibonacci | L | Y | — | 1 | LevelTest | [fibonacci.md](../indicators/fibonacci.md) |
| 40 | `support_resistance` | Support/Resistance | L | Y | — | 2 | LevelTest, TrendFlip | [support_resistance.md](../indicators/support_resistance.md) |
| 41 | `pivot_points` | Pivot Points | L | Y | — | 5 | Crossover×2, LevelTest×3 | [pivot_points.md](../indicators/pivot_points.md) |
| 42 | `patterns` | Patterns | L | Y | — | 3 | PatternForming×3 | [chart_patterns.md](../indicators/chart_patterns.md) |
| 43 | `candlestick` | Candlestick | L | Y | — | 1 | PatternForming | [candlestick.md](../indicators/candlestick.md) |

**Subtotal: 5 indicators, 12 signal emissions. All directional.**

#### REGIME (4 indicators)

| # | Key | Display Name | Cls | Dir | Div | Signal Count | SignalKinds | Doc |
|---|-----|-------------|-----|-----|-----|-------------|-------------|-----|
| 44 | `aroon` | Aroon | H | Y | — | 6 | Crossover×2, TrendFlip×2, Threshold×2 | [aroon.md](../indicators/aroon.md) |
| 45 | `choppiness` | Choppiness | H | N | — | 2 | Threshold×2 | [choppiness.md](../indicators/choppiness.md) |
| 46 | `linreg_slope` | LinReg Slope | G | Y | — | 3 | ZeroLineCross, Threshold×2 | [linreg_slope.md](../indicators/linreg_slope.md) |
| 47 | `zscore` | Z-Score | L | Y | — | 2 | Threshold, ZeroLineCross | [zscore.md](../indicators/zscore.md) |

**Subtotal: 4 indicators, 13 signal emissions. 1 non-directional gate (Choppiness).**

#### INSTITUTIONAL (4 indicators)

| # | Key | Display Name | Cls | Dir | Div | Signal Count | SignalKinds | Doc |
|---|-----|-------------|-----|-----|-----|-------------|-------------|-----|
| 48 | `smc_structure` | SMC Structure | L | Y | — | 2 | Breakout, TrendFlip | [smc_structure.md](../indicators/smc_structure.md) |
| 49 | `smc_liquidity` | SMC Liquidity | L | Y | — | 1 | PatternForming | [smc_liquidity.md](../indicators/smc_liquidity.md) |
| 50 | `smc_fvg` | SMC Fair Value Gap | L | Y | — | 1 | LevelTest | [smc_fvg.md](../indicators/smc_fvg.md) |
| 51 | `smc_order_blocks` | SMC Order Blocks | L | Y | — | 1 | LevelTest | [smc_order_blocks.md](../indicators/smc_order_blocks.md) |

**Subtotal: 4 indicators, 5 signal emissions. All directional.**

---

### GRAND TOTAL

| Metric | Count |
|--------|-------|
| Registry entries | **51** |
| Directional indicators (scoring contributors) | **44** |
| Non-directional gates | **7** (ADX, Volume, RVOL, ATR, BBWP, HV, Choppiness) |
| Total signal emission sites | **115** |
| Unique SignalKind types | **12** |
| Indicator groups | **7** |
| Divergence-bearing indicators | **8** (RSI, MACD, Stochastic, ChandeMO, OBV, CMF, MFI, Squeeze) |
| Divergence scored keys | **8** (`*_divergence` entries, injected by generalized DivergenceDetector) |
| Emission sources | `all.rs` (61), `signals.rs` (53), `context.rs` (1) |

---

### D. Normalized Value Schema

Every indicator produces a `NormalizedIndicatorValue` per candle per timeframe:

```rust
NormalizedIndicatorValue {
    raw_value: f64,           // Original calculator output (price, %, ratio, etc.)
    normalized: f64,          // [-1.0, +1.0] signed continuous scale
    state_label: String,      // Semantic label (e.g., "RSI_OVERBOUGHT", "SQUEEZE_ON")
    values: HashMap<String, f64>,  // Sub-values (e.g., ema_stack: {fast, medium, slow, long})
    signals: Vec<IndicatorSignal>,   // Discrete signal events this snapshot
    confidence: f64,          // [0.0, 1.0] — base = |normalized|, boosted by confirmed signals
}
```

**Normalization conventions per group:**

| Group | Normalization Method | Range Interpretation |
|-------|---------------------|---------------------|
| Trend | Stack position, distance from EMA, directional flag | +1 = strongly bullish alignment, −1 = strongly bearish |
| Momentum | Piecewise RSI-style OB/OS mapping; CMO/100; slope tanh | +1 = extreme bullish momentum, −1 = extreme bearish |
| Volume | Raw magnitude gate (0) or slope tanh for directional | +1 = strong accumulation, −1 = strong distribution |
| Volatility | Raw gate (0) for non-directional; BB/Squeeze position for directional | +1 = bullish expansion, −1 = bearish expansion |
| Structure | Proximity to level, flip direction, pattern quality | +1 = at bullish level, −1 = at bearish level |
| Regime | Oscillator position on [-100,100] → /100 or chop gate | +1 = strong positive regime, −1 = strong negative |
| Institutional | Zone position, BOS/CHoCH direction | +1 = bullish institution flow, −1 = bearish |

**Magnitude conventions (used by IASL Analyst Agent):**
| \|normalized\| | Label |
|---------------|-------|
| > 0.70 | Strong |
| 0.30 – 0.70 | Moderate |
| < 0.30 | Weak / Neutral |

---

### E. Signal Emission System — 12 SignalKinds

Every indicator emits zero or more `IndicatorSignal` events per candle. The 12 signal kinds cover all possible discrete events:

**1. Divergence** — Oscillator disagrees with price direction.
- Bullish: price makes lower low, oscillator makes higher low
- Bearish: price makes higher high, oscillator makes lower high
- Confirmed only when structural S/R break occurs (0.2% tolerance)
- 8 divergence-bearing oscillators: RSI, MACD, Stochastic, ChandeMO, OBV, CMF, MFI, Squeeze

**2. Crossover** — Two lines intersect.
- EMA crossovers (fast/medium/slow/long), MACD line/signal, DI+/DI−, %K/%D, Aroon Up/Down, PSAR/price, Hull MA/price, pivot/price
- 27 crossover emission sites across the registry

**3. Threshold** — Value crosses a predefined boundary.
- RSI: 70/30 (80/20 in strong trend), 50 midline
- CCI: +100/−100, +200/−200
- ADX: 20 (emerging), 25 (strong), 40 (exhaustion)
- Stochastic: 80/20
- MFI: 80/20
- ChandeMO: ±50
- 38 threshold emission sites

**4. Breakout** — Price closes beyond a channel/band/envelope.
- Donchian, Keltner, Bollinger, Ichimoku, Volume Profile, SMC structure, StdDev Channel
- 16 breakout emission sites

**5. BandTouch** — Price touches a band without closing beyond it.
- Bollinger, Donchian, Keltner, StdDev Channel, Supertrend
- 12 band-touch emission sites

**6. ZeroLineCross** — Oscillator crosses the zero line.
- MACD histogram, RSI 50, CMO 0, CCI 0, Williams %R −50, AO 0, Force Index 0, LinReg Slope 0, Z-Score mean, CMF 0, MFI 50, Stochastic K/D 50
- 19 zero-line-cross emission sites

**7. CompressionRelease** — Volatility compression releases (energy coiling → directional expansion).
- Squeeze ON→OFF transitions (3 release types: bullish, bearish, neutral)
- BBWP crossing thresholds
- ATR regime shifts
- Choppiness transitions
- 9 compression-release emission sites

**8. LevelTest** — Price tests a structural level.
- Support/Resistance, Fibonacci (GP, retracements), VWAP, Anchored VWAP, Volume Profile (POC/VAH/VAL), Pivot Points (R1-R3/S1-S3), SMC FVG, SMC Order Blocks, Keltner/Donchian/Bollinger midlines, Ichimoku levels
- 24 level-test emission sites

**9. TrendFlip** — Direction changes on trend-following indicators.
- Supertrend line flip, PSAR dot flip, ADX DI crossover, SMC CHoCH, MACD crossover flip, OBV slope flip, Aroon crossover flip
- 15 trend-flip emission sites

**10. VolumeClimax** — Extreme volume event.
- RVOL ≥ 3.0 (exhaustion climax)
- Volume raw magnitude extreme
- 2 volume-climax emission sites

**11. StackChange** — EMA ribbon ordering changes.
- Bullish stack → bearish stack or vice versa
- EMA 10/50, 50/100, 100/200 pair reorderings
- 1 StackChange emission (signals derived from EMA pair crossovers on the `ema_stack` key)

**12. PatternForming** — Candlestick or chart pattern detected.
- Candlestick: 29-species recognition (hammer, engulfing, doji, etc.)
- Chart Patterns: Triangle, Wedge, Channel detections
- SMC liquidity sweeps
- 5 pattern-forming emission sites

**Total: 115 discrete signal emission sites across 12 SignalKinds.**

---

### F. Signal Lifecycle & Confidence

Every signal follows a lifecycle state machine:

```
Potential ──► Confirmed ──► Active ──► Expired
   │              │            │
   └── (structural  └── (each    └── (after N bars
        break OR         bar)         without
        RVOL gate)                    renewal)
```

**Confirmation rules:**
- **Divergence**: Potential when oscillator disagrees with price. Confirmed when S/R boundary breaks with 0.2% tolerance.
- **Breakout**: Potential when price touches beyond band. Confirmed when candle closes beyond band AND RVOL ≥ 1.5.
- **Crossover**: Immediate confirmation (no structural gate) but must pass ADX slope > 0 for DI crossovers.
- **Squeeze Release**: Confirmed when squeeze_duration ≥ 5 bars. Reject "premature breakouts."

**Signal strength:** `base_strength × confidence × freshness_decay`. Fresher signals (age_bars = 0) carry full weight. Signals age linearly over their active window.

**Confidence scoring:**
```
indicator_confidence = base_confidence + signal_boost
base_confidence = |normalized|
signal_boost = +0.2 if confirmed signal active, +0.1 if potential signal active
```

Per-indicator confidence ∈ [0, 1] and is displayed as a percentage in the telemetry matrix.

---

### G. Group-Level Aggregation

Each candle produces 7 group summaries for downstream consumption:

| Group | Aggregation Fields |
|-------|-------------------|
| Trend | Dominant direction (bullish/bearish/neutral), confirmed crossover count, EMA stack state, mean confidence |
| Momentum | Dominant direction, overbought/oversold count, active divergence count, mean RSI-family reading |
| Volume | Accumulation/distribution bias, RVOL regime, OBV/CMF/MFI agreement %, mean confidence |
| Volatility | Compression/expansion state, BBWP percentile, ATR regime, Squeeze status |
| Structure | Active S/R levels count, nearest level distance, pattern presence, Fibonacci GP status |
| Regime | Regime label, 6-indicator agreement %, transition flags |
| Institutional | Active BOS/CHoCH, active OB count, unmitigated FVG count, sweep status |

These summaries are consumed by the IASL Analyst Agent to produce its 8-section institutional document.

**Emission source mapping:**

| Source File | Emissions | Description |
|-------------|-----------|-------------|
| `all.rs` | 61 | Regular indicators with standard signal patterns |
| `signals.rs` | 53 | Indicators requiring complex multi-condition signal logic |
| `context.rs` | 1 | Context-level signal aggregation |

---

## Integration

### Feeds Into
- **IRCL (Layer 2)** — Regime indicator values (ADX, BBWP, Choppiness, Aroon, Squeeze) + confidence
- **ISML (Layer 3)** — Structure indicators (S/R, Fibonacci, SMC, Volume Profile, Patterns) + level signals
- **ICSL (Layer 4)** — All 44 directional contributors + 7 non-directional gate values

### Receives From
- **L0 (Raw Market Data)** — OHLCV candles from the 5-timeframe pipeline (micro/fast/slow/macro + 1h/4h)

### Cross-References
- [Indicator System Master Spec](../indicator-system-master-spec.md) — Registry manifest design, scoring model, phase checklist
- [Indicators Guide (AI Rulebook)](../indicators-guide.md) — Signal threshold matrices for LLM consumption
- [Individual Indicator Docs](../indicators/) — 42 files covering all 51 entries (divergence keys covered in parent docs)
