# Indicator Documentation Index

> 50 indicators across 8 functional groups. Registry-verified count: 50 authoritative `IndicatorMeta` entries in `crates/shared/src/indicators/registry.rs`. Divergence is a `SignalKind` emitted on its parent indicator — divergences are **not** separate registry entries and produce **no** separate JSON keys. Eight parent indicators are annotated `supports_divergence: true` (see the `Div` column). All signal types match the authoritative registry.
>
> **Counts policy.** The per-SignalKind breakdown in the *Summary* table below is authoritative; if any other doc disagrees, this file wins. Counts are re-derived from `crates/shared/src/indicators/registry.rs` and updated on every registry change.
>
> **Numbering.** File names follow `04-02-NN-kebab-case.md` where `NN` is the zero-padded registry row index (01 → 50). The **registry key** (column below) remains snake_case and matches the Rust enum variant used in code; the **filename** uses kebab-case per the doc convention.

---

## TREND (10 indicators)

| # | Filename | Key | Display Name | Class | Dir | Signals | Doc File |
|---|---|-----|-------------|-------|-----|---------|----------|
| 01 | `04-02-01-ema-stack.md` | `ema_stack` | EMA Ribbon | Lagging | Y | StackChange, Crossover×4 | [04-02-01-ema-stack.md](04-02-01-ema-stack.md) |
| 02 | `04-02-02-supertrend.md` | `supertrend` | Supertrend | Lagging | Y | TrendFlip, Crossover×2, BandTouch×2 | [04-02-02-supertrend.md](04-02-02-supertrend.md) |
| 03 | `04-02-03-donchian.md` | `donchian` | Donchian | Lagging | Y | Breakout×2, BandTouch×2, LevelTest×2 | [04-02-03-donchian.md](04-02-03-donchian.md) |
| 04 | `04-02-04-keltner.md` | `keltner` | Keltner | Lagging | Y | Breakout×2, BandTouch×2, LevelTest×2 | [04-02-04-keltner.md](04-02-04-keltner.md) |
| 05 | `04-02-05-adx.md` | `adx` | ADX | Lagging | Y | TrendFlip, Threshold | [04-02-05-adx.md](04-02-05-adx.md) |
| 06 | `04-02-06-vwap.md` | `vwap` | VWAP | Lagging | Y | LevelTest | [04-02-06-vwap.md](04-02-06-vwap.md) |
| 07 | `04-02-07-anchored-vwap.md` | `anchored_vwap` | Anchored VWAP | Lagging | Y | LevelTest×2, Crossover×2 | [04-02-07-anchored-vwap.md](04-02-07-anchored-vwap.md) |
| 08 | `04-02-08-ichimoku.md` | `ichimoku` | Ichimoku Cloud | Hybrid | Y | Crossover×3, Breakout×2, LevelTest×3, TrendFlip×1 | [04-02-08-ichimoku.md](04-02-08-ichimoku.md) |
| 09 | `04-02-09-psar.md` | `psar` | Parabolic SAR | Lagging | Y | TrendFlip×2, Crossover×3 | [04-02-09-psar.md](04-02-09-psar.md) |
| 10 | `04-02-10-hull-ma.md` | `hull_ma` | Hull MA | Lagging | Y | Crossover×2 | [04-02-10-hull-ma.md](04-02-10-hull-ma.md) |

---

## MOMENTUM (7 indicators)

