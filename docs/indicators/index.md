# Indicator Documentation Index

> Complete index of all 51 registry entries across 7 functional groups. Each indicator's signal types match the authoritative registry in `crates/shared/src/indicators/registry.rs`.

---

## TREND (10 indicators)

| # | Key | Display Name | Class | Dir | Signals | Doc File |
|---|-----|-------------|-------|-----|---------|----------|
| 1 | `ema_stack` | EMA Ribbon | Lagging | Y | StackChange, Crossover×4 | [ema.md](ema.md) |
| 2 | `supertrend` | Supertrend | Lagging | Y | TrendFlip, Crossover×2, BandTouch | [supertrend.md](supertrend.md) |
| 3 | `donchian` | Donchian | Lagging | Y | Breakout×2, BandTouch×2, LevelTest×2 | [donchian.md](donchian.md) |
| 4 | `keltner` | Keltner | Lagging | Y | Breakout×2, BandTouch×2, LevelTest×2 | [keltner.md](keltner.md) |
| 5 | `adx` | ADX | Lagging | N (Gate) | TrendFlip, Threshold | [adx.md](adx.md) |
| 6 | `vwap` | VWAP | Lagging | Y | LevelTest | [vwap.md](vwap.md) |
| 7 | `anchored_vwap` | Anchored VWAP | Lagging | Y | LevelTest×2, Crossover×2 | [anchored_vwap.md](anchored_vwap.md) |
| 8 | `ichimoku` | Ichimoku Cloud | Hybrid | Y | Crossover×2, Breakout×2, LevelTest×3, TrendFlip×2 | [ichimoku.md](ichimoku.md) |
| 9 | `psar` | Parabolic SAR | Lagging | Y | TrendFlip×2, Crossover×3 | [psar.md](psar.md) |
| 10 | `hull_ma` | Hull MA | Lagging | Y | Crossover×2 | [hull_ma.md](hull_ma.md) |

---

## MOMENTUM (11 indicators)

| # | Key | Display Name | Class | Dir | Div | Signals | Doc File |
|---|-----|-------------|-------|-----|-----|---------|----------|
| 11 | `rsi` | RSI | Leading | Y | Y | ZeroLineCross, Divergence, Threshold×5 | [rsi.md](rsi.md) |
| 12 | `rsi_divergence` | RSI Divergence | Leading | Y | Y | Divergence | (covered in rsi.md) |
| 13 | `stochastic` | Stochastic | Leading | Y | Y | Crossover×2, ZeroLineCross×2, Divergence, Threshold×4 | [stochastic.md](stochastic.md) |
| 14 | `stochastic_divergence` | Stoch Divergence | Leading | Y | Y | Divergence | (covered in stochastic.md) |
| 15 | `chandemo` | Chande MO | Leading | Y | Y | ZeroLineCross, Divergence, Threshold×4 | [chandemo.md](chandemo.md) |
| 16 | `chandemo_divergence` | CMO Divergence | Leading | Y | Y | Divergence | (covered in chandemo.md) |
| 17 | `williams_r` | Williams %R | Leading | Y | — | Threshold, ZeroLineCross | [williams_r.md](williams_r.md) |
| 18 | `awesome_oscillator` | AO | Leading | Y | — | ZeroLineCross×2, Threshold×2 | [awesome_oscillator.md](awesome_oscillator.md) |
| 19 | `cci` | CCI | Leading | Y | — | Threshold×4, ZeroLineCross | [cci.md](cci.md) |
| 20 | `macd` | MACD | Lagging | Y | Y | Crossover×2, ZeroLineCross, Divergence, Threshold | [macd.md](macd.md) |
| 21 | `macd_divergence` | MACD Divergence | Lagging | Y | Y | Divergence | (covered in macd.md) |

---

## VOLUME (10 indicators)

| # | Key | Display Name | Class | Dir | Div | Signals | Doc File |
|---|-----|-------------|-------|-----|-----|---------|----------|
| 22 | `volume` | Volume | Hybrid | N (Gate) | — | VolumeClimax | [volume.md](volume.md) |
| 23 | `rvol` | RVOL | Hybrid | N (Gate) | — | VolumeClimax | [rvol.md](rvol.md) |
| 24 | `volume_profile` | Volume Profile | Hybrid | Y | — | Breakout×2, LevelTest×2 | [volume_profile.md](volume_profile.md) |
| 25 | `obv` | OBV | Lagging | Y | Y | TrendFlip×2, Divergence×2, Threshold×3 | [obv.md](obv.md) |
| 26 | `obv_divergence` | OBV Divergence | Lagging | Y | Y | Divergence | (covered in obv.md) |
| 27 | `cmf` | Chaikin MF | Hybrid | Y | Y | ZeroLineCross×2, Divergence×2, Threshold×4 | [cmf.md](cmf.md) |
| 28 | `cmf_divergence` | CMF Divergence | Hybrid | Y | Y | Divergence | (covered in cmf.md) |
| 29 | `mfi` | Money Flow Idx | Hybrid | Y | Y | Threshold×4, Divergence×2 | [mfi.md](mfi.md) |
| 30 | `mfi_divergence` | MFI Divergence | Hybrid | Y | Y | Divergence | (covered in mfi.md) |
| 31 | `force_index` | Force Idx | Hybrid | Y | — | ZeroLineCross, Threshold | [force_index.md](force_index.md) |

