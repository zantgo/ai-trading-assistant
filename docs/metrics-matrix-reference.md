# Metrics Matrix — Complete Indicator Reference

Every indicator in the Market Monitor is declared in the registry
(`crates/shared/src/indicators/registry.rs`) as an `IndicatorMeta` entry.
This document provides the complete reference table with all classification
axes, signal capabilities, and scoring roles.

All indicators contribute equally to the Market Monitor's directional scoring.
Per-indicator weighting is intentionally absent — this is an observational tool,
not a trading system optimizer. Every directional indicator contributes equally
to the signed mean, and non-directional gates act as multipliers on overall
conviction without entering the signed sum.

---

## Summary Counts

| Metric | Count |
|---|---|
| Registry indicator entries | **58** |
| Unique indicator calculators | **43** |
| Divergence mirror scoring keys | **8** |
| Derivatives / Order Book keys | **7** |
| Functional groups | **8** |
| SignalKind variants | **12** |
| Total signal-kind × indicator declarations | **101** |
| Directional contributors | **49** |
| Non-directional gates | **9** |

---

## Master Indicator Table

**Column legend:**
- **D** = Directional, **G** = Gate
- **Div**: ✓ = supports divergence
- **Source**: P=Price, V=Volume, C=Composite, D=Derivatives, O=OrderBook
- **Scale**: B100 = Bounded 0–100, R = UnboundedRatio, P$ = PriceAbsolute, B = BooleanOnOff, % = PercentUnit

### Trend (10 entries)

| # | Key | Display Name | Class | Scoring | Source | Scale | Render | Div | Signal Types |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `ema_stack` | EMA Ribbon | Lagging | D | P | P$ | PriceOverlay | — | StackChange, Crossover |
| 2 | `supertrend` | Supertrend | Lagging | D | P | P$ | PriceOverlay | — | TrendFlip, Crossover |
| 3 | `donchian` | Donchian | Lagging | D | P | P$ | PriceOverlay | — | Breakout, BandTouch |
| 4 | `keltner` | Keltner | Lagging | D | P | P$ | PriceOverlay | — | Breakout, BandTouch |
| 5 | `adx` | ADX | Lagging | G | P | R | Pane | — | TrendFlip, Threshold |
| 6 | `vwap` | VWAP | Lagging | D | P | P$ | PriceOverlay | — | LevelTest |
| 7 | `anchored_vwap` | Anchored VWAP | Lagging | D | P | P$ | PriceOverlay | — | Crossover, LevelTest |
| 8 | `ichimoku` | Ichimoku Cloud | Hybrid | D | P | P$ | PriceOverlay | — | Crossover, Breakout, TrendFlip, LevelTest |
| 9 | `hull_ma` | Hull MA | Lagging | D | P | P$ | PriceOverlay | — | Crossover |
| 10 | `psar` | Parabolic SAR | Lagging | D | P | P$ | PriceOverlay | — | TrendFlip, Crossover |

### Momentum (11 entries)

| # | Key | Display Name | Class | Scoring | Source | Scale | Render | Div | Signal Types |
|---|---|---|---|---|---|---|---|---|---|
| 11 | `rsi` | RSI | Leading | D | P | B100 | Pane | ✓ | Divergence, Threshold, ZeroLineCross |
| 12 | `rsi_divergence` | RSI Divergence | Leading | D | P | B100 | Marker | ✓ | Divergence |
| 13 | `stochastic` | Stochastic | Leading | D | P | B100 | Pane | ✓ | Crossover, Threshold, Divergence |
| 14 | `chandemo` | Chande MO | Leading | D | P | B100 | Pane | ✓ | ZeroLineCross, Threshold, Divergence |
| 15 | `williams_r` | Williams %R | Leading | D | P | B100 | Pane | — | Threshold, ZeroLineCross |
| 16 | `awesome_oscillator` | AO | Leading | D | P | R | Pane | — | ZeroLineCross, Threshold |
| 17 | `cci` | CCI | Leading | D | P | R | Pane | — | Threshold, ZeroLineCross |
| 18 | `macd` | MACD | Lagging | D | P | R | Pane | ✓ | Crossover, ZeroLineCross, Divergence |
| 19 | `macd_divergence` | MACD Divergence | Lagging | D | P | R | Marker | ✓ | Divergence |
| 20 | `stochastic_divergence` | Stoch Divergence | Leading | D | P | B100 | Marker | ✓ | Divergence |
| 21 | `chandemo_divergence` | CMO Divergence | Leading | D | P | B100 | Marker | ✓ | Divergence |

