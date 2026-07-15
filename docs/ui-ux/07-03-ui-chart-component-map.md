# UI Chart Component Map

**Version:** 1.0
**Status:** Approved
**Purpose:** Per-indicator mapping from registry key to frontend rendering location. Companion to [UI Overview](07-01-ui-overview-spec.md) and [Dashboard Layout](07-02-ui-dashboard-layout.md).

The platform has **50 registered indicators** but **20 dedicated chart components** (or markers rendering on the main `PriceChart`) in `crates/frontend/src/lib/components/`. The remaining indicators are rendered either as price-chart overlays on the main `PriceChart` component or via the indicator-pane reuse pattern (shared "Oscillator" / "Derivatives" pane). This file enumerates every indicator's rendering destination so the mapping is not implicit. The §2 "Dedicated Pane Components" table lists 20 entries — the 18 single-purpose chart components plus the `patterns` and `candlestick` entries, which are marker overlays rendered directly on `PriceChart`.

---

## 1. Price-Chart Overlays (rendered on `PriceChart`)

The following indicators are drawn directly on the main price chart (OHLCV + overlays):

| Registry Key | Display Name | Overlay Form |
|---|---|---|
| `ema_stack` | EMA Ribbon | 4 line series (fast / medium / slow / long) |
| `donchian` | Donchian | Upper & lower bands as line series |
| `keltner` | Keltner | Upper, middle, lower channels as line series |
| `vwap` | VWAP | Single line series |
| `anchored_vwap` | Anchored VWAP | Single line series, anchor-point selectable |
| `supertrend` | Supertrend | Line + direction-flip markers |
| `ichimoku` | Ichimoku Cloud | Tenkan, Kijun, Senkou A, Senkou B, Chikou as 5 line series |
| `psar` | Parabolic SAR | Dotted markers overlaid on price |
| `hull_ma` | Hull MA | Single line series |
| `bollinger` | Bollinger Bands | Upper, middle, lower bands |
| `stddev_channel` | StdDev Channel | ±Nσ envelope |
| `support_resistance` | Support/Resistance | Horizontal level lines |
| `fibonacci` | Fibonacci | Retracement/extension level lines |
| `pivot_points` | Pivot Points | Classic pivot level lines |
| `smc_structure` | SMC Structure | CHoCH / BOS markers on price |
| `smc_liquidity` | SMC Liquidity | Liquidity-pool markers |
| `smc_fvg` | SMC Fair Value Gap | FVG zone shading |
| `smc_order_blocks` | SMC Order Blocks | Order-block zone shading |

---

## 2. Dedicated Pane Components

The following indicators have their own dedicated chart component:

| Registry Key | Display Name | Component | Notes |
|---|---|---|---|
| `rsi` | RSI | `RsiChart` | OB/OS lines at 70 / 30 (or 80 / 20 in strong trend). |
| `macd` | MACD | `MacdChart` | Line / signal / histogram. |
| `adx` | ADX | `AdxChart` | ADX + DI+ / DI−. |
| `atr` | ATR | `AtrChart` | Single line series. |
| `squeeze` | TTM Squeeze | `SqueezeChart` | Momentum histogram with squeeze dots. |
| `bbwp` | BBWP | `BbwpChart` | BB Width Percentile. |
| `volume` | Volume | `VolumeChart` | Volume bars + rolling average line. |
| `rvol` | Relative Volume | `RvolChart` | RVOL line with institutional / climax bands. |
| `stochastic` | Stochastic | `StochasticChart` | %K / %D with OB/OS lines. |
| `chandemo` | Chande MO | `ChandeMoChart` | Line with zero line. |
| `obv` | On-Balance Volume | `ObvChart` | Cumulative line. |
| `cmf` | Chaikin MF | `CmfChart` | Line with zero line. |
| `mfi` | Money Flow Index | `MfiChart` | Line with OB/OS lines. |
| `hv` | Historical Volatility | `HvChart` | Line series. |
| `aroon` | Aroon | `AroonChart` | Aroon Up / Aroon Down. |
| `choppiness` | Choppiness Index | `ChoppinessChart` | Line with chop/trend bands. |
| `linreg_slope` | LinReg Slope | `LinRegSlopeChart` | Line with zero line. |
| `zscore` | Z-Score | `ZScoreChart` | Line with zero line. |
| `patterns` | Chart Patterns | markers on `PriceChart` | Pattern detection markers. |
| `candlestick` | Candlestick Patterns | markers on `PriceChart` | Pattern detection markers. |

---

## 3. Reused / Generic Panes (rendered in shared panes)

The following indicators share existing chart components rather than having dedicated ones. Their data is rendered in a generic oscillator or context pane.

| Registry Key | Display Name | Rendered In |
|---|---|---|
| `awesome_oscillator` | AO | generic "Oscillator" pane |
| `williams_r` | Williams %R | generic "Oscillator" pane |
| `cci` | CCI | generic "Oscillator" pane |
| `force_index` | Force Index | generic "Oscillator" pane |
| `volume_profile` | Volume Profile | price overlay histogram on `PriceChart` |
| `open_interest` | Open Interest | generic "Derivatives" pane |
| `oi_delta` | OI Delta | generic "Derivatives" pane |
| `funding_rate` | Funding Rate | generic "Derivatives" pane |
| `oi_price_divergence` | OI-Price Divergence | divergence marker on `PriceChart` |
| `order_flow_imbalance` | Order Flow Imbalance | generic "Derivatives" pane |
| `spread` | Spread | generic "Derivatives" pane |
| `depth_bias` | Depth Bias | generic "Derivatives" pane |

---

## 4. Aggregate Counts

| Render Bucket | Indicator Count |
|---|---|
| Price-chart overlays | 18 |
| Dedicated panes | 20 (incl. `patterns` and `candlestick` markers on PriceChart) |
| Reused / generic panes | 12 |
| **Total** | **50** |

---

## 5. Cross-References

- [UI Overview](07-01-ui-overview-spec.md) — Chart architecture (§6) and the `ChartToggles` enable/disable model.
- [Dashboard Layout](07-02-ui-dashboard-layout.md) — Panel placement.
- [Indicator Index](../engines/market-monitoring-engine/indicators/04-02-00-indicator-index.md) — Authoritative registry.