---

## VOLATILITY (7 indicators)

| # | Key | Display Name | Class | Dir | Div | Signals | Doc File |
|---|-----|-------------|-------|-----|-----|---------|----------|
| 32 | `atr` | ATR | Lagging | N (Gate) | — | Threshold, CompressionRelease | [atr.md](atr.md) |
| 33 | `bollinger` | Bollinger | Hybrid | Y | — | Breakout×2, BandTouch×2, LevelTest×3 | [bollinger.md](bollinger.md) |
| 34 | `bbwp` | BBWP | Leading | N (Gate) | — | CompressionRelease, Threshold | [bbwp.md](bbwp.md) |
| 35 | `squeeze` | TTM Squeeze | Hybrid | Y | Y | CompressionRelease×3, Divergence, Threshold | [squeeze_momentum.md](squeeze_momentum.md) |
| 36 | `squeeze_divergence` | Squeeze Divergence | Hybrid | Y | Y | Divergence | (covered in squeeze_momentum.md) |
| 37 | `hv` | Hist. Volatility | Lagging | N (Gate) | — | Threshold | [hv.md](hv.md) |
| 38 | `stddev_channel` | StdDev Chnl | Hybrid | Y | — | Breakout×2, BandTouch×2, LevelTest | [stddev_channel.md](stddev_channel.md) |

---

## STRUCTURE (5 indicators)

| # | Key | Display Name | Class | Dir | Signals | Doc File |
|---|-----|-------------|-------|-----|---------|----------|
| 39 | `fibonacci` | Fibonacci | Leading | Y | LevelTest | [fibonacci.md](fibonacci.md) |
| 40 | `support_resistance` | Support/Resistance | Leading | Y | LevelTest×2, Breakout×2 | [support_resistance.md](support_resistance.md) |
| 41 | `pivot_points` | Pivot Points | Leading | Y | LevelTest×3, Breakout×2, Crossover×2 | [pivot_points.md](pivot_points.md) |
| 42 | `patterns` | Patterns | Leading | Y | PatternForming×3 | [chart_patterns.md](chart_patterns.md) |
| 43 | `candlestick` | Candlestick | Leading | Y | PatternForming×2 | [candlestick.md](candlestick.md) |

---

## REGIME (4 indicators)

| # | Key | Display Name | Class | Dir | Signals | Doc File |
|---|-----|-------------|-------|-----|---------|----------|
| 44 | `aroon` | Aroon | Hybrid | Y | Crossover×2, TrendFlip×2, Threshold×2 | [aroon.md](aroon.md) |
| 45 | `choppiness` | Choppiness | Hybrid | N (Gate) | Threshold×2, CompressionRelease | [choppiness.md](choppiness.md) |
| 46 | `linreg_slope` | LinReg Slope | Lagging | Y | ZeroLineCross, Threshold×2 | [linreg_slope.md](linreg_slope.md) |
| 47 | `zscore` | Z-Score | Leading | Y | Threshold, ZeroLineCross | [zscore.md](zscore.md) |

---

## INSTITUTIONAL (4 indicators)

| # | Key | Display Name | Class | Dir | Signals | Doc File |
|---|-----|-------------|-------|-----|---------|----------|
| 48 | `smc_structure` | SMC Structure | Leading | Y | Breakout, TrendFlip | [smc_structure.md](smc_structure.md) |
| 49 | `smc_liquidity` | SMC Liquidity | Leading | Y | PatternForming | [smc_liquidity.md](smc_liquidity.md) |
| 50 | `smc_fvg` | SMC Fair Value Gap | Leading | Y | LevelTest | [smc_fvg.md](smc_fvg.md) |
| 51 | `smc_order_blocks` | SMC Order Blocks | Leading | Y | LevelTest×2, TrendFlip×2 | [smc_order_blocks.md](smc_order_blocks.md) |

---

## Summary

| Metric | Count |
|--------|-------|
| Total Registry Entries | 51 |
| Directional (scoring contributors) | 44 |
| Non-Directional Gates | 7 (ADX, Volume, RVOL, ATR, BBWP, HV, Choppiness) |
| Divergence-Bearing Indicators | 8 (RSI, MACD, Stochastic, ChandeMO, OBV, CMF, MFI, Squeeze) |
| Divergence Scored Keys | 8 (`*_divergence` entries) |
| Total Signal Emission Sites | 115 |
| SignalKind Types | 12 |

---

## Cross-References

- [ITIL — Institutional Technical Indicator Layer](../layers/01-itil-technical-indicator.md) — Full layer specification with signal lifecycle and group aggregation
- [Indicators Guide (AI Rulebook)](../indicators-guide.md) — Condensed signal threshold reference for LLM agents
- [Indicator System Master Spec](../indicator-system-master-spec.md) — Registry manifest design and phase checklist
- [Glossary](../glossary.md) — All indicator acronyms and signal kind definitions