### Volume (10 entries)

| # | Key | Display Name | Class | Scoring | Source | Scale | Render | Div | Signal Types |
|---|---|---|---|---|---|---|---|---|---|
| 22 | `volume` | Volume | Hybrid | G | V | R | Pane | — | VolumeClimax |
| 23 | `rvol` | RVOL | Hybrid | G | V | R | Pane | — | VolumeClimax |
| 24 | `volume_profile` | Volume Profile | Hybrid | D | V | P$ | PriceLevels | — | Breakout, LevelTest, TrendFlip |
| 25 | `obv` | OBV | Lagging | D | V | R | Pane | ✓ | Divergence, TrendFlip |
| 26 | `cmf` | Chaikin MF | Hybrid | D | V | R | Pane | ✓ | ZeroLineCross, Divergence |
| 27 | `mfi` | Money Flow Idx | Hybrid | D | V | B100 | Pane | ✓ | Threshold, Divergence |
| 28 | `force_index` | Force Idx | Hybrid | D | C | R | Pane | — | ZeroLineCross, Threshold |
| 29 | `mfi_divergence` | MFI Divergence | Hybrid | D | V | B100 | Marker | ✓ | Divergence |
| 30 | `cmf_divergence` | CMF Divergence | Hybrid | D | V | R | Marker | ✓ | Divergence |
| 31 | `obv_divergence` | OBV Divergence | Lagging | D | V | R | Marker | ✓ | Divergence |

### Volatility (7 entries)

| # | Key | Display Name | Class | Scoring | Source | Scale | Render | Div | Signal Types |
|---|---|---|---|---|---|---|---|---|---|
| 32 | `atr` | ATR | Lagging | G | C | P$ | Pane | — | Threshold, CompressionRelease |
| 33 | `bollinger` | Bollinger | Hybrid | D | C | P$ | PriceOverlay | — | Breakout, BandTouch |
| 34 | `bbwp` | BBWP | Leading | G | C | B100 | Pane | — | CompressionRelease |
| 35 | `squeeze` | TTM Squeeze | Hybrid | D | C | B | Pane | ✓ | CompressionRelease, Divergence |
| 36 | `hv` | Hist. Volatility | Lagging | G | C | % | Pane | — | Threshold |
| 37 | `stddev_channel` | StdDev Chnl | Hybrid | D | C | P$ | PriceOverlay | — | Breakout, BandTouch |
| 38 | `squeeze_divergence` | Squeeze Divergence | Hybrid | D | C | B | Marker | ✓ | Divergence |

### Market Structure (5 entries)

| # | Key | Display Name | Class | Scoring | Source | Scale | Render | Div | Signal Types |
|---|---|---|---|---|---|---|---|---|---|
| 39 | `fibonacci` | Fibonacci | Leading | D | P | P$ | PriceLevels | — | LevelTest |
| 40 | `support_resistance` | Support/Resistance | Leading | D | P | P$ | PriceLevels | — | LevelTest, Breakout |
| 41 | `pivot_points` | Pivot Points | Leading | D | P | P$ | PriceLevels | — | LevelTest, Breakout, Crossover |
| 42 | `patterns` | Patterns | Leading | D | P | % | Marker | — | PatternForming |
| 43 | `candlestick` | Candlestick | Leading | D | P | R | Marker | — | PatternForming |

### Market Regime (4 entries)

| # | Key | Display Name | Class | Scoring | Source | Scale | Render | Div | Signal Types |
|---|---|---|---|---|---|---|---|---|---|
| 44 | `aroon` | Aroon | Hybrid | D | P | B100 | Pane | — | Crossover, Threshold, TrendFlip |
| 45 | `choppiness` | Choppiness | Hybrid | G | P | B100 | Pane | — | Threshold, CompressionRelease |
| 46 | `linreg_slope` | LinReg Slope | Lagging | D | P | R | Pane | — | ZeroLineCross, Threshold |
| 47 | `zscore` | Z-Score | Leading | D | P | R | Pane | — | Threshold, ZeroLineCross |

### Institutional / SMC (4 entries)

| # | Key | Display Name | Class | Scoring | Source | Scale | Render | Div | Signal Types |
|---|---|---|---|---|---|---|---|---|---|
| 48 | `smc_structure` | SMC Structure | Leading | D | C | R | Marker | — | Breakout, TrendFlip |
| 49 | `smc_liquidity` | SMC Liquidity | Leading | D | C | R | Marker | — | Threshold, PatternForming |
| 50 | `smc_fvg` | SMC Fair Value Gap | Leading | D | C | R | Marker | — | LevelTest |
| 51 | `smc_order_blocks` | SMC Order Blocks | Leading | D | C | R | Marker | — | LevelTest, TrendFlip |