| # | Filename | Key | Display Name | Class | Dir | Div | Signals | Doc File |
|---|---|-----|-------------|-------|-----|-----|---------|----------|
| 11 | `04-02-11-rsi.md` | `rsi` | RSI | Leading | Y | Y | ZeroLineCross, Divergence, Threshold×5 | [04-02-11-rsi.md](04-02-11-rsi.md) |
| 12 | `04-02-12-stochastic.md` | `stochastic` | Stochastic | Leading | Y | Y | Crossover×2, Divergence, Threshold×4 | [04-02-12-stochastic.md](04-02-12-stochastic.md) |
| 13 | `04-02-13-chandemo.md` | `chandemo` | Chande MO | Leading | Y | Y | ZeroLineCross, Divergence, Threshold×4 | [04-02-13-chandemo.md](04-02-13-chandemo.md) |
| 14 | `04-02-14-williams-r.md` | `williams_r` | Williams %R | Leading | Y | — | Threshold, ZeroLineCross | [04-02-14-williams-r.md](04-02-14-williams-r.md) |
| 15 | `04-02-15-awesome-oscillator.md` | `awesome_oscillator` | AO | Leading | Y | — | ZeroLineCross×2, Threshold×2 | [04-02-15-awesome-oscillator.md](04-02-15-awesome-oscillator.md) |
| 16 | `04-02-16-cci.md` | `cci` | CCI | Leading | Y | — | Threshold×4, ZeroLineCross | [04-02-16-cci.md](04-02-16-cci.md) |
| 17 | `04-02-17-macd.md` | `macd` | MACD | Lagging | Y | Y | Crossover×2, ZeroLineCross, Divergence, Threshold | [04-02-17-macd.md](04-02-17-macd.md) |

---

## VOLUME (7 indicators)

| # | Filename | Key | Display Name | Class | Dir | Div | Signals | Doc File |
|---|---|-----|-------------|-------|-----|-----|---------|----------|
| 18 | `04-02-18-volume.md` | `volume` | Volume | Hybrid | N (Gate) | — | VolumeClimax | [04-02-18-volume.md](04-02-18-volume.md) |
| 19 | `04-02-19-rvol.md` | `rvol` | RVOL | Hybrid | N (Gate) | — | VolumeClimax | [04-02-19-rvol.md](04-02-19-rvol.md) |
| 20 | `04-02-20-volume-profile.md` | `volume_profile` | Volume Profile | Hybrid | Y | — | Breakout×2, LevelTest×2 | [04-02-20-volume-profile.md](04-02-20-volume-profile.md) |
| 21 | `04-02-21-obv.md` | `obv` | OBV | Lagging | Y | Y | TrendFlip×2, Divergence×2, Threshold×3 | [04-02-21-obv.md](04-02-21-obv.md) |
| 22 | `04-02-22-cmf.md` | `cmf` | Chaikin MF | Hybrid | Y | Y | ZeroLineCross×2, Divergence×2, Threshold×4 | [04-02-22-cmf.md](04-02-22-cmf.md) |
| 23 | `04-02-23-mfi.md` | `mfi` | Money Flow Idx | Hybrid | Y | Y | Threshold×4, Divergence×2 | [04-02-23-mfi.md](04-02-23-mfi.md) |
| 24 | `04-02-24-force-index.md` | `force_index` | Force Idx | Hybrid | Y | — | ZeroLineCross, Threshold | [04-02-24-force-index.md](04-02-24-force-index.md) |

---

## VOLATILITY (6 indicators)

| # | Filename | Key | Display Name | Class | Dir | Div | Signals | Doc File |
|---|---|-----|-------------|-------|-----|-----|---------|----------|
| 25 | `04-02-25-atr.md` | `atr` | ATR | Lagging | N (Gate) | — | Threshold, VolatilityCycle | [04-02-25-atr.md](04-02-25-atr.md) |
| 26 | `04-02-26-bollinger.md` | `bollinger` | Bollinger | Hybrid | Y | — | Breakout×2, BandTouch×2, LevelTest×3 | [04-02-26-bollinger.md](04-02-26-bollinger.md) |
| 27 | `04-02-27-bbwp.md` | `bbwp` | BBWP | Leading | N (Gate) | — | VolatilityCycle, Threshold | [04-02-27-bbwp.md](04-02-27-bbwp.md) |
| 28 | `04-02-28-squeeze.md` | `squeeze` | TTM Squeeze | Hybrid | Y | Y | VolatilityCycle×3, Divergence, Threshold×3 | [04-02-28-squeeze.md](04-02-28-squeeze.md) |
| 29 | `04-02-29-hv.md` | `hv` | Hist. Volatility | Lagging | N (Gate) | — | Threshold | [04-02-29-hv.md](04-02-29-hv.md) |
| 30 | `04-02-30-stddev-channel.md` | `stddev_channel` | StdDev Chnl | Hybrid | Y | — | Breakout×2, BandTouch×2, LevelTest | [04-02-30-stddev-channel.md](04-02-30-stddev-channel.md) |

---

## STRUCTURE (5 indicators)

