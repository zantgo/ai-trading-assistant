# Z-Score (20)
**Group:** Regime

## 1. Introduction — Trading Function
The Z-Score measures how many standard deviations the current price is from its rolling mean. It is a mean-reversion indicator: positive Z-scores indicate the price is statistically stretched above its mean (overextended, expect reversion lower); negative Z-scores indicate price is stretched below its mean (overextended, expect reversion higher). The Z-Score is directionally inverted — high positive Z = bearish, low negative Z = bullish — because the trading thesis is reversion toward the mean.

## 2. Mathematical Formula
```
mean = SMA(close, period)
σ = standard_deviation(close, period)
Z = (close - mean) / σ
```

## 3. Normalization
The normalized score in [-1, 1] is an inverted linear mapping:
```
norm = clamp(-Z / 3.0)
```
The inversion means a high positive Z-score (price far above mean) produces a negative normalized value (bearish mean-reversion expectation). Labels: `ZSCORE_OVEREXTENDED_HIGH`, `ZSCORE_OVEREXTENDED_LOW`, `ZSCORE_ABOVE_MEAN`, `ZSCORE_BELOW_MEAN`, `ZSCORE_AT_MEAN` (|Z| < 0.5).

## 4. Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Threshold | ZSCORE_OVEREXTENDED_HIGH | |Z| ≥ 2 (price extremely above mean) | Bearish (mean reversion) |
| Threshold | ZSCORE_OVEREXTENDED_LOW | |Z| ≥ 2 (price extremely below mean) | Bullish (mean reversion) |
| ZeroLineCross | Z-Score zero cross | Z crosses 0 (reverts past the mean). Transition-only via prev-bar Z comparison in engine. | Bullish / Bearish |

## 5. Configuration
```toml
[indicators]
zscore_period = 20
```