### Derivatives Data (7 entries)

| # | Key | Display Name | Class | Scoring | Source | Scale | Render | Div | Signal Types |
|---|---|---|---|---|---|---|---|---|---|
| 52 | `open_interest` | Open Interest | Hybrid | D | D | R | Pane | — | Threshold |
| 53 | `oi_delta` | OI Delta | Leading | D | D | R | Pane | — | Threshold, ZeroLineCross |
| 54 | `funding_rate` | Funding Rate | Hybrid | G | D | % | Pane | — | Threshold |
| 55 | `oi_price_divergence` | OI-Price Divergence | Leading | D | D | R | Marker | — | Divergence |
| 56 | `order_flow_imbalance` | Order Flow Imbalance | Leading | D | O | R | Pane | — | Threshold |
| 57 | `spread` | Spread | Hybrid | G | O | % | Pane | — | Threshold |
| 58 | `depth_bias` | Depth Bias | Leading | D | O | R | Pane | — | Threshold |

---

## Signal Distribution

| SignalKind | Declared By | Count |
|---|---|---|
| **Threshold** | rsi, stochastic, chandemo, williams_r, ao, cci, mfi, force_index, atr, hv, aroon, choppiness, linreg_slope, zscore, smc_liquidity, open_interest, oi_delta, funding_rate, order_flow_imbalance, spread, depth_bias, adx | **22** |
| **Divergence** | rsi, rsi_div, stochastic, chandemo, macd, macd_div, stoch_div, chandemo_div, obv, cmf, mfi, mfi_div, cmf_div, obv_div, squeeze, squeeze_div, oi_price_div | **17** |
| **ZeroLineCross** | rsi, chandemo, williams_r, ao, cci, macd, cmf, force_index, linreg_slope, zscore, oi_delta | **11** |
| **Crossover** | ema_stack, supertrend, anchored_vwap, ichimoku, hull_ma, psar, stochastic, macd, pivot_points, aroon | **10** |
| **Breakout** | donchian, keltner, ichimoku, bollinger, stddev_channel, volume_profile, support_resistance, pivot_points, smc_structure | **9** |
| **LevelTest** | vwap, anchored_vwap, ichimoku, fibonacci, support_resistance, pivot_points, volume_profile, smc_fvg, smc_order_blocks | **9** |
| **TrendFlip** | supertrend, adx, ichimoku, psar, obv, volume_profile, aroon, smc_structure, smc_order_blocks | **9** |
| **BandTouch** | donchian, keltner, bollinger, stddev_channel | **4** |
| **CompressionRelease** | atr, bbwp, squeeze, choppiness | **4** |
| **PatternForming** | patterns, candlestick, smc_liquidity | **3** |
| **VolumeClimax** | volume, rvol | **2** |
| **StackChange** | ema_stack | **1** |
| | | **Total: 101** |

---

## Per-Group Summary

| Functional Group | Registry Entries | Directional | Gates | Signal Declarations |
|---|---|---|---|---|
| Trend | 10 | 9 | 1 | 20 |
| Momentum | 11 | 11 | 0 | 22 |
| Volume | 10 | 8 | 2 | 16 |
| Volatility | 7 | 4 | 3 | 11 |
| Structure | 5 | 5 | 0 | 8 |
| Regime | 4 | 3 | 1 | 9 |
| Institutional | 4 | 4 | 0 | 7 |
| DerivativesData | 7 | 5 | 2 | 8 |
| **Total** | **58** | **49** | **9** | **101** |

---

## Gate Indicators

Nine indicators are non-directional gates. They never enter the signed scoring
sum but act as multipliers on overall conviction:

| Key | Display Name | Group | Purpose |
|---|---|---|---|
| `adx` | ADX | Trend | Trend strength gauge |
| `atr` | ATR | Volatility | Volatility magnitude baseline |
| `bbwp` | BBWP | Volatility | Compression/expansion cycle detection |
| `hv` | Hist. Volatility | Volatility | Historical volatility baseline |
| `volume` | Volume | Volume | Raw activity level |
| `rvol` | RVOL | Volume | Relative volume vs average |
| `choppiness` | Choppiness | Regime | Market noise level |
| `funding_rate` | Funding Rate | DerivativesData | Perpetual swap sentiment |
| `spread` | Spread | DerivativesData | Liquidity condition |

---

## Indicator Evaluation Axes