| # | Filename | Key | Display Name | Class | Dir | Signals | Doc File |
|---|---|-----|-------------|-------|-----|---------|----------|
| 31 | `04-02-31-fibonacci.md` | `fibonacci` | Fibonacci | Leading | Y | LevelTest | [04-02-31-fibonacci.md](04-02-31-fibonacci.md) |
| 32 | `04-02-32-support-resistance.md` | `support_resistance` | Support/Resistance | Leading | Y | LevelTest×2, Breakout×2 | [04-02-32-support-resistance.md](04-02-32-support-resistance.md) |
| 33 | `04-02-33-pivot-points.md` | `pivot_points` | Pivot Points | Leading | Y | LevelTest×3, Breakout×2, Crossover×2 | [04-02-33-pivot-points.md](04-02-33-pivot-points.md) |
| 34 | `04-02-34-patterns.md` | `patterns` | Patterns | Leading | Y | PatternForming×3 | [04-02-34-patterns.md](04-02-34-patterns.md) |
| 35 | `04-02-35-candlestick.md` | `candlestick` | Candlestick | Leading | Y | PatternForming×2 | [04-02-35-candlestick.md](04-02-35-candlestick.md) |

---

## REGIME (4 indicators)

| # | Filename | Key | Display Name | Class | Dir | Signals | Doc File |
|---|---|-----|-------------|-------|-----|---------|----------|
| 36 | `04-02-36-aroon.md` | `aroon` | Aroon | Hybrid | Y | TrendFlip×2, Threshold×2 | [04-02-36-aroon.md](04-02-36-aroon.md) |
| 37 | `04-02-37-choppiness.md` | `choppiness` | Choppiness | Hybrid | N (Gate) | Threshold×2, VolatilityCycle | [04-02-37-choppiness.md](04-02-37-choppiness.md) |
| 38 | `04-02-38-linreg-slope.md` | `linreg_slope` | LinReg Slope | Lagging | Y | ZeroLineCross, Threshold×2 | [04-02-38-linreg-slope.md](04-02-38-linreg-slope.md) |
| 39 | `04-02-39-zscore.md` | `zscore` | Z-Score | Leading | Y | Threshold×2, ZeroLineCross | [04-02-39-zscore.md](04-02-39-zscore.md) |

---

## INSTITUTIONAL (4 indicators)

| # | Filename | Key | Display Name | Class | Dir | Signals | Doc File |
|---|---|-----|-------------|-------|-----|---------|----------|
| 40 | `04-02-40-smc-structure.md` | `smc_structure` | SMC Structure | Leading | Y | Breakout, TrendFlip | [04-02-40-smc-structure.md](04-02-40-smc-structure.md) |
| 41 | `04-02-41-smc-liquidity.md` | `smc_liquidity` | SMC Liquidity | Leading | Y | PatternForming | [04-02-41-smc-liquidity.md](04-02-41-smc-liquidity.md) |
| 42 | `04-02-42-smc-fvg.md` | `smc_fvg` | SMC Fair Value Gap | Leading | Y | LevelTest | [04-02-42-smc-fvg.md](04-02-42-smc-fvg.md) |
| 43 | `04-02-43-smc-order-blocks.md` | `smc_order_blocks` | SMC Order Blocks | Leading | Y | LevelTest×2, TrendFlip×2 | [04-02-43-smc-order-blocks.md](04-02-43-smc-order-blocks.md) |

---

## DERIVATIVES DATA (7 indicators)