Every indicator in the Metrics Matrix exposes multiple dimensions beyond its
raw numerical value. These axes provide rich contextual metadata for downstream
consumers (Alignment, Risk, Analysis matrices).

| Axis | Description | Possible Values | Implementation |
|---|---|---|---|
| **Value** | Raw numerical output in native units | RSI=63.4, ATR=128.4 | `raw_value: f64` |
| **State** | Human-readable classification | Bullish / Bearish / Neutral | `state_label: String` |
| **Normalized** | Unified [-1.0, +1.0] scale | Continuous | `normalized: f64` |
| **Confidence** | Estimated reliability | 0.0–1.0 | `confidence: f64` |
| **Direction** | Current movement trajectory | Rising / Falling / Flat | *deferred* |
| **Strength** | Intensity of the reading | Weak / Moderate / Strong / Extreme | *deferred* |
| **Market Regime** | Environment where indicator operates | Trending / Ranging / Expansion / Compression / Transition | *deferred* |
| **Freshness** | How recently condition developed | New / Recent / Aging / Expired | *deferred* |
| **Quality** | Overall quality assessment | Poor / Normal / Healthy / Excellent | *deferred* |

---

## Signal Evaluation Axes

Signals are discrete events derived from indicators. Each signal carries
metadata for downstream interpretation.

| Axis | Description | Possible Values | Implementation |
|---|---|---|---|
| **Signal Type** | What event occurred | 12 SignalKind variants | `kind: SignalKind` |
| **Direction** | Bullish / Bearish / Neutral | 3 variants | `direction: SignalDirection` |
| **Status** | Lifecycle stage | Potential → Confirmed / Active | `status: SignalStatus` |
| **Strength** | How significant (0-1) | Continuous | `strength: f64` |
| **Freshness** | Bars since first appearance | 0=just triggered, N=old | `age_bars: u32` |
| **Confirmation** | Validation state | Pending / Confirmed / Rejected | `status: SignalStatus` |
| **Multi-Timeframe** | Same signal across TFs | Count of TFs with same signal | `signal_cross_tf_count` in Alignment |
| **Confidence** | Signal-level reliability | *deferred* | *deferred* |
| **Market Regime** | Regime-appropriateness | *deferred* | *deferred* |
| **Risk Level** | Low / Medium / High | *deferred* | *deferred* |
| **Priority** | Critical / High / Medium / Low | *deferred* | *deferred* |

---

## Derived Metrics

Derived Metrics are higher-level analytical summaries computed from indicators
and signals within the Metrics Matrix. They represent interpreted market
context, not raw indicator values.

| Derived Metric | Description | Implementation |
|---|---|---|
| **Market Regime** | TRENDING / RANGE / EXPANSION / COMPRESSION | `MarketContext.regime` |
| **Trend Score** | Trend-group equal-weighted mean | `MarketContext.trend.score` |
| **Momentum Score** | Momentum-group equal-weighted mean | `MarketContext.momentum.score` |
| **Volume Score** | Volume magnitude | `MarketContext.volume.score` |
| **Volatility Score** | BBWP-derived | `MarketContext.volatility.score` |
| **Liquidity State** | VWAP + volume confidence proxy | `MarketContext.liquidity` |
| **Overall Confidence** | Local equal-weighted bias | `MarketContext.overall_score` |
| **Trend Quality** | ADX + EMA structure assessment | *deferred* |
| **Breakout Probability** | Compression + squeeze release odds | *deferred* |
| **Continuation Probability** | Trend strength persistence | *deferred* |
| **Reversal Probability** | Exhaustion + divergence odds | *deferred* |
| **Mean Reversion Probability** | BBWP + Z-Score extremes | *deferred* |
| **Liquidity State** | Thin / Low / Normal / High / Institutional | *deferred* |
| **Market Phase** | Accumulation / Markup / Distribution / Markdown | *deferred* |
| **Strategy Recommendation** | Best-fit strategy for current conditions | Analysis Matrix |
| **Trade Readiness** | Not Ready / Building / Ready / Confirmed / Late | Analysis Matrix |

---

## Cross-References

- **Formal ontology and axes**: [ontology.md](ontology.md)
- **Six-matrix architecture**: [monitor-matrices-reference.md](monitor-matrices-reference.md)
- **Master specification**: [indicator-system-master-spec.md](indicator-system-master-spec.md)
- **Registry source**: `crates/shared/src/indicators/registry.rs` (single source of truth)
- **Signal model**: `crates/shared/src/indicators/normalized/mod.rs` (SignalKind, IndicatorSignal)