| # | Filename | Key | Display Name | Class | Dir | Signals | Doc File |
|---|---|-----|-------------|-------|-----|---------|----------|
| 44 | `04-02-44-open-interest.md` | `open_interest` | Open Interest | Leading | N (Gate) | Threshold | [04-02-44-open-interest.md](04-02-44-open-interest.md) |
| 45 | `04-02-45-oi-delta.md` | `oi_delta` | OI Delta | Leading | Y | Threshold, ZeroLineCross | [04-02-45-oi-delta.md](04-02-45-oi-delta.md) |
| 46 | `04-02-46-funding-rate.md` | `funding_rate` | Funding Rate | Leading | N (Gate) | Threshold | [04-02-46-funding-rate.md](04-02-46-funding-rate.md) |
| 47 | `04-02-47-oi-price-divergence.md` | `oi_price_divergence` | OI-Price Divergence | Leading | Y | Divergence | [04-02-47-oi-price-divergence.md](04-02-47-oi-price-divergence.md) |
| 48 | `04-02-48-order-flow-imbalance.md` | `order_flow_imbalance` | Order Flow Imbalance | Leading | Y | Threshold | [04-02-48-order-flow-imbalance.md](04-02-48-order-flow-imbalance.md) |
| 49 | `04-02-49-spread.md` | `spread` | Spread | Leading | N (Gate) | Threshold | [04-02-49-spread.md](04-02-49-spread.md) |
| 50 | `04-02-50-depth-bias.md` | `depth_bias` | Depth Bias | Leading | Y | Threshold | [04-02-50-depth-bias.md](04-02-50-depth-bias.md) |

> **Note on `oi_price_divergence`:** Unlike the eight `supports_divergence` oscillators (whose divergences are nested `Divergence` signals on the parent key), `oi_price_divergence` is a **standalone registry entry** with its own JSON key — it compares open interest against price rather than an oscillator against price.

---

## Summary

| Metric | Count |
|--------|-------|
| Authoritative Registry Entries | 50 (10 Trend + 7 Momentum + 7 Volume + 6 Volatility + 5 Structure + 4 Regime + 4 Institutional + 7 Derivatives) |
| Files in this directory | 51 (50 entries + this master index) |
| Directional (scoring contributors) | 41 |
| Non-Directional Gates | 9 (Volume, RVOL, ATR, BBWP, HV, Choppiness, Funding Rate, Spread, Open Interest) |
| Divergence-Bearing Indicators (`supports_divergence: true`) | 8 (RSI, MACD, Stochastic, ChandeMO, OBV, CMF, MFI, Squeeze) |
| Standalone Divergence Indicators | 1 (`oi_price_divergence` — own registry entry & JSON key) |
| Total Signal-Kind × Indicator Declarations | **101** (one per `(indicator, SignalKind)` pair; `×N` counts multiplicity *within* a single declaration, e.g. 5 RSI threshold zones). Per-SignalKind breakdown: Divergence 9, Crossover 10, Threshold 26, Breakout 9, BandTouch 5, ZeroLineCross 11, VolatilityCycle 4, LevelTest 13, TrendFlip 8, VolumeClimax 2, StackChange 1, PatternForming 3 (registry-verified; sums to 101). *(A previous version listed **102** with ZeroLineCross=12; the overcount came from attributing ZeroLineCross to `stochastic` and `mfi`, whose indicator docs do not actually declare ZeroLineCross signals in their Signals tables — see [04-02-12-stochastic.md](../engines/market-monitoring-engine/indicators/04-02-12-stochastic.md) and [04-02-23-mfi.md](../engines/market-monitoring-engine/indicators/04-02-23-mfi.md).)* |
| SignalKind Types | 12 (Divergence, Crossover, Threshold, Breakout, BandTouch, ZeroLineCross, VolatilityCycle, LevelTest, TrendFlip, VolumeClimax, StackChange, PatternForming) |

Divergence companions do **not** appear as separate rows or JSON keys — a divergence is an `IndicatorSignal { kind: Divergence, ... }` in the parent indicator's `signals` array.

---

## Cross-References

- [Metrics Matrix](../../../matrices/02-07-metrics-matrix.md) — Unified single-timeframe observation schema (indicator + signal telemetry objects)
- [MME Layer 1 — Metrics](../../../engines/market-monitoring-engine/03-02-02-mme-layer1-metrics.md) — Indicator computation and 12-axis projection specification
- [MME Indicators Guide](../../../engines/market-monitoring-engine/03-02-09-mme-indicators-guide.md) — Condensed signal threshold reference (readable rulebook)
- [MME Signals Guide](../../../engines/market-monitoring-engine/03-02-10-mme-signals-guide.md) — Signal detection rulebook, indexed by SignalKind
- [Signal Specifications](../signals/05-02-00-signals-index.md) — Per-SignalKind detailed specifications (sibling index)
- [Ontology](../../../conceptual-foundations/01-01-ontology.md) — Formal terminology, acronyms, and evaluation-axis definitions
